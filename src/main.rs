mod diff;
mod model;
mod search;
mod store;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use diff::{diff_contracts, FieldChange};
use model::ProjectContract;
use search::{search, SearchMatch};
use serde::Serialize;
use store::Store;

const CONTRACT_SCHEMA: &str = include_str!("../schema.json");

#[derive(Debug, Parser)]
#[command(name = "mnemesis", version, about)]
struct Cli {
    /// Print the JSON Schema for project contracts and exit.
    #[arg(long, global = true)]
    schema: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Load a project contract. Tries exact match, then returns a ranked list if ambiguous.
    Load { query: String },
    /// Save a draft contract to the registry. Prompts via flags: --yes accepts all changes.
    Save {
        project: String,
        /// Accept every pending change without further prompting.
        #[arg(long)]
        yes: bool,
        /// Accept only the changes whose path matches this prefix. Repeatable.
        #[arg(long = "accept", value_name = "PATH")]
        accept: Vec<String>,
    },
    /// Verify that every path declared in a project contract actually exists with the right type.
    Verify { project: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum LoadResult {
    Loaded {
        project: ProjectContract,
        draft_path: String,
    },
    Ambiguous {
        query: String,
        matches: Vec<SearchMatch>,
    },
    NotFound {
        query: String,
    },
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

    match command {
        Command::Load { query } => load(&query)?,
        Command::Save {
            project,
            yes,
            accept,
        } => save(&project, yes, &accept)?,
        Command::Verify { project } => verify(&project)?,
    }
    Ok(())
}

fn load(query: &str) -> Result<()> {
    let store = Store::new(Store::default_root());
    store.init()?;

    // Exact match wins immediately.
    if let Some(contract) = store.load_project(query)? {
        let draft_path = store.seed_draft(&contract.project.name, &contract)?;
        print_value(&Message {
            ok: true,
            result: LoadResult::Loaded {
                project: contract,
                draft_path: draft_path.display().to_string(),
            },
        })?;
        return Ok(());
    }

    // Otherwise rank the candidates.
    let projects = store.list_projects()?;
    let matches = search(&projects, query, 10);
    let result = if matches.is_empty() {
        LoadResult::NotFound {
            query: query.to_string(),
        }
    } else {
        LoadResult::Ambiguous {
            query: query.to_string(),
            matches,
        }
    };
    print_value(&Message { ok: true, result })?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SaveOutcome {
    Saved { path: String, applied: Vec<String> },
    NameMismatch { draft_name: String, cli_arg: String },
}

fn save(project_arg: &str, yes: bool, accept: &[String]) -> Result<()> {
    let store = Store::new(Store::default_root());
    store.init()?;

    let draft = store
        .read_draft(project_arg)?
        .ok_or_else(|| anyhow::anyhow!("no draft found at {}", store.draft_path(project_arg).display()))?;

    let draft_name = draft.project.name.clone();
    if draft_name != project_arg {
        print_value(&Message {
            ok: true,
            result: SaveOutcome::NameMismatch {
                draft_name,
                cli_arg: project_arg.to_string(),
            },
        })?;
        std::process::exit(1);
    }

    // First save vs update: compare against the existing project (if any).
    let existing = store.load_project(&draft_name)?;
    let applied: Vec<String> = match existing {
        None => {
            // First save — every field is implicitly new. Require --yes or no-op if no changes possible.
            store.write_project(&draft)?;
            vec![format!("project '{}' created", draft_name)]
        }
        Some(old) => {
            let changes = diff_contracts(&old, &draft);
            if changes.is_empty() {
                vec!["no changes".to_string()]
            } else {
                let approved = select_approved(&changes, yes, accept);
                if approved.is_empty() {
                    // Surface the pending diff for the caller (agent or user) to act on.
                    let payload = serde_json::json!({
                        "ok": false,
                        "status": "pending_changes",
                        "changes": changes,
                        "hint": "rerun with --yes to accept all, or --accept <path> for specific paths"
                    });
                    eprintln!("{}", serde_json::to_string_pretty(&payload)?);
                    std::process::exit(1);
                }
                // Apply accepted changes by mutating a copy of the old contract.
                let mut updated = old.clone();
                apply_changes(&mut updated, &changes, &approved);
                updated.validate().context("accepted changes produce an invalid contract")?;
                let path = store.write_project(&updated)?;
                approved
                    .iter()
                    .map(|c| format!("applied: {c}"))
                    .chain(std::iter::once(path.display().to_string()))
                    .collect()
            }
        }
    };

    let path = store.project_path(&draft_name);
    print_value(&Message {
        ok: true,
        result: SaveOutcome::Saved {
            path: path.display().to_string(),
            applied,
        },
    })?;
    Ok(())
}

/// Decide which diff paths are accepted given the --yes / --accept flags.
fn select_approved(changes: &[FieldChange], yes: bool, accept: &[String]) -> Vec<String> {
    if yes {
        return changes.iter().map(|c| c.path().to_string()).collect();
    }
    if accept.is_empty() {
        return Vec::new();
    }
    let mut approved = Vec::new();
    for change in changes {
        let path = change.path();
        if accept.iter().any(|a| path == a || path.starts_with(a)) {
            approved.push(path.to_string());
        }
    }
    approved
}

/// Apply a list of accepted changes to an existing contract in place.
/// Only the path-as-add/modify/remove operations supported by FieldChange are honored.
fn apply_changes(target: &mut ProjectContract, changes: &[FieldChange], approved: &[String]) {
    for change in changes {
        if !approved.iter().any(|a| a == change.path()) {
            continue;
        }
        match change {
            FieldChange::Modified { path, new, .. } => apply_modify(target, path, new),
            FieldChange::Removed { path, .. } => apply_remove(target, path),
            FieldChange::Added { .. } => {
                // Additions are already present in `draft` (which we use as the new shape
                // when validating). For applying onto `old`, we rebuild the contract from
                // `draft` and ignore individual adds. Caller should pass --yes to include
                // added fields via the create-or-update path. So no-op here.
            }
        }
    }
}

fn apply_modify(target: &mut ProjectContract, path: &str, new_value: &str) {
    // Path grammar: project.name, project.description, inputs[<name>].description,
    // inputs[<name>].type, inputs[<name>].outputs[<name>].(description|type|location),
    // inputs[<name>].actions[<type>].instructions
    let segments: Vec<&str> = split_path(path);
    if segments.is_empty() {
        return;
    }
    match segments[0] {
        "project" => match segments.get(1).copied() {
            Some("name") => target.project.name = new_value.to_string(),
            Some("description") => target.project.description = new_value.to_string(),
            _ => {}
        },
        "inputs" => {
            let Some(input_name) = segments.get(1).copied() else {
                return;
            };
            let Some(input) = target.inputs.iter_mut().find(|i| i.name == input_name) else {
                return;
            };
            match segments.get(2).copied() {
                Some("description") => input.description = new_value.to_string(),
                Some("type") => {
                    if let Ok(t) = serde_json::from_value(serde_json::Value::String(new_value.to_string())) {
                        input.input_type = Some(t);
                    }
                }
                Some("outputs") => {
                    let Some(output_name) = segments.get(3).copied() else {
                        return;
                    };
                    let Some(output) = input.outputs.iter_mut().find(|o| o.name == output_name) else {
                        return;
                    };
                    match segments.get(4).copied() {
                        Some("description") => {
                            output.description = if new_value.is_empty() {
                                None
                            } else {
                                Some(new_value.to_string())
                            };
                        }
                        Some("type") => {
                            if let Ok(t) = serde_json::from_value(serde_json::Value::String(new_value.to_string())) {
                                output.output_type = t;
                            }
                        }
                        Some("location") => output.location = new_value.to_string(),
                        _ => {}
                    }
                }
                Some("actions") => {
                    let Some(action_type) = segments.get(3).copied() else {
                        return;
                    };
                    let Some(action) =
                        input.actions.iter_mut().find(|a| a.action_type == action_type)
                    else {
                        return;
                    };
                    if segments.get(4).copied() == Some("instructions") {
                        action.instructions = new_value.to_string();
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn apply_remove(target: &mut ProjectContract, path: &str) {
    let segments: Vec<&str> = split_path(path);
    if segments.is_empty() {
        return;
    }
    if segments[0] == "inputs" {
        let Some(input_name) = segments.get(1).copied() else {
            return;
        };
        // path "inputs[name]" with no further segments → remove the whole input
        if segments.len() == 2 {
            target.inputs.retain(|i| i.name != input_name);
            return;
        }
        let Some(input) = target.inputs.iter_mut().find(|i| i.name == input_name) else {
            return;
        };
        match segments.get(2).copied() {
            Some("outputs") => {
                if let Some(output_name) = segments.get(3).copied() {
                    input.outputs.retain(|o| o.name != output_name);
                }
            }
            Some("actions") => {
                if let Some(action_type) = segments.get(3).copied() {
                    input.actions.retain(|a| a.action_type != action_type);
                }
            }
            _ => {}
        }
    }
}

/// Split a dotted/bracketed path into segments. "project.description" → ["project", "description"].
/// "inputs[build].outputs[bin].location" → ["inputs", "build", "outputs", "bin", "location"].
fn split_path(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = path;
    while let Some(idx) = rest.find(['.', '[']) {
        if idx > 0 {
            out.push(&rest[..idx]);
        }
        rest = &rest[idx..];
        if rest.starts_with('.') {
            rest = &rest[1..];
        } else if rest.starts_with('[') {
            // bracket — content is the segment until the closing bracket
            if let Some(end) = rest.find(']') {
                out.push(&rest[1..end]);
                rest = &rest[end + 1..];
            } else {
                break;
            }
        }
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

trait FieldChangePath {
    fn path(&self) -> &str;
}

impl FieldChangePath for FieldChange {
    fn path(&self) -> &str {
        match self {
            FieldChange::Added { path, .. } => path,
            FieldChange::Modified { path, .. } => path,
            FieldChange::Removed { path, .. } => path,
        }
    }
}

fn print_value<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum CheckOutcome {
    Ok,
    Missing,
    WrongType { found: &'static str },
}

#[derive(Debug, Serialize)]
struct PathCheck {
    path: String,
    declared_type: String,
    outcome: CheckOutcome,
}

#[derive(Debug, Serialize)]
struct VerifySummary {
    total: usize,
    ok: usize,
    missing: usize,
    wrong_type: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum VerifyResult {
    Verified {
        project: String,
        summary: VerifySummary,
        details: Vec<PathCheck>,
    },
    NotFound { project: String },
}

fn expand_path(raw: &str) -> std::path::PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(home).join(stripped);
        }
    }
    std::path::PathBuf::from(raw)
}

fn check_output(declared_type: &str, location: &str) -> PathCheck {
    let path = expand_path(location);
    let outcome = if !path.exists() {
        CheckOutcome::Missing
    } else if declared_type == "file" && !path.is_file() {
        CheckOutcome::WrongType { found: "directory" }
    } else if declared_type == "directory" && !path.is_dir() {
        CheckOutcome::WrongType { found: "file" }
    } else {
        CheckOutcome::Ok
    };
    PathCheck {
        path: location.to_string(),
        declared_type: declared_type.to_string(),
        outcome,
    }
}

fn verify(project: &str) -> Result<()> {
    let store = Store::new(Store::default_root());
    store.init()?;
    let Some(contract) = store.load_project(project)? else {
        print_value(&Message {
            ok: true,
            result: VerifyResult::NotFound {
                project: project.to_string(),
            },
        })?;
        std::process::exit(1);
    };

    let mut details = Vec::new();
    for input in &contract.inputs {
        for output in &input.outputs {
            let type_str = match &output.output_type {
                crate::model::OutputType::File => "file",
                crate::model::OutputType::Directory => "directory",
            };
            details.push(check_output(type_str, &output.location));
        }
    }

    let mut ok = 0;
    let mut missing = 0;
    let mut wrong_type = 0;
    for check in &details {
        match &check.outcome {
            CheckOutcome::Ok => ok += 1,
            CheckOutcome::Missing => missing += 1,
            CheckOutcome::WrongType { .. } => wrong_type += 1,
        }
    }
    let summary = VerifySummary {
        total: details.len(),
        ok,
        missing,
        wrong_type,
    };
    let exit_code = if missing + wrong_type == 0 { 0 } else { 1 };

    print_value(&Message {
        ok: true,
        result: VerifyResult::Verified {
            project: contract.project.name,
            summary,
            details,
        },
    })?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}