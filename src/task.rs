//! The two prompts: DESIGN.md (an idea becomes anything.toml) and TASK.md (anything.toml becomes a
//! working tool). Plain markdown, so any agent, or a person, can do them.

use crate::gate::{Report, Status};
use crate::spec::{self, Spec};
use crate::stack::{self, Owner, StackDef};
use std::fmt::Write as _;

pub fn design(name: &str, idea: &str, stack: Option<&str>, current: &str) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Design task: {name}\n");
    let _ = writeln!(md, "Turn this idea into `{}`, the spec anything-anything builds from:\n", spec::FILE);
    let _ = writeln!(md, "> {}\n", idea.trim());
    let _ = writeln!(md, "## Stacks\n");
    for (id, about) in stack::all() {
        let _ = writeln!(md, "- `{id}`: {about}");
    }
    match stack {
        Some(s) => {
            let _ = writeln!(md, "\nUse the `{s}` stack: the user picked it.");
        }
        None => {
            let _ = writeln!(
                md,
                "\nPick the stack that fits the idea best. A tool for developers or agents is usually a CLI (pick the \
language its users would install it with). Something people look at or play in a browser is `web`. Anything that \
needs another language, engine or runtime is `any`: then you write its [[step]]s and [run] yourself, using the \
latest stable versions of everything."
            );
        }
    }
    let _ = writeln!(md, "\n## What makes a good spec\n");
    let _ = writeln!(md, "- It does one thing very well, for anything: any language, any file, any input. That's the -anything promise.");
    let _ = writeln!(md, "- 1-6 commands with short, obvious names. The main one is what people will type first.");
    let _ = writeln!(md, "- `notes` says how it works: the approach, the edge cases, what's out of scope. It's the brief for whoever builds it.");
    let _ = writeln!(md, "- Examples are the proof. At least two per command (CLI) or five in total (web/any): the normal case, an edge case, an error.");
    let _ = writeln!(md, "  They must be offline and deterministic, and check real behaviour with exact text or json values, not just that something printed.");
    let _ = writeln!(md, "- `when` finishes the sentence \"Use when...\" for an agent deciding whether to use the tool.");
    let _ = writeln!(md, "- Keep the name: `{name}`. Leave owner, npm and version as they are.\n");
    let _ = writeln!(md, "## Rules\n");
    let _ = writeln!(md, "- Only write `{}`. Don't create any other file.", spec::FILE);
    let _ = writeln!(md, "- Check it with `anything-anything verify`: its first gate, spec, must pass (the build gates will fail, nothing is built yet).\n");
    let _ = writeln!(md, "## The current {}\n\n```toml\n{}\n```\n", spec::FILE, current.trim());
    let _ = writeln!(md, "## The format\n\n{}", spec::REFERENCE);
    md
}

