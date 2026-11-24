//! The built-in detection rules, in the order `rules` lists them, and their compiled form.
//!
//! The patterns are written so that this file matches none of them: secrets-anything scans itself clean.

use regex::{Regex, RegexSet};
use std::sync::OnceLock;

/// One built-in rule.
pub struct Rule {
    pub id: &'static str,
    pub about: &'static str,
    pub pattern: &'static str,
}

/// Every rule, most specific first: when matches overlap, the earlier rule wins.
pub const RULES: &[Rule] = &[
    Rule {
        id: "private-key",
        about: "PEM/OpenSSH/PGP private key header (-----BEGIN ... PRIVATE KEY-----)",
        pattern: r"-----BEGIN (?:(?:RSA|EC|DSA|OPENSSH|PGP|ENCRYPTED) )?PRIVATE KEY(?: BLOCK)?-----",
    },
    Rule {
        id: "aws-access-key",
        about: "AWS access key ID (AKIA, ASIA, ABIA, ACCA + 16 chars)",
        pattern: r"(?:AKIA|ASIA|ABIA|ACCA)[A-Z0-9]{16}",
    },
    Rule {
        id: "aws-secret-key",
        about: "AWS secret access key: 40-char base64 value of a key naming aws and secret",
        pattern: r#"(?i)[\w.-]*(?:aws[\w.-]*secret|secret[\w.-]*aws)[\w.-]*["']?\s*(?:=>|:=|=|:)\s*["']?([A-Za-z0-9/+=]{40})(?:[^A-Za-z0-9/+=]|$)"#,
    },
    Rule {
        id: "github-token",
        about: "GitHub token (personal, OAuth, user, server, refresh, fine-grained PAT)",
        pattern: r"gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{82}",
    },
    Rule { id: "gitlab-token", about: "GitLab personal access token (glpat-)", pattern: r"glpat-[A-Za-z0-9_-]{20,}" },
    Rule {
        id: "slack-token",
        about: "Slack bot, app, user or refresh token (xoxb-, xoxa-, xoxp-, ...)",
        pattern: r"xox[baprs]-[A-Za-z0-9-]{10,}",
    },
    Rule {
        id: "slack-webhook",
        about: "Slack incoming webhook URL",
        pattern: r"https://hooks\.slack\.com/services/T[A-Za-z0-9_]+/B[A-Za-z0-9_]+/[A-Za-z0-9_]+",
    },
    Rule {
        id: "stripe-key",
        about: "Stripe live secret or restricted key (test keys are not secrets)",
        pattern: r"(?:sk|rk)_live_[A-Za-z0-9]{24,}",
    },
    Rule { id: "google-api-key", about: "Google API key (AIza + 35 chars)", pattern: r"AIza[A-Za-z0-9_-]{35}" },
    Rule {
        id: "openai-key",
        about: "OpenAI API key (sk-, sk-proj-, sk-svcacct-)",
        pattern: r"sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{40,}",
    },
    Rule { id: "anthropic-key", about: "Anthropic API key (sk-ant-)", pattern: r"sk-ant-[A-Za-z0-9_-]{40,}" },
    Rule { id: "npm-token", about: "npm access token (npm_ + 36 chars)", pattern: r"npm_[A-Za-z0-9]{36}" },
    Rule {
        id: "jwt",
        about: "JSON Web Token: three base64url parts, header and payload starting eyJ",
        pattern: r"eyJ[A-Za-z0-9_-]{7,}\.eyJ[A-Za-z0-9_-]{7,}\.[A-Za-z0-9_-]{10,}",
    },
    Rule {
        id: "url-credentials",
        about: "Password in a URL (scheme://user:password@host)",
        pattern: r#"[A-Za-z][A-Za-z0-9+.-]*://([^\s:/?#@"'`]+):([^\s/?#@"'`]+)@[A-Za-z0-9\[{$<%]"#,
    },
    Rule {
        id: "password-assignment",
        about: "Strong literal assigned to a password, secret, token, api key, auth or credential key",
        pattern: r#"(?i)["']?[\w.-]*(?:password|passwd|pwd|secret|token|api[_.-]?key|access[_.-]?key|private[_.-]?key|client[_.-]?secret|auth|credential)[\w.-]*["']?\s*(?:=>|:=|=|:)\s*(?:"([^"]*)"|'([^']*)'|`([^`]*)`|([^\s"'`,;]+))"#,
    },
];

/// All rules compiled once: one set to find which rules hit a line, then each regex for the spans.
pub struct Compiled {
    pub set: RegexSet,
    pub each: Vec<Regex>,
}

pub fn compiled() -> &'static Compiled {
    static COMPILED: OnceLock<Compiled> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let patterns: Vec<&str> = RULES.iter().map(|r| r.pattern).collect();
        Compiled {
            set: RegexSet::new(&patterns).expect("built-in rules compile"),
            each: patterns.iter().map(|p| Regex::new(p).expect("built-in rule compiles")).collect(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_rules_in_order() {
        assert_eq!(RULES.len(), 15);
        assert_eq!(RULES[0].id, "private-key");
        assert_eq!(RULES[3].id, "github-token");
        assert_eq!(RULES[14].id, "password-assignment");
        assert_eq!(compiled().each.len(), 15);
    }

    #[test]
    fn ids_are_unique() {
        for (i, r) in RULES.iter().enumerate() {
            assert!(RULES[i + 1..].iter().all(|o| o.id != r.id), "{}", r.id);
        }
    }
}
