mod error;
mod pipewire;
mod splitter;
mod tui;

use clap::{Parser, Subcommand};
use splitter::SplitState;

#[derive(Parser)]
#[command(name = "pw-splitter")]
#[command(about = "PipeWire audio routing TUI for splitting audio streams")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all active splits
    List,
    /// Stop a specific split by name
    Stop {
        /// Name of the split to stop
        name: String,
    },
    /// Stop all active splits
    StopAll,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::List) => list_splits(),
        Some(Commands::Stop { name }) => stop_split(&name),
        Some(Commands::StopAll) => stop_all_splits(),
        None => run_tui(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_tui() -> error::Result<()> {
    tui::run()
}

fn list_splits() -> error::Result<()> {
    let splits = SplitState::list_all()?;

    if splits.is_empty() {
        println!("No active splits.");
        return Ok(());
    }

    println!("Active splits:");
    println!("{:-<60}", "");

    for split in splits {
        let (recording_running, local_running) = splitter::check_loopbacks_running(&split);

        println!("Name: {}", split.name);
        println!("  Source: {}", split.source_application_name);
        println!(
            "  Recording to: {} [{}]",
            split.recording_dest_application_name, split.recording_dest_media_name
        );
        println!("  Local output: {}", split.original_output_node_name);
        println!(
            "  Loopbacks: recording={}, local={}",
            if recording_running {
                "running"
            } else {
                "stopped"
            },
            if local_running { "running" } else { "stopped" }
        );
        println!("{:-<60}", "");
    }

    Ok(())
}

fn stop_split(name: &str) -> error::Result<()> {
    println!("Stopping split: {}", name);
    splitter::stop_split(name)?;
    println!("Split stopped successfully.");
    Ok(())
}

fn stop_all_splits() -> error::Result<()> {
    let stopped = splitter::stop_all_splits()?;

    if stopped.is_empty() {
        println!("No active splits to stop.");
    } else {
        println!("Stopped {} split(s):", stopped.len());
        for name in stopped {
            println!("  - {}", name);
        }
    }

    Ok(())
}
