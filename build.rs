use std::env;

use vergen_gix::{Build, Cargo, Emitter, Gix, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = Build::all_build();
    let cargo = Cargo::all_cargo();
    let git = Gix::all_git();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    // cargo 在构建时向 build.rs 注入 PROFILE 环境变量（如 "debug" / "release"）
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=VERGEN_CARGO_PROFILE={profile}");

    Ok(())
}
