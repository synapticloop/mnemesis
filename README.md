# Mnemesis

Mnemesis is a filesystem-backed project contract registry for AI agents. A project contract records:

- a unique project name and description;
- one or more possible inputs;
- the file and directory outputs associated with each input;
- typed outcome actions containing instructions for the agent.

## Status

Working scaffold. Toolchain: Rust 1.96 stable. Build, test, and clippy all pass. The release binary lives at `~/.local/bin/mnemesis` on this host.

## Build

```bash
cargo build --release
cargo test
```

Install the binary somewhere on the Hermes terminal `PATH`:

```bash
install -m 0755 target/release/mnemesis ~/.local/bin/mnemesis
```

YAML serialisation uses the `yaml_serde` crate. Output formatting follows block style with list dashes at column 0 (`- name:` rather than `  - name:`); both forms are valid YAML and `load`/`resolve` round-trip identically, but a contract written by the binary will not byte-match a contract written by hand.

## Store

The default store is:

```text
~/.mnemesis/projects/*.yaml
```

Override it with either:

```bash
export MNEMESIS_HOME=/path/to/store
mnemesis --store /path/to/store list
```

Each project is stored as `<project-name>.yaml`.

## Quick start

```bash
mnemesis init
mnemesis create examples/company-website.yaml
mnemesis list
mnemesis search "website deploy"
mnemesis resolve "company website"
mnemesis load company-website
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

The skill only teaches Hermes how to interact with the CLI. Hermes performs discovery with `resolve`, reads the returned contract, selects the applicable input, creates the declared outputs, and follows the instructions for the actual outcome action.

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

## Initial commands

- `init`
- `list`
- `search <query>`
- `resolve <query>`
- `load <project>`
- `validate <file>`
- `create <file>`
- `upsert <file>`
- `remove <project>`
