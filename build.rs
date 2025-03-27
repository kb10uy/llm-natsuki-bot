use std::process::Command;

use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn main() {
    export_git_commit_hash();
    export_built_at_datetime();
}

fn export_built_at_datetime() {
    let now = OffsetDateTime::now_local().expect("failed to fetch time");
    let formatted = now.format(&Rfc3339).expect("failed to format time");
    println!("cargo:rustc-env=BUILT_AT_DATETIME={formatted}");
}

fn export_git_commit_hash() {
    let git_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("error in fetching commit hash");
    let commit_hash = String::from_utf8(git_output.stdout).expect("invalid commit hash");
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit_hash.trim());
}
