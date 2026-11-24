//! Fills the template keys: the tool's identity (base), and what follows from its commands
//! (tables for the README and skill, MCP tool schemas, the manifest, Rust code for the rust stack).

use crate::names::{self, Meta};
use crate::render::Ctx;
use crate::spec::{Arg, Command, Kind, Spec};
use crate::stack::StackDef;
use crate::util;
use anyhow::Result;
use serde_json::{Value, json};
use std::fmt::Write as _;

fn rust_lit(s: &str) -> String {
    format!("{s:?}")
}

fn json_lit(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// A Rust raw string literal with enough #s for its content.
fn rust_raw(s: &str) -> String {
    let mut hashes = String::from("#");
    while s.contains(&format!("\"{hashes}")) {
        hashes.push('#');
    }
    format!("r{hashes}\"{s}\"{hashes}")
}

/// The keys every template can use, from the spec alone.
pub fn base(spec: &Spec, meta: &Meta) -> Ctx {
    let mut c = Ctx::new();
    let mut put = |k: &str, v: String| {
        c.insert(k.to_string(), v);
    };
    let description = if spec.when.is_empty() { spec.pitch.clone() } else { format!("{} {}", spec.pitch, spec.when) };
    put("name", meta.name.clone());
    put("verb", meta.verb.clone());
    put("snake", meta.snake.clone());
    put("prefix", meta.prefix.clone());
    put("title", meta.title.clone());
    put("pitch", spec.pitch.clone());
    put("tagline", meta.tagline.clone());
    put("version", spec.version.clone());
    put("owner", meta.owner.clone());
    put("repo", meta.repo.clone());
    put("repo_url", meta.repo_url.clone());
    put("npm", meta.npm.clone());
    put("color", meta.color.clone());
    put("color_hex", meta.color.trim_start_matches('#').to_string());
    put("color_rgb", format!("{};{};{}", meta.rgb.0, meta.rgb.1, meta.rgb.2));
    put("year", util::year().to_string());
    put("home", names::HOME.to_string());
    put("stack", spec.stack.clone());
    put("pitch_json", json_lit(&spec.pitch));
    put("tagline_json", json_lit(&meta.tagline));
    put("description_json", json_lit(&description));
    put("pitch_rs", rust_lit(&spec.pitch));
    put("keywords_json", serde_json::to_string(&meta.keywords).unwrap_or_default());
    c
}

fn usage(a: &Arg) -> String {
    let value = format!("<{}>", a.name);
    match (a.positional, a.kind) {
        (true, Kind::List) => format!("[{}...]", a.name),
        (true, _) if a.required => value,
        (true, _) => format!("[{}]", a.name),
        (false, Kind::Bool) => format!("[--{}]", a.name),
        (false, _) if a.required => format!("--{} {value}", a.name),
        (false, _) => format!("[--{} {value}]", a.name),
    }
}

fn synopsis(spec: &Spec, c: &Command) -> String {
    let mut s = format!("{} {}", spec.name, c.name);
    for a in &c.args {
        s.push(' ');
        s.push_str(&usage(a));
    }
    s
}

fn tool_schema(spec: &Spec, c: &Command) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for a in &c.args {
        let mut p = json!({ "type": a.kind.json_type() });
        if a.kind == Kind::List {
            p["items"] = json!({ "type": "string" });
        }
        if !a.help.is_empty() {
            p["description"] = json!(a.help);
        }
        if a.default.is_some() {
            p["default"] = a.default_json();
        }
        if a.required {
            required.push(json!(a.key()));
        }
        properties.insert(a.key(), p);
    }
    json!({
        "name": spec.tool(c),
        "description": c.about,
        "inputSchema": { "type": "object", "properties": properties, "required": required, "additionalProperties": false },
    })
}

pub fn tools(spec: &Spec) -> Vec<Value> {
    spec.commands.iter().map(|c| tool_schema(spec, c)).collect()
}

fn instructions(spec: &Spec) -> String {
    let names: Vec<String> = spec.commands.iter().map(|c| spec.tool(c)).collect();
    format!("{}: {} Tools: {}. Every tool returns text for people and, where it has data, structured content.", spec.name, spec.pitch, names.join(", "))
}

