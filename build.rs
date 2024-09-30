use std::{env, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(testnet)");

    // Use the `git` command to get the current commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let git_commit_hash = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash.trim());

    if env::var("TESTNET").is_ok() {
        println!("cargo:rustc-cfg=testnet");
    }
}
