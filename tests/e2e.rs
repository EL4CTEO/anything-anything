//! The real binary, every stack: generate a tool from a spec, check that verify fails while it's a stub,
//! "build" it with a stand-in agent (tests/fixtures/fake-agent.mjs copies a known implementation and
//! edits a generated file, which must be put back), and check that every gate passes.
//! A stack whose toolchain isn't installed is skipped. Needs node for the stand-in agent.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_anything-anything");

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

fn has(program: &str, arg: &str) -> bool {
    let program =
        if cfg!(windows) && matches!(program, "npm" | "npx") { format!("{program}.cmd") } else { program.to_string() };
    Command::new(program).arg(arg).output().is_ok_and(|o| o.status.success())
}

fn python() -> bool {
    has("python3", "--version") || has("python", "--version")
}

/// (stdout and stderr, success, stdout alone)
struct Run(String, bool, String);

fn aa(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(BIN).args(args).current_dir(dir).env("NO_COLOR", "1").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    Run(format!("{stdout}{}", String::from_utf8_lossy(&out.stderr)), out.status.success(), stdout)
}

fn case(fixture: &str, stack: &str, name: &str, needs: bool) {
    if !needs || !has("node", "--version") {
        eprintln!("skipped {stack}: toolchain not installed");
        return;
    }
    let work = std::env::temp_dir().join(format!("aa-e2e-{stack}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let spec = fixtures().join(fixture).join(format!("{stack}.toml"));
    let r = aa(&work, &["new", name, "--spec", spec.to_str().unwrap(), "--no-git"]);
    assert!(r.1, "new failed:\n{}", r.0);
    let root = work.join(format!("{name}-anything"));
    assert!(root.join("README.md").is_file() && root.join("docs/logo.svg").is_file(), "{}", r.0);

    let r = aa(&root, &["sync", "--check"]);
    assert!(r.1, "a fresh project is in sync:\n{}", r.0);

    let r = aa(&root, &["verify", "--json"]);
    assert!(!r.1, "verify passes on a stub:\n{}", r.0);
    let report: serde_json::Value = serde_json::from_str(&r.2).unwrap_or_else(|e| panic!("{e}\n{}", r.0));
    let stubs = report["gates"].as_array().unwrap().iter().find(|g| g["name"] == "stubs").unwrap();
    if stack != "any" {
        assert_eq!(stubs["status"], "fail", "{stubs}");
    }

    let agent = format!(
        "node {} {}",
        fixtures().join("fake-agent.mjs").display(),
        fixtures().join(fixture).join(format!("impl-{stack}")).display()
    )
    .replace('\\', "/");
    let r = aa(&root, &["build", "--agent", &agent, "--attempts", "1"]);
    assert!(r.1, "build failed:\n{}", r.0);
    assert!(r.0.contains("works: every gate passes"), "{}", r.0);
    let r = aa(&root, &["sync", "--check"]);
    assert!(r.1, "the edited generated file was put back:\n{}", r.0);
    let r = aa(&root, &["status"]);
    assert!(r.1 && r.0.contains("build #1"), "{}", r.0);
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn python_cli() {
    case("greet", "python-cli", "greet", python());
}

#[test]
fn node_cli() {
    case("greet", "node-cli", "greet", has("npm", "--version"));
}

#[test]
fn rust_cli() {
    case("greet", "rust-cli", "greet", has("cargo", "--version"));
}

#[test]
fn web() {
    case("dice", "web", "dice", has("npm", "--version"));
}

#[test]
fn any() {
    case("shout", "any", "shout", python());
}

#[test]
fn spec_and_stacks_and_mcp() {
    let dir = std::env::temp_dir();
    let r = aa(&dir, &["spec"]);
    assert!(r.1 && r.0.contains("[[example]]"));
    let r = aa(&dir, &["stacks"]);
    for s in ["rust-cli", "python-cli", "node-cli", "web", "any"] {
        assert!(r.0.contains(s), "{}", r.0);
    }
    let r = aa(&dir, &["new", "anything"]);
    assert!(!r.1 && r.0.contains("you're running it"), "{}", r.0);
}
