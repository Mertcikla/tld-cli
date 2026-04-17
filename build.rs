use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TLD_BUILD_VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    if let Some(counter_path) = local_counter_path() {
        println!("cargo:rerun-if-changed={}", counter_path.display());
    }

    let version = env::var("TLD_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(resolve_git_version);

    println!("cargo:rustc-env=TLD_BUILD_VERSION={version}");
}

fn resolve_git_version() -> String {
    let output = Command::new("git")
        .args(["describe", "--tags", "--dirty", "--always"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            local_build_version(&value)
        }
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn local_build_version(raw_version: &str) -> String {
    let normalized = normalize_version(raw_version);
    if is_exact_tag_build(&normalized) {
        normalized
    } else {
        let base = normalized.trim_end_matches("-dirty");
        format!("{base}-dev.{}", next_local_build_number())
    }
}

fn normalize_version(value: &str) -> String {
    value.strip_prefix('v').unwrap_or(value).to_string()
}

fn is_exact_tag_build(version: &str) -> bool {
    !version.contains("-g") && !version.ends_with("-dirty")
}

fn next_local_build_number() -> u64 {
    let Some(path) = local_counter_path() else {
        return 1;
    };

    let current = fs::read_to_string(&path)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{next}\n"));

    next
}

fn local_counter_path() -> Option<PathBuf> {
    let out_dir = env::var_os("OUT_DIR")?;
    let target_dir = Path::new(&out_dir).ancestors().nth(4)?;
    Some(target_dir.join(".tld-local-build"))
}
