//! `secrets-anything check [--name <name>]`: Find leaked secrets in text from stdin
//!
//! Yours: anything-anything never overwrites this file.

use super::Output;
use super::args::CheckArgs;
use crate::engine;
use anyhow::{Context, Result, bail};
use std::io::{IsTerminal, Read};

/// True when running as `secrets-anything mcp`: stdin is then the protocol, not text to check.
fn in_mcp_server() -> bool {
    std::env::args().skip(1).find(|a| !a.starts_with('-')).is_some_and(|a| a == "mcp")
}

pub fn run(args: &CheckArgs) -> Result<Output> {
    if in_mcp_server() {
        bail!("check reads text from stdin, which is the MCP channel here: use secrets_scan on a file instead");
    }
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        crate::ui::step("reading text from stdin (end with Ctrl-D, or Ctrl-Z then Enter on Windows)");
    }
    let mut bytes = Vec::new();
    stdin.read_to_end(&mut bytes).context("can't read stdin")?;
    let text = String::from_utf8_lossy(&bytes);
    let name = args.name.replace('\\', "/");
    let report = engine::detect::scan_text(&text, &name, false);
    Ok(engine::output(&report))
}
