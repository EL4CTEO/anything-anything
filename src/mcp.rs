//! MCP server on stdio (JSON-RPC 2.0, one message per line), dual-era: 2026-07-28 clients discover it
//! and send the version with every request; older clients use the initialize handshake.
//! Here the calling agent is the builder: it gets the spec format and the task, and verify judges its work.

use crate::spec::{self, Spec};
use crate::{forge, gate, hub, scaffold, stack, state, task, ui, util};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::PathBuf;

pub const LATEST: &str = "2026-07-28";
pub const SUPPORTED: &[&str] = &["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26"];
const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";

const INSTRUCTIONS: &str = "anything-anything makes -anything tools (CLIs in Rust, Python or TypeScript, web apps and games, \
or any stack) and proves they work. You are the builder. Loop: anything_spec_format to learn anything.toml -> \
anything_create with the spec you wrote (or an idea and a stack) -> anything_task for what to build -> write the code \
-> anything_verify, fix what fails, repeat until every gate passes. Never edit anything.toml or the generated files \
to make a gate pass; if the spec is wrong, fix the spec with the user.";

fn tools() -> Value {
    let path = json!({ "type": "string", "description": "The project folder (default: the server's working folder)" });
    let tool = |name: &str, description: &str, properties: Value, required: Value| {
        json!({ "name": name, "description": description,
                "inputSchema": { "type": "object", "properties": properties, "required": required, "additionalProperties": false } })
    };
    json!([
        tool("anything_stacks", "The stacks a tool can be built on, with what each is for.", json!({}), json!([])),
        tool(
            "anything_spec_format",
            "The anything.toml reference: every field, with examples. Read it before writing a spec.",
            json!({}),
            json!([])
        ),
        tool(
            "anything_create",
            "Create a new tool folder from a spec (TOML text) or from an idea and a stack, with every generated file in place. Returns the build task.",
            json!({ "name": { "type": "string", "description": "What it does to anything, e.g. translate (becomes translate-anything)" },
                    "spec": { "type": "string", "description": "The full anything.toml" },
                    "idea": { "type": "string", "description": "One line, when there's no spec yet" },
                    "stack": { "type": "string", "description": "rust-cli, python-cli, node-cli, web or any" },
                    "dir": { "type": "string", "description": "The folder to create it in (default: ./<name>)" } }),
            json!(["name"])
        ),
        tool(
            "anything_task",
            "What's left to build, as a task: the commands, the examples to pass, what failed last time.",
            json!({ "path": path }),
            json!([])
        ),
        tool(
            "anything_sync",
            "Regenerate the files anything-anything owns from anything.toml (after editing the spec). Never touches your code.",
            json!({ "path": path }),
            json!([])
        ),
        tool(
            "anything_verify",
            "Run every gate: spec, generated files, stubs, the stack's steps, MCP, examples. Returns what failed and why. \
             Give examples to re-run only the ones you're fixing; finish with a run without it.",
            json!({ "path": path,
                    "examples": { "type": "array", "items": { "type": "string" },
                                  "description": "Only run the examples whose names contain one of these (any case). Default: all" } }),
            json!([])
        ),
        tool(
            "anything_status",
            "The spec, the last verify and every build so far.",
            json!({ "path": path }),
            json!([])
        ),
        tool(
            "anything_hub",
            "The most-starred -anything tools on GitHub, or a search.",
            json!({ "query": { "type": "string" }, "limit": { "type": "integer", "default": 20 } }),
            json!([])
        ),
    ])
}

fn root(args: &Value) -> PathBuf {
    util::absolute(&args["path"].as_str().map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")))
}

fn call(name: &str, args: &Value) -> Result<Value> {
    Ok(match name {
        "anything_stacks" => {
            json!(stack::all().into_iter().map(|(id, about)| json!({ "id": id, "about": about })).collect::<Vec<_>>())
        }
        "anything_spec_format" => json!({ "format": spec::REFERENCE }),
        "anything_create" => {
            let what = args["name"].as_str().context("name is required")?;
            let tmp_spec = match args["spec"].as_str() {
                Some(text) => {
                    Spec::parse(text)?;
                    let p = std::env::temp_dir().join(format!("anything-spec-{}.toml", std::process::id()));
                    std::fs::write(&p, text)?;
                    Some(p)
                }
                None => None,
            };
            let (root, _) = forge::create(
                &std::env::current_dir()?,
                forge::NewOpts {
                    what: what.into(),
                    idea: args["idea"].as_str().unwrap_or_default().into(),
                    stack: args["stack"].as_str().map(String::from),
                    spec: tmp_spec.clone(),
                    out: args["dir"].as_str().map(PathBuf::from),
                    owner: None,
                    npm: None,
                    color: None,
                    git: true,
                    agent: None,
                    build: false,
                },
            )?;
            if let Some(p) = tmp_spec {
                let _ = std::fs::remove_file(p);
            }
            let spec = Spec::load(&root)?;
            let def = stack::load(&spec)?;
            let problems = spec.problems(&def);
            json!({ "path": util::slashes(&root), "name": spec.name, "stack": spec.stack, "spec_problems": problems,
                    "task": if problems.is_empty() { task::build(&spec, &def, None) } else { task::design(&spec.name, &spec.pitch, Some(&spec.stack), &std::fs::read_to_string(root.join(spec::FILE))?) } })
        }
        "anything_task" => {
            let r = root(args);
            let p = forge::write_task(&r, &[])?;
            json!({ "task": std::fs::read_to_string(p)? })
        }
        "anything_sync" => {
            let r = root(args);
            let s = scaffold::sync(&r, &Spec::load(&r)?, false)?;
            json!({ "created": s.created, "updated": s.updated, "patched": s.patched, "unchanged": s.unchanged })
        }
        "anything_verify" => {
            let only: Vec<String> = args["examples"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default();
            serde_json::to_value(gate::verify_only(&root(args), &only)?)?
        }
        "anything_status" => {
            let r = root(args);
            let spec = Spec::load(&r)?;
            let s = state::load(&r);
            json!({ "name": spec.name, "stack": spec.stack, "commands": spec.commands.iter().map(|c| &c.name).collect::<Vec<_>>(),
                    "examples": spec.examples.len(), "last_verify": s.last, "runs": s.runs })
        }
        "anything_hub" => {
            let l = hub::list(
                args["query"].as_str().unwrap_or_default(),
                args["limit"].as_u64().unwrap_or(20) as usize,
                false,
            );
            json!({ "tools": l.tools, "offline_list_from": l.offline })
        }
        other => anyhow::bail!("unknown tool {other}"),
    })
}

fn progress_note(token: &Value, progress: u64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "notifications/progress", "params": { "progressToken": token, "progress": progress, "message": message } })
}

