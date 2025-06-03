// use anyhow::Result;
use clap::Parser;
use jsonrpsee::{
    core::RpcResult,
    server::{RpcModule, Server},
    types::ErrorObject,
};
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::sync::RwLock;
use tracing::{event, Level};
use wallet::XWallet;

const PASSWORD_LENGTH: usize = 8;

lazy_static! {
    static ref GLOBAL_WALLET: RwLock<XWallet> = RwLock::new(XWallet::new());
}

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
            println!(
                "please set wallet password (at least {} characters):",
                PASSWORD_LENGTH
            );
            let pswd = rpassword::read_password().unwrap();
            if pswd.len() < PASSWORD_LENGTH {
                return Err(anyhow::anyhow!("password too short"));
            }
            println!("please reenter wallet password :");
            let pswd2 = rpassword::read_password().unwrap();
            if pswd == pswd2 {
                let mut wallet = GLOBAL_WALLET.write().unwrap();
                wallet.password = pswd;
                wallet.name = None;
                wallet.import_from_mnemonic(&mnemonic)?;
                println!("import wallet success.");
                event!(Level::INFO, "import wallet from mnemonic success")
            } else {
                return Err(anyhow::anyhow!("passwords not match"));
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
                if pswd.len() < PASSWORD_LENGTH {
                    return Err(ErrorObject::owned(
                        -32001,
                        "wrong password",
                        Some("too short"),
                    ));
                }

                let mut wallet = GLOBAL_WALLET.write().unwrap();
                if wallet.password == pswd {
                    return Ok("success");
                } else if !wallet.password.is_empty() {
                    return Err(ErrorObject::owned(-32001, "wrong password", Some("")));
                }
                wallet.name = None;
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

    module.register_method::<RpcResult<&str>, _>("Xdag.Lock", |params, _, _| {
        match params.one::<String>() {
            Ok(pswd) => {
                if pswd.len() < PASSWORD_LENGTH {
                    return Err(ErrorObject::owned(
                        -32001,
                        "wrong password",
                        Some("too short"),
                    ));
                }

                let mut wallet = GLOBAL_WALLET.write().unwrap();
                if wallet.password == pswd {
                    wallet.password = "".to_string();
                    Ok("success")
                } else if !wallet.password.is_empty() {
                    return Err(ErrorObject::owned(-32001, "wrong password", Some("")));
                } else {
                    return Err(ErrorObject::owned(
                        -32002,
                        "wallet already locked",
                        Some(""),
                    ));
                }
            }
            Err(e) => Err(e),
        }
    })?;

    module.register_method::<RpcResult<String>, _>("Xdag.Account", |params, _, _| {
        let wallet = GLOBAL_WALLET.read().unwrap();
        if wallet.password.is_empty() {
            return Err(ErrorObject::owned(-32003, "wallet locked", Some("")));
        }
        Ok(wallet.address.clone())
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
