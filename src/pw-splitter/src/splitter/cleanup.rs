use crate::error::Result;
use crate::pipewire;
use crate::splitter::state::SplitState;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Tear down an active split and restore original connections
pub fn teardown_split(state: &SplitState) -> Result<()> {
    // Step 1: Kill loopback processes
    kill_process(state.loopback_to_recording_pid);
    kill_process(state.loopback_to_local_pid);

    // Step 2: Restore original links
    for link in &state.original_links {
        let _ = pipewire::create_link(&link.output_port, &link.input_port);
    }

    // Step 3: Delete state file
    state.delete()?;

    Ok(())
}

/// Stop a split by name
pub fn stop_split(name: &str) -> Result<()> {
    let state = SplitState::load(name)?;
    teardown_split(&state)
}

/// Stop all active splits
pub fn stop_all_splits() -> Result<Vec<String>> {
    let states = SplitState::list_all()?;
    let mut stopped = Vec::new();

    for state in states {
        if teardown_split(&state).is_ok() {
            stopped.push(state.name);
        }
    }

    Ok(stopped)
}

/// Kill a process by PID
fn kill_process(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output();
}

/// Check if loopback processes are still running
pub fn check_loopbacks_running(state: &SplitState) -> (bool, bool) {
    let recording_running = is_process_running(state.loopback_to_recording_pid);
    let local_running = is_process_running(state.loopback_to_local_pid);
    (recording_running, local_running)
}

/// Check if a process is running
fn is_process_running(pid: u32) -> bool {
    // Check /proc/<pid> exists
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}

/// Restart a crashed loopback process for recording
pub fn restart_loopback_to_recording(state: &mut SplitState) -> Result<u32> {
    let loopback_desc = format!(
        "{} -> {}",
        state.source_application_name, state.recording_dest_application_name
    );

    let child = pipewire::spawn_loopback_no_target(&state.recording_loopback_name, &loopback_desc)?;

    let new_pid = child.id();
    state.loopback_to_recording_pid = new_pid;

    // Wait for loopback to create ports
    thread::sleep(Duration::from_millis(300));

    // Reconnect source to loopback capture and loopback playback to destination
    // Note: This is a simplified restart - the source should already be connected
    // if only the loopback crashed
    pipewire::connect_loopback_to_recording_dest(
        &state.recording_loopback_name,
        state.recording_dest_node_id,
    )?;

    state.save()?;

    // Let the child run detached
    std::mem::forget(child);

    Ok(new_pid)
}

/// Restart the local loopback process
pub fn restart_loopback_to_local(state: &mut SplitState) -> Result<u32> {
    let loopback_desc = format!("{} -> Local", state.source_application_name);

    let child = pipewire::spawn_loopback_no_target(&state.local_loopback_name, &loopback_desc)?;

    let new_pid = child.id();
    state.loopback_to_local_pid = new_pid;

    // Wait for loopback to create ports
    thread::sleep(Duration::from_millis(300));

    state.save()?;

    // Let the child run detached
    std::mem::forget(child);

    Ok(new_pid)
}