pub fn build(spec: &Spec, def: &StackDef, last: Option<&Report>) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Build task: {}\n", spec.name);
    let _ = writeln!(md, "{}\n", spec.pitch);
    if !spec.notes.trim().is_empty() {
        let _ = writeln!(md, "## How it should work\n\n{}\n", spec.notes.trim());
    }
    let _ = writeln!(md, "## The stack: {}\n\n{}\n", def.id, def.brief.trim());
    if def.commands {
        let _ = writeln!(md, "## Commands\n");
        for c in &spec.commands {
            let _ = writeln!(md, "- `{}`: {}", c.name, c.about);
            for a in &c.args {
                let _ = writeln!(
                    md,
                    "  - `{}` ({}{}{}){}",
                    a.key(),
                    serde_json::to_string(&a.kind).unwrap_or_default().trim_matches('"'),
                    if a.positional { ", positional" } else { "" },
                    a.default.as_ref().map(|d| format!(", default {d:?}")).unwrap_or_default(),
                    if a.help.is_empty() { String::new() } else { format!(": {}", a.help) }
                );
            }
        }
        md.push('\n');
    }
    let _ = writeln!(md, "## Examples it must pass\n");
    let _ = writeln!(md, "Each runs in a fresh empty folder holding only its `files`. `stdout` items must appear in the output, `json` keys are JSON pointers into the output of the same command with `--json`.\n");
    let _ = writeln!(md, "```toml");
    for e in &spec.examples {
        if let Ok(t) = toml::to_string(&ExampleView(e)) {
            let _ = writeln!(md, "[[example]]\n{}", t.trim());
        }
    }
    let _ = writeln!(md, "```\n");
    if let Ok((_, files)) = stack::files(spec) {
        let shell: Vec<String> = files.iter().filter(|f| f.owner == Owner::Shell).map(|f| format!("`{}`", f.path)).collect();
        let _ = writeln!(md, "## Don't edit\n");
        let _ = writeln!(md, "`{}` and the files generated from it: {}. They're put back after you finish, and verify fails if they differ.", spec::FILE, shell.join(", "));
        let _ = writeln!(md, "If the spec itself is wrong (an example can't be right), say so in your final reply instead of working around it.\n");
    }
    let _ = writeln!(md, "## Rules\n");
    let _ = writeln!(md, "- Build it for real: complete, correct, fast, with clear errors. No stubs, no placeholders, no `{}` left anywhere.", crate::gate::MARKER);
    let _ = writeln!(md, "- Use the latest stable versions of the language, libraries and tools. Prefer the standard library; add a dependency only when it clearly earns its place.");
    let _ = writeln!(md, "- Works the same on Windows, macOS and Linux. Print paths with forward slashes.");
    let _ = writeln!(md, "- Offline unless the spec says otherwise. Deterministic output (sorted, stable).");
    let _ = writeln!(md, "- Add your own unit tests for the core logic, next to the stack's.");
    let _ = writeln!(md, "- Update README.md (concise: what it does, install, usage, one real example) and CHANGELOG.md.");
    let _ = writeln!(md, "- Run `anything-anything verify` after each piece of work. You're done when every gate passes.\n");
    if let Some(r) = last.filter(|r| !r.ok) {
        let _ = writeln!(md, "## What failed last time\n");
        for g in r.gates.iter().filter(|g| g.status == Status::Fail) {
            let detail: String = g.detail.lines().take(80).collect::<Vec<_>>().join("\n");
            let _ = writeln!(md, "### {}\n\n```\n{}\n```\n", g.name, detail);
        }
    }
    let _ = writeln!(md, "When every gate passes, reply with what you built in a few lines.");
    md
}

/// An example as TOML for the task, without the empty fields.
struct ExampleView<'a>(&'a spec::Example);

impl serde::Serialize for ExampleView<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let e = self.0;
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("name", &e.name)?;
        if !e.args.is_empty() {
            m.serialize_entry("args", &e.args)?;
        }
        if !e.sh.is_empty() {
            m.serialize_entry("sh", &e.sh)?;
        }
        if !e.http.is_empty() {
            m.serialize_entry("http", &e.http)?;
        }
        if !e.body.is_empty() {
            m.serialize_entry("body", &e.body)?;
        }
        if let Some(st) = e.status {
            m.serialize_entry("status", &st)?;
        }
        if let Some(stdin) = &e.stdin {
            m.serialize_entry("stdin", stdin)?;
        }
        if e.exit != 0 {
            m.serialize_entry("exit", &e.exit)?;
        }
        if !e.stdout.is_empty() {
            m.serialize_entry("stdout", &e.stdout)?;
        }
        if !e.not_stdout.is_empty() {
            m.serialize_entry("not_stdout", &e.not_stdout)?;
        }
        if !e.files.is_empty() {
            m.serialize_entry("files", &e.files)?;
        }
        if !e.json.is_empty() {
            m.serialize_entry("json", &e.json)?;
        }
        if !e.files_after.is_empty() {
            m.serialize_entry("files_after", &e.files_after)?;
        }
        m.end()
    }
}
