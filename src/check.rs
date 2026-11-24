//! Runs the examples: each in a fresh empty folder with its files, against the CLI, a shell line,
//! or the server. Whatever the stack, this is what "it works" means.

use crate::spec::{Example, Run, Spec};
use crate::util;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleResult {
    pub name: String,
    pub ok: bool,
    pub secs: f64,
    /// What was wrong, with the output that shows it. Empty when it passed.
    pub problem: String,
}

pub struct Captured {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Text as compared: one kind of newline, one kind of slash.
fn norm(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\\', "/")
}

fn env_of(run: &Run, root: &Path, port: Option<u16>) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = run.env.iter().map(|(k, v)| (k.clone(), expand(v, root, port))).collect();
    env.insert("ANYTHING_ROOT".into(), util::slashes(root));
    env.insert("NO_COLOR".into(), "1".into());
    if let Some(p) = port {
        env.insert("PORT".into(), p.to_string());
    }
    env
}

fn expand(s: &str, root: &Path, port: Option<u16>) -> String {
    let s = util::expand(s, root);
    match port {
        Some(p) => s.replace("{port}", &p.to_string()),
        None => s,
    }
}

/// Runs argv in dir, feeding stdin, killing it after timeout.
pub fn exec(
    argv: &[String],
    dir: &Path,
    env: &BTreeMap<String, String>,
    stdin: Option<&str>,
    timeout: Duration,
) -> Result<Captured> {
    let mut child = util::command(argv)
        .current_dir(dir)
        .envs(env)
        .env_remove("CLAUDECODE")
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("couldn't start {}", argv[0]))?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let text = text.to_string();
        std::thread::spawn(move || {
            let _ = pipe.write_all(text.as_bytes());
        });
    }
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            String::from_utf8_lossy(&buf).into_owned()
        })
    };
    let out = reader(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let err = reader(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let start = Instant::now();
    let (code, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status.code(), false);
        }
        if start.elapsed() > timeout {
            util::kill_tree(&mut child);
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok(Captured { code, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default(), timed_out })
}

struct Sandbox(PathBuf);

impl Sandbox {
    fn new(e: &Example) -> Result<Sandbox> {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "anything-example-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        for (rel, text) in &e.files {
            let p = dir.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, text)?;
        }
        Ok(Sandbox(dir))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn show(c: &Captured) -> String {
    let cut = |s: &str| util::tail(s, 30);
    format!(
        "exit: {}\nstdout:\n{}\nstderr:\n{}",
        c.code.map_or("none (killed)".into(), |c| c.to_string()),
        cut(&c.stdout),
        cut(&c.stderr)
    )
}

fn check_text(e: &Example, text: &str, problems: &mut Vec<String>) {
    let seen = norm(text);
    for want in &e.stdout {
        if !seen.contains(&norm(want)) {
            problems.push(format!("the output should contain {want:?}"));
        }
    }
    for not in &e.not_stdout {
        if seen.contains(&norm(not)) {
            problems.push(format!("the output should not contain {not:?}"));
        }
    }
}

fn check_json(e: &Example, text: &str, problems: &mut Vec<String>) {
    if e.json.is_empty() {
        return;
    }
    let value: Value = match serde_json::from_str(text.trim()) {
        Ok(v) => v,
        Err(err) => {
            problems.push(format!("the --json output isn't JSON ({err}):\n{}", util::tail(text, 15)));
            return;
        }
    };
    for (pointer, want) in &e.json {
        match value.pointer(pointer) {
            Some(got) if same(got, want) => {}
            Some(got) => problems.push(format!("json {pointer:?} is {got}, should be {want}")),
            None => problems.push(format!("json {pointer:?} is missing")),
        }
    }
}

/// Equal, with 3 == 3.0 and slashes normalized in strings.
fn same(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => norm(a) == norm(b),
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y)),
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| same(v, w)))
        }
        _ => got == want,
    }
}

