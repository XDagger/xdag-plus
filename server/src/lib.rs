// use anyhow::Result;
use clap::Parser;
use jsonrpsee::{
    core::RpcResult,
    server::{RpcModule, Server},
    types::ErrorObject,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;
use tokio::runtime::Runtime;
use tracing::{event, Level};
use wallet::XWallet;

const PASSWORD_LENGTH: usize = 8;

lazy_static! {
    static ref GLOBAL_WALLET: RwLock<XWallet> = RwLock::new(XWallet::new());
}

lazy_static! {
    static ref IS_TEST_NET: RwLock<bool> = RwLock::new(false);
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 10001)]
    port: u16,
    #[arg(short, long, help = "default is 127.0.0.1")]
    ip: Option<std::net::IpAddr>,
    #[arg(short, long, help = "Path to the mnemonic file", value_name = "FILE")]
    mnemonic: Option<PathBuf>,
    #[arg(short, long, action)]
    test_net: bool,
}

#[derive(Deserialize, Clone)]
struct SendRequest {
    #[serde(default)]
    amount: String,
    #[serde(default)]
    address: String,
    #[serde(default)]
    remark: String,
}

#[derive(Serialize, Clone)]
struct SendResult {
    #[serde(default, rename = "Status")]
    status: String,
    #[serde(default, rename = "TxHash")]
    tx_hash: String,
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

    {
        let mut is_test_net = IS_TEST_NET.write().unwrap();
        *is_test_net = cli.test_net;
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

    module.register_method::<RpcResult<String>, _>("Xdag.Account", |_, _, _| {
        let wallet = GLOBAL_WALLET.read().unwrap();
        if wallet.password.is_empty() {
            return Err(ErrorObject::owned(-32003, "wallet locked", Some("")));
        }
        Ok(wallet.address.clone())
    })?;

    module.register_blocking_method::<RpcResult<String>, _>("Xdag.Balance", |params, _, _| {
        let is_test_net = IS_TEST_NET.read().unwrap();
        let wallet = GLOBAL_WALLET.read().unwrap();
        if wallet.password.is_empty() {
            return Err(ErrorObject::owned(-32003, "wallet locked", Some("")));
        }
        let address = match params.one::<String>() {
            Ok(addr) => {
                let res = bs58::decode(&addr).with_check(None).into_vec();
                if res.is_err() {
                    return Err(ErrorObject::owned(
                        -32004,
                        "invalide address characters",
                        Some(""),
                    ));
                }

                addr.clone()
            }
            Err(_) => wallet.address.clone(),
        };
        match Runtime::new()
            .unwrap()
            .block_on(rpc::get_balance(*is_test_net, &address))
        {
            Ok(balance) => Ok(balance),
            Err(e) => Err(ErrorObject::owned(
                -32004,
                "get balance failed",
                Some(e.to_string()),
            )),
        }
    })?;

    module.register_blocking_method::<RpcResult<SendResult>, _>(
        "Xdag.Transfer",
        |params, _, _| {
            let is_test_net = IS_TEST_NET.read().unwrap();
            let wallet = GLOBAL_WALLET.read().unwrap();
            if wallet.password.is_empty() {
                return Err(ErrorObject::owned(-32003, "wallet locked", Some("")));
            }
            match params.one::<SendRequest>() {
                Ok(request) => {
                    let res = bs58::decode(&request.address).with_check(None).into_vec();
                    if res.is_err() {
                        return Err(ErrorObject::owned(-32005, "address format error", Some("")));
                    }

                    let amount = request.amount.parse::<f64>().unwrap_or(0.0);
                    if amount == 0.0 {
                        return Err(ErrorObject::owned(
                            -32005,
                            "invalide transfer amount",
                            Some(""),
                        ));
                    }
                    if rpc::check_remark(&request.remark).is_err() {
                        return Err(ErrorObject::owned(-32005, "remark format error", Some("")));
                    }
                    if wallet.address == request.address {
                        return Err(ErrorObject::owned(
                            -32005,
                            "invalide transfer address",
                            Some(""),
                        ));
                    }
                    let res = Runtime::new().unwrap().block_on(rpc::send_xdag(
                        *is_test_net,
                        &wallet.mnemonic,
                        &wallet.address,
                        &request.address,
                        amount,
                        &request.remark,
                    ));

                    if let Err(e) = res {
                        return Err(ErrorObject::owned(
                            -32005,
                            "get nonce from node error",
                            Some(e.to_string()),
                        ));
                    }
                    let hash = res.unwrap();
                    Ok(SendResult {
                        status: "success".to_string(),
                        tx_hash: hash.clone(),
                    })
                }
                Err(e) => Err(e),
            }
        },
    )?;

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
