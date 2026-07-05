# Mnemesis

Mnemesis is a filesystem-backed project contract registry for AI agents. A project contract records:

- a unique project name and description;
- one or more possible inputs;
- the file and directory outputs associated with each input;
- typed outcome actions containing instructions for the agent.

The CLI has two commands — `load` and `save` — backed by a per-project draft workflow. An agent discovers existing contracts via `load`, edits the corresponding draft, then commits with `save`.

## Status

Working scaffold. Toolchain: Rust 1.96 stable. Build, test, and clippy all pass. The release binary lives at `~/.local/bin/mnemesis` on this host.

## Storage layout

```text
~/.mnemesis/
  projects/<name>.yaml    # committed contracts (the registry)
  drafts/<name>.yaml      # working copies for editing
```

Override the store root with the `MNEMESIS_HOME` environment variable.

## Build

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
```

Install the binary somewhere on the Hermes terminal `PATH`:

```bash
install -m 0755 target/release/mnemesis ~/.local/bin/mnemesis
```

YAML serialisation uses the `yaml_serde` crate. Output formatting follows block style with list dashes at column 0 (`- name:` rather than `  - name:`); both forms are valid YAML and round-trip identically, but a contract written by the binary will not byte-match a contract written by hand.

## Commands

```bash
mnemesis load <query>                  # load by exact name, or return ranked candidates
mnemesis save <project>                # save the draft to projects/<project>.yaml
mnemesis save <project> --yes          # accept every pending change
mnemesis save <project> --accept <path>   # accept only this diff path (repeatable)
mnemesis --schema                      # print the embedded JSON Schema
```

## Quick start

```bash
# Load an existing project (seeds a draft you can edit):
mnemesis load company-website

# The response includes draft_path. Edit that file, then save:
mnemesis save company-website          # if changed, prints the diff and exits 1
mnemesis save company-website --yes    # accept everything and write

# Load a fuzzy match to discover projects:
mnemesis load "company site"           # returns ranked candidates if ambiguous

# Create a brand-new contract from the example:
cp examples/company-website.yaml ~/.mnemesis/drafts/my-new-project.yaml
$EDITOR ~/.mnemesis/drafts/my-new-project.yaml   # change project.name, fields, etc.
mnemesis save my-new-project --yes
```

The CLI emits JSON by default so Hermes can interpret results reliably.

## Hermes skill

Copy the skill directory into Hermes:

```bash
mkdir -p ~/.hermes/skills
cp -R hermes-skill/mnemesis ~/.hermes/skills/mnemesis
```

It then becomes available as:

```text
/mnemesis
```

The skill teaches Hermes the load/edit/save workflow plus the draft diff confirmation flow.

## Schema

```yaml
schema_version: 1
project:
  name: company-website
  description: Build and deploy the company website.
inputs:
  - name: website-source
    description: Source directory containing the website.
    type: directory
    outputs:
      - name: build-directory
        type: directory
        location: ./dist
    actions:
      - type: successful-build
        instructions: |
          Verify the output and report the successful build.
```

Names and action types use lowercase kebab-case. Output types are `file` or `directory`.