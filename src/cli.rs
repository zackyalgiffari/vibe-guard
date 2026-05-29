use crate::config::Config;
use crate::engine::{classify, StructuredDiff};
use crate::indexer::{self, CoverageReport};
use crate::intent::{self, IntentReport};
use crate::report::Outcome;
use crate::{git, safety, summary};
use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use owo_colors::OwoColorize;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "vibe-guard",
    version,
    about = "Semantic Diff & Intent Guard — a structural validation layer for vibe coders"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze the current diff and verify it against your stated intent.
    Check(CheckArgs),
    /// Print the resolved config (creating a default file if none exists).
    Config,
    /// Manage the local file-freshness index.
    Index {
        #[command(subcommand)]
        cmd: IndexCmd,
    },
}

#[derive(Subcommand)]
enum IndexCmd {
    /// Rebuild the index from the current HEAD.
    Sync,
}

#[derive(Args)]
struct CheckArgs {
    /// The developer's intent / original prompt.
    #[arg(long)]
    intent: Option<String>,
    /// Path to a pre-generated unified diff (default: git diff HEAD).
    #[arg(long)]
    diff: Option<PathBuf>,
    /// Override the local LLM model (default: from config).
    #[arg(long)]
    model: Option<String>,
    /// Comma-separated language filter (rust,ts,py,go).
    #[arg(long)]
    lang: Option<String>,
    /// Skip the Intent Guard; run AST analysis only.
    #[arg(long = "no-llm")]
    no_llm: bool,
    /// Auto-approve non-sensitive, high-confidence changes.
    #[arg(long)]
    yes: bool,
    /// Output the report as JSON.
    #[arg(long)]
    json: bool,
}

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Check(args) => run_check(args),
        Command::Config => run_config(),
        Command::Index { cmd } => match cmd {
            IndexCmd::Sync => {
                let n = indexer::sync()?;
                println!(
                    "Indexed {n} files at HEAD → {}",
                    Config::dir().join("index.json").display()
                );
                Ok(ExitCode::SUCCESS)
            }
        },
    }
}

fn run_config() -> Result<ExitCode> {
    let cfg = Config::load()?;
    let path = if !Config::path().exists() {
        let p = cfg.save()?;
        println!("Created default config at {}", p.display());
        p
    } else {
        Config::path()
    };
    println!("# {}", path.display());
    println!("{}", toml::to_string_pretty(&cfg)?);
    Ok(ExitCode::SUCCESS)
}

fn run_check(args: CheckArgs) -> Result<ExitCode> {
    let cfg = Config::load()?;
    let lang_filter: Vec<String> = args
        .lang
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Build the structured diff, either from a patch file or from git.
    let structured: StructuredDiff = if let Some(diff_path) = &args.diff {
        let patch = std::fs::read_to_string(diff_path)?;
        classify::build_from_patch(&patch, &lang_filter)?
    } else {
        if !git::in_repo() {
            bail!(
                "not inside a git repository. Run inside a repo, or pass --diff <file>.\n\
                 (To start tracking this project: `git init && git add -A && git commit -m init`)"
            );
        }
        let files = git::changed_files()?;
        classify::build(&files, &lang_filter)?
    };

    let summary_line = summary::vibe_line(&structured);

    // Phase 3 context coverage + intent guard (skipped with --no-llm).
    let paths: Vec<String> = structured.files.iter().map(|f| f.path.clone()).collect();
    let coverage: Option<CoverageReport> = if args.no_llm || args.diff.is_some() {
        None
    } else {
        Some(indexer::coverage(&paths)?)
    };

    let intent_report: Option<IntentReport> = if args.no_llm {
        None
    } else {
        let cov = coverage.clone().unwrap_or(CoverageReport {
            total: 0,
            fresh: 0,
            stale: vec![],
            uncovered: vec![],
        });
        Some(intent::evaluate(
            args.intent.as_deref().unwrap_or(""),
            &structured,
            &cov,
            &cfg,
            args.model.as_deref(),
        ))
    };

    // Phase 4 safety scan (git mode only — needs working-tree content).
    let sensitive = if args.diff.is_some() {
        Vec::new()
    } else {
        safety::scan(&structured, &cfg)
    };

    let outcome = Outcome {
        diff: &structured,
        summary: &summary_line,
        coverage: coverage.as_ref(),
        intent: intent_report.as_ref(),
        sensitive: &sensitive,
        confidence_threshold: cfg.confidence_threshold,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&outcome.to_json())?);
        return Ok(ExitCode::SUCCESS);
    }

    outcome.print_human();

    decide(&outcome, &cfg, args.yes)
}

/// Decide whether to approve, prompting the developer when warranted.
fn decide(outcome: &Outcome, cfg: &Config, yes: bool) -> Result<ExitCode> {
    let has_functional = outcome.diff.functional_files().count() > 0;

    // Sensitive files always require an explicit typed confirmation.
    if !outcome.sensitive.is_empty() {
        println!();
        println!(
            "{}",
            "⚠️  Sensitive files are involved. Type 'CONFIRM' to proceed."
                .red()
                .bold()
        );
        let answer = prompt("Confirm [type CONFIRM]: ")?;
        return Ok(approved(answer.trim() == "CONFIRM"));
    }

    let low_confidence = outcome
        .intent
        .map(|r| {
            r.confidence < cfg.confidence_threshold || !r.intent_match || !r.side_effects.is_empty()
        })
        .unwrap_or(false);

    let needs_prompt = if outcome.intent.is_some() {
        low_confidence
    } else {
        // --no-llm: warn only when there are real (non-boilerplate) changes.
        has_functional
    };

    if !has_functional {
        println!();
        println!(
            "{}",
            "✅ No functional changes — nothing to review.".green()
        );
        return Ok(ExitCode::SUCCESS);
    }

    if !needs_prompt {
        println!();
        println!("{}", "✅ Approved — proceed.".green());
        return Ok(ExitCode::SUCCESS);
    }

    if yes {
        println!();
        println!(
            "{}",
            "⚠️  Warning present, but --yes given — proceeding.".yellow()
        );
        return Ok(ExitCode::SUCCESS);
    }

    println!();
    let answer = prompt("Proceed? [y/N]: ")?;
    let a = answer.trim().to_ascii_lowercase();
    Ok(approved(a == "y" || a == "yes"))
}

fn approved(ok: bool) -> ExitCode {
    if ok {
        println!("{}", "✅ Approved.".green());
        ExitCode::SUCCESS
    } else {
        println!("{}", "✖ Aborted.".red());
        ExitCode::from(1)
    }
}

/// Print a prompt and read one line of stdin. Non-interactive stdin (EOF)
/// yields an empty string, which the callers treat as "decline".
fn prompt(msg: &str) -> Result<String> {
    print!("{msg}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line)
}
