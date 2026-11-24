//! How results look in a terminal.

use crate::gate::{Report, Status};
use crate::hub::{Listing, Tool, Way};
use crate::scaffold::Synced;
use crate::spec::Spec;
use crate::state::State;
use crate::{ui, util};

pub fn report(r: &Report) {
    ui::line("");
    if r.ok {
        ui::ok(ui::bold(format!("{} works: every gate passes, {}/{} examples", r.name, r.passed(), r.examples.len())));
    } else {
        let failing: Vec<&str> = r.failing().map(|g| g.name.as_str()).collect();
        ui::error(ui::bold(format!("{}: {} failing ({})", r.name, failing.len(), failing.join(", "))));
        for g in r.failing() {
            ui::line(format!("\n  {}", ui::bold(&g.name)));
            for l in g.detail.lines().take(25) {
                ui::line(format!("    {}", ui::dim(l)));
            }
        }
    }
}

pub fn synced(s: &Synced) {
    for p in &s.created {
        ui::line(format!("  {} {p}", ui::green("+")));
    }
    for p in &s.updated {
        ui::line(format!("  {} {p}", ui::yellow("~")));
    }
    for p in &s.patched {
        ui::line(format!("  {} {p} {}", ui::yellow("~"), ui::dim("(version, description)")));
    }
    if s.created.is_empty() && s.updated.is_empty() && s.patched.is_empty() {
        ui::ok(format!("in sync ({} files)", s.unchanged));
    }
}

pub fn status(spec: &Spec, st: &State) {
    ui::line(format!("  {} {}  {}", ui::bold(&spec.name), ui::dim(format!("v{}", spec.version)), ui::dim(&spec.stack)));
    ui::line(format!("  {}", spec.pitch));
    if !spec.commands.is_empty() {
        let names: Vec<&str> = spec.commands.iter().map(|c| c.name.as_str()).collect();
        ui::line(format!("  {} {}", ui::dim("commands:"), names.join(", ")));
    }
    ui::line(format!("  {} {}", ui::dim("examples:"), spec.examples.len()));
    match &st.last {
        Some(r) => {
            let gates: Vec<String> = r
                .gates
                .iter()
                .map(|g| match g.status {
                    Status::Pass => ui::green(&g.name),
                    Status::Fail => ui::red(&g.name),
                    Status::Skip => ui::dim(&g.name),
                })
                .collect();
            ui::line(format!("  {} {}  {}/{} examples", ui::dim("last verify:"), gates.join(" "), r.passed(), r.examples.len()));
        }
        None => ui::line(format!("  {} never", ui::dim("last verify:"))),
    }
    for run in st.runs.iter().rev().take(8) {
        let mark = if run.ok { ui::green("✓") } else { ui::red("✗") };
        ui::line(format!(
            "  {mark} {} #{} by {} {} {}",
            run.kind,
            run.attempt,
            run.agent,
            ui::dim(util::secs(run.secs)),
            ui::dim(util::clip(&run.note, 70))
        ));
    }
}

pub fn tools(l: &Listing) {
    let width = l.tools.iter().map(|t| t.repo.len()).max().unwrap_or(10).min(42);
    for t in &l.tools {
        tool_line(t, width);
    }
    if let Some(date) = &l.offline {
        ui::line(ui::dim(format!("  (the built-in list from {date}; online it's live)")));
    }
}

fn tool_line(t: &Tool, width: usize) {
    ui::line(format!(
        "  {} {:<width$} {:>6}  {}",
        ui::brand("★"),
        t.repo,
        util::compact(t.stars),
        ui::dim(util::clip(&t.about, 70))
    ));
}

pub fn ways(t: &Tool, ways: &[Way]) {
    ui::line(format!("  {}  {} {}", ui::bold(&t.repo), ui::brand("★"), util::compact(t.stars)));
    if !t.about.is_empty() {
        ui::line(format!("  {}", t.about));
    }
    ui::line(format!("  {}", ui::dim(format!("https://github.com/{}", t.repo))));
    ui::line("");
    for w in ways {
        ui::line(format!("  {:<22} {}", ui::dim(&w.label), w.command));
    }
}
