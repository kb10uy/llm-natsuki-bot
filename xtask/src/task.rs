use anyhow::Result;

use crate::util::{get_command_output_in_repository, run_command_in_repository};

pub fn build_image() -> Result<()> {
    let commit_hash = get_command_output_in_repository("git", &["rev-parse", "HEAD"])?;
    let commit_hash_arg = format!("GIT_COMMIT_HASH={}", commit_hash.trim());
    run_command_in_repository("sudo", &["docker", "compose", "build", "--build-arg", &commit_hash_arg])?;
    Ok(())
}

pub fn up() -> Result<()> {
    build_config()?;
    run_command_in_repository("sudo", &["docker", "compose", "up", "-d"])?;
    Ok(())
}

pub fn down() -> Result<()> {
    run_command_in_repository("sudo", &["docker", "compose", "down"])?;
    Ok(())
}

pub fn restart() -> Result<()> {
    build_config()?;
    run_command_in_repository("sudo", &["docker", "compose", "restart"])?;
    Ok(())
}

pub fn development() -> Result<()> {
    build_config()?;
    run_command_in_repository(
        "cargo",
        &[
            "run",
            "--bin",
            "lnb-server",
            "--",
            "-c",
            "data/config.generated.json",
            "-r",
            "data/rate-limits.generated.json",
        ],
    )?;
    Ok(())
}

pub fn development_api() -> Result<()> {
    build_config()?;
    run_command_in_repository(
        "cargo",
        &[
            "run",
            "--bin",
            "lnb-admin-api",
            "--",
            "-c",
            "data/config.generated.json",
        ],
    )?;
    Ok(())
}

pub fn build_config() -> Result<()> {
    run_command_in_repository("jsonnet", &["config.jsonnet", "-o", "data/config.generated.json"])?;
    run_command_in_repository(
        "jsonnet",
        &["rate-limits.jsonnet", "-o", "data/rate-limits.generated.json"],
    )?;
    Ok(())
}
