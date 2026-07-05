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

#[test]
fn verify_returns_ok_when_all_paths_exist() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();

    let existing = dir.path().join("present.txt");
    fs::write(&existing, "hello").unwrap();

    fs::write(
        dir.path().join("projects/all-good.yaml"),
        format!(
            r#"schema_version: 1
project:
  name: all-good
  description: All paths exist.
inputs:
  - name: src
    description: Source.
    type: directory
    outputs:
      - name: file-output
        type: file
        location: "{}"
    actions: []
"#,
            existing.display()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["verify", "all-good"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "verify should pass when paths exist");
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["result"]["status"], "verified");
    assert_eq!(body["result"]["summary"]["ok"], 1);
    assert_eq!(body["result"]["summary"]["missing"], 0);
    assert_eq!(body["result"]["summary"]["wrong_type"], 0);
}

#[test]
fn verify_reports_missing_paths() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();

    fs::write(
        dir.path().join("projects/has-gap.yaml"),
        r#"schema_version: 1
project:
  name: has-gap
  description: Has a missing output.
inputs:
  - name: src
    description: Source.
    type: directory
    outputs:
      - name: missing
        type: file
        location: /nonexistent/path/that/should/not/exist
    actions: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["verify", "has-gap"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "verify should fail on missing paths");
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["result"]["summary"]["missing"], 1);
    assert_eq!(body["result"]["details"][0]["outcome"]["status"], "missing");
}

#[test]
fn verify_reports_wrong_type() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("projects")).unwrap();

    // Declare a file but the path is a directory.
    let confused = dir.path().join("actually-a-dir");
    fs::create_dir(&confused).unwrap();

    fs::write(
        dir.path().join("projects/wrong-type.yaml"),
        format!(
            r#"schema_version: 1
project:
  name: wrong-type
  description: Output type does not match filesystem.
inputs:
  - name: src
    description: Source.
    type: directory
    outputs:
      - name: declared-file
        type: file
        location: "{}"
    actions: []
"#,
            confused.display()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["verify", "wrong-type"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "verify should fail on wrong type");
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["result"]["summary"]["wrong_type"], 1);
    assert_eq!(
        body["result"]["details"][0]["outcome"]["status"],
        "wrong_type"
    );
    assert_eq!(body["result"]["details"][0]["outcome"]["found"], "directory");
}

#[test]
fn verify_returns_not_found_for_missing_project() {
    let dir = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mnemesis"))
        .args(["verify", "no-such-project"])
        .env("MNEMESIS_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["result"]["status"], "not_found");
    assert_eq!(body["result"]["project"], "no-such-project");
}