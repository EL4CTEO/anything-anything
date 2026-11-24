//! The pipeline: new (idea -> folder), design (idea -> anything.toml), build (anything.toml -> a tool
//! that passes verify). The agent does the work; anything-anything checks it, puts back what it
//! mustn't touch, and hands it the failures until every gate passes.

use crate::gate::{self, Report};
use crate::spec::{self, Spec};
use crate::state::{self, RunRec};
use crate::{agent, names, scaffold, stack, task, ui, util};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AgentOpts {
    pub agent: String,
    pub attempts: usize,
    pub timeout: Duration,
}

pub struct NewOpts {
    pub what: String,
    pub idea: String,
    pub stack: Option<String>,
    pub spec: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub owner: Option<String>,
    pub npm: Option<String>,
    pub color: Option<String>,
    pub git: bool,
    pub agent: Option<AgentOpts>,
    pub build: bool,
}

/// The GitHub login of whoever runs this, from gh, when it's logged in.
fn github_login() -> Option<String> {
    let gh = util::which("gh")?;
    let out = std::process::Command::new(gh).args(["api", "user", "--jq", ".login"]).output().ok()?;
    let login = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !login.is_empty()).then_some(login)
}

fn toml_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// The anything.toml a project starts from, before anyone designs it.
fn starter(name: &str, idea: &str, stack: &str, owner: &str, npm: Option<&str>, color: Option<&str>) -> String {
    let mut t = format!(
        "# {name}: made with anything-anything. `anything-anything spec` explains every field.\n\n\
name = {}\npitch = {}\ntagline = \"\"\nwhen = \"\"\nversion = \"0.1.0\"\nstack = {}\nowner = {}\n",
        toml_str(name),
        toml_str(idea.trim()),
        toml_str(stack),
        toml_str(owner)
    );
    if let Some(n) = npm {
        t.push_str(&format!("npm = {}\n", toml_str(n)));
    }
    if let Some(c) = color {
        t.push_str(&format!("color = {}\n", toml_str(c)));
    }
    t.push_str("keywords = []\nnotes = \"\"\"\n\"\"\"\n");
    t
}

/// Sets top-level keys of anything.toml back to what they were, keeping everything else (and the comments).
fn pin(root: &Path, keys: &[(&str, &str)]) -> Result<Vec<String>> {
    let path = root.join(spec::FILE);
    let text = std::fs::read_to_string(&path)?;
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e| anyhow::anyhow!("{}: {e}", spec::FILE))?;
    let mut changed = Vec::new();
    for (k, v) in keys {
        if doc.get(k).and_then(|x| x.as_str()) != Some(*v) {
            doc[*k] = toml_edit::value(*v);
            changed.push(k.to_string());
        }
    }
    if !changed.is_empty() {
        std::fs::write(&path, doc.to_string())?;
    }
    Ok(changed)
}

pub fn create(cwd: &Path, o: NewOpts) -> Result<(PathBuf, Option<Report>)> {
    let name = names::tool_name(&o.what)?;
    let root = util::absolute(&o.out.clone().unwrap_or_else(|| cwd.join(&name)));
    if root.join(spec::FILE).exists() {
        bail!("{} is already a project: run anything-anything -C {} build", root.display(), root.display());
    }
    if root.exists() && std::fs::read_dir(&root)?.next().is_some() {
        bail!("{} isn't empty: pick another --out", root.display());
    }
    if let Some(s) = &o.stack {
        stack::all().iter().find(|(id, _)| id == s).with_context(|| {
            format!("no stack {s:?}; there's {}", stack::all().into_iter().map(|(id, _)| id).collect::<Vec<_>>().join(", "))
        })?;
    }
    std::fs::create_dir_all(&root)?;
    let owner = o.owner.clone().or_else(github_login).unwrap_or_default();
    let text = match &o.spec {
        Some(file) => std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?,
        None => {
            let stack = o.stack.clone().unwrap_or_else(|| "any".into());
            let idea = if o.idea.trim().is_empty() { format!("{} anything.", names::pascal(name.trim_end_matches("-anything"))) } else { o.idea.clone() };
            starter(&name, &idea, &stack, &owner, o.npm.as_deref(), o.color.as_deref())
        }
    };
    std::fs::write(root.join(spec::FILE), &text)?;
    if o.spec.is_some() {
        Spec::parse(&text)?;
        let mut keys = vec![("name", name.as_str())];
        if Spec::parse(&text)?.owner.is_empty() && !owner.is_empty() {
            keys.push(("owner", owner.as_str()));
        }
        pin(&root, &keys)?;
    }
    ui::ok(format!("{} {}", ui::bold(&name), ui::dim(root.display())));
    if let (Some(a), None) = (&o.agent, &o.spec) {
        design(&root, &o.idea, o.stack.as_deref(), a)?;
    }
    let spec = Spec::load(&root)?;
    let synced = scaffold::sync(&root, &spec, false)?;
    ui::ok(format!("{} files from the {} stack", synced.created.len(), spec.stack));
    if o.git && util::which("git").is_some() && !root.join(".git").exists() {
        let _ = std::process::Command::new("git").args(["init", "-q", "-b", "main"]).current_dir(&root).status();
    }
    let report = match (&o.agent, o.build) {
        (Some(a), true) => Some(build(&root, a)?),
        _ => None,
    };
    Ok((root, report))
}

