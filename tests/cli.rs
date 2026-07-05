use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn validates_example_contract() {
    let dir = tempdir().unwrap();
    let contract = dir.path().join("project.yaml");
    fs::write(
        &contract,
        r#"schema_version: 1
project:
  name: example-project
  description: Example project.
inputs:
  - name: source
    description: Source files.
    type: directory
    outputs:
      - name: build
        type: directory
        location: ./dist
    actions:
      - type: successful-build
        instructions: Report the build outputs.
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["validate", contract.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}
