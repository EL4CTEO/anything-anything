//! anything-anything: make your own -anything tool. Describe it in a line; your coding agent builds it
//! in the stack you want; its own examples prove it works.

mod agent;
mod check;
mod codegen;
mod doctor;
mod forge;
mod gate;
mod hub;
mod logo;
mod mcp;
mod names;
mod proc;
mod publish;
mod render;
mod scaffold;
mod show;
mod spec;
mod stack;
mod state;
mod task;
mod templates;
mod ui;
mod util;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use spec::Spec;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "anything-anything", version, about, arg_required_else_help = true)]
#[command(
    after_help = "One command:\n  anything-anything new translate \"translate any app's UI strings to any language\" --agent claude\n\nStep by step:\n  anything-anything new translate \"...\" --stack python-cli   (no agent: write anything.toml yourself)\n  anything-anything build --agent claude\n  anything-anything verify\n  anything-anything publish"
)]
struct Cli {
    /// Project folder (default: the current folder)
    #[arg(short = 'C', long = "dir", global = true, default_value = ".")]
    dir: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone)]
struct AgentArgs {
    /// The coding agent: claude, codex, or any command that reads a prompt on stdin
    #[arg(long)]
    agent: Option<String>,
    /// Tries before giving up (each one gets the last one's failures)
    #[arg(long, default_value_t = 3)]
    attempts: usize,
    /// Minutes an agent run may take
    #[arg(long, default_value_t = 60)]
    minutes: u64,
}

impl AgentArgs {
    fn opts(&self) -> Option<forge::AgentOpts> {
        self.agent.clone().map(|agent| forge::AgentOpts {
            agent,
            attempts: self.attempts.max(1),
            timeout: Duration::from_secs(self.minutes.max(1) * 60),
        })
    }

    fn need(&self) -> Result<forge::AgentOpts> {
        match self.opts() {
            Some(o) => Ok(o),
            None => bail!(
                "which agent? --agent claude, --agent codex, or --agent \"any command that reads a prompt on stdin\""
            ),
        }
    }
}