/// An idea becomes anything.toml: the agent writes it, anything-anything checks it, until it's valid.
pub fn design(root: &Path, idea: &str, stack_choice: Option<&str>, a: &AgentOpts) -> Result<Spec> {
    let before = Spec::load(root)?;
    let pinned = [("name", before.name.clone()), ("owner", before.owner.clone()), ("version", before.version.clone()), ("npm", before.npm.clone())];
    let idea = if idea.trim().is_empty() { before.pitch.clone() } else { idea.to_string() };
    let mut feedback = String::new();
    for attempt in 1..=a.attempts.max(1) {
        let current = std::fs::read_to_string(root.join(spec::FILE))?;
        let md = format!("{}{feedback}", task::design(&before.name, &idea, stack_choice, &current));
        std::fs::create_dir_all(state::dir(root))?;
        std::fs::write(state::dir(root).join("DESIGN.md"), md)?;
        ui::step(format!("{} is designing {}{}", a.agent, before.name, if attempt > 1 { format!(" (attempt {attempt})") } else { String::new() }));
        let start = Instant::now();
        agent::run(&a.agent, root, "Read .anything/DESIGN.md in this folder and do what it says.", &state::logs(root).join(format!("design-{attempt}.log")), a.timeout)?;
        // a design writes one file
        for rel in util::files_under(root) {
            if rel != spec::FILE {
                let _ = std::fs::remove_file(root.join(&rel));
            }
        }
        let keys: Vec<(&str, &str)> = pinned.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (*k, v.as_str())).collect();
        let problems = match Spec::load(root) {
            Err(e) => vec![format!("{e:#}")],
            Ok(_) => {
                pin(root, &keys)?;
                let spec = Spec::load(root)?;
                match stack::load(&spec) {
                    Err(e) => vec![format!("{e:#}")],
                    Ok(def) => spec.problems(&def),
                }
            }
        };
        let ok = problems.is_empty();
        state::record(root, RunRec { at: util::now(), kind: "design".into(), agent: a.agent.clone(), attempt, ok, secs: start.elapsed().as_secs_f64(), note: problems.join("; ") })?;
        if ok {
            let spec = Spec::load(root)?;
            ui::ok(format!("designed: {} stack, {} commands, {} examples", spec.stack, spec.commands.len(), spec.examples.len()));
            return Ok(spec);
        }
        for p in &problems {
            ui::warn(p);
        }
        feedback = format!("\n\n## Your last version had problems\n\nFix these in {}:\n\n- {}\n", spec::FILE, problems.join("\n- "));
    }
    bail!("the design still has problems after {} attempts: fix {} by hand, then run anything-anything build", a.attempts, root.join(spec::FILE).display())
}

/// Writes .anything/TASK.md, for an agent run by anything-anything or by you.
pub fn write_task(root: &Path, notes: &[String]) -> Result<PathBuf> {
    let spec = Spec::load(root)?;
    let def = stack::load(&spec)?;
    let mut md = task::build(&spec, &def, state::load(root).last.as_ref());
    if !notes.is_empty() {
        md.push_str(&format!("\n## Also\n\n- {}\n", notes.join("\n- ")));
    }
    let path = state::dir(root).join("TASK.md");
    std::fs::create_dir_all(state::dir(root))?;
    std::fs::write(&path, md)?;
    Ok(path)
}

/// The agent builds; verify judges; the failures go back to the agent. Until it passes or attempts run out.
pub fn build(root: &Path, a: &AgentOpts) -> Result<Report> {
    let spec = Spec::load(root)?;
    let def = stack::load(&spec)?;
    let problems = spec.problems(&def);
    if !problems.is_empty() {
        bail!("fix {} first:\n- {}", spec::FILE, problems.join("\n- "));
    }
    scaffold::sync(root, &spec, false)?;
    let spec_text = std::fs::read_to_string(root.join(spec::FILE))?;
    let mut notes: Vec<String> = Vec::new();
    let mut report = None;
    for attempt in 1..=a.attempts.max(1) {
        write_task(root, &notes)?;
        notes.clear();
        ui::step(format!("{} is building {}{}", a.agent, spec.name, if attempt > 1 { format!(" (attempt {attempt}/{})", a.attempts) } else { String::new() }));
        let start = Instant::now();
        let ran = agent::run(&a.agent, root, "Read .anything/TASK.md in this folder and do what it says.", &state::logs(root).join(format!("build-{attempt}.log")), a.timeout);
        if std::fs::read_to_string(root.join(spec::FILE))? != spec_text {
            std::fs::write(root.join(spec::FILE), &spec_text)?;
            notes.push(format!("You changed {}; it was put back. If the spec is wrong, say so instead.", spec::FILE));
        }
        let restored = scaffold::sync(root, &spec, false)?;
        if !restored.updated.is_empty() {
            notes.push(format!("You edited generated files ({}); they were put back. Put your code in the files that are yours.", restored.updated.join(", ")));
        }
        for n in &notes {
            ui::warn(n);
        }
        if let Err(e) = &ran {
            ui::warn(format!("{e:#}"));
        }
        ui::step("verify");
        let r = gate::verify(root)?;
        let failing: Vec<String> = r.failing().map(|g| g.name.clone()).collect();
        state::record(root, RunRec { at: util::now(), kind: "build".into(), agent: a.agent.clone(), attempt, ok: r.ok, secs: start.elapsed().as_secs_f64(), note: failing.join(", ") })?;
        if r.ok {
            return Ok(r);
        }
        report = Some(r);
    }
    Ok(report.expect("at least one attempt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_parses() {
        let t = starter("x-anything", "Does \"x\" to anything.", "any", "", Some("@me/x-anything"), None);
        let s = Spec::parse(&t).unwrap();
        assert_eq!(s.pitch, "Does \"x\" to anything.");
        assert_eq!(s.npm, "@me/x-anything");
    }
}
