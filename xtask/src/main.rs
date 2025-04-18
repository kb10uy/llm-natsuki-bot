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
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.task {
        ArgumentsTask::BuildImage => {}
    }
    Ok(())
}
