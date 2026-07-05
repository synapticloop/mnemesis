---
name: mnemesis
description: Load and edit project contracts that define inputs, expected filesystem outputs, and outcome-specific instructions. Use when the user asks about a registered project, asks to update or draft a contract, or refers to a project by name/description.
---
# Mnemesis

Mnemesis is a filesystem-backed project contract registry accessed through the `mnemesis` Rust CLI. The CLI has exactly two commands: `load` and `save`. Everything you do with a contract goes through them.

The slash command loads these operating instructions. It does not itself choose or execute a project.

## Storage layout

The default store is `~/.mnemesis/`, containing two subdirectories:

- `projects/<name>.yaml` — committed contracts (the registry).
- `drafts/<name>.yaml` — the working copy you edit before saving.

Override the store root with the `MNEMESIS_HOME` environment variable.

## Commands

```bash
mnemesis load <query>                  # load a contract by name or partial match
mnemesis save <project>                # save the draft for <project> to the registry
mnemesis save <project> --yes          # accept every pending change without further prompting
mnemesis save <project> --accept <path>   # accept only the change at <path>; repeatable
mnemesis verify <project>              # check that every declared output path exists with the right type
mnemesis --schema                      # print the embedded JSON Schema for contracts
```

## Load

`load <query>` does three things in order:

1. **Exact match.** If `projects/<query>.yaml` exists, the contract is loaded verbatim and copied to `drafts/<query>.yaml`. The response includes `status: "loaded"`, the full `project`, and the `draft_path` you should edit.
2. **Fuzzy match.** If no exact match, `load` ranks all projects by similarity (token coverage, jaro-winkler, exact-name bonus) and returns `status: "ambiguous"` with a scored list of candidates.
3. **No match.** If the registry is empty or no candidate scores above the noise floor, the response is `status: "not_found"` with the query echoed back.

### Multi-word queries

The CLI does NOT parse multi-word arguments — `mnemesis load foo bar` exits 2 with "unexpected argument 'bar' found". And if the user pastes "load mmx charts", falling back to the naive hyphen-join (`mmx-charts`) gives a different lookup than either token. Before declaring `not_found`, try the obvious normalisations in order:

1. Hyphen-join the words: `load mmx-charts` from "load mmx charts".
2. Try each individual word as a token (`load mmx`, `load charts`) — one of them may match an existing project that the full phrase wouldn't.
3. Try the most-likely single token (the longer one usually wins, since descriptions are noun-phrases: "load mmx" beats "load charts" when the project is mmx-something).

Only after 2-3 normalisations all return `not_found` should you surface the "no contract yet, want me to draft?" question. Don't ask on the first miss — it's almost always a tokenisation issue.

Edit the file at `draft_path` to make changes. Never edit `projects/*.yaml` directly — they will be overwritten the next time `save` writes that project.

## Save

`save <project>` reads `drafts/<project>.yaml`, validates it, and writes it to `projects/<project>.yaml` atomically. The behaviour depends on whether the project already exists:

- **First save (no existing project).** The draft becomes the registry entry. No diff required.
- **Update save (project exists).** The draft is diffed against the existing project. Every change is reported as a path like `project.description`, `inputs[build].outputs[bin].location`, or `inputs[deploy].actions[successful-deploy].instructions`. For the full path grammar and `--accept` matching rules, see `references/diff-path-grammar.md`.

Three outcomes from `save`:

- **`status: "saved"`** — the project file was updated. The response includes the list of accepted paths and the destination path.
- **`status: "name_mismatch"`** — the draft's `project.name` does not match `<project>`. Edit the draft's `project.name` to match and retry, or pass the matching name on the command line.
- **Pending changes, exit 1** — the draft differs from the existing project. The response prints the full diff as JSON to stderr, listing every `added`, `modified`, or `removed` field. Re-run with `--yes` to accept every change, or with `--accept <path>` (repeatable) to accept only specific fields by their diff path.

Validation runs on every save; a contract with an empty slug, duplicate input name, or missing location is rejected before any file is written.

## Verify

`verify <project>` reads the committed contract from `projects/<project>.yaml` and checks that every declared `outputs[]` path actually exists on the filesystem with the declared type (`file` or `directory`). Tilde-prefixed paths are expanded against `$HOME`.

The response includes:

- `summary`: counts of `total`, `ok`, `missing`, and `wrong_type` checks.
- `details`: one entry per output, with the raw path, declared type, and outcome.

