#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Result;
fn main() -> Result<()> {
    xdagplus_lib::main()
}
