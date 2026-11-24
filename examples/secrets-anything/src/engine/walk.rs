//! Walking a folder in parallel: what gets scanned, and what never does.

use super::detect::{Report, scan_text};
use anyhow::{Context, Result, bail};
use ignore::{WalkBuilder, WalkState};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// Folders that are never pushed or never yours.
const SKIP_DIRS: &[&str] = &[".git", ".hg", ".svn", "node_modules", "target", "dist", "build", "vendor", ".venv"];

/// Lock files: generated, full of hashes, never hand-written secrets.
const LOCK_FILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "poetry.lock",
    "uv.lock",
    "go.sum",
    "composer.lock",
    "Gemfile.lock",
];

/// Files over this size are skipped.
pub const MAX_SIZE: u64 = 1024 * 1024;

/// A NUL byte this early means binary.
const BINARY_PROBE: usize = 8 * 1024;

fn skipped_name(name: &str) -> bool {
    LOCK_FILES.contains(&name) || name.ends_with(".min.js")
}

/// The file's text, or None for binary files. Invalid UTF-8 is read lossily.
fn read_text(path: &Path) -> Result<Option<String>> {
    let bytes = fs::read(path).with_context(|| format!("can't read {}", slash(path)))?;
    if bytes[..bytes.len().min(BINARY_PROBE)].contains(&0) {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

/// A path with forward slashes, on every OS.
pub fn slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Relative to the scanned folder, `/`-separated.
fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// The path that decides the fixture rule: the folder as given joined with the file's relative path,
/// so `scan tests/fixtures` still knows its files are fixtures.
fn rule_path(root: &Path, rel: &str) -> String {
    let root = slash(root);
    if root.is_empty() || root == "." { rel.to_string() } else { format!("{}/{rel}", root.trim_end_matches('/')) }
}

/// Scan a folder or a single file.
pub fn scan_path(path: &Path, all: bool) -> Result<Report> {
    let meta = fs::metadata(path).with_context(|| format!("no such file or folder: {}", slash(path)))?;
    if meta.is_file() {
        return scan_file(path, all);
    }
    if !meta.is_dir() {
        bail!("not a file or folder: {}", slash(path));
    }
    scan_dir(path, all)
}

fn scan_file(path: &Path, all: bool) -> Result<Report> {
    let shown = slash(path);
    if fs::metadata(path)?.len() > MAX_SIZE {
        crate::ui::warn(format!("skipped {shown}: over 1 MiB"));
        return Ok(Report::default());
    }
    match read_text(path)? {
        Some(text) => Ok(scan_text(&text, &shown, all)),
        None => {
            crate::ui::warn(format!("skipped {shown}: binary"));
            Ok(Report::default())
        }
    }
}

/// True when the folder is in a git repo (a `.git` dir or file in it or above it).
fn inside_git_repo(root: &Path) -> bool {
    let Ok(abs) = root.canonicalize() else { return false };
    abs.ancestors().any(|dir| dir.join(".git").exists())
}

fn scan_dir(root: &Path, all: bool) -> Result<Report> {
    // Inside a repo: git's own rules, ignore files from the scanned folder up to the repo root.
    // Outside one: the folder's own ignore files still count (require_git(false)), but none above it,
    // since without a repo root the crate would apply every ancestor's (a `~/.gitignore` of `*` hides all).
    let in_repo = inside_git_repo(root);
    let total = Mutex::new(Report::default());
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .ignore(true)
        .require_git(in_repo)
        .parents(in_repo)
        .follow_links(false)
        .filter_entry(|e| {
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            !(e.depth() > 0 && is_dir && SKIP_DIRS.iter().any(|d| e.file_name() == *d))
        })
        .build_parallel();
    walker.run(|| {
        let total = &total;
        Box::new(move |entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    crate::ui::warn(e);
                    return WalkState::Continue;
                }
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return WalkState::Continue;
            }
            let name = entry.file_name().to_string_lossy();
            if skipped_name(&name) || entry.metadata().map(|m| m.len() > MAX_SIZE).unwrap_or(true) {
                return WalkState::Continue;
            }
            let rel = relative(root, entry.path());
            match read_text(entry.path()) {
                Ok(Some(text)) => {
                    let mut report = scan_text(&text, &rule_path(root, &rel), all);
                    for f in &mut report.findings {
                        f.path.clone_from(&rel);
                    }
                    total.lock().unwrap_or_else(|p| p.into_inner()).merge(report);
                }
                Ok(None) => {}
                Err(e) => crate::ui::warn(format!("{e:#}")),
            }
            WalkState::Continue
        })
    });
    let mut report = total.into_inner().unwrap_or_else(|p| p.into_inner());
    report.sort();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const GH: &str = "ghp_R7mK2xQ9vLpW4nT8sYc3BfH6jD1aZ5eU0oGi"; // secrets-anything: allow

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!("secrets-anything-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn write(&self, rel: &str, content: impl AsRef<[u8]>) {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn walks_honoring_ignores_and_skips() {
        let t = TempDir::new("walk");
        let line = format!("t = \"{GH}\"\n");
        t.write("src/a.py", &line);
        t.write("b/c/d.py", &line);
        t.write(".gitignore", "secret.env\n");
        t.write("secret.env", &line);
        t.write(".ignore", "skipme/\n");
        t.write("skipme/x.py", &line);
        t.write("node_modules/pkg/i.js", &line);
        t.write("target/debug/x.rs", &line);
        t.write("package-lock.json", &line);
        t.write("app.min.js", &line);
        t.write("bin.dat", [line.as_bytes(), b"\0"].concat());
        t.write("big.txt", line.repeat(30_000));
        t.write("bad.py", [b"\xff\xfe ".as_slice(), line.as_bytes()].concat());
        let r = scan_path(&t.0, false).unwrap();
        let paths: Vec<_> = r.findings.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["b/c/d.py", "bad.py", "src/a.py"]);
    }

    #[test]
    fn single_file_and_missing_path() {
        let t = TempDir::new("single");
        t.write("one.py", format!("t = \"{GH}\"\n"));
        let r = scan_path(&t.0.join("one.py"), false).unwrap();
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].path.ends_with("/one.py"));
        assert!(scan_path(&t.0.join("nope"), false).is_err());
    }

    #[test]
    fn scanning_a_fixture_folder_still_skips() {
        let t = TempDir::new("fixdir");
        t.write("fixtures/c.json", format!("{{\"token\": \"{GH}\"}}"));
        let r = scan_path(&t.0.join("fixtures"), false).unwrap();
        assert_eq!((r.findings.len(), r.skipped), (0, 1));
    }
}
