//! The hub: every -anything tool on GitHub. Live from the GitHub API, or from the list built in
//! (hub.json) when offline. `show` works out how a tool installs; `install` runs it.

use crate::{ui, util};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub repo: String,
    pub about: String,
    pub stars: u64,
    pub language: String,
}

#[derive(Deserialize)]
struct Snapshot {
    updated: String,
    tools: Vec<Tool>,
}

const SNAPSHOT: &str = include_str!("../hub.json");

pub struct Listing {
    pub tools: Vec<Tool>,
    /// None: live. Some(date): the built-in list, from that date.
    pub offline: Option<String>,
}

fn http() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}

fn get(url: &str) -> Result<(u16, String)> {
    let mut req =
        http().get(url).header("User-Agent", "anything-anything").header("Accept", "application/vnd.github+json");
    if url.starts_with("https://api.github.com")
        && let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN"))
    {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let mut r = req.call().with_context(|| format!("GET {url}"))?;
    let status = r.status().as_u16();
    Ok((status, r.body_mut().read_to_string().unwrap_or_default()))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn snapshot() -> (Vec<Tool>, String) {
    let s: Snapshot = serde_json::from_str(SNAPSHOT).expect("hub.json");
    (s.tools, s.updated)
}

fn search_live(q: &str, limit: usize) -> Result<Vec<Tool>> {
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page={}",
        encode(q),
        limit.clamp(1, 100)
    );
    let (status, body) = get(&url)?;
    if status != 200 {
        bail!("GitHub answered {status}");
    }
    let v: Value = serde_json::from_str(&body)?;
    Ok(v["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|i| Tool {
                    repo: i["full_name"].as_str().unwrap_or_default().to_string(),
                    about: i["description"].as_str().unwrap_or_default().to_string(),
                    stars: i["stargazers_count"].as_u64().unwrap_or(0),
                    language: i["language"].as_str().unwrap_or_default().to_string(),
                })
                .filter(|t| t.repo.to_lowercase().contains("anything"))
                .collect()
        })
        .unwrap_or_default())
}

/// Tools whose name has "anything" in it, most stars first. words narrows it down.
pub fn list(words: &str, limit: usize, offline: bool) -> Listing {
    let q = if words.trim().is_empty() {
        "anything in:name".to_string()
    } else {
        format!("{} anything in:name,description", words.trim())
    };
    if !offline {
        match search_live(&q, limit) {
            Ok(tools) => return Listing { tools, offline: None },
            Err(e) => ui::warn(format!("GitHub search failed ({e:#}); showing the built-in list")),
        }
    }
    let (mut tools, updated) = snapshot();
    let words: Vec<String> = words.split_whitespace().map(str::to_lowercase).collect();
    tools.retain(|t| words.iter().all(|w| t.repo.to_lowercase().contains(w) || t.about.to_lowercase().contains(w)));
    tools.truncate(limit);
    Listing { tools, offline: Some(updated) }
}

/// Tools made with anything-anything (it tags them with the topic when it publishes).
pub fn made(limit: usize) -> Result<Vec<Tool>> {
    search_live("topic:anything-anything", limit)
}

#[derive(Debug, Clone, Serialize)]
pub struct Way {
    pub label: String,
    pub command: String,
    /// Typed in Claude Code, not a shell.
    pub in_claude: bool,
}

fn raw(repo: &str, path: &str) -> Option<String> {
    let (status, body) = get(&format!("https://raw.githubusercontent.com/{repo}/HEAD/{path}")).ok()?;
    (status == 200).then_some(body)
}

fn exists(url: &str) -> bool {
    get(url).is_ok_and(|(s, _)| s == 200)
}

