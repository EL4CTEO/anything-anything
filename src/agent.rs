//! Runs a coding agent on a prompt in a folder: claude, codex, or any command that reads a prompt on stdin.
//! Its output streams to a log, and a line per action to the terminal.

use crate::{ui, util};
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

/// What the agent may do. Shell access is limited to the stack's tools and anything-anything verify.
const CLAUDE_TOOLS: &str = "Read Edit Write Glob Grep WebSearch WebFetch TodoWrite \
Bash(anything-anything verify:*) Bash(anything-anything status:*) Bash(anything-anything spec:*) \
Bash(cargo:*) Bash(rustc:*) Bash(python:*) Bash(python3:*) Bash(uv:*) Bash(uvx:*) Bash(pip:*) Bash(ruff:*) Bash(pytest:*) \
Bash(node:*) Bash(npm:*) Bash(npx:*) Bash(pnpm:*) Bash(bun:*) Bash(deno:*) Bash(tsc:*) Bash(go:*) Bash(dotnet:*) Bash(java:*) Bash(gradle:*) Bash(mvn:*) \
Bash(git status:*) Bash(git diff:*) Bash(ls:*) Bash(cat:*) Bash(mkdir:*) Bash(echo:*)";

/// The argv for an --agent value.
pub fn argv(spec: &str) -> Vec<String> {
    match spec {
        "claude" => [
            "claude",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
            CLAUDE_TOOLS,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        "codex" => ["codex", "exec", "--full-auto", "-"].iter().map(|s| s.to_string()).collect(),
        other => other.split_whitespace().map(String::from).collect(),
    }
}

/// One line of an agent's output, as something to show. claude's stream-json becomes "Edit src/x.rs".
fn summarize(line: &str) -> Option<String> {
    let Ok(v) = serde_json::from_str::<Value>(line) else {
        let t = line.trim();
        return (!t.is_empty()).then(|| util::clip(t, 110));
    };
    match v["type"].as_str()? {
        "assistant" => {
            let parts = v["message"]["content"].as_array()?;
            let mut out = Vec::new();
            for p in parts {
                if p["type"] == "tool_use" {
                    let input = &p["input"];
                    let target = ["file_path", "command", "pattern", "url", "query"]
                        .iter()
                        .find_map(|k| input[k].as_str())
                        .unwrap_or_default();
                    out.push(util::clip(&format!("{} {}", p["name"].as_str().unwrap_or("tool"), target), 110));
                }
            }
            (!out.is_empty()).then(|| out.join(" · "))
        }
        "result" => Some(format!("done: {}", util::clip(v["result"].as_str().unwrap_or_default(), 100))),
        _ => None,
    }
}

/// Runs the agent with prompt on stdin, in dir, until it exits or timeout.
pub fn run(spec: &str, dir: &Path, prompt: &str, log_path: &Path, timeout: Duration) -> Result<()> {
    let argv = argv(spec);
    ensure!(!argv.is_empty(), "empty --agent");
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut log = File::create(log_path)?;
    // the agent's shell finds this anything-anything first, even when it isn't installed
    let mut path =
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).into_iter().collect::<Vec<_>>();
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    let mut child = util::command(&argv)
        .current_dir(dir)
        .env("PATH", std::env::join_paths(path)?)
        .env_remove("CLAUDECODE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("couldn't start the agent ({}): is it installed and on PATH?", argv[0]))?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(prompt.as_bytes())?;
    drop(stdin);
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    for pipe in [
        child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);
    let start = Instant::now();
    let mut last = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                writeln!(log, "{line}")?;
                if let Some(s) = summarize(&line) {
                    ui::detail(&s);
                    last.push(s);
                    if last.len() > 15 {
                        last.remove(0);
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        if start.elapsed() > timeout {
            util::kill_tree(&mut child);
            bail!("the agent ran past {} minutes and was stopped. Log: {}", timeout.as_secs() / 60, log_path.display());
        }
    }
    let status = child.wait()?;
    ensure!(status.success(), "the agent exited with {status}. Log: {}\n{}", log_path.display(), last.join("\n"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_custom() {
        assert_eq!(argv("claude")[0], "claude");
        assert_eq!(argv("codex"), ["codex", "exec", "--full-auto", "-"]);
        assert_eq!(argv("aider --yes"), ["aider", "--yes"]);
    }

    #[test]
    fn stream_json_becomes_one_line_per_action() {
        let edit = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/a.rs"}}]}}"#;
        assert_eq!(summarize(edit).as_deref(), Some("Edit src/a.rs"));
        assert_eq!(summarize(r#"{"type":"system"}"#), None);
        assert_eq!(summarize("plain text").as_deref(), Some("plain text"));
    }
}
