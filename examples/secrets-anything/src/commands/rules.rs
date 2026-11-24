//! `secrets-anything rules`: List the built-in detection rules
//!
//! Yours: anything-anything never overwrites this file.

use super::Output;
use super::args::RulesArgs;
use crate::engine::rules::RULES;
use anyhow::Result;
use serde_json::json;

pub fn run(_args: &RulesArgs) -> Result<Output> {
    let width = RULES.iter().map(|r| r.id.len()).max().unwrap_or(0);
    let text: String = RULES.iter().map(|r| format!("{:<width$}  {}\n", r.id, r.about)).collect();
    let rules: Vec<_> = RULES.iter().map(|r| json!({ "id": r.id, "about": r.about })).collect();
    Ok(Output::new(text, json!({ "count": RULES.len(), "rules": rules })))
}
