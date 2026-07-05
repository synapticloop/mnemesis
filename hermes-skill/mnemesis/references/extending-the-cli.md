# Extending the mnemesis CLI

When the user asks for a new subcommand, follow these conventions. They were established by the `verify` subcommand and match the rest of the CLI surface.

## Anatomy of a new subcommand

A subcommand is five things:

1. **Variant on the `Command` enum** in `src/main.rs`. The `///` doc comment becomes the help text.
2. **Arm in the `run()` match.** If it can fail by exiting non-zero, do `Command::Foo { .. } => foo(...)?,` and let the function call `std::process::exit(code)`.
3. **Top-level function** (e.g. `fn verify(project: &str) -> Result<()>`) that does the work and calls `print_value(&Message { ok: true, result: ... })?`.
4. **Result enum** if the command has more than one outcome (e.g. `VerifyResult::{Verified, NotFound}`).
5. **Response shapes** — usually a `summary` struct with counts plus a `details` array.

## Response conventions

All responses are wrapped in `Message<T> { ok: bool, result: T }`. Successful runs use `ok: true`; the CLI's top-level `main()` wraps `run()` and emits `ok: false` on any `Err`. So **never** set `ok: false` from inside a subcommand handler — let the error propagate.

Subcommands that have outcome variants return a `#[serde(tag = "status", rename_all = "snake_case")]` enum. The `status` field becomes the discriminator. Examples in the code:

- `LoadResult::{Loaded, Ambiguous, NotFound}`
- `SaveOutcome::{Saved, NameMismatch}`
- `VerifyResult::{Verified, NotFound}`

## Exit codes

- `0` — success (every check passed, project found and acted on, etc.)
- `1` — soft failure that still produced a structured response (project not found, pending changes, verify found missing paths). Print the JSON response, then `std::process::exit(1)`.
- `2` — clap rejected the arguments (e.g. unknown subcommand, missing required arg). clap handles this automatically; do not catch.

For read-only commands (`verify`, future `list`, future `config`), exit 1 means "the operation succeeded in producing output, but the result indicates a problem you should act on." This matches shell conventions for tools like `grep` (exit 0 = found, exit 1 = not found).

## Path expansion

Contract output locations use `~/...` syntax. The CLI does **not** use `std::fs::canonicalize` or shell expansion — there is a helper:

```rust
fn expand_path(raw: &str) -> std::path::PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(home).join(stripped);
        }
    }
    std::path::PathBuf::from(raw)
}
```

Reuse this in any new command that touches the filesystem on behalf of a contract.

## How to add `verify` (worked example)

The `verify` subcommand was added with these pieces:

- New `Verify { project: String }` variant on `Command`.
- `CheckOutcome::{Ok, Missing, WrongType { found: &'static str }}` enum (tagged).
- `PathCheck { path, declared_type, outcome }` struct.
- `VerifySummary { total, ok, missing, wrong_type }` struct with counts.
- `VerifyResult::{Verified { project, summary, details }, NotFound { project }}` enum.
- `verify(project: &str) -> Result<()>` function: loads the project, walks `inputs[].outputs[]`, calls `check_output` per output, builds summary, prints JSON, exits 1 if any check failed or the project was missing.

If a future subcommand needs to check filesystem paths in the same way, copy the `check_output` function and the `PathCheck` shape. They are the right primitives.

## Testing new subcommands

Add at least one test per outcome variant in `tests/cli.rs`. Use `tempfile::tempdir()` to isolate state, set `MNEMESIS_HOME` to the temp dir, and write the project YAML directly under `<tmp>/projects/<name>.yaml`. Pattern:

```rust
let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
    .args(["new-cmd", "<args>"])
    .env("MNEMESIS_HOME", dir.path())
    .output()
    .unwrap();
assert!(output.status.success());
let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
```

For commands that print to stderr on failure (`save`'s `pending_changes` JSON, future error paths), combine `output.stdout` and `output.stderr` before parsing.

## Build / lint / install

After changing source:

```bash
cd ~/projects/mnemesis
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
install -m 0755 target/release/mnemesis ~/.local/bin/mnemesis
```

Then test against a real contract in `~/.mnemesis/projects/` to confirm the release binary works. The skill installation in `~/.hermes/skills/mnemesis/` should also be kept in sync with the source repo copy at `~/projects/mnemesis/hermes-skill/mnemesis/`.

## Anti-patterns to avoid

- **Interactive prompts** — the CLI is fully JSON-in/JSON-out. If a confirmation flow is needed, expose it as a flag (`--yes`, `--accept <path>`) that the caller passes explicitly. Never read from stdin.
- **Global state** — the CLI is intentionally stateless across invocations. Do not add a session file, lock file, or "current project" tracker. If you need per-session state, the agent owns it in working memory; see the main SKILL.md.
- **Mutating commands that don't go through `save`** — anything that writes to `projects/*.yaml` should be a write through `store.write_project()`. Drafts are written via `store.seed_draft()`. Both use atomic tmp+rename.
- **Implicit path resolution** — do not assume paths are relative to CWD, to `$HOME`, or to the registry root. Use the declared location verbatim (after `~/` expansion) so the agent can predict what the contract is checking against.