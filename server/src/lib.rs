// use anyhow::Result;
use clap::Parser;
use jsonrpsee::{
    core::RpcResult,
    server::{RpcModule, Server},
    types::ErrorObject,
};
use std::path::PathBuf;
use tracing::{event, Level};
use wallet::XWallet;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value_t = 10001)]
    port: u16,
    #[arg(long, help = "default is 127.0.0.1")]
    ip: Option<std::net::IpAddr>,
    #[arg(long, help = "Path to the mnemonic file", value_name = "FILE")]
    mnemonic: Option<PathBuf>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let file_path = std::path::Path::new("./xdagj_wallet/xdagj_wallet.bin");
    if !file_path.exists() && cli.mnemonic.is_none() {
        return Err(anyhow::anyhow!(
            "Wallet file not found and no mnemonic file provided"
        ));
    }

    if !file_path.exists() {
        if let Some(mnemonic_path) = cli.mnemonic {
            // Handle mnemonic file
            let mnemonic = std::fs::read_to_string(mnemonic_path)?;
            // Process mnemonic to create wallet
            // ...
            println!();
            println!("please set wallet password (at least 6 characters):");
            let pswd = rpassword::read_password().unwrap();
            if pswd.len() < 6 {
                return Err(anyhow::anyhow!("password too short."));
            }
            println!("please reenter wallet password :");
            let pswd2 = rpassword::read_password().unwrap();
            if pswd == pswd2 {
                let mut wallet = XWallet::new();
                wallet.password = pswd;
                wallet.name = None;
                wallet.import_from_mnemonic(&mnemonic)?;
                println!("import wallet success.");
                event!(Level::INFO, "import wallet from mnemonic success.")
            } else {
                return Err(anyhow::anyhow!("passwords not match!"));
            }
        } else {
            return Err(anyhow::anyhow!(
                "Wallet file not found and no mnemonic file provided"
            ));
        }
    }

    let mut addr: String;
    if let Some(ip) = cli.ip {
        addr = ip.to_string();
    } else {
        addr = "127.0.0.1".to_string();
    }
    addr = addr + ":" + &cli.port.to_string();

    let server = Server::builder()
        .build(addr.parse::<std::net::SocketAddr>()?)
        .await?;
    let mut module = RpcModule::new(());
    module.register_method::<RpcResult<&str>, _>("Xdag.Unlock", |params, _, _| {
        match params.one::<String>() {
            Ok(pswd) => {
                if pswd.len() < 6 {
                    return Err(ErrorObject::owned(
                        -32001,
                        "wrong password",
                        Some("too short"),
                    ));
                }
                let mut wallet = XWallet::new();
                match wallet.unlock(&pswd, None) {
                    Ok(()) => Ok("success"),
                    Err(e) => Err(ErrorObject::owned(
                        -32001,
                        "wrong password",
                        Some(e.root_cause().to_string()),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    })?;

    let addr = server.local_addr()?;
    println!(
        "XDAG Plus Server listening on {:?}:{:?}",
        addr.ip(),
        addr.port()
    );
    event!(
        Level::INFO,
        "XDAG Plus Server listening on {:?}:{:?}",
        addr.ip(),
        addr.port()
    );
    let handle = server.start(module);

    handle.stopped().await;
    Ok(())
}