fn manifest(spec: &Spec) -> Value {
    json!({
        "name": spec.name,
        "version": spec.version,
        "pitch": spec.pitch,
        "instructions": instructions(spec),
        "commands": spec.commands.iter().map(|c| json!({
            "name": c.name,
            "module": c.module(),
            "about": c.about,
            "tool": spec.tool(c),
            "args": c.args.iter().map(|a| json!({
                "name": a.name, "key": a.key(), "kind": a.kind, "help": a.help,
                "positional": a.positional, "required": a.required, "default": a.default_json(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "tools": tools(spec),
    })
}

/// Every key: base plus what follows from the commands and the stack.
pub fn slots(spec: &Spec, meta: &Meta, def: &StackDef, mut c: Ctx) -> Result<Ctx> {
    let mut table = String::from("| command | what it does |\n|---|---|\n");
    let mut skill = String::new();
    for cmd in &spec.commands {
        let _ = writeln!(table, "| `{}` | {} |", synopsis(spec, cmd).replace('|', "\\|"), cmd.about);
        let _ = writeln!(skill, "- `{}`: {}", synopsis(spec, cmd), cmd.about);
        for a in &cmd.args {
            if !a.help.is_empty() {
                let _ = writeln!(skill, "  - `{}`: {}", usage(a), a.help);
            }
        }
    }
    let tool_names: Vec<String> = spec.commands.iter().map(|cmd| format!("`{}`", spec.tool(cmd))).collect();
    let example = spec
        .examples
        .iter()
        .find(|e| !e.args.is_empty())
        .map(|e| format!("{} {}", spec.name, e.args.join(" ")))
        .unwrap_or_else(|| spec.name.clone());
    let tools_json = serde_json::to_string_pretty(&tools(spec))?;
    let manifest_json = serde_json::to_string_pretty(&manifest(spec))?;
    let mut put = |k: &str, v: String| {
        c.insert(k.to_string(), v);
    };
    put("commands_table_md", table.trim_end().to_string());
    put("skill_commands_md", skill.trim_end().to_string());
    put("mcp_tools_md", tool_names.join(", "));
    put("tool_count", spec.commands.len().to_string());
    put("usage", example);
    put("install", crate::render::render(&def.install, &base(spec, meta))?);
    put("manifest_json", manifest_json);
    put("instructions_json", json_lit(&instructions(spec)));
    if def.codegen == "rust" {
        rust(spec, &mut c, &tools_json);
    }
    Ok(c)
}

/// Keys for one command's own file.
pub fn command_slots(spec: &Spec, cmd: &Command, c: &mut Ctx) {
    let mut args = String::new();
    for a in &cmd.args {
        let _ = writeln!(args, "  - `{}` ({}{}): {}", a.key(), a.kind.json_type(), if a.required { ", required" } else { "" }, a.help);
    }
    c.insert("command".into(), cmd.name.clone());
    c.insert("command_module".into(), cmd.module());
    c.insert("command_pascal".into(), names::pascal(&cmd.name));
    c.insert("command_about".into(), cmd.about.clone());
    c.insert("command_tool".into(), spec.tool(cmd));
    c.insert("command_synopsis".into(), synopsis(spec, cmd));
    c.insert("command_args_md".into(), if args.is_empty() { "  (none)".into() } else { args.trim_end().to_string() });
}

// ---- rust ----

fn rust_type(a: &Arg) -> String {
    let base = match a.kind {
        Kind::String => "String",
        Kind::Path => "std::path::PathBuf",
        Kind::Int => "i64",
        Kind::Float => "f64",
        Kind::Bool => return "bool".into(),
        Kind::List => return "Vec<String>".into(),
    };
    if a.required || a.default.is_some() { base.to_string() } else { format!("Option<{base}>") }
}

/// The Rust expression for an arg's default, None when Default::default() is the same thing.
fn rust_default(a: &Arg) -> Option<String> {
    let d = a.default.as_ref()?;
    match a.kind {
        Kind::String if !d.is_empty() => Some(format!("{}.to_string()", rust_lit(d))),
        Kind::Path if !d.is_empty() => Some(format!("std::path::PathBuf::from({})", rust_lit(d))),
        Kind::Int if d.parse::<i64>().ok() != Some(0) => Some(d.clone()),
        Kind::Float if d.parse::<f64>().ok() != Some(0.0) => {
            let v: f64 = d.parse().ok()?;
            Some(format!("{v:?}"))
        }
        Kind::List => {
            let items: Vec<String> = d.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("{}.to_string()", rust_lit(s))).collect();
            (!items.is_empty()).then(|| format!("vec![{}]", items.join(", ")))
        }
        _ => None,
    }
}

fn rust_attrs(a: &Arg) -> String {
    let mut attrs: Vec<String> = Vec::new();
    if !a.positional {
        attrs.push("long".into());
    }
    if a.kind == Kind::List {
        attrs.push("value_delimiter = ','".into());
    }
    if let Some(d) = &a.default {
        attrs.push(format!("default_value = {}", rust_lit(d)));
    }
    if a.positional && a.kind == Kind::List && a.required {
        attrs.push("required = true".into());
    }
    attrs.join(", ")
}

fn rust(spec: &Spec, c: &mut Ctx, tools_json: &str) {
    let (mut variants, mut dispatch, mut mods, mut args, mut calls) =
        (String::new(), String::new(), String::new(), String::new(), String::new());
    for cmd in &spec.commands {
        let p = names::pascal(&cmd.name);
        let m = cmd.module();
        let _ = writeln!(variants, "    /// {}\n    #[command(name = {})]\n    {p}(commands::args::{p}Args),", cmd.about, rust_lit(&cmd.name));
        let _ = writeln!(dispatch, "        Cmd::{p}(args) => commands::{m}::run(&args),");
        let _ = writeln!(mods, "pub mod {m};");
        let _ = writeln!(calls, "        {} => call::<commands::args::{p}Args>(arguments, commands::{m}::run),", rust_lit(&spec.tool(cmd)));
        let defaults: Vec<(String, Option<String>)> = cmd.args.iter().map(|a| (a.key(), rust_default(a))).collect();
        let derive_default = defaults.iter().all(|(_, d)| d.is_none());
        let _ = writeln!(args, "/// `{}`: {}", synopsis(spec, cmd), cmd.about);
        let _ = writeln!(
            args,
            "#[derive(clap::Args, serde::Deserialize, Debug, Clone{})]\n#[serde(default, deny_unknown_fields)]\npub struct {p}Args {{",
            if derive_default { ", Default" } else { "" }
        );
        for a in &cmd.args {
            for line in a.help.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(args, "    /// {}", line.trim());
            }
            let attrs = rust_attrs(a);
            if !attrs.is_empty() {
                let _ = writeln!(args, "    #[arg({attrs})]");
            }
            let _ = writeln!(args, "    pub {}: {},", a.key(), rust_type(a));
        }
        args.push_str("}\n\n");
        if !derive_default {
            let _ = writeln!(args, "impl Default for {p}Args {{\n    fn default() -> Self {{\n        {p}Args {{");
            for (key, d) in &defaults {
                let _ = writeln!(args, "            {key}: {},", d.clone().unwrap_or_else(|| "Default::default()".into()));
            }
            args.push_str("        }\n    }\n}\n\n");
        }
    }
    let names: Vec<String> = spec.commands.iter().map(|cmd| rust_lit(&spec.tool(cmd))).collect();
    c.insert("rust_variants".into(), variants.trim_end().to_string());
    c.insert("rust_dispatch".into(), dispatch.trim_end().to_string());
    c.insert("rust_mods".into(), mods.trim_end().to_string());
    c.insert("rust_args".into(), args.trim_end().to_string());
    c.insert("rust_mcp_calls".into(), calls.trim_end().to_string());
    c.insert("rust_tools".into(), rust_raw(tools_json));
    c.insert("rust_tool_names".into(), names.join(", "));
    c.insert("rust_instructions".into(), rust_lit(&instructions(spec)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_strings_fit_their_content() {
        assert_eq!(rust_raw("a"), "r#\"a\"#");
        assert_eq!(rust_raw("x\"#y"), "r##\"x\"#y\"##");
    }
}
