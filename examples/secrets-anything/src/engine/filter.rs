//! What isn't a leak: fixture files, placeholder values, weak literals. And which files are config files.

/// Path segments that mark test data.
const FIXTURE_DIRS: &[&str] = &[
    "fixtures",
    "fixture",
    "__fixtures__",
    "testdata",
    "test-data",
    "__mocks__",
    "mocks",
    "__snapshots__",
    "snapshots",
];

/// File name endings that mark a template or a snapshot, like `.env.example`.
const FIXTURE_SUFFIXES: &[&str] = &[".example", ".sample", ".template", ".dist", ".snap"];

/// Words that mark a value as a placeholder, matched in any case.
const PLACEHOLDER_WORDS: &[&str] =
    &["example", "sample", "dummy", "placeholder", "your", "xxxx", "changeme", "redacted", "<"];

/// Passwords that are defaults, not secrets: `user:password`, `root:root`, `admin:admin`.
const DEFAULT_PASSWORDS: &[&str] =
    &["password", "passwd", "pass", "pwd", "secret", "admin", "root", "guest", "test", "user"];

/// Extensions of config files, where unquoted values count.
const CONFIG_EXTENSIONS: &[&str] = &["yaml", "yml", "toml", "ini", "cfg", "conf", "properties", "json"];

fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split(['/', '\\']).filter(|s| !s.is_empty())
}

fn file_name(path: &str) -> &str {
    segments(path).last().unwrap_or("")
}

/// True when the path is test data: a fixture folder anywhere in it, or a template/snapshot file name.
pub fn is_fixture(path: &str) -> bool {
    let name = file_name(path).to_ascii_lowercase();
    segments(path).any(|s| FIXTURE_DIRS.iter().any(|d| s.eq_ignore_ascii_case(d)))
        || FIXTURE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}

/// True for `.env*` files and the config extensions; everything else (and no extension) is code.
pub fn is_config(path: &str) -> bool {
    let name = file_name(path).to_ascii_lowercase();
    if name.starts_with(".env") {
        return true;
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => CONFIG_EXTENSIONS.contains(&ext),
        _ => false,
    }
}

/// True when the value is obviously not a real secret.
pub fn is_placeholder(value: &str) -> bool {
    let lower = value.to_lowercase();
    if PLACEHOLDER_WORDS.iter().any(|w| lower.contains(w)) {
        return true;
    }
    let wrapped = |open: &str, close: &str| {
        value.len() > open.len() + close.len() && value.starts_with(open) && value.ends_with(close)
    };
    if wrapped("${", "}") || wrapped("{{", "}}") || wrapped("%(", ")s") || wrapped("{", "}") {
        return true;
    }
    if let Some(var) = value.strip_prefix('$')
        && !var.is_empty()
        && var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return true;
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => chars.all(|c| c == first),
        None => true,
    }
}

/// True for the password half of a default login like `user:password` or `root:root`.
pub fn is_default_login(user: &str, password: &str) -> bool {
    password.eq_ignore_ascii_case(user) || DEFAULT_PASSWORDS.iter().any(|d| password.eq_ignore_ascii_case(d))
}

/// A literal strong enough to be a password: 8+ characters and 3 of lowercase, uppercase, digit, symbol.
pub fn is_strong(value: &str) -> bool {
    if value.chars().count() < 8 {
        return false;
    }
    let has = |f: fn(&char) -> bool| value.chars().any(|c| f(&c));
    let classes = [
        has(|c| c.is_lowercase()),
        has(|c| c.is_uppercase()),
        has(|c| c.is_ascii_digit()),
        has(|c| !c.is_alphanumeric() && !c.is_whitespace()),
    ];
    classes.iter().filter(|&&b| b).count() >= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        assert!(is_fixture("tests/fixtures/creds.json"));
        assert!(is_fixture("pkg/testdata/a.txt"));
        assert!(is_fixture("src/__snapshots__/x.ts.snap"));
        assert!(is_fixture(".env.example"));
        assert!(is_fixture("config/app.yml.dist"));
        assert!(is_fixture("a\\__mocks__\\b.js"));
        assert!(!is_fixture("src/fixtures.rs"));
        assert!(!is_fixture("config.py"));
        assert!(!is_fixture("stdin"));
    }

    #[test]
    fn config_files() {
        assert!(is_config(".env"));
        assert!(is_config("deploy/.env.production"));
        assert!(is_config("settings.yaml"));
        assert!(is_config("a/b/Cargo.TOML"));
        assert!(is_config("app.json"));
        assert!(!is_config("main.go"));
        assert!(!is_config("Dockerfile"));
        assert!(!is_config("stdin"));
        assert!(!is_config(".json"));
    }

    #[test]
    fn placeholders() {
        for v in [
            "your-Password-1",
            "AKIAIOSFODNN7EXAMPLE",
            "${DB_PASSWORD}",
            "{{ vault.pw }}",
            "%(pw)s",
            "$DB_PASS",
            "{password}",
            "********",
            "xxxxxxxxxxxx",
            "<token>",
            "ChangeMe123!",
            "REDACTED",
            "",
        ] {
            assert!(is_placeholder(v), "{v}");
        }
        for v in ["Tr0ub4dor-xK9q2", "S3cr3t-Pa55w0rd", "${"] {
            assert!(!is_placeholder(v), "{v}");
        }
    }

    #[test]
    fn default_logins() {
        assert!(is_default_login("root", "root"));
        assert!(is_default_login("user", "password"));
        assert!(is_default_login("Admin", "admin"));
        assert!(!is_default_login("admin", "S3cr3t-Pa55w0rd"));
    }

    #[test]
    fn strength() {
        assert!(is_strong("Tr0ub4dor-xK9q2"));
        assert!(is_strong("abcDEF12"));
        assert!(!is_strong("abcdefgh123"));
        assert!(!is_strong("Ab1!"));
        assert!(!is_strong("hello world"));
    }
}
