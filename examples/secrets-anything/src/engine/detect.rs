//! Finding secrets in one file's text: the rules, then the no-false-alarm filters.

use super::filter;
use super::rules::{self, RULES};
use regex::Captures;

/// The comment marker that silences a line.
pub const ALLOW_MARKER: &str = "secrets-anything: allow";

/// One leaked secret. Only the redacted form is kept: the whole secret never leaves the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub rule: &'static str,
    pub secret: String,
}

/// What a scan found, and how many matches it skipped as fixtures or placeholders.
#[derive(Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub skipped: usize,
}

impl Report {
    pub fn merge(&mut self, other: Report) {
        self.findings.extend(other.findings);
        self.skipped += other.skipped;
    }

    /// By path, then line, then column: stable output whatever order files were scanned in.
    pub fn sort(&mut self) {
        self.findings.sort_by(|a, b| {
            (a.path.as_str(), a.line, a.column, a.rule).cmp(&(b.path.as_str(), b.line, b.column, b.rule))
        });
    }
}

/// The first 4 characters, then `****` (fewer for very short secrets, so none is ever shown whole).
pub fn redact(secret: &str) -> String {
    let n = secret.chars().count();
    let shown: String = secret.chars().take(4.min(n / 2)).collect();
    format!("{shown}****")
}

/// A rule's match on a line: the byte span of the secret, and whether it looks like a placeholder.
struct Candidate {
    rule: usize,
    start: usize,
    end: usize,
    placeholder: bool,
}

/// Keywords of password-assignment keys, with separators removed.
const KEY_WORDS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "apikey",
    "accesskey",
    "privatekey",
    "clientsecret",
    "auth",
    "credential",
];

/// The key still names a secret once "author" is taken out (`author = "..."` isn't a credential).
fn names_secret(key: &str) -> bool {
    let key: String =
        key.to_lowercase().replace("author", "").chars().filter(|c| !matches!(c, '_' | '-' | '.')).collect();
    KEY_WORDS.iter().any(|w| key.contains(w))
}

fn candidate(rule: usize, caps: &Captures, config: bool) -> Option<Candidate> {
    let whole = caps.get(0)?;
    let make = |m: regex::Match, placeholder: bool| Candidate {
        rule,
        start: m.start(),
        end: m.end(),
        placeholder: placeholder || filter::is_placeholder(m.as_str()),
    };
    match RULES[rule].id {
        "aws-secret-key" => Some(make(caps.get(1)?, false)),
        "openai-key" if whole.as_str().starts_with("sk-ant-") => None,
        "url-credentials" => {
            let (user, password) = (caps.get(1)?, caps.get(2)?);
            Some(make(password, filter::is_default_login(user.as_str(), password.as_str())))
        }
        "password-assignment" => {
            let quoted = (1..=3).find_map(|i| caps.get(i));
            let value = match quoted {
                Some(v) => v,
                None if config => caps.get(4)?,
                None => return None,
            };
            let key = &whole.as_str()[..value.start() - whole.start()];
            (names_secret(key) && filter::is_strong(value.as_str())).then(|| make(value, false))
        }
        _ => Some(make(whole, false)),
    }
}

