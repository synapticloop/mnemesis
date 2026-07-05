---
name: mnemesis
description: Discover and load project contracts that define inputs, expected filesystem outputs, and outcome-specific instructions.
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
mnemesis --schema                      # print the embedded JSON Schema for contracts
```

## Load

`load <query>` does three things in order:

1. **Exact match.** If `projects/<query>.yaml` exists, the contract is loaded verbatim and copied to `drafts/<query>.yaml`. The response includes `status: "loaded"`, the full `project`, and the `draft_path` you should edit.
2. **Fuzzy match.** If no exact match, `load` ranks all projects by similarity (token coverage, jaro-winkler, exact-name bonus) and returns `status: "ambiguous"` with a scored list of candidates.
3. **No match.** If the registry is empty or no candidate scores above the noise floor, the response is `status: "not_found"` with the query echoed back.

Edit the file at `draft_path` to make changes. Never edit `projects/*.yaml` directly — they will be overwritten the next time `save` writes that project.

## Save

`save <project>` reads `drafts/<project>.yaml`, validates it, and writes it to `projects/<project>.yaml` atomically. The behaviour depends on whether the project already exists:

- **First save (no existing project).** The draft becomes the registry entry. No diff required.
- **Update save (project exists).** The draft is diffed against the existing project. Every change is reported as a path like `project.description`, `inputs[build].outputs[bin].location`, or `inputs[deploy].actions[successful-deploy].instructions`.

Three outcomes from `save`:

- **`status: "saved"`** — the project file was updated. The response includes the list of accepted paths and the destination path.
- **`status: "name_mismatch"`** — the draft's `project.name` does not match `<project>`. Edit the draft's `project.name` to match and retry, or pass the matching name on the command line.
- **Pending changes, exit 1** — the draft differs from the existing project. The response prints the full diff as JSON to stderr, listing every `added`, `modified`, or `removed` field. Re-run with `--yes` to accept every change, or with `--accept <path>` (repeatable) to accept only specific fields by their diff path.

Validation runs on every save; a contract with an empty slug, duplicate input name, or missing location is rejected before any file is written.

## Procedure for working with a contract

1. `mnemesis load <query>` to find or fetch the contract. If the result is `ambiguous`, ask the user which project they meant and re-run with the exact name. If `not_found`, draft a new contract from scratch (see "Drafting" below).
2. When the response is `loaded`, read `draft_path` from the response and edit that file.
3. Make the changes in the draft. Keep `project.name` stable unless the user wants a rename (in which case the new name must also be passed as the `<project>` arg to `save`).
4. `mnemesis save <project>` to apply. If the response is `pending_changes`, inspect the diff JSON. Either:
   - Present the changes to the user and ask which to keep, then re-run `save` with the appropriate `--accept` flags.
   - Or, if the user has already approved the whole edit, re-run `save <project> --yes`.
5. Confirm by reading back with `mnemesis load <project>` and comparing the contract.

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

## Rules

- Do not invent project contracts, output locations, output types, or outcome instructions.
- Do not claim an output exists until it has been verified on the filesystem.
- Do not claim deployment succeeded merely because a build succeeded.
- Action types are project-defined strings; interpret them by their instructions rather than assuming a global fixed list.
- Edit `drafts/*.yaml`, never `projects/*.yaml`. The registry file is owned by `save`.
- If a contract is not found, draft a new one rather than guessing at structure.
- When `save` returns `pending_changes`, surface the diff to the user before re-running with `--accept` or `--yes`.