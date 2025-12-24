mod error;
mod pipewire;
mod splitter;
mod tui;

use argh::FromArgs;
use splitter::SplitState;

/// PipeWire audio routing TUI for splitting audio streams
#[derive(FromArgs)]
struct Cli {
    /// print version information
    #[argh(switch, short = 'V')]
    version: bool,

    #[argh(subcommand)]
    command: Option<Commands>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Commands {
    List(ListCmd),
    Stop(StopCmd),
    StopAll(StopAllCmd),
}

/// List all active splits
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
struct ListCmd {}

/// Stop a specific split by name
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "stop")]
struct StopCmd {
    /// name of the split to stop
    #[argh(positional)]
    name: String,
}

/// Stop all active splits
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "stop-all")]
struct StopAllCmd {}

fn main() {
    let cli: Cli = argh::from_env();

    if cli.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    let result = match cli.command {
        Some(Commands::List(_)) => list_splits(),
        Some(Commands::Stop(cmd)) => stop_split(&cmd.name),
        Some(Commands::StopAll(_)) => stop_all_splits(),
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
