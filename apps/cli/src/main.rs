use anyhow::{Result, bail};
use std::{env, println};
use tidbit_filesystem_core::scan_directory;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();

    if command != "scan" {
        bail!("Usage: tidbit scan <directory>");
    }

    let path = args.next().unwrap_or_else(|| ".".to_string());
    println!("Scanning directory: {}", path);

    let report = scan_directory(path, 10)?;

    let json = serde_json::to_string_pretty(&report)?;

    println!("{}", json);

    Ok(())
}