fn check_files(e: &Example, dir: &Path, problems: &mut Vec<String>) {
    for (rel, want) in &e.files_after {
        match std::fs::read_to_string(dir.join(rel)) {
            Ok(text) if norm(&text).contains(&norm(want)) => {}
            Ok(text) => problems.push(format!("{rel} should contain {want:?}, it has:\n{}", util::tail(&text, 10))),
            Err(_) => problems.push(format!("{rel} should exist afterwards")),
        }
    }
}

fn timeout(e: &Example) -> Duration {
    Duration::from_secs(if e.timeout == 0 { 60 } else { e.timeout })
}

/// One run of an example (args or sh), checked. with_json: append --json and check the json keys.
fn process(root: &Path, run: &Run, e: &Example, with_json: bool) -> Result<Vec<String>> {
    let sandbox = Sandbox::new(e)?;
    let argv: Vec<String> = if e.mode() == "sh" {
        vec![expand(&e.sh, root, None)]
    } else {
        let mut a: Vec<String> = run.cmd.iter().map(|s| expand(s, root, None)).collect();
        a.extend(e.args.iter().cloned());
        if with_json {
            a.push("--json".into());
        }
        a
    };
    let c = exec(&argv, &sandbox.0, &env_of(run, root, None), e.stdin.as_deref(), timeout(e))?;
    let mut problems = Vec::new();
    if c.timed_out {
        problems.push(format!("it didn't finish in {}s", timeout(e).as_secs()));
    } else if c.code != Some(e.exit) {
        problems.push(format!("it exited {}, should be {}", c.code.map_or("?".into(), |c| c.to_string()), e.exit));
    }
    if with_json {
        check_json(e, &c.stdout, &mut problems);
    } else {
        check_text(e, &format!("{}{}", c.stdout, if e.mode() == "sh" { c.stderr.as_str() } else { "" }), &mut problems);
        if e.mode() == "sh" {
            check_json(e, &c.stdout, &mut problems);
        }
        check_files(e, &sandbox.0, &mut problems);
    }
    if !problems.is_empty() {
        problems.push(format!("{}{}", if with_json { "with --json, " } else { "" }, show(&c)));
    }
    Ok(problems)
}

