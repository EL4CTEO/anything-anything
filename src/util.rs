//! Small helpers: paths, programs, processes, formatting.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

pub fn slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Finds a program on PATH (trying PATHEXT on Windows).
pub fn which(name: &str) -> Option<PathBuf> {
    if name.contains(['/', '\\']) {
        let p = PathBuf::from(name);
        return p.is_file().then_some(p);
    }
    // Windows can't run an extensionless file (npm ships a sh script called npm next to npm.cmd)
    let mut exts = vec![String::new()];
    if cfg!(windows) {
        if Path::new(name).extension().is_none() {
            exts.clear();
        }
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
        exts.extend(pathext.split(';').filter(|s| !s.is_empty()).map(|s| s.to_ascii_lowercase()));
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// The first of "a|b|c" that is installed.
pub fn first_installed(alternatives: &str) -> Option<String> {
    alternatives.split('|').find(|n| which(n).is_some()).map(String::from)
}

/// A command from an argv. One argument with spaces in it is a shell line; npm-style .cmd shims go through cmd.exe.
pub fn command(argv: &[String]) -> Command {
    let shell_line = argv.len() == 1 && argv[0].contains(char::is_whitespace);
    if shell_line {
        return shell(&argv[0]);
    }
    if cfg!(windows) {
        let shim = which(&argv[0])
            .and_then(|p| p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()))
            .is_some_and(|e| e == "cmd" || e == "bat");
        if shim {
            let mut c = Command::new("cmd");
            c.arg("/C").args(argv);
            return c;
        }
    }
    let mut c = Command::new(&argv[0]);
    c.args(&argv[1..]);
    c
}

/// A shell line, run by cmd on Windows (verbatim: /S strips the outer quotes) and sh elsewhere.
fn shell(line: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        c.raw_arg(format!("/D /S /C \"{line}\""));
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(line);
        c
    }
}

/// Kills a process and, on Windows, everything it started.
pub fn kill_tree(child: &mut Child) {
    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Runtime placeholders in a stack's or spec's commands: {root}, {exe}, {python}.
pub fn expand(s: &str, root: &Path) -> String {
    s.replace("{root}", &slashes(root)).replace("{exe}", std::env::consts::EXE_SUFFIX).replace("{python}", &python())
}

pub fn python() -> String {
    first_installed("python3|python").unwrap_or_else(|| "python".into())
}

pub fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// At most n chars, with an ellipsis.
pub fn clip(text: &str, n: usize) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= n { one } else { format!("{}…", one.chars().take(n - 1).collect::<String>()) }
}

pub fn secs(s: f64) -> String {
    if s >= 60.0 {
        format!("{}m{:02}s", (s / 60.0) as u64, (s % 60.0) as u64)
    } else if s >= 1.0 {
        format!("{s:.1}s")
    } else {
        format!("{:.0}ms", s * 1e3)
    }
}

pub fn compact(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// FNV-1a: stable across builds (the std hasher isn't).
pub fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

pub fn year() -> u64 {
    // days since 1970 -> civil year, good for the next few centuries
    let days = now() / 86_400;
    let mut year = 1970;
    let mut left = days;
    loop {
        let len = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 366 } else { 365 };
        if left < len {
            return year;
        }
        left -= len;
        year += 1;
    }
}

/// A TCP port nothing listens on right now.
pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(4173)
}

/// Every file under dir (relative, forward slashes), skipping build output and dependencies.
pub fn files_under(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(
                    name.as_str(),
                    ".git" | "target" | "node_modules" | "__pycache__" | ".anything" | "dist" | ".venv" | "venv"
                ) {
                    walk(base, &path, out);
                }
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(slashes(rel));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(compact(12_480), "12.5k");
        assert_eq!(secs(0.25), "250ms");
        assert_eq!(secs(75.0), "1m15s");
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc");
        assert_eq!(clip("one   two three", 8), "one two…");
        assert_eq!(fnv("a"), 0xaf63_dc4c_8601_ec8c);
        assert!(year() >= 2025);
    }

    #[test]
    fn placeholders() {
        let root = Path::new("/tmp/x");
        assert_eq!(expand("{root}/bin/t{exe}", root), format!("/tmp/x/bin/t{}", std::env::consts::EXE_SUFFIX));
    }
}