#[derive(Subcommand, Clone)]
enum HubCmd {
    /// The most-starred -anything tools
    Top {
        #[arg(long, default_value_t = 25)]
        limit: usize,
        /// Use the built-in list, don't ask GitHub
        #[arg(long)]
        offline: bool,
    },
    /// Search -anything tools
    Search {
        #[arg(required = true)]
        words: Vec<String>,
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long)]
        offline: bool,
    },
    /// Tools made with anything-anything
    Made {
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// What a tool is and every way to install it (owner/repo, or a name)
    Show { repo: String },
    /// Install a tool the best way it can be (asks first)
    Install {
        repo: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
enum Cmd {
    /// Make a new tool: design it from your idea, generate it, have your agent build it, verify it
    New {
        /// What it does to anything: translate becomes translate-anything
        what: String,
        /// The idea, in a line
        idea: Vec<String>,
        /// rust-cli, python-cli, node-cli, web or any (default: the agent picks)
        #[arg(long)]
        stack: Option<String>,
        /// Start from this anything.toml instead of designing one
        #[arg(long)]
        spec: Option<PathBuf>,
        /// Where to make it (default: ./<name>)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Design and generate it, don't build it yet
        #[arg(long)]
        no_build: bool,
        /// GitHub user or organization (default: who gh is logged in as)
        #[arg(long)]
        owner: Option<String>,
        /// npm package name (default: @<owner>/<name>)
        #[arg(long)]
        npm: Option<String>,
        /// Brand colour, #rrggbb
        #[arg(long)]
        color: Option<String>,
        /// Don't git init
        #[arg(long)]
        no_git: bool,
        #[command(flatten)]
        agent: AgentArgs,
    },
    /// Have your agent (re)write anything.toml from an idea
    Design {
        idea: Vec<String>,
        #[command(flatten)]
        agent: AgentArgs,
    },
    /// Have your agent build it until every gate passes
    Build {
        #[command(flatten)]
        agent: AgentArgs,
    },
    /// Write .anything/TASK.md, what's left to build, for any agent (or you)
    Task,
    /// Regenerate the files anything-anything owns from anything.toml. Never touches your code
    Sync {
        /// Only say what would change
        #[arg(long)]
        check: bool,
    },
    /// Every gate: spec, generated files, stubs, the stack's steps, MCP, examples
    Verify {
        #[arg(long)]
        json: bool,
        /// Only run the examples whose names contain this (any case; repeat it for more)
        #[arg(long = "example", short = 'e', value_name = "TEXT")]
        examples: Vec<String>,
    },
    /// The spec, the last verify and every build
    Status,
    /// The anything.toml format, every field
    Spec,
    /// The stacks a tool can be built on
    Stacks,
    /// Every -anything tool on GitHub
    Hub {
        #[command(subcommand)]
        cmd: Option<HubCmd>,
    },
    /// Push it to GitHub and tag a release (shows the plan; --yes does it)
    Publish {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        private: bool,
    },
    /// What's installed: toolchains, git and gh, agents
    Doctor,
    /// Run as an MCP server on stdio (Claude Code, Codex, Cursor, ...)
    Mcp,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = util::absolute(&cli.dir);
    match run(cli.cmd, &root) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            ui::error(format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}

fn finish(report: &gate::Report, root: &Path) -> bool {
    show::report(report);
    if report.ok {
        // a partial run proves only part of it: the whole verify comes before publish
        let next = if report.only.is_empty() { "publish" } else { "verify" };
        ui::next(format!("anything-anything -C {} {next}", root.display()));
    }
    report.ok
}

fn run(cmd: Cmd, root: &Path) -> Result<bool> {
    match cmd {
        Cmd::Mcp => return mcp::serve().map(|()| true),
        Cmd::Spec => {
            println!("{}", spec::REFERENCE);
            return Ok(true);
        }
        Cmd::Verify { json: true, examples } => {
            ui::set_quiet(true);
            let r = gate::verify_only(root, &examples)?;
            println!("{}", serde_json::to_string_pretty(&r)?);
            return Ok(r.ok);
        }
        _ => {}
    }
    ui::banner();
    match cmd {
        Cmd::New { what, idea, stack, spec, out, no_build, owner, npm, color, no_git, agent } => {
            let opts = forge::NewOpts {
                what,
                idea: idea.join(" "),
                stack,
                spec,
                out,
                owner,
                npm,
                color,
                git: !no_git,
                agent: agent.opts(),
                build: !no_build,
            };
            let has_agent = opts.agent.is_some();
            let (dir, report) = forge::create(root, opts)?;
            match report {
                Some(r) => Ok(finish(&r, &dir)),
                None => {
                    let spec = Spec::load(&dir)?;
                    let problems = spec.problems(&stack::load(&spec)?);
                    if problems.is_empty() {
                        forge::write_task(&dir, &[])?;
                        ui::ok("ready to build: .anything/TASK.md says what");
                        ui::next(format!("anything-anything -C {} build --agent claude", dir.display()));
                    } else {
                        ui::line(format!("  {} to finish in {}:", ui::bold("the spec"), spec::FILE));
                        for p in &problems {
                            ui::line(format!("  {} {p}", ui::dim("·")));
                        }
                        ui::next(if has_agent {
                            format!("anything-anything -C {} design --agent claude", dir.display())
                        } else {
                            format!(
                                "edit {}, or: anything-anything -C {} design --agent claude",
                                dir.join(spec::FILE).display(),
                                dir.display()
                            )
                        });
                    }
                    Ok(true)
                }
            }
        }
        Cmd::Design { idea, agent } => {
            let spec = Spec::load(root)?;
            forge::design(
                root,
                &idea.join(" "),
                Some(&spec.stack).filter(|s| *s != "any").map(|s| s.as_str()),
                &agent.need()?,
            )?;
            show::synced(&scaffold::sync(root, &Spec::load(root)?, false)?);
            ui::next("anything-anything build --agent claude");
            Ok(true)
        }
        Cmd::Build { agent } => {
            let r = forge::build(root, &agent.need()?)?;
            Ok(finish(&r, root))
        }
        Cmd::Task => {
            let p = forge::write_task(root, &[])?;
            ui::ok(format!("wrote {}: hand it to your agent, then run anything-anything verify", p.display()));
            Ok(true)
        }
        Cmd::Sync { check } => {
            let spec = Spec::load(root)?;
            let s = scaffold::sync(root, &spec, check)?;
            show::synced(&s);
            Ok(!check || (s.created.is_empty() && s.updated.is_empty()))
        }
        Cmd::Verify { examples, .. } => {
            let r = gate::verify_only(root, &examples)?;
            Ok(finish(&r, root))
        }
        Cmd::Status => {
            show::status(&Spec::load(root)?, &state::load(root));
            Ok(true)
        }
        Cmd::Stacks => {
            for (id, about) in stack::all() {
                ui::line(format!("  {} {id:<11} {}", ui::brand("•"), about));
            }
            Ok(true)
        }
        Cmd::Hub { cmd } => {
            match cmd.unwrap_or(HubCmd::Top { limit: 25, offline: false }) {
                HubCmd::Top { limit, offline } => show::tools(&hub::list("", limit, offline)),
                HubCmd::Search { words, limit, offline } => show::tools(&hub::list(&words.join(" "), limit, offline)),
                HubCmd::Made { limit } => show::tools(&hub::Listing { tools: hub::made(limit)?, offline: None }),
                HubCmd::Show { repo } => {
                    let (t, w) = hub::show(&repo)?;
                    show::ways(&t, &w);
                }
                HubCmd::Install { repo, yes } => hub::install(&repo, yes)?,
            }
            Ok(true)
        }
        Cmd::Publish { yes, private } => {
            publish::run(root, yes, private)?;
            Ok(true)
        }
        Cmd::Doctor => {
            doctor::run()?;
            Ok(true)
        }
        Cmd::Mcp | Cmd::Spec => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line() {
        let cli = Cli::try_parse_from([
            "anything-anything",
            "new",
            "translate",
            "translate",
            "any",
            "app",
            "--agent",
            "claude",
            "--stack",
            "web",
        ])
        .unwrap();
        let Cmd::New { what, idea, stack, agent, .. } = cli.cmd else { panic!("not new") };
        assert_eq!(
            (what.as_str(), idea.join(" ").as_str(), stack.as_deref()),
            ("translate", "translate any app", Some("web"))
        );
        assert_eq!(agent.opts().unwrap().attempts, 3);
        assert!(Cli::try_parse_from(["anything-anything", "hub", "search"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["anything-anything", "-C", "x", "verify", "--json"]).unwrap().cmd,
            Cmd::Verify { json: true, ref examples } if examples.is_empty()
        ));
        let Cmd::Verify { examples, .. } =
            Cli::try_parse_from(["anything-anything", "verify", "--example", "hello", "-e", "count"]).unwrap().cmd
        else {
            panic!("not verify")
        };
        assert_eq!(examples, ["hello", "count"]);
    }
}
