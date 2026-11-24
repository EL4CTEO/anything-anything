//! Names: the tool's name and everything derived from it (crate, package, module, MCP prefix, repo).

use crate::spec::Spec;
use crate::util;
use anyhow::{Result, bail};

/// Where anything-anything itself lives: every generated README links back here.
pub const HOME: &str = "EL4CTEO/anything-anything";

/// Words that are keywords in Rust, Python or JavaScript: not allowed as command or argument names,
/// so every stack can use the name as an identifier as it is.
const RESERVED: &[&str] = &[
    "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "crate", "def", "default", "del",
    "delete", "do", "dyn", "elif", "else", "enum", "except", "export", "extends", "extern", "false", "finally", "fn",
    "for", "from", "function", "global", "if", "impl", "import", "in", "instanceof", "is", "lambda", "let", "loop",
    "match", "mod", "move", "mut", "new", "none", "nonlocal", "not", "null", "or", "and", "pass", "pub", "raise", "ref",
    "return", "self", "static", "struct", "super", "switch", "this", "throw", "trait", "true", "try", "type", "typeof",
    "unsafe", "use", "var", "void", "where", "while", "with", "yield",
];

pub fn reserved(word: &str) -> bool {
    RESERVED.contains(&word.replace('-', "_").as_str())
}

/// lower-kebab: starts with a letter, words of [a-z0-9] joined by single dashes.
pub fn kebab_ok(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// "Translate", "translate_anything", "TRANSLATE anything" -> "translate-anything".
pub fn tool_name(input: &str) -> Result<String> {
    let mut s = String::new();
    for c in input.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() || !s.starts_with(|c: char| c.is_ascii_lowercase()) {
        bail!("give it a name that starts with a letter, like `translate` (it becomes translate-anything)");
    }
    if s == "anything" || s == "anything-anything" {
        bail!("anything-anything already exists: you're running it. Pick what yours does to anything, like `translate`");
    }
    Ok(if s.ends_with("-anything") { s } else { format!("{s}-anything") })
}

pub fn snake(s: &str) -> String {
    s.replace('-', "_")
}

pub fn pascal(s: &str) -> String {
    s.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

/// Everything a template needs to know about the tool's identity, with the spec's blanks filled in.
#[derive(Debug, Clone)]
pub struct Meta {
    pub name: String,
    pub verb: String,
    pub snake: String,
    /// MCP tool prefix: "translate" for translate-anything, so tools are translate_scan, ...
    pub prefix: String,
    pub title: String,
    pub tagline: String,
    pub owner: String,
    pub repo: String,
    pub repo_url: String,
    pub npm: String,
    pub color: String,
    pub rgb: (u8, u8, u8),
    pub keywords: Vec<String>,
}

const PALETTE: &[&str] =
    &["#7c3aed", "#2563eb", "#0891b2", "#0d9488", "#16a34a", "#d97706", "#ea580c", "#dc2626", "#db2777", "#9333ea"];

pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some((p(0)?, p(2)?, p(4)?))
}

impl Meta {
    pub fn of(spec: &Spec) -> Meta {
        let name = spec.name.clone();
        let verb = name.strip_suffix("-anything").unwrap_or(&name).to_string();
        let owner = if spec.owner.is_empty() { "OWNER".to_string() } else { spec.owner.clone() };
        let npm = if !spec.npm.is_empty() {
            spec.npm.clone()
        } else if owner != "OWNER" {
            format!("@{}/{name}", owner.to_ascii_lowercase())
        } else {
            name.clone()
        };
        let color = if rgb(&spec.color).is_some() {
            spec.color.to_ascii_lowercase()
        } else {
            PALETTE[(util::fnv(&name) % PALETTE.len() as u64) as usize].to_string()
        };
        let tagline = if spec.tagline.is_empty() { spec.pitch.clone() } else { spec.tagline.clone() };
        let mut keywords = spec.keywords.clone();
        for k in ["anything-anything", "mcp", "claude-code"] {
            if !keywords.iter().any(|w| w == k) {
                keywords.push(k.to_string());
            }
        }
        Meta {
            title: pascal(&verb) + " Anything",
            snake: snake(&name),
            prefix: snake(&verb),
            repo: format!("{owner}/{name}"),
            repo_url: format!("https://github.com/{owner}/{name}"),
            rgb: rgb(&color).unwrap_or((124, 58, 237)),
            name,
            verb,
            tagline,
            owner,
            npm,
            color,
            keywords,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(tool_name("Translate").unwrap(), "translate-anything");
        assert_eq!(tool_name("  pdf to md ").unwrap(), "pdf-to-md-anything");
        assert_eq!(tool_name("todo_anything").unwrap(), "todo-anything");
        assert!(tool_name("anything").is_err());
        assert!(tool_name("42").is_err());
        assert!(kebab_ok("list-all") && !kebab_ok("List") && !kebab_ok("a--b") && !kebab_ok("a-"));
        assert_eq!(pascal("list-all"), "ListAll");
        assert!(reserved("type") && reserved("from") && !reserved("scan"));
    }
}
