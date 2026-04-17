use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_tld_help() {
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "tld manages software architecture diagrams as code",
    ));
}

#[test]
fn test_tld_version_matches_package_version() {
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("--version");
    cmd.assert().success().stdout(predicate::eq(format!(
        "tld {}\n",
        env!("CARGO_PKG_VERSION")
    )));
}

#[test]
fn test_tld_add_help() {
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("add").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Add or update an element"));
}

#[test]
fn test_tld_analyze_help_no_longer_exposes_lsp_flag() {
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("analyze").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--lsp").not());
}

#[test]
fn test_tld_analyze_idempotency_simple() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    // Create a dummy file with no symbols to analyze
    let file_path = dir.path().join("ignored.txt");
    fs::write(&file_path, "this is just text, no symbols here").unwrap();

    // First run
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("analyze").arg(wdir);
    cmd.assert().success();

    let elements_path = dir.path().join("elements.yaml");
    assert!(
        !elements_path.exists(),
        "elements.yaml should not be created if no symbols found"
    );

    // Second run
    let mut cmd2 = Command::cargo_bin("tld").unwrap();
    cmd2.arg("-w").arg(wdir).arg("analyze").arg(wdir);
    cmd2.assert().success();

    assert!(
        !elements_path.exists(),
        "Analyze should still not create file on second run if no symbols found"
    );
}

#[test]
fn test_tld_add_upsert_and_conflict() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    // 1. Valid add
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("add")
        .arg("Service A")
        .arg("--kind")
        .arg("service");
    cmd.assert().success();

    // 2. Valid upsert (adding technology)
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("add")
        .arg("Service A")
        .arg("--technology")
        .arg("rust");
    cmd.assert().success();

    // 3. Conflict (overriding kind)
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("add")
        .arg("Service A")
        .arg("--kind")
        .arg("database");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Conflict: field 'kind'"));

    // 4. Validation (empty name) - positional but let's check if it handles it (clap might catch it if missing, but we added a check)
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("add").arg("  ");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Element name cannot be empty"));
}

#[test]
fn test_tld_connect_checks_and_conflicts() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    // Setup elements
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("add").arg("ServiceA");
    cmd.assert().success();
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("add").arg("ServiceB");
    cmd.assert().success();

    // 1. Valid connect
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("connect")
        .arg("servicea")
        .arg("serviceb")
        .arg("--relationship")
        .arg("calls");
    cmd.assert().success();

    // 2. Existence check + Suggestion
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("connect")
        .arg("servicec")
        .arg("serviceb");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Element 'servicec' not found"));

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("connect")
        .arg("serivce-a")
        .arg("serviceb"); // typo
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean 'servicea'?"));

    // 3. Duplicate/matching upsert
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("connect")
        .arg("servicea")
        .arg("serviceb")
        .arg("--relationship")
        .arg("calls");
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("already exists with identical"));

    // 4. Property conflict
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("connect")
        .arg("servicea")
        .arg("serviceb")
        .arg("--relationship")
        .arg("uses");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Conflict: connector"));
}

#[test]
fn test_tld_update_listing_and_help() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    // Setup element
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("add")
        .arg("ServiceA")
        .arg("--technology")
        .arg("rust");
    cmd.assert().success();

    // 1. List fields for element
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w")
        .arg(wdir)
        .arg("update")
        .arg("element")
        .arg("servicea");
    cmd.assert()
        .success()
        .stderr(predicate::str::contains(
            "Available fields for element 'servicea'",
        ))
        .stdout(predicate::str::contains("technology"))
        .stdout(predicate::str::contains("rust"));

    // 2. Help for element
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("update").arg("element").arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "Valid fields: name, kind, technology",
    ));
}

#[test]
fn test_tld_analyze_empty() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("analyze").arg(wdir);
    cmd.assert().success().stderr(predicate::str::contains(
        "No symbols or architectural elements were found",
    ));
}

#[test]
fn test_tld_analyze_workspace_root_uses_configured_repositories() {
    let dir = tempdir().unwrap();
    let wdir = dir.path();

    fs::create_dir_all(wdir.join("repo-a")).unwrap();
    fs::create_dir_all(wdir.join("repo-b/internal")).unwrap();
    fs::create_dir_all(wdir.join("scratch")).unwrap();

    fs::write(
        wdir.join(".tld.yaml"),
        r#"project_name: Multi Repo
exclude: []
repositories:
  repo-a:
    localDir: repo-a
  repo-b:
    localDir: repo-b
    exclude:
      - internal/
"#,
    )
    .unwrap();

    write_typescript_fixture(
        &wdir.join("repo-a/order.ts"),
        "function placeOrder() { return charge(); }\nfunction charge() { return 1; }\n",
    );
    write_typescript_fixture(
        &wdir.join("repo-b/inventory.ts"),
        "function reserveStock() { return currentStock(); }\nfunction currentStock() { return 2; }\n",
    );
    write_typescript_fixture(
        &wdir.join("repo-b/internal/ignored.ts"),
        "function internalOnly() { return 3; }\n",
    );
    write_typescript_fixture(
        &wdir.join("scratch/debug.ts"),
        "function debugOnly() { return 4; }\n",
    );

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(wdir).arg("analyze").arg(wdir);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("repositories=2"));

    let elements = fs::read_to_string(wdir.join("elements.yaml")).unwrap();
    assert!(elements.contains("placeOrder"));
    assert!(elements.contains("reserveStock"));
    assert!(!elements.contains("internalOnly"));
    assert!(!elements.contains("debugOnly"));
}

fn write_typescript_fixture(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, source).unwrap();
}
