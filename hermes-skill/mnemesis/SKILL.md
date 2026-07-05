---
name: mnemesis
description: Discover and load project contracts that define inputs, expected filesystem outputs, and outcome-specific instructions.
---
# Mnemesis

Mnemesis is a filesystem-backed project contract registry accessed through the `mnemesis` Rust CLI.

The slash command loads these operating instructions. It does not itself choose or execute a project.

## When to use

Use Mnemesis when the user refers to a known project, asks what artefacts a project should produce, or asks you to build, deploy, package, publish, or otherwise complete work governed by a stored project contract.

## Commands

Run commands through the terminal:

```bash
mnemesis list
mnemesis search "<project description>"
mnemesis resolve "<project name or description>"
mnemesis load "<exact-project-name>"
```

Mutation commands:

```bash
mnemesis validate ./project.yaml
mnemesis create ./project.yaml
mnemesis upsert ./project.yaml
mnemesis remove "<exact-project-name>"
```

All commands return JSON by default. Use `--format yaml` only when YAML is easier to inspect.

## Contract schema

Print the JSON Schema for project contracts at any time:

```bash
mnemesis --schema
```

Validate a draft before saving:

```bash
mnemesis validate ./project.yaml
```

## Drafting a contract

When composing a contract from a source (README, cron job, verbal description, etc.):

1. Choose a `project.name` matching `[a-z0-9][a-z0-9-]*`. This is the file basename — every contract becomes `~/.mnemesis/projects/<name>.yaml`.
2. Write a one-sentence `project.description`. This is the primary search surface; vague descriptions make `resolve` return unhelpful options.
3. List the `inputs[]` — each represents a distinct starting state the agent might face. Typical contracts have one or two; pick the smallest set that covers the real outcomes.
4. For each input, declare its `outputs[]` (file or directory paths the agent will create) and `actions[]` (typed outcomes with concrete instructions).
5. Name inputs, outputs, and action types in lowercase kebab-case. Use `successful-*` / `failed-*` prefixes for outcomes.
6. Run `mnemesis validate` to catch slug errors, duplicate names, and empty fields before saving.
7. Save with `mnemesis create` (fails on existing name) or `mnemesis upsert` (overwrites).

Validate after editing an existing contract on disk — `load` and `resolve` re-validate every read, so a broken YAML surfaces immediately.

## Procedure

1. Use `mnemesis resolve` with the user's project wording.
2. If the response status is `options`, present the project names and descriptions as concise choices. Do not invent a selection when the options are materially ambiguous.
3. If the response status is `resolved`, read the complete project contract.
4. Determine which declared input applies to the current request.
5. Follow only the outputs associated with that input.
6. Create each output at its declared `location` and respect whether its `type` is `file` or `directory`.
7. Determine the actual outcome, such as `successful-build`, `successful-deploy`, `failed-build`, or another project-defined action type.
8. Follow the `instructions` for the matching action type.

## Rules

- Do not invent project contracts, output locations, output types, or outcome instructions.
- Do not claim an output exists until it has been verified on the filesystem.
- Do not claim deployment succeeded merely because a build succeeded.
- Action types are project-defined strings; interpret them by their instructions rather than assuming a global fixed list.
- If no contract is found, say so and continue only with the user's explicit instructions.
- Use `MNEMESIS_HOME` or `--store` when the registry is not in `~/.mnemesis`.
