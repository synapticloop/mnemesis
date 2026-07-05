mod model;
mod search;
mod store;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use model::ProjectContract;
use search::search;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use store::Store;

const CONTRACT_SCHEMA: &str = include_str!("../schema.json");

#[derive(Debug, Parser)]
#[command(name = "mnemesis", version, about)]
struct Cli {
    #[arg(long, global = true, env = "MNEMESIS_HOME")]
    store: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value = "json")]
    format: Format,

    /// Print the JSON Schema for project contracts and exit.
    #[arg(long, global = true)]
    schema: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Json,
    Yaml,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create the store directory.
    Init,
    /// List project names and descriptions.
    List,
    /// Search project names and descriptions and return ranked options.
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Load one project contract by exact project name.
    Load { project: String },
    /// Resolve a project reference to one contract or multiple options.
    Resolve {
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    /// Create a project from a YAML or JSON file.
    Create { file: PathBuf },
    /// Create or replace a project from a YAML or JSON file.
    Upsert { file: PathBuf },
    /// Validate a project file without saving it.
    Validate { file: PathBuf },
    /// Delete a project by exact name.
    Remove { project: String },
}

#[derive(Debug, Serialize)]
struct ProjectSummary {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ResolveResult {
    Resolved { project: ProjectContract },
    Options { query: String, matches: Vec<search::SearchMatch> },
    NotFound { query: String },
}

#[derive(Debug, Serialize)]
struct Message<T: Serialize> {
    ok: bool,
    result: T,
}

fn main() {
    if let Err(error) = run() {
        let payload = serde_json::json!({
            "ok": false,
            "error": error.to_string()
        });
        eprintln!("{}", serde_json::to_string_pretty(&payload).unwrap());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.schema {
        println!("{}", CONTRACT_SCHEMA);
        return Ok(());
    }
    let Some(command) = cli.command else {
        return Err(anyhow::anyhow!("a subcommand is required (try --help)"));
    };
    let store = Store::new(cli.store.unwrap_or_else(Store::default_root));

    match command {
        Command::Init => {
            store.init()?;
            print_value(&Message { ok: true, result: "store initialized" }, cli.format)?;
        }
        Command::List => {
            let summaries: Vec<_> = store
                .list()?
                .into_iter()
                .map(|p| ProjectSummary { name: p.project.name, description: p.project.description })
                .collect();
            print_value(&Message { ok: true, result: summaries }, cli.format)?;
        }
        Command::Search { query, limit } => {
            let matches = search(&store.list()?, &query, limit);
            print_value(&Message { ok: true, result: matches }, cli.format)?;
        }
        Command::Load { project } => {
            let contract = store.load(&project)?;
            print_value(&Message { ok: true, result: contract }, cli.format)?;
        }
        Command::Resolve { query, limit } => {
            if let Ok(project) = store.load(&query) {
                print_value(&Message { ok: true, result: ResolveResult::Resolved { project } }, cli.format)?;
                return Ok(());
            }
            let matches = search(&store.list()?, &query, limit);
            let result = match matches.as_slice() {
                [] => ResolveResult::NotFound { query },
                [only] if only.score >= 0.85 => {
                    ResolveResult::Resolved { project: store.load(&only.name)? }
                }
                [first, second, ..] if first.score >= 0.85 && first.score - second.score >= 0.15 => {
                    ResolveResult::Resolved { project: store.load(&first.name)? }
                }
                _ => ResolveResult::Options { query, matches },
            };
            print_value(&Message { ok: true, result }, cli.format)?;
        }
        Command::Create { file } => {
            let contract = read_contract_file(&file)?;
            let path = store.write(&contract, false)?;
            print_value(&Message { ok: true, result: path.display().to_string() }, cli.format)?;
        }
        Command::Upsert { file } => {
            let contract = read_contract_file(&file)?;
            let path = store.write(&contract, true)?;
            print_value(&Message { ok: true, result: path.display().to_string() }, cli.format)?;
        }
        Command::Validate { file } => {
            let contract = read_contract_file(&file)?;
            contract.validate()?;
            print_value(&Message { ok: true, result: "valid" }, cli.format)?;
        }
        Command::Remove { project } => {
            store.remove(&project)?;
            print_value(&Message { ok: true, result: project }, cli.format)?;
        }
    }
    Ok(())
}

fn read_contract_file(path: &PathBuf) -> Result<ProjectContract> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let contract = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => serde_json::from_str(&raw).context("parse JSON contract")?,
        _ => yaml_serde::from_str(&raw).context("parse YAML contract")?,
    };
    Ok(contract)
}

fn print_value<T: Serialize>(value: &T, format: Format) -> Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(value)?),
        Format::Yaml => print!("{}", yaml_serde::to_string(value)?),
    }
    Ok(())
}
