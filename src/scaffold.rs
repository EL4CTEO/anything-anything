//! Puts a stack's files on disk: shell files are rewritten to match anything.toml, seed files are
//! written only when missing (then patched: a version bump reaches Cargo.toml / package.json / pyproject.toml).

use crate::spec::Spec;
use crate::stack::{self, GenFile, Owner, StackDef};
use crate::util;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

/// Runs a stack's formatter on a generated file, so the stack's own format step leaves it alone.
fn format(root: &Path, def: &StackDef, path: &str, text: String) -> String {
    let Some(f) = def.formats.iter().find(|f| path.ends_with(&format!(".{}", f.ext))) else { return text };
    let argv: Vec<String> = f.run.iter().map(|a| util::expand(a, root)).collect();
    if argv.is_empty() || util::which(&argv[0]).is_none() {
        return text;
    }
    let formatted = (|| {
        let mut child = util::command(&argv)
            .current_dir(if root.is_dir() { root } else { Path::new(".") })
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(text.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
    })();
    formatted.unwrap_or(text)
}

/// The files as they should be, shell files formatted.
pub fn expected(root: &Path, spec: &Spec) -> Result<(StackDef, Vec<GenFile>)> {
    let (def, mut files) = stack::files(spec)?;
    for f in files.iter_mut().filter(|f| f.owner == Owner::Shell) {
        f.text = format(root, &def, &f.path, std::mem::take(&mut f.text));
    }
    Ok((def, files))
}

fn read(root: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(root.join(rel)).ok().map(|t| t.replace("\r\n", "\n"))
}

/// Shell files that are missing or not what anything.toml says they should be.
pub fn drift(root: &Path, files: &[GenFile]) -> Vec<String> {
    files
        .iter()
        .filter(|f| f.owner == Owner::Shell && read(root, &f.path).as_deref() != Some(f.text.as_str()))
        .map(|f| f.path.clone())
        .collect()
}

#[derive(Debug, Default)]
pub struct Synced {
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub patched: Vec<String>,
    pub unchanged: usize,
}

/// Brings the project in line with anything.toml. dry: only say what would change.
pub fn sync(root: &Path, spec: &Spec, dry: bool) -> Result<Synced> {
    let (def, files) = stack::files(spec)?;
    let mut out = Synced::default();
    // seeds first: a shell file's formatter may read a seed (rustfmt.toml)
    for owner in [Owner::Seed, Owner::Shell] {
        for f in files.iter().filter(|f| f.owner == owner) {
            let text = if owner == Owner::Shell { format(root, &def, &f.path, f.text.clone()) } else { f.text.clone() };
            let current = read(root, &f.path);
            let new = match &current {
                None => true,
                Some(now) if owner == Owner::Shell && *now != text => false,
                _ => {
                    out.unchanged += 1;
                    continue;
                }
            };
            if !dry {
                let path = root.join(&f.path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
            }
            if new { out.created.push(f.path.clone()) } else { out.updated.push(f.path.clone()) }
        }
    }
    if !dry {
        out.patched = patch(root, spec)?;
    }
    Ok(out)
}

/// Keeps the package manifests' version and description in step with anything.toml.
fn patch(root: &Path, spec: &Spec) -> Result<Vec<String>> {
    let mut patched = Vec::new();
    let toml_files = [("Cargo.toml", "package"), ("pyproject.toml", "project")];
    for (file, table) in toml_files {
        let Some(text) = read(root, file) else { continue };
        let Ok(mut doc) = text.parse::<toml_edit::DocumentMut>() else { continue };
        let Some(t) = doc.get_mut(table).and_then(|t| t.as_table_like_mut()) else { continue };
        let mut changed = false;
        for (key, value) in [("version", &spec.version), ("description", &spec.pitch)] {
            if t.get(key).and_then(|v| v.as_str()) != Some(value.as_str()) && t.contains_key(key) {
                t.insert(key, toml_edit::value(value.as_str()));
                changed = true;
            }
        }
        if changed {
            std::fs::write(root.join(file), doc.to_string())?;
            patched.push(file.to_string());
        }
    }
    if let Some(text) = read(root, "package.json")
        && let Ok(mut pkg) = serde_json::from_str::<serde_json::Value>(&text)
    {
        let mut changed = false;
        for (key, value) in [("version", &spec.version), ("description", &spec.pitch)] {
            if pkg.get(key).is_some() && pkg[key].as_str() != Some(value.as_str()) {
                pkg[key] = serde_json::json!(value);
                changed = true;
            }
        }
        if changed {
            std::fs::write(root.join("package.json"), serde_json::to_string_pretty(&pkg)? + "\n")?;
            patched.push("package.json".into());
        }
    }
    Ok(patched)
}
