use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
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
fn test_tld_add_help() {
    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("add").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Add or update an element"));
}

#[test]
fn test_tld_analyze_idempotency_simple() {
    let dir = tempdir().unwrap();
    let wdir = dir.path().to_str().unwrap();

    // Create a dummy file to analyze
    let file_path = dir.path().join("main.rs");
    fs::write(&file_path, "fn main() { println!(\"hello\"); }").unwrap();

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
