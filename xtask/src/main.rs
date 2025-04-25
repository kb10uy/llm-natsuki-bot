mod task;
mod util;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
struct Arguments {
    #[clap(subcommand)]
    task: ArgumentsTask,
}

#[derive(Debug, Parser)]
enum ArgumentsTask {
    BuildImage,
    Up,
    Down,
    Restart,

    #[clap(aliases = &["dev"])]
    Development,

    #[clap(aliases = &["dev-api"])]
    DevelopmentApi,
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.task {
        ArgumentsTask::BuildImage => task::build_image()?,
        ArgumentsTask::Up => task::up()?,
        ArgumentsTask::Down => task::down()?,
        ArgumentsTask::Restart => task::restart()?,
        ArgumentsTask::Development => task::development()?,
        ArgumentsTask::DevelopmentApi => task::development_api()?,
    }
    Ok(())
}
