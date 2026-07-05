# Diff path grammar

When `mnemesis save` finds the draft differs from the existing project, it returns a diff where every change is identified by a stable path. The grammar is what `--accept <path>` matches against.

## Path forms

```
project.name
project.description
inputs[<input-name>].description
inputs[<input-name>].type
inputs[<input-name>].outputs[<output-name>].description
inputs[<input-name>].outputs[<output-name>].type
inputs[<input-name>].outputs[<output-name>].location
inputs[<input-name>].actions[<action-type>].instructions
```

- `.` separates a section from a field
- `[<value>]` addresses a specific item in an array by its `name` (for inputs/outputs) or `type` (for actions)
- The path is unique per field — no two fields in a contract share the same path

## Matching semantics for `--accept`

`--accept` matches by **prefix**:

- `--accept project.description` matches the one field `project.description`
- `--accept inputs[deploy]` matches every change under `inputs[deploy]` — its description, type, any of its outputs, any of its actions
- `--accept inputs[deploy].actions[successful-deploy]` matches that one action's instructions only

Multiple `--accept` flags are OR'd together. `--yes` accepts everything regardless of `--accept`.

## Diff response shape

Each change in the diff response has this shape:

```json
{
  "kind": "modified",
  "path": "project.description",
  "old": "Build and deploy the company website.",
  "new": "Build, test, and deploy the company website."
}
```

Three kinds:

- `added` — a new field that wasn't in the old version (new input, new output, new action, new top-level field)
- `modified` — the field exists in both, values differ
- `removed` — a field in the old version was deleted in the new (a whole input, output, or action removed)

When `--yes` is passed, every change is accepted. The CLI re-validates the resulting contract before writing; if accepting the chosen changes produces an invalid contract (e.g. removing the only input of a project that requires at least one), the save fails with a validation error and nothing is written.

## Programmatic use

When an LLM calls `mnemesis save <project>` and the response is `pending_changes`, parse the diff JSON and decide which paths to accept. Two common strategies:

- **Conservative:** `mnemesis save <project> --accept <path>` for each path the user explicitly approved. Skip anything not approved.
- **Bulk:** `mnemesis save <project> --yes` when the user has approved the whole edit (e.g. after seeing the diff and saying "go ahead").

Never pass `--accept project.name` — renaming a project is a destructive operation (it moves the file from `projects/<old>.yaml` to `projects/<new>.yaml`), and the CLI does not currently support it through the diff flow. If the user wants to rename, do `mnemesis remove <old>` followed by `mnemesis save <new>`.

## Reusing this grammar

The path-with-bracketed-segment pattern (project.section.field, arrays addressed by stable key inside `[...]`) is a reusable shape for any field-level diff system. If you build a similar feature elsewhere, prefer naming segments (inputs by `name`, actions by `type`) over indices — indices shift when items are added/removed and break diff references; stable names don't.