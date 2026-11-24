//! The logic the commands of secrets-anything share. Yours: anything-anything never overwrites this folder.
//! secrets-anything: Find leaked API keys, tokens, passwords and private keys in any codebase, in any language, before you push. Fast, offline, no false alarms on test fixtures.

pub mod detect;
pub mod filter;
pub mod rules;
pub mod walk;

use detect::Report;
use serde_json::{Value, json};

/// The summary line: "1 secret found", "3 secrets found", "no secrets found", plus what was skipped.
pub fn summary(report: &Report) -> String {
    let found = match report.findings.len() {
        0 => "no secrets found".to_string(),
        1 => "1 secret found".to_string(),
        n => format!("{n} secrets found"),
    };
    if report.skipped > 0 {
        format!("{found} ({} skipped as fixtures or placeholders, --all to show)", report.skipped)
    } else {
        found
    }
}

/// One line per finding, then the summary.
pub fn text(report: &Report) -> String {
    let mut out = String::new();
    for f in &report.findings {
        out.push_str(&format!("{}:{}:{}  {}  {}\n", f.path, f.line, f.column, f.rule, f.secret));
    }
    out.push_str(&summary(report));
    out
}

/// `{"count", "skipped", "findings": [{"path", "line", "column", "rule", "secret"}]}`.
pub fn json(report: &Report) -> Value {
    let findings: Vec<Value> = report
        .findings
        .iter()
        .map(|f| json!({ "path": f.path, "line": f.line, "column": f.column, "rule": f.rule, "secret": f.secret }))
        .collect();
    json!({ "count": report.findings.len(), "skipped": report.skipped, "findings": findings })
}

/// Text, data, and exit 1 when anything was found.
pub fn output(report: &Report) -> crate::commands::Output {
    let out = crate::commands::Output::new(text(report), json(report));
    if report.findings.is_empty() { out } else { out.with_exit(1) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use detect::Finding;

    fn finding(path: &str) -> Finding {
        Finding { path: path.into(), line: 2, column: 12, rule: "aws-access-key", secret: "AKIA****".into() }
    }

    #[test]
    fn summaries() {
        let mut r = Report::default();
        assert_eq!(summary(&r), "no secrets found");
        r.skipped = 3;
        assert_eq!(summary(&r), "no secrets found (3 skipped as fixtures or placeholders, --all to show)");
        r.skipped = 0;
        r.findings.push(finding("config.py"));
        assert_eq!(text(&r), "config.py:2:12  aws-access-key  AKIA****\n1 secret found");
        r.findings.push(finding("b.py"));
        assert_eq!(summary(&r), "2 secrets found");
    }

    #[test]
    fn json_shape_and_exit() {
        let mut r = Report::default();
        assert_eq!(json(&r), json!({ "count": 0, "skipped": 0, "findings": [] }));
        assert_eq!(output(&r).exit, 0);
        r.findings.push(finding("config.py"));
        let v = json(&r);
        assert_eq!(v["findings"][0]["column"], 12);
        assert_eq!(v["findings"][0]["secret"], "AKIA****");
        assert_eq!(output(&r).exit, 1);
    }
}
