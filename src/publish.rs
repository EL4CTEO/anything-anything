//! Puts a verified tool on GitHub: commit, create the repo (tagged anything-anything, so the hub finds
//! it), push, and tag the version so its release workflow builds and publishes it.

use crate::spec::Spec;
use crate::{gate, ui, util};
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub struct Step {
    pub what: String,
    pub argv: Vec<String>,
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).current_dir(root).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn topic(word: &str) -> Option<String> {
    let t: String = word.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let t = t.trim_matches('-').to_string();
    (!t.is_empty() && t.len() <= 50).then_some(t)
}

fn step(what: &str, argv: &[&str]) -> Step {
    Step { what: what.into(), argv: argv.iter().map(|s| s.to_string()).collect() }
}

pub fn plan(root: &Path, spec: &Spec, private: bool) -> Result<Vec<Step>> {
    if spec.owner.is_empty() || spec.owner == "OWNER" {
        bail!("set owner (your GitHub user or organization) in anything.toml first");
    }
    let repo = format!("{}/{}", spec.owner, spec.name);
    let tag = format!("v{}", spec.version);
    let mut steps = Vec::new();
    if !root.join(".git").exists() {
        steps.push(step("start a git repository", &["git", "init", "-b", "main"]));
    }
    let dirty = git(root, &["status", "--porcelain"]).is_none_or(|s| !s.is_empty());
    if dirty {
        steps.push(step("stage everything", &["git", "add", "-A"]));
        steps.push(Step { what: "commit".into(), argv: vec!["git".into(), "commit".into(), "-m".into(), format!("{} {tag}", spec.name)] });
    }
    if git(root, &["remote", "get-url", "origin"]).is_none() {
        let visibility = if private { "--private" } else { "--public" };
        steps.push(Step {
            what: format!("create github.com/{repo} and push"),
            argv: ["gh", "repo", "create", &repo, visibility, "--source", ".", "--remote", "origin", "--push", "--description", &spec.pitch]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        });
    } else {
        steps.push(step("push", &["git", "push", "-u", "origin", "HEAD"]));
    }
    let mut topics = vec!["anything-anything".to_string()];
    topics.extend(spec.keywords.iter().filter_map(|k| topic(k)));
    topics.dedup();
    let mut edit = vec!["gh".to_string(), "repo".into(), "edit".into(), repo.clone()];
    for t in topics.iter().take(20) {
        edit.push("--add-topic".into());
        edit.push(t.clone());
    }
    steps.push(Step { what: "tag the repo so the hub lists it".into(), argv: edit });
    if git(root, &["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")]).is_none() {
        steps.push(step(&format!("tag {tag}"), &["git", "tag", &tag]));
    }
    steps.push(Step { what: format!("push {tag}: the release workflow builds and publishes it"), argv: vec!["git".into(), "push".into(), "origin".into(), tag] });
    Ok(steps)
}

pub fn run(root: &Path, yes: bool, private: bool) -> Result<()> {
    let spec = Spec::load(root)?;
    for tool in ["git", "gh"] {
        util::which(tool).with_context(|| format!("{tool} isn't installed"))?;
    }
    let steps = plan(root, &spec, private)?;
    ui::line(ui::bold(format!("  publishing {} v{}", spec.name, spec.version)));
    for (i, s) in steps.iter().enumerate() {
        ui::line(format!("  {}. {}  {}", i + 1, s.what, ui::dim(s.argv.join(" "))));
    }
    if !yes {
        ui::line(ui::dim("  nothing done yet: this goes public. Run it again with --yes to do it."));
        return Ok(());
    }
    ui::step("verify first");
    let report = gate::verify(root)?;
    if !report.ok {
        bail!("verify fails: a tool that doesn't pass its own examples isn't published");
    }
    for s in &steps {
        ui::step(&s.what);
        let status = util::command(&s.argv).current_dir(root).status().with_context(|| format!("couldn't run {}", s.argv[0]))?;
        if !status.success() {
            bail!("{} failed ({status})", s.argv.join(" "));
        }
    }
    ui::ok(format!("https://github.com/{}/{}", spec.owner, spec.name));
    ui::line(ui::dim("  npm / crates.io / PyPI publishing runs in the release workflow once the repo has the tokens it names."));
    Ok(())
}
