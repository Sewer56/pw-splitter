mod error;
mod pipewire;
mod splitter;
mod tui;

use pico_args::Arguments;
use splitter::SplitState;

fn main() {
    let mut args = Arguments::from_env();

    let version = args.contains(["-V", "--version"]);

    if version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    let subcommand: Option<String> = args.subcommand().ok().flatten();

    let result = match subcommand.as_ref().map(|s| s.as_str()) {
        Some("list") => list_splits(),
        Some("stop") => {
            let name: String = args.free_from_str().unwrap_or_else(|_| {
                eprintln!("Error: missing split name for 'stop' command");
                std::process::exit(1);
            });
            stop_split(&name)
        }
        Some("stop-all") => stop_all_splits(),
        None | Some(_) => run_tui(),
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
