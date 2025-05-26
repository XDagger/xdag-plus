// use anyhow::Result;
use clap::Parser;
use jsonrpsee::server::{RpcModule, Server};
use std::net::SocketAddr;
use tracing::{event, Level};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value_t = 10001)]
    port: u16,
    #[arg(long)]
    ip: Option<std::net::IpAddr>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut addr: String;
    if let Some(ip) = cli.ip {
        addr = ip.to_string();
    } else {
        addr = "127.0.0.1".to_string();
    }
    addr = addr + ":" + &cli.port.to_string();

    let server = Server::builder().build(addr.parse::<SocketAddr>()?).await?;
    let mut module = RpcModule::new(());
    module.register_method("say_hello", |_, _, _| "lo")?;

    let addr = server.local_addr()?;
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
