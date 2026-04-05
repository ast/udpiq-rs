use anyhow::Result;
use clap::{Parser, Subcommand};

mod alsa;
mod commands;

#[derive(Parser)]
#[command(about = "Stream raw IQ samples from an ALSA device over UDP")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Capture IQ samples from ALSA and stream over UDP
    Stream(commands::stream::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Stream(args) => commands::stream::run(args),
    }
}
