//! `secrets-anything scan [path] [--all]`: Find leaked secrets in a folder or file
//!
//! Yours: anything-anything never overwrites this file.

use super::Output;
use super::args::ScanArgs;
use crate::engine;
use anyhow::Result;

pub fn run(args: &ScanArgs) -> Result<Output> {
    let report = engine::walk::scan_path(&args.path, args.all)?;
    Ok(engine::output(&report))
}
