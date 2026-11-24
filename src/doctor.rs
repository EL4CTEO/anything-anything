//! What's installed: the toolchains each stack needs, git and gh for publishing, and the agents.

use crate::{proc, stack, ui, util};
use anyhow::Result;

fn row(ok: bool, name: &str, note: &str) {
    let mark = if ok { ui::green("✓") } else { ui::dim("·") };
    ui::line(format!("  {mark} {name:<10} {}", ui::dim(note)));
}

fn version(argv: &[&str]) -> Option<String> {
    let path = util::which(argv[0])?;
    let out = std::process::Command::new(path).args(&argv[1..]).output().ok()?;
    let text = String::from_utf8_lossy(if out.stdout.is_empty() { &out.stderr } else { &out.stdout }).trim().to_string();
    out.status.success().then(|| text.lines().next().unwrap_or_default().to_string())
}

fn tool(name: &str, argv: &[&str], why: &str) -> bool {
    match version(argv) {
        Some(v) => {
            row(true, name, &v);
            true
        }
        None => {
            row(false, name, &format!("not found: {why}"));
            false
        }
    }
}

pub fn run() -> Result<()> {
    ui::line(ui::bold("  stacks"));
    for (id, about) in stack::all() {
        ui::line(format!("  {} {id:<10} {}", ui::brand("•"), ui::dim(about)));
    }
    ui::line("");
    ui::line(ui::bold("  toolchains"));
    tool("cargo", &["cargo", "--version"], "rust-cli");
    let fmt = proc::probe(&["cargo".into(), "fmt".into(), "--version".into()]);
    row(fmt, "rustfmt", if fmt { "cargo fmt" } else { "rustup component add rustfmt" });
    let clippy = proc::probe(&["cargo".into(), "clippy".into(), "--version".into()]);
    row(clippy, "clippy", if clippy { "cargo clippy" } else { "rustup component add clippy" });
    tool(&util::python(), &[&util::python(), "--version"], "python-cli");
    tool("uv", &["uv", "--version"], "python-cli installs and the lint step (uvx ruff)");
    tool("node", &["node", "--version"], "node-cli and web (Node 24+ runs TypeScript as it is)");
    tool("npm", &["npm", "--version"], "node-cli and web");
    ui::line("");
    ui::line(ui::bold("  publishing"));
    tool("git", &["git", "--version"], "needed to publish");
    if tool("gh", &["gh", "--version"], "needed to publish") {
        let auth = std::process::Command::new("gh").args(["auth", "status"]).output().is_ok_and(|o| o.status.success());
        row(auth, "gh auth", if auth { "logged in" } else { "gh auth login" });
    }
    ui::line("");
    ui::line(ui::bold("  agents"));
    let mut any = false;
    for (name, argv) in [("claude", ["claude", "--version"]), ("codex", ["codex", "--version"])] {
        any |= tool(name, &argv, "--agent");
    }
    if !any {
        ui::line(ui::dim("    any command that reads a prompt on stdin works: --agent \"my-agent --flag\""));
    }
    Ok(())
}
