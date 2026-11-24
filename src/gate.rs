//! verify: every gate a tool has to pass to count as built. The spec is valid, the generated files are
//! as generated, nothing is a stub, the stack's steps pass (format, build, lint, test), the MCP server
//! lists every command, and every example does what it says.

use crate::check::{self, ExampleResult};
use crate::scaffold;
use crate::spec::Spec;
use crate::stack::StackDef;
use crate::state;
use crate::{proc, ui, util};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

pub const MARKER: &str = "ANYTHING:BUILD";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub name: String,
    pub status: Status,
    pub secs: f64,
    /// Why it failed or was skipped, with the output that shows it.
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub ok: bool,
    pub name: String,
    pub stack: String,
    pub gates: Vec<Gate>,
    pub examples: Vec<ExampleResult>,
    pub at: u64,
}

impl Report {
    pub fn passed(&self) -> usize {
        self.examples.iter().filter(|e| e.ok).count()
    }

    pub fn failing(&self) -> impl Iterator<Item = &Gate> {
        self.gates.iter().filter(|g| g.status == Status::Fail)
    }
}

struct Gates(Vec<Gate>);

impl Gates {
    fn add(&mut self, name: &str, status: Status, secs: f64, detail: impl Into<String>) {
        let detail = detail.into();
        match status {
            Status::Pass => ui::ok(format!("{name} {}", ui::dim(util::secs(secs)))),
            Status::Skip => ui::line(format!("{} {name} {}", ui::dim("·"), ui::dim(util::clip(&detail, 90)))),
            Status::Fail => {
                ui::error(format!("{name}: {}", util::clip(detail.lines().next().unwrap_or_default(), 100)))
            }
        }
        self.0.push(Gate { name: name.into(), status, secs, detail });
    }
}

/// Text that means "not built yet", in the stack's sources.
pub fn stubs(root: &Path, def: &StackDef) -> Vec<String> {
    let mut markers: Vec<&str> = def.stubs.iter().map(String::as_str).collect();
    markers.push(MARKER);
    let mut found = Vec::new();
    for dir in &def.sources {
        let base = root.join(dir);
        for rel in util::files_under(&base) {
            let Ok(text) = std::fs::read_to_string(base.join(&rel)) else { continue };
            for (i, line) in text.lines().enumerate() {
                if markers.iter().any(|m| line.contains(m)) {
                    found.push(format!("{dir}/{rel}:{}: {}", i + 1, util::clip(line.trim(), 80)));
                }
            }
        }
    }
    found
}

pub fn verify(root: &Path) -> Result<Report> {
    let mut g = Gates(Vec::new());
    let start = Instant::now();
    let finish = |g: Gates, spec: Option<&Spec>, examples: Vec<ExampleResult>| -> Result<Report> {
        let report = Report {
            ok: !g.0.iter().any(|x| x.status == Status::Fail),
            name: spec.map(|s| s.name.clone()).unwrap_or_default(),
            stack: spec.map(|s| s.stack.clone()).unwrap_or_default(),
            gates: g.0,
            examples,
            at: util::now(),
        };
        state::remember(root, &report)?;
        Ok(report)
    };
    let spec = match Spec::load(root) {
        Ok(s) => s,
        Err(e) => {
            g.add("spec", Status::Fail, 0.0, format!("{e:#}"));
            return finish(g, None, Vec::new());
        }
    };
    let (def, files) = match scaffold::expected(root, &spec) {
        Ok(x) => x,
        Err(e) => {
            g.add("spec", Status::Fail, 0.0, format!("{e:#}"));
            return finish(g, Some(&spec), Vec::new());
        }
    };
    let problems = spec.problems(&def);
    let t = start.elapsed().as_secs_f64();
    if problems.is_empty() {
        g.add("spec", Status::Pass, t, "");
    } else {
        g.add("spec", Status::Fail, t, problems.join("\n"));
    }

    let t = Instant::now();
    let drift = scaffold::drift(root, &files);
    if drift.is_empty() {
        g.add("files", Status::Pass, t.elapsed().as_secs_f64(), "");
    } else {
        g.add(
            "files",
            Status::Fail,
            t.elapsed().as_secs_f64(),
            format!(
                "generated from {}, but edited or missing (anything-anything sync puts them back):\n{}",
                crate::spec::FILE,
                drift.join("\n")
            ),
        );
    }

    let t = Instant::now();
    let found = stubs(root, &def);
    if found.is_empty() {
        g.add("stubs", Status::Pass, t.elapsed().as_secs_f64(), "");
    } else {
        g.add("stubs", Status::Fail, t.elapsed().as_secs_f64(), format!("not built yet:\n{}", found.join("\n")));
    }

    let mut blocked: Option<String> = None;
    for step in spec.steps(&def) {
        if let Some(after) = &blocked {
            g.add(&step.name, Status::Skip, 0.0, format!("skipped: {after} failed"));
            continue;
        }
        let probe: Vec<String> = step.probe.iter().map(|a| util::expand(a, root)).collect();
        if !proc::probe(&probe) {
            g.add(&step.name, Status::Skip, 0.0, format!("skipped: {} isn't installed", probe.join(" ")));
            continue;
        }
        let argv: Vec<String> = step.run.iter().map(|a| util::expand(a, root)).collect();
        let env = step.env.iter().map(|(k, v)| (k.clone(), util::expand(v, root))).collect();
        let timeout = Duration::from_secs(if step.timeout == 0 { 900 } else { step.timeout });
        ui::step(format!("{} {}", step.name, ui::dim(argv.join(" "))));
        let log = state::logs(root).join(format!("{}.log", step.name));
        match proc::run(root, &argv, &env, timeout, &log) {
            Ok(out) if out.ok || step.fix => g.add(&step.name, Status::Pass, out.secs, ""),
            Ok(out) => {
                let why =
                    if out.timed_out { format!("timed out after {}s", timeout.as_secs()) } else { "failed".into() };
                g.add(
                    &step.name,
                    Status::Fail,
                    out.secs,
                    format!("{} {why}:\n{}", argv.join(" "), util::tail(&out.output, 60)),
                );
                if !step.soft {
                    blocked = Some(step.name.clone());
                }
            }
            Err(e) => {
                g.add(&step.name, Status::Fail, 0.0, format!("{e:#}"));
                if !step.soft {
                    blocked = Some(step.name.clone());
                }
            }
        }
    }

    let run = spec.run_def(&def);
    if def.mcp {
        if let Some(after) = &blocked {
            g.add("mcp", Status::Skip, 0.0, format!("skipped: {after} failed"));
        } else {
            let t = Instant::now();
            match mcp_check(root, &spec, &run) {
                Ok(()) => g.add("mcp", Status::Pass, t.elapsed().as_secs_f64(), ""),
                Err(e) => g.add("mcp", Status::Fail, t.elapsed().as_secs_f64(), format!("{e:#}")),
            }
        }
    }

    let mut examples = Vec::new();
    if let Some(after) = &blocked {
        g.add("examples", Status::Skip, 0.0, format!("skipped: {after} failed"));
    } else if !spec.examples.is_empty() {
        let t = Instant::now();
        ui::step(format!("examples {}", ui::dim(format!("({})", spec.examples.len()))));
        examples = check::run_all(root, &spec, &run, &state::logs(root));
        for e in &examples {
            if e.ok {
                ui::detail(format!("{} {}", ui::green("✓"), e.name));
            } else {
                ui::detail(format!(
                    "{} {}: {}",
                    ui::red("✗"),
                    e.name,
                    util::clip(e.problem.lines().next().unwrap_or_default(), 80)
                ));
            }
        }
        let failed: Vec<String> = examples
            .iter()
            .filter(|e| !e.ok)
            .map(|e| format!("example {:?}:\n{}", e.name, indent(&e.problem)))
            .collect();
        let passed = examples.len() - failed.len();
        let status = if failed.is_empty() { Status::Pass } else { Status::Fail };
        g.add(
            "examples",
            status,
            t.elapsed().as_secs_f64(),
            format!("{passed}/{} pass\n{}", examples.len(), failed.join("\n")),
        );
    }
    finish(g, Some(&spec), examples)
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n")
}

