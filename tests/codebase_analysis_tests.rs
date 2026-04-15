use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

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

        // Ensure success
        let output = cmd.output().expect("Failed to execute command");
        if !output.status.success() {
            eprintln!(
                "Command failed for {}: {}",
                lang,
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Command failed");
        }

        // 1. Verify elements.yaml exists and is not empty
        let elements_path = dir.path().join("elements.yaml");
        assert!(
            elements_path.exists(),
            "elements.yaml should exist for {} codebase",
            lang
        );
        let elements_data =
            fs::read_to_string(&elements_path).expect("Failed to read elements.yaml");
        assert!(
            !elements_data.trim().is_empty(),
            "elements.yaml should not be empty for {} codebase",
            lang
        );

        // 2. Verify connectors.yaml exists and is not empty
        let connectors_path = dir.path().join("connectors.yaml");
        assert!(
            connectors_path.exists(),
            "connectors.yaml should exist for {} codebase",
            lang
        );
        let connectors_data =
            fs::read_to_string(&connectors_path).expect("Failed to read connectors.yaml");
        assert!(
            !connectors_data.trim().is_empty(),
            "connectors.yaml should not be empty for {} codebase",
            lang
        );

        // 3. Verify at least one element has a view
        assert!(
            elements_data.contains("has_view: true"),
            "At least one element should have a view for {} codebase",
            lang
        );

        // Count elements, views, and connectors
        let element_count = elements_data.matches("  name:").count();
        let view_count = elements_data.matches("has_view: true").count();
        let connector_count = connectors_data.matches("  source:").count();

        println!("Success for {} codebase!", lang);
        println!("  - Elements:   {}", element_count);
        println!("  - Views:      {}", view_count);
        println!("  - Connectors: {}", connector_count);
    }
}
