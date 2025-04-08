#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Result;
use tracing::{event, Level};

fn main() -> Result<()> {
    // time format in logs
    let time_fmt = time::format_description::parse(
        "[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second]",
    )
    .unwrap();

    let time_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(time_offset, time_fmt);

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("xdag_plus.log")
        .unwrap();
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    tracing_subscriber::fmt::fmt()
        // .json()
        .with_timer(timer)
        .with_writer(non_blocking)
        .with_ansi(false) // without colors
        .init();

    if let Err(e) = xdagplus_lib::main() {
        event!(Level::ERROR, "main error: {:?}", e.root_cause().to_string());
        Err(e)
    } else {
        Ok(())
    }
}
