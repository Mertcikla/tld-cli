#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn tld_cmd(workdir: &Path, config_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("tld").expect("binary should build");
    cmd.current_dir(workdir).env("TLD_CONFIG_DIR", config_dir);
    cmd
}

#[test]
fn test_examples_md_flow_stays_executable() {
    let dir = tempdir().expect("tempdir");
    let cfg = tempdir().expect("config tempdir");
    let workspace = dir.path();
    let workspace_dir = workspace.join(".tld");

    tld_cmd(workspace, cfg.path())
        .arg("init")
        .assert()
        .success();

    tld_cmd(workspace, cfg.path())
        .args([
            "add",
            "Backend",
            "--kind",
            "service",
            "--tag",
            "layer:domain",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args([
            "add",
            "Api",
            "--parent",
            "backend",
            "--technology",
            "Go",
            "--tag",
            "protocol:rest",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args([
            "add",
            "Database",
            "--parent",
            "backend",
            "--technology",
            "PostgreSQL",
            "--tag",
            "role:database",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args([
            "connect",
            "api",
            "database",
            "--view",
            "backend",
            "--label",
            "reads-writes",
            "--relationship",
            "uses",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args(["update", "element", "api", "description", "Public_HTTP_API"])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args([
            "update",
            "connector",
            "backend:api:database:reads-writes",
            "direction",
            "both",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .arg("views")
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args(["validate", "--skip-symbols"])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .arg("check")
        .assert()
        .success();

    let connectors = fs::read_to_string(workspace_dir.join("connectors.yaml")).expect("connectors");
    assert!(connectors.contains("direction: both"));

    tld_cmd(workspace, cfg.path())
        .args([
            "remove",
            "connector",
            "--view",
            "backend",
            "--from",
            "api",
            "--to",
            "database",
        ])
        .assert()
        .success();
    tld_cmd(workspace, cfg.path())
        .args(["remove", "element", "database"])
        .assert()
        .success();

    let elements = fs::read_to_string(workspace_dir.join("elements.yaml")).expect("elements");
    assert!(!elements.contains("database:\n  name: Database"));
}

#[test]
fn test_cli_guide_matches_current_cli_surface() {
    let guide = fs::read_to_string("docs/CLI_GUIDE.md").expect("read guide");
    let readme = fs::read_to_string("README.md").expect("read readme");

    assert!(guide.contains("tld add <name> [flags]"));
    assert!(guide.contains("tld connect <source-ref> <target-ref> [flags]"));
    assert!(guide.contains("tld plan [--recreate-ids] [-v|--verbose] [-o|--output <file>]"));
    assert!(guide.contains("--ref <slug>"));
    assert!(!guide.contains("tld connect --from"));
    assert!(!guide.contains("--position-x"));
    assert!(!guide.contains("--position-y"));
    assert!(!guide.contains("--changed-since"));
    assert!(!guide.contains("--deep"));
    assert!(readme.contains("--ref web-api"));
}

#[test]
fn test_server_backed_guide_commands_parse_with_current_flags() {
    let dir = tempdir().expect("tempdir");
    let cfg = tempdir().expect("config tempdir");
    let workspace = dir.path();

    tld_cmd(workspace, cfg.path())
        .arg("init")
        .assert()
        .success();

    tld_cmd(workspace, cfg.path())
        .args(["plan", "--verbose", "--output", "plan-report.txt"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No API key found")
                .or(predicate::str::contains("No server URL configured")),
        );

    tld_cmd(workspace, cfg.path())
        .args(["apply", "--force"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No API key found")
                .or(predicate::str::contains("No server URL configured")),
        );

    tld_cmd(workspace, cfg.path())
        .args(["pull", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Org ID required in .tld.yaml"));

    tld_cmd(workspace, cfg.path())
        .args(["status", "--long"])
        .assert()
        .success();
}