/// A running server for http examples, killed when dropped.
pub struct Server {
    child: Child,
    pub port: u16,
    log: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        util::kill_tree(&mut self.child);
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

pub fn start(root: &Path, run: &Run, log: &Path) -> Result<Server> {
    let port = util::free_port();
    let argv: Vec<String> = run.serve.iter().map(|s| expand(s, root, Some(port))).collect();
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(log)?;
    let child = util::command(&argv)
        .current_dir(root)
        .envs(env_of(run, root, Some(port)))
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file)
        .spawn()
        .with_context(|| format!("couldn't start the server ({})", argv.join(" ")))?;
    let mut server = Server { child, port, log: log.to_path_buf() };
    let ready = if run.ready.is_empty() { "/" } else { run.ready.as_str() };
    let url = format!("http://127.0.0.1:{port}{ready}");
    let start = Instant::now();
    let http = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .new_agent();
    loop {
        if let Ok(r) = http.get(&url).call()
            && r.status().as_u16() < 500
        {
            return Ok(server);
        }
        if let Some(status) = server.child.try_wait()? {
            let out = std::fs::read_to_string(&server.log).unwrap_or_default();
            bail!("the server exited ({status}) before answering {url}:\n{}", util::tail(&out, 20));
        }
        if start.elapsed() > Duration::from_secs(60) {
            let out = std::fs::read_to_string(&server.log).unwrap_or_default();
            bail!("the server didn't answer {url} within 60s:\n{}", util::tail(&out, 20));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn request(server: &Server, e: &Example) -> Result<Vec<String>> {
    let mut parts = e.http.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let url = format!("http://127.0.0.1:{}{path}", server.port);
    let http = agent();
    let response = match method {
        "GET" => http.get(&url).call(),
        "HEAD" => http.head(&url).call(),
        "DELETE" => http.delete(&url).call(),
        "POST" => http.post(&url).content_type(content_type(&e.body)).send(e.body.as_bytes()),
        "PUT" => http.put(&url).content_type(content_type(&e.body)).send(e.body.as_bytes()),
        _ => http.patch(&url).content_type(content_type(&e.body)).send(e.body.as_bytes()),
    };
    let mut problems = Vec::new();
    let mut response = match response {
        Ok(r) => r,
        Err(err) => {
            problems.push(format!("{method} {path} failed: {err}"));
            return Ok(problems);
        }
    };
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    let want = e.status.unwrap_or(200);
    if status != want {
        problems.push(format!("{method} {path} answered {status}, should be {want}"));
    }
    check_text(e, &body, &mut problems);
    check_json(e, &body, &mut problems);
    if !problems.is_empty() {
        problems.push(format!("body:\n{}", util::tail(&body, 25)));
    }
    Ok(problems)
}

fn content_type(body: &str) -> &'static str {
    if body.trim_start().starts_with(['{', '[']) { "application/json" } else { "text/plain" }
}

/// Every example, in order. A server is started once for the http ones.
pub fn run_all(root: &Path, spec: &Spec, run: &Run, log_dir: &Path) -> Vec<ExampleResult> {
    let mut server: Option<Result<Server, String>> = None;
    let mut out = Vec::new();
    for e in &spec.examples {
        let start = Instant::now();
        let problems = match e.mode() {
            "http" => {
                let s = server.get_or_insert_with(|| start_server(root, run, log_dir));
                match s {
                    Ok(s) => request(s, e).unwrap_or_else(|err| vec![format!("{err:#}")]),
                    Err(err) => vec![err.clone()],
                }
            }
            _ => {
                let mut p = process(root, run, e, false).unwrap_or_else(|err| vec![format!("{err:#}")]);
                if p.is_empty() && e.mode() == "cli" && !e.json.is_empty() {
                    p = process(root, run, e, true).unwrap_or_else(|err| vec![format!("{err:#}")]);
                }
                p
            }
        };
        out.push(ExampleResult {
            name: e.name.clone(),
            ok: problems.is_empty(),
            secs: start.elapsed().as_secs_f64(),
            problem: problems.join("\n"),
        });
    }
    out
}

fn start_server(root: &Path, run: &Run, log_dir: &Path) -> Result<Server, String> {
    start(root, run, &log_dir.join("server.log")).map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(sh: &str) -> Example {
        Example { name: "t".into(), sh: sh.into(), ..Example::default() }
    }

    #[test]
    fn shell_examples_check_output_files_and_json() {
        let root = std::env::temp_dir();
        let run = Run::default();
        let mut e = example(if cfg!(windows) { "echo {\"n\": 3}" } else { "echo '{\"n\": 3}'" });
        e.files.insert("in.txt".into(), "hello".into());
        e.stdout = vec!["\"n\"".into()];
        e.json.insert("/n".into(), serde_json::json!(3.0));
        assert_eq!(process(&root, &run, &e, false).unwrap(), Vec::<String>::new());
        e.stdout = vec!["missing".into()];
        e.files_after.insert("in.txt".into(), "nope".into());
        let p = process(&root, &run, &e, false).unwrap().join("\n");
        assert!(p.contains("should contain \"missing\"") && p.contains("in.txt should contain"), "{p}");
        let mut code = example("exit 3");
        code.exit = 3;
        code.not_stdout = vec!["boom".into()];
        assert!(process(&root, &run, &code, false).unwrap().is_empty());
    }

    #[test]
    fn json_compares_like_a_person_would() {
        assert!(same(&serde_json::json!(3), &serde_json::json!(3.0)));
        assert!(same(&serde_json::json!("src\\a.py"), &serde_json::json!("src/a.py")));
        assert!(!same(&serde_json::json!([1]), &serde_json::json!([1, 2])));
    }
}
