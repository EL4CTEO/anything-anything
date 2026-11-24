//! .anything/: what happened so far (every agent run and the last verify), and the logs.

use crate::gate::Report;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRec {
    pub at: u64,
    /// design or build
    pub kind: String,
    pub agent: String,
    pub attempt: usize,
    pub ok: bool,
    pub secs: f64,
    pub note: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub runs: Vec<RunRec>,
    pub last: Option<Report>,
}

pub fn dir(root: &Path) -> PathBuf {
    root.join(".anything")
}

pub fn logs(root: &Path) -> PathBuf {
    dir(root).join("logs")
}

pub fn load(root: &Path) -> State {
    std::fs::read_to_string(dir(root).join("state.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(root: &Path, state: &State) -> Result<()> {
    std::fs::create_dir_all(dir(root))?;
    std::fs::write(dir(root).join("state.json"), serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn record(root: &Path, run: RunRec) -> Result<()> {
    let mut s = load(root);
    s.runs.push(run);
    save(root, &s)
}

pub fn remember(root: &Path, report: &Report) -> Result<()> {
    let mut s = load(root);
    s.last = Some(report.clone());
    save(root, &s)
}
