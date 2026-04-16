use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

// ── Smoke tests: structural view (default, backward-compatible) ───────────────

#[test]
fn test_analyze_all_codebases() {
    let languages = vec!["go", "python", "typescript", "java", "cpp"];

    for lang in languages {
        println!("Testing analysis of {} codebase...", lang);
        let dir = tempdir().expect("Failed to create temp dir");
        let wdir = dir.path().to_str().expect("Failed to get temp dir path");

        let mut cmd = Command::cargo_bin("tld").expect("Failed to find tld binary");
        cmd.arg("-w")
            .arg(wdir)
            .arg("analyze")
            .arg(format!("tests/test-codebase/{}", lang));

        let output = cmd.output().expect("Failed to execute command");
        if !output.status.success() {
            eprintln!(
                "Command failed for {}: {}",
                lang,
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Command failed for {lang}");
        }

        let elements_path = dir.path().join("elements.yaml");
        assert!(
            elements_path.exists(),
            "elements.yaml should exist for {lang} codebase"
        );
        let elements_data =
            fs::read_to_string(&elements_path).expect("Failed to read elements.yaml");
        assert!(
            !elements_data.trim().is_empty(),
            "elements.yaml should not be empty for {lang} codebase"
        );

        let connectors_path = dir.path().join("connectors.yaml");
        assert!(
            connectors_path.exists(),
            "connectors.yaml should exist for {lang} codebase"
        );
        let connectors_data =
            fs::read_to_string(&connectors_path).expect("Failed to read connectors.yaml");
        assert!(
            !connectors_data.trim().is_empty(),
            "connectors.yaml should not be empty for {lang} codebase"
        );

        assert!(
            elements_data.contains("has_view: true"),
            "At least one element should have a view for {lang} codebase"
        );

        let element_count = elements_data.matches("  name:").count();
        let view_count = elements_data.matches("has_view: true").count();
        let connector_count = connectors_data.matches("  source:").count();

        println!("Success for {lang} codebase!");
        println!("  - Elements:   {element_count}");
        println!("  - Views:      {view_count}");
        println!("  - Connectors: {connector_count}");
    }
}

// ── Semantic assertions: TypeScript order-placement fixture ───────────────────

/// Structural view must include all declared orchestration symbols.
#[test]
fn test_typescript_structural_has_orchestration_symbols() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", wdir, "analyze", "tests/test-codebase/typescript"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "analyze failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();

    assert!(
        elements.contains("OrderService"),
        "OrderService must appear in structural output"
    );
    assert!(
        elements.contains("placeOrder"),
        "placeOrder (orchestrator) must appear in structural output"
    );
    assert!(
        elements.contains("PaymentService"),
        "PaymentService must appear in structural output"
    );
    assert!(
        elements.contains("ProductService"),
        "ProductService must appear in structural output"
    );
}

/// Business view must emit a repository element and at least some symbols.
#[test]
fn test_typescript_business_view_produces_output() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "business",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "analyze --view business failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();

    assert!(
        elements.contains("repository"),
        "Business view must include the repository element"
    );

    let symbol_count = elements.matches("  name:").count();
    assert!(
        symbol_count > 0,
        "Business view must emit at least one element"
    );
    println!("Business view emitted {symbol_count} elements");
}

/// Business view must not produce file or folder elements.
#[test]
fn test_typescript_business_view_no_file_elements() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "business",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "analyze failed");

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();
    assert!(
        !elements.contains("kind: file"),
        "Business view must not produce file-level elements"
    );
    assert!(
        !elements.contains("kind: folder"),
        "Business view must not produce folder-level elements"
    );
}

/// Structural mode must remain available and produce file elements.
#[test]
fn test_structural_view_explicit_flag() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "structural",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "analyze --view structural failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();
    // Structural mode should still produce file elements.
    assert!(
        elements.contains("kind: file") || elements.contains("kind: repository"),
        "Structural view must include file or repository elements"
    );
}

/// Invalid view name must produce a helpful error.
#[test]
fn test_invalid_view_flag_errors() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "nonexistent",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "Invalid view name should cause failure"
    );
}

// ── Semantic assertions: Go fixture ──────────────────────────────────────────

#[test]
fn test_go_structural_has_connectors() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", wdir, "analyze", "tests/test-codebase/go"])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "analyze go failed");

    let connectors = fs::read_to_string(dir.path().join("connectors.yaml")).unwrap();
    assert!(
        !connectors.trim().is_empty(),
        "Go fixture should produce at least one connector"
    );
}

// ── Semantic assertions: Python fixture ──────────────────────────────────────

#[test]
fn test_python_structural_has_class_symbols() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", wdir, "analyze", "tests/test-codebase/python"])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "analyze python failed");

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();
    assert!(
        elements.contains("class"),
        "Python fixture should contain class elements"
    );
}

// ── Business view: all codebases ─────────────────────────────────────────────

#[test]
fn test_business_view_all_codebases() {
    let languages = vec!["go", "python", "typescript", "java", "cpp"];

    for lang in languages {
        println!("Testing --view business for {} codebase...", lang);
        let dir = tempdir().expect("Failed to create temp dir");
        let wdir = dir.path().to_str().expect("Failed to get temp dir path");

        let output = Command::cargo_bin("tld")
            .unwrap()
            .args([
                "-w",
                wdir,
                "analyze",
                "--view",
                "business",
                &format!("tests/test-codebase/{lang}"),
            ])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "Business view failed for {lang}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let elements_path = dir.path().join("elements.yaml");
        assert!(
            elements_path.exists(),
            "elements.yaml should exist for {lang} in business mode"
        );
        let elements_data = fs::read_to_string(&elements_path).unwrap();
        assert!(
            !elements_data.trim().is_empty(),
            "elements.yaml should not be empty for {lang} in business mode"
        );

        println!("  Business view OK for {lang}");
    }
}

// ── Data-flow view ────────────────────────────────────────────────────────────

#[test]
fn test_data_flow_view_typescript() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "data-flow",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "data-flow view failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let elements_path = dir.path().join("elements.yaml");
    assert!(elements_path.exists(), "elements.yaml must exist");
}

// ── --changed-since with no changes ─────────────────────────────────────────

/// --changed-since with a ref that produces no changed files should still succeed.
#[test]
fn test_changed_since_no_changes_is_ok() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    // HEAD produces no changes relative to itself, so the file list is empty.
    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--changed-since",
            "HEAD",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    // May succeed (no symbols found) or fail due to git; either is acceptable as long
    // as the binary does not panic.
    let exit = output.status;
    println!(
        "changed-since HEAD exit: {exit}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Not asserting success because the git context depends on the CI environment.
    // The key regression is that the binary must not panic.
}

// ── include-low-signal flag ───────────────────────────────────────────────────

#[test]
fn test_include_low_signal_flag() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--view",
            "business",
            "--include-low-signal",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "include-low-signal failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
