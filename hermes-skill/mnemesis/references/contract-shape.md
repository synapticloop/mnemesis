# Contract shape

A mnemesis project contract is one YAML file under `~/.mnemesis/projects/<name>.yaml`. The shape:

```yaml
schema_version: 1
project:
  name: company-website              # slug [a-z0-9][a-z0-9-]*
  description: One sentence. This is the search surface.
inputs:
  - name: website-source             # slug
    description: Source directory.
    type: directory                  # file | directory | text | url | other
    outputs:
      - name: build-directory        # slug
        type: directory              # file | directory
        location: ./dist
    actions:
      - type: successful-build       # slug, project-defined vocabulary
        instructions: |
          Confirm every declared output exists.
          Summarise artefacts and their locations.
```

## Validation rules (enforced on every load, draft read, and save)

- `schema_version` must equal `1`
- `project.name`, every `inputs[].name`, every `outputs[].name`, every `actions[].type` must match `[a-z0-9][a-z0-9-]*` (lowercase, hyphens, no leading/trailing hyphen)
- All `name`/`description`/`instructions`/`location` fields must be non-empty after trim
- No duplicate `inputs[].name` within a contract
- No duplicate `outputs[].name` within an input
- No duplicate `actions[].type` within an input
- `outputs[].type` must be one of `file` or `directory`
- `inputs[].type` defaults to absent (treated as a free-form input); when set, must be one of `file | directory | text | url | other`

Validation runs on every entry point that touches a YAML file (`load`, `save`, draft reads, store reads). A broken contract surfaces immediately at whichever entry point touched it, with a JSON error that includes the file path and the validation message.

## Action types are project-defined

The schema does NOT constrain `actions[].type` to a fixed vocabulary. Each project picks its own set (commonly `successful-build`, `failed-build`, `successful-deploy`, `failed-deploy`, `successful-publish`, etc.). When following a contract, the agent reads the `instructions` for the matching action type rather than assuming a global meaning.

## Storage layout

The default store is `~/.mnemesis/`, with two subdirectories:

- `projects/<name>.yaml` — committed contracts (the registry). Owned by `save`.
- `drafts/<name>.yaml` — working copies the agent edits. Created by `load`, mutated by hand or by the agent, consumed by `save`.

Override the store root with the `MNEMESIS_HOME` environment variable. There is no `--store` flag — if you need a per-project store, set `MNEMESIS_HOME` in the relevant shell.

Writes are atomic (tmp + rename); reads re-validate on every load. Multiple drafts per project are NOT possible — `drafts/<name>.yaml` is the single working file for `<name>`, and `load` overwrites it on each invocation.

## Worked example

See `examples/company-website.yaml` in the mnemesis source repo for a multi-action, multi-output contract (4 actions: `successful-build`, `successful-deploy`, `failed-build`, `failed-deploy`; 2 outputs: `build-directory` + `deployment-manifest`). Useful as a copy-paste starting point for new contracts.