/// Scan one file's text. `path` is how the file is reported; it drives the fixture and config-file rules.
/// With `all`, fixtures and placeholders are reported instead of skipped.
pub fn scan_text(text: &str, path: &str, all: bool) -> Report {
    let compiled = rules::compiled();
    let mut report = Report::default();
    if !compiled.set.is_match(text) {
        return report;
    }
    let config = filter::is_config(path);
    let fixture = filter::is_fixture(path);
    for (n, line) in text.lines().enumerate() {
        if line.contains(ALLOW_MARKER) {
            continue;
        }
        let hits = compiled.set.matches(line);
        if !hits.matched_any() {
            continue;
        }
        let mut found: Vec<Candidate> = Vec::new();
        for rule in hits.iter() {
            for caps in compiled.each[rule].captures_iter(line) {
                if let Some(c) = candidate(rule, &caps, config) {
                    found.push(c);
                }
            }
        }
        // One finding per span: the most specific rule (the earliest) wins.
        found.sort_by_key(|c| (c.rule, c.start));
        let mut kept: Vec<Candidate> = Vec::new();
        for c in found {
            if !kept.iter().any(|k| c.start < k.end && k.start < c.end) {
                kept.push(c);
            }
        }
        for c in kept {
            if !all && (fixture || c.placeholder) {
                report.skipped += 1;
                continue;
            }
            report.findings.push(Finding {
                path: path.to_string(),
                line: n + 1,
                column: line[..c.start].chars().count() + 1,
                rule: RULES[c.rule].id,
                secret: redact(&line[c.start..c.end]),
            });
        }
    }
    report.sort();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake secrets, one per line with the allow marker so that scanning this repo stays clean.
    // The token-shaped ones are split with concat! so no whole key sits in the source: GitHub's
    // push protection (and other scanners) can't tell a test fixture from a leak.
    const GH: &str = concat!("ghp_", "R7mK2xQ9vLpW4nT8sYc3BfH6jD1aZ5eU0oGi"); // secrets-anything: allow
    const AWS: &str = concat!("AKIA", "Q3EGRIPJWXZH4LNT"); // secrets-anything: allow
    const AWS_SECRET: &str = concat!("wJalrXUtnFEMI/K7MDENG", "/bPxRfiCYzK9q2LmNoP"); // secrets-anything: allow
    const PEM: &str = "-----BEGIN OPENSSH PRIVATE KEY-----"; // secrets-anything: allow
    const ANT: &str = concat!("sk-ant-", "api03-R7mK2xQ9vLpW4nT8sYc3BfH6jD1aZ5eU0oGiR7mK2xQ9"); // secrets-anything: allow
    const OAI: &str = concat!("sk-", "proj-R7mK2xQ9vLpW4nT8sYc3BfH6jD1aZ5eU0oGiR7mK2xQ9"); // secrets-anything: allow
    const JWT: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K27uhbUJU1p1r"; // secrets-anything: allow
    const PW: &str = "Tr0ub4dor-xK9q2"; // secrets-anything: allow
    const URL_PW: &str = "S3cr3t-Pa55w0rd"; // secrets-anything: allow

    fn rules_of(r: &Report) -> Vec<&str> {
        r.findings.iter().map(|f| f.rule).collect()
    }

    #[test]
    fn redacts() {
        assert_eq!(redact(AWS), "AKIA****");
        assert_eq!(redact(PEM), "----****");
        assert_eq!(redact("abc"), "a****");
        assert_eq!(redact(""), "****");
    }

    #[test]
    fn aws_key_in_python() {
        let r = scan_text(&format!("import os\nAWS_KEY = \"{AWS}\"\n"), "config.py", false);
        assert_eq!(r.findings.len(), 1);
        let f = &r.findings[0];
        assert_eq!((f.line, f.column, f.rule, f.secret.as_str()), (2, 12, "aws-access-key", "AKIA****"));
    }

    #[test]
    fn column_counts_characters() {
        let r = scan_text(&format!("s = \"é→ {GH}\""), "a.py", false);
        assert_eq!(r.findings[0].column, 9);
    }

    #[test]
    fn private_key_is_the_begin_line() {
        let r = scan_text(&format!("{PEM}\nb3BlbnNzaC1rZXktdjEAAAAA\n"), "id_ed25519", false);
        assert_eq!(rules_of(&r), ["private-key"]);
        assert_eq!((r.findings[0].line, r.findings[0].column), (1, 1));
    }

    #[test]
    fn named_rule_beats_password_assignment() {
        let r = scan_text(&format!("api_token = \"{GH}\""), "a.py", false);
        assert_eq!(rules_of(&r), ["github-token"]);
        let r = scan_text(&format!("aws_secret_access_key = {AWS_SECRET}"), ".env", false);
        assert_eq!(rules_of(&r), ["aws-secret-key"]);
        assert_eq!(r.findings[0].column, 25);
    }

    #[test]
    fn anthropic_is_not_openai() {
        let r = scan_text(&format!("a = \"{ANT}\"\nb = \"{OAI}\""), "a.py", false);
        assert_eq!(rules_of(&r), ["anthropic-key", "openai-key"]);
    }

    #[test]
    fn jwt() {
        let r = scan_text(&format!("Authorization: Bearer {JWT}"), "req.http", false);
        assert_eq!(rules_of(&r), ["jwt"]);
    }

    #[test]
    fn password_assignment_in_config_and_code() {
        let yaml = format!("database:\n  password: {PW}\n");
        let r = scan_text(&yaml, "settings.yaml", false);
        assert_eq!(rules_of(&r), ["password-assignment"]);
        assert_eq!((r.findings[0].line, r.findings[0].column, r.findings[0].secret.as_str()), (2, 13, "Tr0u****"));
        // In code, only quoted literals count.
        assert!(scan_text(&yaml, "settings.py", false).findings.is_empty());
        assert_eq!(rules_of(&scan_text(&format!("db_pwd = '{PW}'"), "a.py", false)), ["password-assignment"]);
        assert_eq!(
            rules_of(&scan_text(&format!("{{\"clientSecret\": \"{PW}\"}}"), "a.json", false)),
            ["password-assignment"]
        );
    }

    #[test]
    fn not_values() {
        for line in [
            "const password = process.env.DB_PASSWORD;",
            "password = os.environ[\"DB_PASSWORD\"]",
            "pw = config.password",
            "token = \"abcdefghijkl\"",
            "author = \"Jane Doe 2024\"",
        ] {
            let r = scan_text(line, "a.js", false);
            assert!(r.findings.is_empty() && r.skipped == 0, "{line}");
        }
    }

    #[test]
    fn url_credentials() {
        let line = format!("DATABASE_URL=postgres://admin:{URL_PW}@db.internal:5432/app");
        let r = scan_text(&line, ".env", false);
        assert_eq!(rules_of(&r), ["url-credentials"]);
        assert_eq!((r.findings[0].column, r.findings[0].secret.as_str()), (31, "S3cr****"));
        let r = scan_text("u = \"postgres://root:root@localhost/db\"", "a.py", false);
        assert_eq!((r.findings.len(), r.skipped), (0, 1));
    }

    #[test]
    fn fixtures_and_placeholders_are_skipped_unless_all() {
        let json = format!("{{\"token\": \"{GH}\"}}");
        let r = scan_text(&json, "tests/fixtures/creds.json", false);
        assert_eq!((r.findings.len(), r.skipped), (0, 1));
        let r = scan_text(&json, "tests/fixtures/creds.json", true);
        assert_eq!((r.findings.len(), r.skipped), (1, 0));
        assert_eq!(r.findings[0].column, 12);
        let r = scan_text("DB_PASSWORD=your-Password-1", ".env", false);
        assert_eq!((r.findings.len(), r.skipped), (0, 1));
        let r = scan_text("const key = \"AKIAIOSFODNN7EXAMPLE\";", "app.js", false);
        assert_eq!((r.findings.len(), r.skipped), (0, 1));
    }

    #[test]
    fn allow_marker_always_wins() {
        let line = format!("API_TOKEN = \"{GH}\"  # {ALLOW_MARKER}");
        for all in [false, true] {
            let r = scan_text(&line, "ci.py", all);
            assert_eq!((r.findings.len(), r.skipped), (0, 0));
        }
    }

    #[test]
    fn sorted_by_line_then_column() {
        let text = format!("x = \"{AWS}\" + \"{GH}\"\ny = \"{AWS}\"");
        let r = scan_text(&text, "a.py", false);
        let pos: Vec<_> = r.findings.iter().map(|f| (f.line, f.column)).collect();
        assert_eq!(pos, [(1, 6), (1, 31), (2, 6)]);
    }
}
