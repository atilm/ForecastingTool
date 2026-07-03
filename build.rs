use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();

    let rerun_trigger = rerun_trigger_path();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    fs::write(&rerun_trigger, timestamp).unwrap();

    println!("cargo:rerun-if-changed={}", rerun_trigger.display());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}

fn rerun_trigger_path() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("git-hash-rerun-trigger")
}
