use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn load_exact_match_seeds_draft() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();
    let contract = dir.path().join("projects/example-project.yaml");
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
        .args(["load", "example-project"])
        .env("MNEMESIS_HOME", dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "load exact match should succeed");

    let draft = dir.path().join("drafts/example-project.yaml");
    assert!(draft.exists(), "load should seed the draft file");
}

#[test]
fn load_not_found_returns_not_found_status() {
    let dir = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["load", "definitely-not-a-project"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(
        body.contains("\"not_found\""),
        "expected not_found status, got: {body}"
    );
}

#[test]
fn save_creates_project_from_draft() {
    let dir = tempdir().unwrap();
    let draft = dir.path().join("drafts/new-project.yaml");
    fs::create_dir_all(draft.parent().unwrap()).unwrap();
    fs::write(
        &draft,
        r#"schema_version: 1
project:
  name: new-project
  description: A new project.
inputs: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["save", "new-project"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("\"saved\""), "expected saved status, got: {body}");

    let saved = dir.path().join("projects/new-project.yaml");
    assert!(saved.exists(), "save should write the project file");
}

#[test]
fn save_rejects_when_draft_name_mismatches_cli_arg() {
    let dir = tempdir().unwrap();
    let draft = dir.path().join("drafts/cli-name.yaml");
    fs::create_dir_all(draft.parent().unwrap()).unwrap();
    fs::write(
        &draft,
        r#"schema_version: 1
project:
  name: different-name
  description: Different name inside.
inputs: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["save", "cli-name"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "name mismatch should fail");
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(
        body.contains("\"name_mismatch\""),
        "expected name_mismatch status, got: {body}"
    );
}

#[test]
fn save_rejects_pending_changes_without_yes_or_accept() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();
    fs::create_dir_all(dir.path().join("drafts")).unwrap();

    let project = dir.path().join("projects/existing.yaml");
    fs::write(
        &project,
        r#"schema_version: 1
project:
  name: existing
  description: Original description.
inputs: []
"#,
    )
    .unwrap();
    let draft = dir.path().join("drafts/existing.yaml");
    fs::write(
        &draft,
        r#"schema_version: 1
project:
  name: existing
  description: Updated description.
inputs: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["save", "existing"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "pending changes should fail");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("\"pending_changes\""),
        "expected pending_changes status, got: {combined}"
    );
    assert!(
        combined.contains("project.description"),
        "diff should reference the changed path, got: {combined}"
    );
}

#[test]
fn save_with_yes_accepts_all_changes() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();
    fs::create_dir_all(dir.path().join("drafts")).unwrap();

    fs::write(
        dir.path().join("projects/existing.yaml"),
        r#"schema_version: 1
project:
  name: existing
  description: Original description.
inputs: []
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("drafts/existing.yaml"),
        r#"schema_version: 1
project:
  name: existing
  description: Updated description.
inputs: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["save", "existing", "--yes"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("\"saved\""), "expected saved status, got: {body}");

    let saved = fs::read_to_string(dir.path().join("projects/existing.yaml")).unwrap();
    assert!(
        saved.contains("Updated description"),
        "project file should reflect accepted change, got: {saved}"
    );
}

#[test]
fn schema_flag_prints_json_schema() {
    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .arg("--schema")
        .output()
        .unwrap();
    assert!(output.status.success());
    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("schema output should be valid JSON");
    assert_eq!(parsed["title"], "Mnemesis Project Contract");
}