/// A repo's details and every way to install it, best first.
pub fn show(repo: &str) -> Result<(Tool, Vec<Way>)> {
    let repo = resolve(repo)?;
    let (status, body) = get(&format!("https://api.github.com/repos/{repo}"))?;
    if status != 200 {
        bail!("{repo}: GitHub answered {status}");
    }
    let v: Value = serde_json::from_str(&body)?;
    let tool = Tool {
        repo: v["full_name"].as_str().unwrap_or(&repo).to_string(),
        about: v["description"].as_str().unwrap_or_default().to_string(),
        stars: v["stargazers_count"].as_u64().unwrap_or(0),
        language: v["language"].as_str().unwrap_or_default().to_string(),
    };
    let (_, listing) = get(&format!("https://api.github.com/repos/{repo}/contents"))?;
    let root: Vec<String> = serde_json::from_str::<Value>(&listing)
        .ok()
        .and_then(|v| v.as_array().map(|a| a.iter().filter_map(|e| e["name"].as_str().map(String::from)).collect()))
        .unwrap_or_default();
    let has = |n: &str| root.iter().any(|r| r == n);
    let short = repo.rsplit('/').next().unwrap_or(&repo).to_string();
    let url = format!("https://github.com/{repo}");
    let mut ways = Vec::new();
    let mut way =
        |label: &str, command: String, in_claude: bool| ways.push(Way { label: label.into(), command, in_claude });

    let npm_name = |path: &str| -> Option<String> {
        let pkg: Value = serde_json::from_str(&raw(&repo, path)?).ok()?;
        if pkg["private"].as_bool() == Some(true) || pkg.get("bin").is_none() {
            return None;
        }
        let name = pkg["name"].as_str()?.to_string();
        exists(&format!("https://registry.npmjs.org/{}", name.replace('/', "%2F"))).then_some(name)
    };
    if let Some(name) = npm_name(&format!("npm/{short}/package.json"))
        .or_else(|| if has("package.json") { npm_name("package.json") } else { None })
    {
        way("npm", format!("npm i -g {name}"), false);
    }
    if has("pyproject.toml")
        && let Some(text) = raw(&repo, "pyproject.toml")
        && let Ok(doc) = text.parse::<toml::Table>()
        && let Some(name) = doc.get("project").and_then(|p| p.get("name")).and_then(|n| n.as_str())
    {
        if exists(&format!("https://pypi.org/pypi/{name}/json")) {
            way("PyPI", format!("uv tool install {name}"), false);
        } else {
            way("Python (from GitHub)", format!("uv tool install git+{url}"), false);
        }
    }
    if has("Cargo.toml")
        && let Some(text) = raw(&repo, "Cargo.toml")
        && let Ok(doc) = text.parse::<toml::Table>()
        && let Some(name) = doc.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str())
    {
        if exists(&format!("https://crates.io/api/v1/crates/{name}")) {
            way("crates.io", format!("cargo install {name}"), false);
        } else {
            way("Rust (from GitHub)", format!("cargo install --git {url}"), false);
        }
    }
    if has("go.mod") {
        way("Go", format!("go install github.com/{repo}@latest"), false);
    }
    if has(".claude-plugin") {
        way("Claude Code plugin", format!("/plugin marketplace add {repo}"), true);
    }
    way("source", format!("git clone {url}"), false);
    Ok((tool, ways))
}

/// "owner/repo" as it is; a bare name is looked up.
fn resolve(name: &str) -> Result<String> {
    if name.contains('/') {
        return Ok(name.trim_start_matches("https://github.com/").trim_end_matches('/').to_string());
    }
    let hits = search_live(&format!("{name} in:name"), 10).unwrap_or_default();
    hits.iter()
        .find(|t| t.repo.rsplit('/').next().is_some_and(|r| r.eq_ignore_ascii_case(name)))
        .or(hits.first())
        .map(|t| t.repo.clone())
        .or_else(|| {
            snapshot()
                .0
                .into_iter()
                .find(|t| t.repo.to_lowercase().ends_with(&format!("/{}", name.to_lowercase())))
                .map(|t| t.repo)
        })
        .with_context(|| format!("no -anything tool called {name}: try anything-anything hub search {name}"))
}

/// Installs with the best shell way, after asking (unless yes).
pub fn install(repo: &str, yes: bool) -> Result<()> {
    let (tool, ways) = show(repo)?;
    let way = ways.iter().find(|w| !w.in_claude).context("no way to install it from a shell")?;
    ui::line(format!("{} {}\n  {}", ui::bold(&tool.repo), ui::dim(util::clip(&tool.about, 90)), way.command));
    if !yes {
        use std::io::{BufRead, Write};
        print!("Run it? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            bail!("not installed");
        }
    }
    let argv: Vec<String> = way.command.split_whitespace().map(String::from).collect();
    let status = util::command(&argv).status().with_context(|| format!("couldn't run {}", argv[0]))?;
    anyhow::ensure!(status.success(), "{} exited with {status}", way.command);
    ui::ok(format!("installed {}", tool.repo));
    for w in ways.iter().filter(|w| w.in_claude) {
        ui::line(format!("  in Claude Code: {}", ui::bold(&w.command)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_list_parses_and_filters() {
        let (tools, updated) = snapshot();
        assert!(tools.len() > 10 && !updated.is_empty());
        let l = list("rust", 50, true);
        assert!(
            l.offline.is_some() && l.tools.iter().all(|t| (t.repo.clone() + &t.about).to_lowercase().contains("rust"))
        );
        assert_eq!(encode("a b/c"), "a+b%2Fc");
    }
}
