use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_tld_init_basic() {
    let dir = tempdir().unwrap();
    let init_dir = ".tld";
    let init_path = dir.path().join(init_dir);

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("init").arg(init_dir).current_dir(dir.path());
    cmd.assert().success();

    assert!(init_path.join("elements.yaml").exists());
    assert!(init_path.join("connectors.yaml").exists());
    assert!(init_path.join(".tld.yaml").exists());

    let config = fs::read_to_string(init_path.join(".tld.yaml")).unwrap();
    assert!(config.contains("project_name:"));
    assert!(config.contains("exclude:"));
    assert!(config.contains("- node_modules"));
}

#[test]
fn test_tld_init_in_git_repo() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Init git repo
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .assert()
        .success();
    
    Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/example/repo.git"])
        .current_dir(repo_path)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("-w").arg(".tld").arg("init").current_dir(repo_path);
    cmd.assert().success();

    let config_path = repo_path.join(".tld").join(".tld.yaml");
    assert!(config_path.exists());

    let config = fs::read_to_string(config_path).unwrap();
    
    // Should have detected repo name (tempdir name)
    let repo_name = repo_path.file_name().unwrap().to_str().unwrap();
    assert!(config.contains(&format!("project_name: {repo_name}")));
    
    // Should have detected origin
    assert!(config.contains("url: https://github.com/example/repo.git"));
    assert!(config.contains("localDir: ."));
}

#[test]
fn test_tld_init_with_custom_dir() {
    let dir = tempdir().unwrap();
    let custom_dir = "my-architecture";

    let mut cmd = Command::cargo_bin("tld").unwrap();
    cmd.arg("init").arg(custom_dir).current_dir(dir.path());
    cmd.assert().success();

    assert!(dir.path().join(custom_dir).join("elements.yaml").exists());
    assert!(dir.path().join(custom_dir).join(".tld.yaml").exists());
}
