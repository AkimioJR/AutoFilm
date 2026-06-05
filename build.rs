use std::env;
use std::process::Command;

fn main() {
    let git_branch = git_output(&["branch", "--show-current"]).filter(|branch| !branch.is_empty());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=.git/packed-refs");
    if let Some(branch) = git_branch.as_deref() {
        println!("cargo:rerun-if-changed=.git/refs/heads/{branch}");
    }

    set_env("AUTOFILM_BUILD_TIME", &chrono::Utc::now().to_rfc3339());
    set_env(
        "AUTOFILM_BUILD_TARGET",
        env::var("TARGET").as_deref().unwrap_or("unknown"),
    );
    set_env(
        "AUTOFILM_BUILD_PROFILE",
        env::var("PROFILE").as_deref().unwrap_or("unknown"),
    );
    set_env(
        "AUTOFILM_RUSTC_VERSION",
        command_output("rustc", &["--version"])
            .as_deref()
            .unwrap_or("unknown"),
    );
    set_env(
        "AUTOFILM_GIT_COMMIT",
        git_output(&["rev-parse", "HEAD"])
            .as_deref()
            .unwrap_or("unknown"),
    );
    set_env(
        "AUTOFILM_GIT_BRANCH",
        git_branch.as_deref().unwrap_or("detached"),
    );
}

fn set_env(key: &str, value: &str) {
    println!("cargo:rustc-env={key}={value}");
}

fn git_output(args: &[&str]) -> Option<String> {
    command_output("git", args)
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
