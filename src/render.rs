//! The template language: `{{key}}` is replaced by a value, nothing else is special.
//! `{{ spaced }}` (GitHub Actions) is left alone. An unknown key is an error, so a typo can't ship.

use anyhow::{Result, bail};
use std::collections::BTreeMap;

pub type Ctx = BTreeMap<String, String>;

pub fn render(template: &str, ctx: &Ctx) -> Result<String> {
    let mut out = String::with_capacity(template.len() + 256);
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let key_len = after.find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
        match key_len {
            Some(n) if n > 0 && after[n..].starts_with("}}") => {
                let key = &after[..n];
                let Some(value) = ctx.get(key) else { bail!("unknown template key {{{{{key}}}}}") };
                out.push_str(value);
                rest = &after[n + 2..];
            }
            _ => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// A template's path: __name__, __verb__, __snake__ and __command__ (a command's module) are placeholders,
/// and _gitignore / _gitattributes become dotfiles (a real .gitignore would apply to this repo).
pub fn path(template: &str, ctx: &Ctx, command: Option<&str>) -> String {
    let mut p = template.to_string();
    for key in ["name", "verb", "snake"] {
        if let Some(v) = ctx.get(key) {
            p = p.replace(&format!("__{key}__"), v);
        }
    }
    if let Some(c) = command {
        p = p.replace("__command__", c);
    }
    p.split('/')
        .map(|part| match part {
            "_gitignore" => ".gitignore",
            "_gitattributes" => ".gitattributes",
            "_npmignore" => ".npmignore",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_literals() {
        let mut ctx = Ctx::new();
        ctx.insert("name".into(), "todo-anything".into());
        assert_eq!(render("a {{name}} b", &ctx).unwrap(), "a todo-anything b");
        assert_eq!(render("${{ matrix.os }} {{ x", &ctx).unwrap(), "${{ matrix.os }} {{ x");
        assert_eq!(render("{{{{name}}}}", &ctx).unwrap(), "{{todo-anything}}");
        assert!(render("{{nmae}}", &ctx).is_err());
        assert_eq!(path("skills/__name__/SKILL.md", &ctx, None), "skills/todo-anything/SKILL.md");
        assert_eq!(path("seed/_gitignore", &ctx, None), "seed/.gitignore");
    }
}