/// Starts `<cli> mcp`, discovers it the 2026-07-28 way and the legacy way, and lists its tools.
fn mcp_check(root: &Path, spec: &Spec, run: &crate::spec::Run) -> Result<()> {
    let mut argv: Vec<String> = run.cmd.iter().map(|a| util::expand(a, root)).collect();
    anyhow::ensure!(!argv.is_empty(), "no [run] cmd to start the MCP server with");
    argv.push("mcp".into());
    let env: Vec<(String, String)> = run.env.iter().map(|(k, v)| (k.clone(), util::expand(v, root))).collect();
    let mut child = util::command(&argv)
        .current_dir(std::env::temp_dir())
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let meta = json!({ "io.modelcontextprotocol/protocolVersion": crate::mcp::LATEST,
                       "io.modelcontextprotocol/clientCapabilities": {},
                       "io.modelcontextprotocol/clientInfo": { "name": "anything-anything", "version": env!("CARGO_PKG_VERSION") } });
    let lines = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": { "_meta": meta } }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "anything-anything", "version": "0" } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list", "params": { "_meta": meta } }),
    ];
    let mut stdin = child.stdin.take().expect("piped");
    for l in &lines {
        writeln!(stdin, "{l}")?;
    }
    stdin.flush()?;
    let stdout = child.stdout.take().expect("piped");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut replies = std::collections::BTreeMap::new();
    while replies.len() < 3 {
        let left = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = rx.recv_timeout(left) else { break };
        if let Ok(v) = serde_json::from_str::<Value>(&line)
            && let Some(id) = v["id"].as_u64()
        {
            replies.insert(id, v);
        }
    }
    drop(stdin);
    util::kill_tree(&mut child);
    let get = |id: u64, what: &str| -> Result<&Value> {
        let v = replies.get(&id).ok_or_else(|| anyhow::anyhow!("no answer to {what} within 30s"))?;
        anyhow::ensure!(v.get("error").is_none(), "{what} answered an error: {}", v["error"]);
        Ok(&v["result"])
    };
    let discover = get(1, "server/discover")?;
    let versions: Vec<&str> = discover["supportedVersions"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    anyhow::ensure!(
        versions.contains(&crate::mcp::LATEST),
        "server/discover should list {} in supportedVersions, got {versions:?}",
        crate::mcp::LATEST
    );
    get(2, "initialize")?;
    let tools = get(3, "tools/list")?;
    let got: BTreeSet<String> = tools["tools"]
        .as_array()
        .map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    let want: BTreeSet<String> = spec.commands.iter().map(|c| spec.tool(c)).collect();
    anyhow::ensure!(got == want, "tools/list should list {want:?}, got {got:?}");
    Ok(())
}
