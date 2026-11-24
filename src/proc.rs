//! Runs a step (a build, a linter, a test suite) with a timeout, its output going to a log.

use crate::util;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub struct Outcome {
    pub ok: bool,
    pub timed_out: bool,
    pub secs: f64,
    pub output: String,
}

/// Runs argv in dir with stdout and stderr going to log.
pub fn run(dir: &Path, argv: &[String], env: &BTreeMap<String, String>, timeout: Duration, log: &Path) -> Result<Outcome> {
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(log)?;
    let start = Instant::now();
    let mut child = util::command(argv)
        .current_dir(dir)
        .envs(env)
        .env("CI", "true")
        .env("NO_COLOR", "1")
        .env("CARGO_TERM_COLOR", "never")
        .env_remove("CLAUDECODE")
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file)
        .spawn()
        .with_context(|| format!("couldn't start {}; is it installed and on PATH?", argv[0]))?;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() > timeout {
            util::kill_tree(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let output = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).replace("\r\n", "\n");
    Ok(Outcome {
        ok: status.is_some_and(|s| s.success()),
        timed_out: status.is_none(),
        secs: start.elapsed().as_secs_f64(),
        output,
    })
}

/// Whether a command runs and exits 0 (`cargo clippy --version`): for steps that need a tool.
pub fn probe(argv: &[String]) -> bool {
    static SEEN: OnceLock<Mutex<HashMap<Vec<String>, bool>>> = OnceLock::new();
    if argv.is_empty() {
        return true;
    }
    let mut seen = SEEN.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    *seen.entry(argv.to_vec()).or_insert_with(|| {
        util::which(&argv[0]).is_some()
            && util::command(argv)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_times_out() {
        let dir = std::env::temp_dir();
        let log = dir.join(format!("anything-anything-proc-{}.log", std::process::id()));
        let env = BTreeMap::new();
        let ok = run(&dir, &["echo hello there".to_string()], &env, Duration::from_secs(20), &log).unwrap();
        assert!(ok.ok && ok.output.contains("hello"), "{}", ok.output);
        let bad = run(&dir, &["exit 3".to_string()], &env, Duration::from_secs(20), &log).unwrap();
        assert!(!bad.ok && !bad.timed_out);
        let sleep = if cfg!(windows) { "ping -n 20 127.0.0.1" } else { "sleep 20" };
        let slow = run(&dir, &[sleep.to_string()], &env, Duration::from_millis(300), &log).unwrap();
        assert!(slow.timed_out && slow.secs < 10.0);
        let _ = std::fs::remove_file(log);
        assert!(!probe(&["definitely-not-installed-xyz".to_string()]));
    }
}
