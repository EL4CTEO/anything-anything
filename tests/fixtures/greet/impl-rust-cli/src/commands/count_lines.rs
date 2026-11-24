use super::Output;
use super::args::CountLinesArgs;
use anyhow::{Context, Result};

pub fn run(args: &CountLinesArgs) -> Result<Output> {
    let mut files = Vec::new();
    let mut text = String::new();
    let mut total = 0;
    for path in &args.files {
        let content = std::fs::read_to_string(path).with_context(|| format!("can't read {path}"))?;
        let lines = content.lines().count() as i64;
        if lines < args.min {
            continue;
        }
        total += lines;
        text.push_str(&format!("{path}: {lines}\n"));
        files.push(serde_json::json!({ "path": path, "lines": lines }));
    }
    text.push_str(&format!("total: {total}"));
    Ok(Output::new(text, serde_json::json!({ "files": files, "total": total })))
}
