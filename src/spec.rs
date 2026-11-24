//! anything.toml: what the tool is, the commands it has, how it's built and run, and the examples
//! that prove it works. The one file a person (or an agent) writes; everything else follows from it.

use crate::names;
use crate::stack::StackDef;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const FILE: &str = "anything.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    String,
    Path,
    Int,
    Float,
    Bool,
    List,
}

impl Kind {
    pub fn json_type(self) -> &'static str {
        match self {
            Kind::String | Kind::Path => "string",
            Kind::Int => "integer",
            Kind::Float => "number",
            Kind::Bool => "boolean",
            Kind::List => "array",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arg {
    pub name: String,
    pub kind: Kind,
    #[serde(default)]
    pub help: String,
    /// Given without a flag: `tool scan src` instead of `tool scan --path src`.
    #[serde(default)]
    pub positional: bool,
    #[serde(default)]
    pub required: bool,
    /// As it would be typed on the command line; a list is comma-separated.
    #[serde(default)]
    pub default: Option<String>,
}

impl Arg {
    pub fn key(&self) -> String {
        names::snake(&self.name)
    }

    /// The default as a JSON value of the arg's kind (null when there is none).
    pub fn default_json(&self) -> Value {
        let Some(d) = &self.default else {
            return match self.kind {
                Kind::Bool => Value::Bool(false),
                Kind::List => Value::Array(Vec::new()),
                _ => Value::Null,
            };
        };
        match self.kind {
            Kind::String | Kind::Path => Value::String(d.clone()),
            Kind::Int => d.parse::<i64>().map(Value::from).unwrap_or(Value::Null),
            Kind::Float => d.parse::<f64>().map(Value::from).unwrap_or(Value::Null),
            Kind::Bool => Value::Bool(d == "true"),
            Kind::List => {
                Value::Array(d.split(',').map(str::trim).filter(|s| !s.is_empty()).map(Value::from).collect())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub name: String,
    pub about: String,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

impl Command {
    pub fn module(&self) -> String {
        names::snake(&self.name)
    }
}

/// How to run what was built. {root} is the project folder, {exe} is ".exe" on Windows, {python} the Python on PATH.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The CLI, as an argv. An example's args go after it.
    #[serde(default)]
    pub cmd: Vec<String>,
    /// A server to start for http examples. It gets the port in $PORT.
    #[serde(default)]
    pub serve: Vec<String>,
    /// The path that answers once the server is up (default "/").
    #[serde(default)]
    pub ready: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl Run {
    /// This run, with the blanks filled from another.
    pub fn or(&self, fallback: &Run) -> Run {
        let mut env = fallback.env.clone();
        env.extend(self.env.clone());
        Run {
            cmd: if self.cmd.is_empty() { fallback.cmd.clone() } else { self.cmd.clone() },
            serve: if self.serve.is_empty() { fallback.serve.clone() } else { self.serve.clone() },
            ready: if self.ready.is_empty() { fallback.ready.clone() } else { self.ready.clone() },
            env,
        }
    }
}

/// One gate of verify: a command that must exit 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub name: String,
    pub run: Vec<String>,
    /// Run the step only if this command works (`cargo clippy --version`); skipped otherwise.
    #[serde(default)]
    pub probe: Vec<String>,
    /// A formatter or fixer: it changes files, and never fails the gate.
    #[serde(default)]
    pub fix: bool,
    /// When it fails, the steps after it still run (a linter). Default: they're skipped.
    #[serde(default)]
    pub soft: bool,
    /// Seconds (default 900).
    #[serde(default)]
    pub timeout: u64,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub name: String,
    /// Run the CLI ([run].cmd) with these arguments...
    #[serde(default)]
    pub args: Vec<String>,
    /// ...or this shell line...
    #[serde(default)]
    pub sh: String,
    /// ...or this request to the server ([run].serve): "GET /api/items".
    #[serde(default)]
    pub http: String,
    /// The request body, for http.
    #[serde(default)]
    pub body: String,
    /// The HTTP status (default 200).
    #[serde(default)]
    pub status: Option<u16>,
    /// Files to create in the empty folder the example runs in.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub exit: i32,
    /// Text the output (or the response body) must contain.
    #[serde(default)]
    pub stdout: Vec<String>,
    #[serde(default)]
    pub not_stdout: Vec<String>,
    /// JSON pointer -> value, checked on the output of the same run with --json (or on the response body).
    #[serde(default)]
    pub json: BTreeMap<String, Value>,
    /// Files that must exist afterwards, and text each must contain.
    #[serde(default)]
    pub files_after: BTreeMap<String, String>,
    /// Seconds (default 60).
    #[serde(default)]
    pub timeout: u64,
}

impl Example {
    pub fn mode(&self) -> &'static str {
        if !self.http.is_empty() {
            "http"
        } else if !self.sh.is_empty() {
            "sh"
        } else {
            "cli"
        }
    }
}

fn first_version() -> String {
    "0.1.0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub name: String,
    pub pitch: String,
    #[serde(default)]
    pub tagline: String,
    /// When an agent should reach for it: the second half of the skill's description.
    #[serde(default)]
    pub when: String,
    #[serde(default = "first_version")]
    pub version: String,
    pub stack: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub npm: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// How it should work, for whoever builds it: approach, edge cases, what's out of scope.
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub run: Run,
    /// Replaces the stack's steps when given.
    #[serde(default, rename = "step")]
    pub steps: Vec<Step>,
    #[serde(default, rename = "command")]
    pub commands: Vec<Command>,
    #[serde(default, rename = "example")]
    pub examples: Vec<Example>,
}

impl Spec {
    pub fn parse(text: &str) -> Result<Spec> {
        toml::from_str(text).map_err(|e| anyhow::anyhow!("{FILE}: {}", e.to_string().trim()))
    }

    pub fn load(root: &Path) -> Result<Spec> {
        let path = root.join(FILE);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("no {FILE} in {}: run anything-anything new first", root.display()))?;
        Spec::parse(&text)
    }

    pub fn command(&self, name: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// MCP tool name of a command: translate_scan.
    pub fn tool(&self, command: &Command) -> String {
        let verb = self.name.strip_suffix("-anything").unwrap_or(&self.name);
        format!("{}_{}", names::snake(verb), command.module())
    }

    pub fn steps<'a>(&'a self, stack: &'a StackDef) -> &'a [Step] {
        if self.steps.is_empty() { &stack.steps } else { &self.steps }
    }

    pub fn run_def(&self, stack: &StackDef) -> Run {
        self.run.or(&stack.run)
    }

    /// Everything wrong with the spec, as sentences. Empty: it's good to build.
    pub fn problems(&self, stack: &StackDef) -> Vec<String> {
        let mut p = Vec::new();
        if !names::kebab_ok(&self.name) || !self.name.ends_with("-anything") || self.name == "anything-anything" {
            p.push(format!("name {:?} must be lower-kebab and end in -anything, like translate-anything", self.name));
        }
        if self.pitch.trim().is_empty() {
            p.push("pitch is empty: say in one sentence what it does".into());
        } else if self.pitch.chars().count() > 220 {
            p.push("pitch is over 220 characters: it's the one-line description everywhere".into());
        }
        let v: Vec<&str> = self.version.split('.').collect();
        if v.len() != 3 || v.iter().any(|n| n.parse::<u32>().is_err()) {
            p.push(format!("version {:?} must look like 0.1.0", self.version));
        }
        if !self.color.is_empty() && names::rgb(&self.color).is_none() {
            p.push(format!("color {:?} must be #rrggbb", self.color));
        }
        if !self.owner.is_empty() && !self.owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            p.push(format!("owner {:?} isn't a GitHub user or organization name", self.owner));
        }
        self.command_problems(stack, &mut p);
        let run = self.run_def(stack);
        for (i, s) in self.steps(stack).iter().enumerate() {
            if s.name.trim().is_empty() || s.run.is_empty() {
                p.push(format!("step {} needs a name and a run command", i + 1));
            }
        }
        if stack.id == "any" && self.steps.is_empty() {
            p.push("the any stack has no steps of its own: add [[step]]s that build and test it".into());
        }
        if self.examples.is_empty() {
            p.push("no [[example]]s: examples are the proof it works, add at least one".into());
        }
        let mut names = BTreeSet::new();
        for e in &self.examples {
            let label = format!("example {:?}", e.name);
            if e.name.trim().is_empty() {
                p.push("an example has no name".into());
            } else if !names.insert(e.name.to_lowercase()) {
                p.push(format!("{label} is there twice"));
            }
            let modes = [!e.args.is_empty(), !e.sh.is_empty(), !e.http.is_empty()].iter().filter(|b| **b).count();
            if modes != 1 {
                p.push(format!("{label} needs exactly one of args, sh or http"));
                continue;
            }
            match e.mode() {
                "cli" => {
                    if run.cmd.is_empty() {
                        p.push(format!("{label} uses args, but there's no [run] cmd to run them with"));
                    }
                    if !self.commands.is_empty() && self.command(&e.args[0]).is_none() {
                        p.push(format!("{label} runs {:?}, which isn't a command", e.args[0]));
                    }
                }
                "http" => {
                    if run.serve.is_empty() {
                        p.push(format!("{label} sends a request, but there's no [run] serve to start"));
                    }
                    let mut parts = e.http.split_whitespace();
                    let method = parts.next().unwrap_or_default();
                    let path = parts.next().unwrap_or_default();
                    if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") || !path.starts_with('/') {
                        p.push(format!("{label}: http must look like \"GET /path\""));
                    }
                    if !e.files.is_empty() || e.stdin.is_some() || !e.files_after.is_empty() {
                        p.push(format!("{label}: files, stdin and files_after don't apply to http"));
                    }
                }
                _ => {}
            }
            if e.mode() != "http" && (e.status.is_some() || !e.body.is_empty()) {
                p.push(format!("{label}: status and body are for http examples"));
            }
            if !(0..=255).contains(&e.exit) {
                p.push(format!("{label}: exit must be 0-255"));
            }
            for path in e.files.keys().chain(e.files_after.keys()) {
                if path.starts_with('/') || path.contains(':') || path.split(['/', '\\']).any(|s| s == "..") {
                    p.push(format!("{label}: {path:?} must be a relative path inside the example folder"));
                }
            }
            for pointer in e.json.keys() {
                if !pointer.is_empty() && !pointer.starts_with('/') {
                    p.push(format!("{label}: json key {pointer:?} must be a JSON pointer like /items/0/name"));
                }
            }
            if e.stdout.is_empty() && e.json.is_empty() && e.files_after.is_empty() && e.not_stdout.is_empty() {
                p.push(format!("{label} checks nothing but the exit code: add stdout, json or files_after"));
            }
        }
        p
    }

    fn command_problems(&self, stack: &StackDef, p: &mut Vec<String>) {
        if !stack.commands {
            if !self.commands.is_empty() {
                p.push(format!("the {} stack has no [[command]]s: describe it in notes and examples", stack.id));
            }
            return;
        }
        if self.commands.is_empty() {
            p.push("no [[command]]s: a CLI needs at least one".into());
        }
        let mut seen = BTreeSet::new();
        for c in &self.commands {
            let label = format!("command {:?}", c.name);
            if !names::kebab_ok(&c.name) || names::reserved(&c.name) || matches!(c.name.as_str(), "mcp" | "help") {
                p.push(format!("{label}: lower-kebab, not mcp or help, not a keyword in Rust/Python/JS"));
            }
            if !seen.insert(c.module()) {
                p.push(format!("{label} is there twice"));
            }
            if c.about.trim().is_empty() {
                p.push(format!("{label} has no about"));
            }
            if !self.examples.iter().any(|e| e.args.first() == Some(&c.name)) {
                p.push(format!("{label} has no example: every command needs one, it's the proof it works"));
            }
            let mut args = BTreeSet::new();
            let mut optional_positional = false;
            let positional: Vec<&Arg> = c.args.iter().filter(|a| a.positional).collect();
            for a in &c.args {
                let at = format!("{label}, arg {:?}", a.name);
                if !names::kebab_ok(&a.name) || names::reserved(&a.name) || matches!(a.name.as_str(), "json" | "help" | "version") {
                    p.push(format!("{at}: lower-kebab, not json/help/version, not a keyword in Rust/Python/JS"));
                }
                if !args.insert(a.key()) {
                    p.push(format!("{at} is there twice"));
                }
                if a.kind == Kind::Bool && (a.positional || a.default.is_some() || a.required) {
                    p.push(format!("{at}: a bool is a flag: not positional, no default, not required"));
                }
                if a.required && a.default.is_some() {
                    p.push(format!("{at}: required and a default don't go together"));
                }
                if let Some(d) = &a.default {
                    let ok = match a.kind {
                        Kind::Int => d.parse::<i64>().is_ok(),
                        Kind::Float => d.parse::<f64>().is_ok(),
                        _ => true,
                    };
                    if !ok {
                        p.push(format!("{at}: default {d:?} isn't a {}", a.kind.json_type()));
                    }
                }
                if a.positional {
                    let optional = !a.required;
                    if optional_positional && !optional && a.kind != Kind::List {
                        p.push(format!("{at}: a required positional can't come after an optional one"));
                    }
                    optional_positional |= optional;
                }
            }
            if positional.iter().rev().skip(1).any(|a| a.kind == Kind::List) {
                p.push(format!("{label}: only the last positional arg can be a list"));
            }
        }
    }
}

/// The format, for people and agents: `anything-anything spec`, the design task, the MCP server.
pub const REFERENCE: &str = include_str!("../docs/spec.md");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack;

    const GOOD: &str = r#"
name = "greet-anything"
pitch = "Greets anyone."
stack = "python-cli"

[[command]]
name = "hello"
about = "Say hello"
[[command.arg]]
name = "who"
kind = "string"
positional = true
required = true

[[example]]
name = "hello world"
args = ["hello", "world"]
stdout = ["Hello, world!"]
json = { "/greeting" = "Hello, world!" }
"#;

    #[test]
    fn a_good_spec_has_no_problems() {
        let spec = Spec::parse(GOOD).unwrap();
        let def = stack::load(&spec).unwrap();
        assert_eq!(spec.problems(&def), Vec::<String>::new());
        assert_eq!(spec.tool(&spec.commands[0]), "greet_hello");
    }

    #[test]
    fn problems_are_specific() {
        let bad = GOOD.replace("args = [\"hello\", \"world\"]", "args = [\"wave\"]").replace("name = \"who\"", "name = \"type\"");
        let spec = Spec::parse(&bad).unwrap();
        let def = stack::load(&spec).unwrap();
        let p = spec.problems(&def).join("\n");
        assert!(p.contains("\"wave\", which isn't a command"), "{p}");
        assert!(p.contains("command \"hello\" has no example"), "{p}");
        assert!(p.contains("arg \"type\""), "{p}");
        assert!(Spec::parse("name = 1").is_err());
        assert!(Spec::parse(&format!("{GOOD}\nsurprise = 1")).is_err(), "unknown keys are refused");
    }

    #[test]
    fn defaults_as_json() {
        let a = |kind, d: &str| Arg { name: "x".into(), kind, help: String::new(), positional: false, required: false, default: Some(d.into()) };
        assert_eq!(a(Kind::List, "TODO, FIXME").default_json(), serde_json::json!(["TODO", "FIXME"]));
        assert_eq!(a(Kind::Int, "3").default_json(), serde_json::json!(3));
    }
}
