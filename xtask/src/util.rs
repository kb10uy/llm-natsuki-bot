use std::{
    env::var,
    path::PathBuf,
    process::{Command, exit},
};

use anyhow::Result;

pub fn run_command_in_repository(command: &str, args: &[&str]) -> Result<()> {
    let mut process = Command::new(command)
        .args(args)
        .current_dir(repository_dir()?)
        .spawn()?;

    let exit_status = process.wait()?;
    if !exit_status.success() {
        exit(exit_status.code().unwrap_or(1));
    }
    Ok(())
}

pub fn get_command_output_in_repository(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(repository_dir()?)
        .output()?;

    Ok(String::from_utf8(output.stdout)?)
}

pub fn repository_dir() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    Ok(manifest_dir.parent().expect("should have path").to_owned())
}