Each entry's `outcome` is one of:

- `ok` — the path exists and matches its declared type.
- `missing` — the path does not exist on disk.
- `wrong_type` with `found: "file"` or `found: "directory"` — the path exists but is the wrong kind.

Exit code is 0 when all checks pass, 1 when any check fails or the project is not in the registry. Use this command to confirm that the contract's claimed filesystem state still matches reality after a build or deploy — `save` does not verify paths itself.

## Tracking the currently loaded contract

The CLI is stateless. There is no global "current project" — each chat session tracks its own.

While working on a contract, keep the following in your working memory for the chat session (persists across `/mnemesis` invocations):

- `loaded_project` — the project name most recently returned by a successful `load`.
- `loaded_draft_path` — the `draft_path` from that response, if any.
- `loaded_at` — when the load happened (so you know if it's still current).

Update these on every `load` call. Clear them only when the session ends or the user moves to a different contract. When the user asks "what's loaded?" or "what contract are you working on?", answer from this memory; if both fields are unset, say "no project is currently loaded in this session".

### Defaulting when the user omits the project name

If the user invokes `/mnemesis save` or `/mnemesis verify` without naming a project, default to the most recent `loaded_project`. The CLI itself doesn't accept a bare `save` or `verify` (clap exits 2), so the agent must substitute the project name before invoking the binary.

Default resolution order:

1. **`loaded_project` from working memory** — use it. This is the common case: the agent loaded the contract earlier in the conversation and the user is following up.
2. **No remembered project, but the conversation is clearly about mnemesis itself** (the slash command is `/mnemesis`, the user is testing the skill, or the most recent topic was the registry itself) — default to `mnemesis`, the self-describing contract that lives at `~/.mnemesis/projects/mnemesis.yaml`.
3. **No remembered project and no clear context** — say "no project is currently loaded; which contract should I save/verify?" and ask.

Never guess at the project name silently when (3) applies — but (1) and (2) are not guesses, they are deterministic defaults derived from session state.

If a `save` call needs the project name and you have lost track, re-run `load` on the most likely candidate — the response will tell you whether you got it right.

## Procedure for working with a contract

1. `mnemesis load <query>` to find or fetch the contract. If the result is `ambiguous`, ask the user which project they meant and re-run with the exact name. If `not_found`, draft a new contract from scratch (see "Drafting" below).
2. When the response is `loaded`, record `loaded_project` and `loaded_draft_path` in your working memory, then read `draft_path` and edit that file.
3. Make the changes in the draft. Keep `project.name` stable unless the user wants a rename (in which case the new name must also be passed as the `<project>` arg to `save`).
4. `mnemesis save <project>` to apply, where `<project>` is your remembered `loaded_project`. If the response is `pending_changes`, inspect the diff JSON. Either:
   - Present the changes to the user and ask which to keep, then re-run `save` with the appropriate `--accept` flags.
   - Or, if the user has already approved the whole edit, re-run `save <project> --yes`.
5. Confirm by reading back with `mnemesis load <project>` and comparing the contract.

### Disambiguating "the project" from "a contract in the registry"

When the user says "load X", X might be:

- **A project name in the registry** — proceed with `load X`.
- **A project the user works on that has no contract yet** — `load X` returns `not_found`. The next step is drafting, not clarifying.
- **A project outside the scope of mnemesis entirely** (e.g. a question about a third-party tool that has nothing to do with the registry) — clarify briefly that mnemesis is for working with contracts in the registry, not for general project documentation.

The signal: if `load X` returns `not_found` and the user previously seemed to expect a contract to exist, the answer is "no contract for X yet; want me to draft one?". If the user's request was about something the registry can never satisfy (a question about a system that has nothing to do with mnemesis), say so and step out. Do not auto-draft if the user clearly wanted something else — but also do not ask "do you want a contract?" before trying `load` first; that wastes a turn.

For self-referential contracts (a contract describing the project the registry itself lives in, or the project you are currently in), drafting in the same turn as the failed load is the right move. See `templates/build-deploy-contract.yaml` for a starting shape.

## Drafting a new contract

When `load` returns `not_found`, draft from scratch:

1. Choose a `project.name` matching `[a-z0-9][a-z0-9-]*`. This is the file basename.
2. Write a one-sentence `project.description` summarising what the project does. This is the primary search surface.
3. List the `inputs[]` — each represents a distinct starting state the agent might face.
4. For each input, declare its `outputs[]` (file or directory paths the agent will create) and `actions[]` (typed outcomes with concrete instructions).
5. Name inputs, outputs, and action types in lowercase kebab-case. Use `successful-*` / `failed-*` prefixes for outcomes.
6. Validate by running `mnemesis load <name>` against the draft (it'll return `not_found`, but the registry read path validates YAML).
7. Save by writing the contract directly to `drafts/<name>.yaml` and running `mnemesis save <name> --yes`.

The full schema is available via `mnemesis --schema` for self-checking.

For a copy-paste starting shape, see `templates/build-deploy-contract.yaml`.

For the conventions used when extending the CLI itself (subcommand shape, response envelopes, exit codes, path expansion, testing), see `references/extending-the-cli.md`.

### What goes in `instructions` (and what doesn't)

`actions[].instructions` describes the **meaning of the outcome** plus any project-specific gotchas — not the recipe for producing it. The contract is *about* the project, not a *manual for* the project.

**Belongs in `instructions`:**

- The semantic intent of the outcome ("build succeeded", "deploy rolled back", "tests passed") in one sentence.
- Project-specific gotchas the next agent wouldn't know — e.g. "the release binary and installed binary must stay in lockstep", "this cron job's python needs `exec /usr/bin/python3` because the default venv lacks matplotlib".
- Cross-cutting invariants — what must be true after this outcome (filesystem state, lock files, version pinning).
- What to *report* to the user (vs the general "report what happened" default).

**Does NOT belong in `instructions`:**

- Step-by-step build / install / test / publish recipes. "Run `cargo build --release`, confirm binary exists, report size" is a description of how to build a Rust project, not what `successful-build` means for this one. Anyone who lands on the contract knows how to build; they need to know what success means here.
- Standard tool usage. `cargo test`, `pytest`, `npm install` — these are general knowledge. Restating them in every contract wastes the agent's context window and bloats the registry.
- Commands that should live in `README.md` or a Makefile. If a sequence is more than two commands, it belongs in the project's own build script, not in the contract.
- Repeating what `verify` already tells the agent. The contract says the output path is `~/foo/bar`; the agent doesn't need the contract to tell it to `ls` that path.

Rule of thumb: if you can delete the sentence without losing anything the next agent wouldn't otherwise know, delete it.

**Before/after example** (mnemesis contract, this project's own self-describing entry):

```yaml
# WRONG — restates how to run cargo
- type: successful-build
  instructions: |
    Run `cargo build --release` from ~/projects/mnemesis.
    Confirm the release binary exists at target/release/mnemesis
    and report its size.

# RIGHT — describes the outcome's meaning and a project-specific invariant
- type: successful-build
  instructions: |
    Build produced the release and debug binaries; report which are
    fresh and which were already current.
```

Same six actions either way, but the second version doesn't waste the agent's time restating `cargo build --release`. It points at the project-specific thing ("which are fresh vs current") that the agent couldn't infer on its own.

### Drafting for an existing-but-unregistered project

A common case: the user is already running the project (scripts in `~/.hermes/scripts/`, code in `~/projects/<name>/`, etc.) and wants the contract to formalise what's there. In this case:

1. **Read the actual code first** — search the project tree for `savefig`, `to_file`, `output`, hard-coded paths. Don't invent output locations; copy them from the source. For cron-driven projects, also read the .sh wrappers — they often pin interpreter paths (`/usr/bin/python3` for matplotlib) that the contract should reference.
2. **Map outputs 1:1** — for each script that produces a file, declare that exact path. If the script writes to `/tmp/foo.png` and `shutil.copy2`s to `/var/www/.../foo.png`, both belong in `outputs[]` (the web copy is the user-visible one, the tmp is the render).
3. **Encode failure modes in `failed-*` actions** — any trap you've already debugged (cron python missing matplotlib, OOM on a specific savefig flag, stale input file) goes into the `instructions` so the next session starts already knowing.
4. **Optionally consolidate the source tree.** The user may want the live scripts copied to `~/projects/<name>/` as part of the same move — confirm before doing this in one pass. If the .sh wrappers hard-code the original script path, flag that they'll need patching to be self-contained.

Don't ask "want me to copy the scripts too?" — just do the contract, then surface the inconsistency (hard-coded path) as a follow-up the user can opt into.

## Rules

- Do not invent project contracts, output locations, output types, or outcome instructions.
- Do not claim an output exists until it has been verified on the filesystem.
- Do not claim deployment succeeded merely because a build succeeded.
- Action types are project-defined strings; interpret them by their instructions rather than assuming a global fixed list.
- Edit `drafts/*.yaml`, never `projects/*.yaml`. The registry file is owned by `save`.
- If a contract is not found, draft a new one rather than guessing at structure.
- When `save` returns `pending_changes`, surface the diff to the user before re-running with `--accept` or `--yes`.

## Common pitfalls

### Bare slash-command invocation does not imply a command

If the user types `/mnemesis` (or `/mnemesis load`, `/mnemesis save`, `/mnemesis verify`) with no further payload, do NOT assume a missing command. The skill may be invoked just to load the procedure into context, to test the slash-command plumbing, or because the user is mid-thought. Working memory stays clear, no command runs, and the agent reports "no project is currently loaded in this session" along with concrete next steps (`load <name>`, `verify <name>`, list known contracts). Three bare invocations in one session is a strong signal the user is testing plumbing, not asking for work.

If the user types `/mnemesis load` or `/mnemesis save` or `/mnemesis verify` with no arg, clap rejects it with exit 2 and a usage line. Surface the usage verbatim — do not paraphrase the help text.

### Installed skill and repo skill can diverge

The installed skill at `~/.hermes/skills/mnemesis/` is the version Hermes loads at session start. The repo source at `~/projects/mnemesis/hermes-skill/mnemesis/` is what gets pushed to github. They can drift if either side is edited locally. If the loaded skill content seems richer than what you last committed (new subsections, reference files, templates), or if a feature described in the skill isn't actually implemented in the CLI, check `diff ~/.hermes/skills/mnemesis/SKILL.md ~/projects/mnemesis/hermes-skill/mnemesis/SKILL.md` and offer to sync the repo. The reverse is also possible: agent edits a copy and forgets to sync the other.

### `patch` mangles multiline YAML block scalars

When `old_string` spans the interior of a `instructions: |` block (or any block-scalar region), `patch` can re-indent the replacement continuation lines, leaving YAML that parses but looks visually wrong (e.g. 16-space indent inside a 6-space block). Verified by:

```bash
python3 -c "import yaml; yaml.safe_load(open('~/.mnemesis/drafts/<name>.yaml'))"
```

For wide rewrites of a block scalar (more than 2-3 lines), prefer `write_file` over `patch`. Use `patch` only for single-line edits or short blocks where you can eyeball the result.

### `patch` on last-line no-trailing-newline files concatenates the next byte

When `old_string` is the last line of a file with no terminating newline, the byte immediately after (often `>/dev/null` on a bash wrapper) gets concatenated into the replacement line, producing unterminated quotes or missing-newline syntax errors. Bash wrappers are a common victim:

```
# WRONG — bash -n will catch the missing close-quote
exec /usr/bin/python3 "$(dirname "$0")/mmx_heatmap_chart.py "$@" >/dev/null
```

Mitigations:

- Include the trailing `\n` explicitly in `old_string` (and `new_string`).
- For whole-file wrapper rewrites, use `write_file`.
- Always verify after: `bash -n path/to/wrapper.sh` for `.sh`, `python3 -c "import yaml; ..."` for YAML.

### Output paths in instructions: prefer `~` over `/home/<user>/`

When writing path references into a contract's `instructions` blocks, use `~/projects/<name>/...` and `~/hermes/...` — not `/home/<user>/...`. The contract may be shared, copied, or read by other agents; absolute home paths embed the current user's identity. This matches the broader "no user-specific paths in committed files" rule that applies to all committed artefacts.

### `MNEMESIS_HOME` set in a stale shell silently redirects the entire CLI

A surprising number of `not_found` results come from a leftover `export MNEMESIS_HOME=/tmp/some-smoke-test-dir` from earlier in the session. The CLI honours `MNEMESIS_HOME` over the default `~/.mnemesis/`, so a stale export makes `load`, `verify`, and `save` all look at the wrong store while the contracts at `~/.mnemesis/projects/` keep accumulating. Symptom: `verify mnemesis` returns `not_found` even though the contract is plainly visible in `ls ~/.mnemesis/projects/`. Before debugging the contract itself, run `echo "$MNEMESIS_HOME"` and unset it if it points somewhere unexpected (`unset MNEMESIS_HOME`). The installed shell session may carry this across `terminal()` invocations within one Hermes chat session.