fn server_info() -> Value {
    json!({ "name": "anything-anything", "version": env!("CARGO_PKG_VERSION") })
}

fn error(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut e = json!({ "code": code, "message": message });
    if let Some(d) = data {
        e["data"] = d;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": e })
}

pub fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let params = &msg["params"];
    if let Some(v) = params["_meta"][VERSION_KEY].as_str()
        && !SUPPORTED.contains(&v)
    {
        return Some(error(
            id,
            -32022,
            "Unsupported protocol version",
            Some(json!({ "supported": SUPPORTED, "requested": v })),
        ));
    }
    let result = match msg["method"].as_str().unwrap_or_default() {
        "server/discover" => json!({
            "resultType": "complete",
            "supportedVersions": SUPPORTED,
            "capabilities": { "tools": {} },
            "instructions": INSTRUCTIONS,
            "ttlMs": 3_600_000,
            "cacheScope": "public",
            "_meta": { "io.modelcontextprotocol/serverInfo": server_info() },
        }),
        "initialize" => {
            let asked = params["protocolVersion"].as_str().unwrap_or("2025-11-25");
            let version = if SUPPORTED.contains(&asked) { asked } else { "2025-11-25" };
            json!({ "protocolVersion": version, "capabilities": { "tools": {} }, "serverInfo": server_info(), "instructions": INSTRUCTIONS })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "resultType": "complete", "tools": tools(), "ttlMs": 300_000, "cacheScope": "public" }),
        "tools/call" => {
            let token = &params["_meta"]["progressToken"];
            if token.is_string() || token.is_number() {
                let token = token.clone();
                let mut n = 0;
                ui::set_progress(Some(Box::new(move |m: &str| {
                    n += 1;
                    let mut out = std::io::stdout().lock();
                    let _ = writeln!(out, "{}", progress_note(&token, n, m));
                    let _ = out.flush();
                })));
            }
            let name = params["name"].as_str().unwrap_or_default();
            let known = tools().as_array().is_some_and(|t| t.iter().any(|t| t["name"] == name));
            if !known {
                ui::set_progress(None);
                return Some(error(id, -32602, &format!("unknown tool {name}"), None));
            }
            let result = call(name, &params["arguments"]);
            ui::set_progress(None);
            match result {
                Ok(v) => {
                    let text = v
                        .get("task")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| serde_json::to_string_pretty(&v).unwrap_or_default());
                    let mut r = json!({ "resultType": "complete", "content": [{ "type": "text", "text": text }], "isError": false });
                    r["structuredContent"] = v;
                    r
                }
                Err(e) => {
                    json!({ "resultType": "complete", "content": [{ "type": "text", "text": format!("{e:#}") }], "isError": true })
                }
            }
        }
        method => return Some(error(id, -32601, &format!("unknown method {method}"), None)),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

pub fn serve() -> Result<()> {
    ui::set_quiet(true);
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(e) => Some(error(Value::Null, -32700, &e.to_string(), None)),
        };
        if let Some(reply) = reply {
            let mut out = std::io::stdout().lock();
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_eras_and_errors() {
        let d = handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": { "_meta": { VERSION_KEY: LATEST } } })).unwrap();
        assert_eq!(d["result"]["supportedVersions"][0], LATEST);
        let i = handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } })).unwrap();
        assert_eq!(i["result"]["protocolVersion"], "2025-06-18");
        let l = handle(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" })).unwrap();
        assert_eq!(l["result"]["tools"].as_array().unwrap().len(), 8);
        let old = handle(&json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/list", "params": { "_meta": { VERSION_KEY: "1900-01-01" } } })).unwrap();
        assert_eq!(old["error"]["code"], -32022);
        assert!(handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
        let bad = handle(&json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "anything_status", "arguments": { "path": "/definitely/not/here" } } })).unwrap();
        assert_eq!(bad["result"]["isError"], true);
        let spec = handle(
            &json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "anything_spec_format" } }),
        )
        .unwrap();
        assert!(spec["result"]["content"][0]["text"].as_str().unwrap().contains("[[example]]"));
    }
}
