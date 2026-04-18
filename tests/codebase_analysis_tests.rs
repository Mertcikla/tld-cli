#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_analyze_all_codebases() {
    let languages = ["go", "python", "typescript", "java-project", "cpp"];

    for lang in languages {
        let dir = tempdir().expect("Failed to create temp dir");
        let wdir = dir.path().to_str().expect("Failed to get temp dir path");

        let output = Command::cargo_bin("tld")
            .unwrap()
            .args([
                "-w",
                wdir,
                "analyze",
                &format!("tests/test-codebase/{lang}"),
            ])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "analyze failed for {lang}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let elements = fs::read_to_string(dir.path().join("elements.yaml"))
            .expect("Failed to read generated elements.yaml");
        let connectors = fs::read_to_string(dir.path().join("connectors.yaml"))
            .expect("Failed to read generated connectors.yaml");

        assert!(
            !elements.trim().is_empty(),
            "elements.yaml should not be empty for {lang}"
        );
        assert!(
            !connectors.trim().is_empty(),
            "connectors.yaml should not be empty for {lang}"
        );
        assert!(
            elements.contains("has_view: true"),
            "at least one element should own a view for {lang}"
        );
    }
}

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
    assert!(elements.contains("OrderService"));
    assert!(elements.contains("placeOrder"));
    assert!(elements.contains("PaymentService"));
    assert!(elements.contains("ProductService"));
}

#[test]
fn test_legacy_view_flag_is_rejected() {
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

    assert!(!output.status.success(), "legacy --view should fail");
}

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
        elements.contains("symbol_kind: class") || elements.contains("kind: class"),
        "Python fixture should preserve class declaration kind"
    );
}

#[test]
fn test_dry_run_does_not_write_workspace_files() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--dry-run",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !dir.path().join("elements.yaml").exists(),
        "dry-run must not write elements.yaml"
    );
    assert!(
        !dir.path().join("connectors.yaml").exists(),
        "dry-run must not write connectors.yaml"
    );
}

#[test]
fn test_noise_threshold_reduces_output() {
    let low_dir = tempdir().expect("Failed to create temp dir");
    let low_wdir = low_dir.path().to_str().unwrap();

    let low_output = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", low_wdir, "analyze", "tests/test-codebase/typescript"])
        .output()
        .expect("Failed to execute low-threshold run");
    assert!(
        low_output.status.success(),
        "default analyze failed: {}",
        String::from_utf8_lossy(&low_output.stderr)
    );
    let low_elements = fs::read_to_string(low_dir.path().join("elements.yaml")).unwrap();
    let low_count = low_elements.matches("  name:").count();

    let high_dir = tempdir().expect("Failed to create temp dir");
    let high_wdir = high_dir.path().to_str().unwrap();

    let high_output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            high_wdir,
            "analyze",
            "--noise-threshold",
            "2",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute high-threshold run");
    assert!(
        high_output.status.success(),
        "high-threshold run failed: {}",
        String::from_utf8_lossy(&high_output.stderr)
    );
    let high_elements = fs::read_to_string(high_dir.path().join("elements.yaml")).unwrap();
    let high_count = high_elements.matches("  name:").count();

    assert!(
        high_count <= low_count,
        "higher noise threshold should not increase element count ({high_count} > {low_count})"
    );
}

#[test]
fn test_verbose_flag_prints_salience_scores() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-v",
            "-w",
            wdir,
            "analyze",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "analyze -v failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Salience scores"),
        "verbose analyze should print salience score list"
    );
}

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
            "--noise-threshold",
            "2",
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

#[test]
fn test_max_elements_caps_structural_output() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let output = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "--max-elements",
            "12",
            "tests/test-codebase/typescript",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "max-elements run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();
    let count = elements.matches("  name:").count();
    assert!(
        count <= 12,
        "auto-collapse should respect the requested element budget ({count} > 12)"
    );
}

fn run_analyze_then_validate(codebase_path: &str) {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap().to_string();

    let analyze_out = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", &wdir, "analyze", codebase_path])
        .output()
        .expect("Failed to run analyze");

    assert!(
        analyze_out.status.success(),
        "tld analyze failed for {codebase_path}: {}",
        String::from_utf8_lossy(&analyze_out.stderr)
    );

    let validate_out = Command::cargo_bin("tld")
        .unwrap()
        .args(["-w", &wdir, "validate", "--skip-symbols"])
        .output()
        .expect("Failed to run validate");

    let stdout = String::from_utf8_lossy(&validate_out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&validate_out.stderr).to_string();

    assert!(
        validate_out.status.success(),
        "tld validate reported errors after tld analyze on {codebase_path}.\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn test_analyze_produces_valid_workspace_for_all_fixtures() {
    let fixtures = ["go", "python", "typescript", "java-project", "cpp"];
    for fixture in fixtures {
        run_analyze_then_validate(&format!("tests/test-codebase/{fixture}"));
    }
}

#[test]
fn test_analyze_deduplicates_symbol_names_across_files() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wdir = dir.path().to_str().unwrap();

    let out = Command::cargo_bin("tld")
        .unwrap()
        .args([
            "-w",
            wdir,
            "analyze",
            "tests/test-codebase/python-name-collision",
        ])
        .output()
        .expect("Failed to run analyze");

    assert!(
        out.status.success(),
        "analyze failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let elements = fs::read_to_string(dir.path().join("elements.yaml")).unwrap();
    let names: Vec<&str> = elements
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("name: ").map(|v| v.trim_matches('"'))
        })
        .collect();

    let mut seen = std::collections::HashSet::new();
    for name in &names {
        assert!(
            seen.insert(*name),
            "duplicate element name \"{name}\" found in generated elements.yaml"
        );
    }
}

#[test]
fn test_analyze_no_dangling_connector_refs_for_symbol_less_files() {
    run_analyze_then_validate("tests/test-codebase/python-name-collision");
}

#[test]
fn test_analyze_name_collision_fixture_passes_validate() {
    run_analyze_then_validate("tests/test-codebase/python-name-collision");
}
