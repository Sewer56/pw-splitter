use crate::error::{PwSplitterError, Result};
use crate::pipewire::parser;
use crate::pipewire::types::*;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Run pw-dump and return parsed objects
pub fn get_pw_objects() -> Result<Vec<PwObject>> {
    let output = Command::new("pw-dump")
        .output()
        .map_err(|e| PwSplitterError::CommandFailed(format!("pw-dump: {}", e)))?;

    if !output.status.success() {
        return Err(PwSplitterError::CommandFailed(format!(
            "pw-dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parser::parse_pw_dump(&json_str)
}

/// Spawn a pw-loopback process with no auto-connect on either side
/// This allows us to manually wire both capture and playback
pub fn spawn_loopback_no_target(loopback_name: &str, loopback_desc: &str) -> Result<Child> {
    // No autoconnect on capture side - we'll manually link from the source
    let capture_props = format!(
        "node.name={} node.description=\"{} input\" node.autoconnect=false",
        loopback_name, loopback_desc
    );

    // No autoconnect on playback side - we'll manually link to the destination
    let playback_props = format!(
        "node.name={} node.description=\"{} output\" node.autoconnect=false",
        loopback_name, loopback_desc
    );

    Command::new("pw-loopback")
        .args([
            &format!("--capture-props={}", capture_props),
            &format!("--playback-props={}", playback_props),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| PwSplitterError::LoopbackSpawnFailed(e.to_string()))
}

/// Connect a loopback's output ports to a Stream/Input/Audio node's input ports
pub fn connect_loopback_to_recording_dest(
    loopback_playback_name: &str,
    dest_node_id: u32,
) -> Result<()> {
    // Wait for loopback to create its ports
    thread::sleep(Duration::from_millis(300));

    let objects = get_pw_objects()?;
    let ports = parser::extract_ports(&objects);

    // Find the loopback's playback output ports by looking for the node with our name
    // The playback side of pw-loopback creates output ports
    let loopback_node_id = parser::find_node_by_name(&objects, loopback_playback_name);

    let loopback_ports: Vec<_> = if let Some(node_id) = loopback_node_id {
        ports
            .iter()
            .filter(|p| {
                p.node_id == node_id
                    && p.direction == crate::pipewire::types::PortDirection::Output
                    && (p.channel == "FL" || p.channel == "FR")
            })
            .collect()
    } else {
        // Fallback: search by port name pattern
        Vec::new()
    };

    // Find the recording destination's input ports by node_id
    // This is critical because multiple nodes can have the same node.name (e.g., "OBS")
    let dest_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == dest_node_id
                && p.direction == crate::pipewire::types::PortDirection::Input
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    if loopback_ports.is_empty() || dest_ports.is_empty() {
        return Err(PwSplitterError::LinkCreationFailed(format!(
            "Could not find ports for loopback ({} ports) or destination node {} ({} ports)",
            loopback_ports.len(),
            dest_node_id,
            dest_ports.len()
        )));
    }

    // Create links for FL and FR using PORT IDs to avoid ambiguity
    // Multiple OBS nodes have the same node.name="OBS", so "OBS:input_FL" is ambiguous
    // Using port IDs directly ensures we connect to the correct node
    for lb_port in &loopback_ports {
        for dest_port in &dest_ports {
            if lb_port.channel == dest_port.channel {
                // Use port IDs for destination to avoid ambiguity with duplicate node names
                let output_port = get_port_link_name(loopback_playback_name, &lb_port.port_name);
                create_link_by_id(&output_port, dest_port.port_id)?;
            }
        }
    }

    Ok(())
}

/// Create a link using port ID for the input (avoids ambiguity with duplicate node names)
pub fn create_link_by_id(output_port: &str, input_port_id: u32) -> Result<()> {
    let output = Command::new("pw-link")
        .args([output_port, &input_port_id.to_string()])
        .output()
        .map_err(|e| PwSplitterError::LinkCreationFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ignore "File exists" error (link already exists)
        if !stderr.contains("File exists") {
            return Err(PwSplitterError::LinkCreationFailed(format!(
                "Failed to link {} -> {}: {}",
                output_port, input_port_id, stderr
            )));
        }
    }

    Ok(())
}

/// Create a link between two ports using pw-link
pub fn create_link(output_port: &str, input_port: &str) -> Result<()> {
    let output = Command::new("pw-link")
        .args([output_port, input_port])
        .output()
        .map_err(|e| PwSplitterError::LinkCreationFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ignore "File exists" error (link already exists)
        if !stderr.contains("File exists") {
            return Err(PwSplitterError::LinkCreationFailed(stderr.to_string()));
        }
    }

    Ok(())
}

/// Destroy a link between two ports using pw-link -d
pub fn destroy_link(output_port: &str, input_port: &str) -> Result<()> {
    let output = Command::new("pw-link")
        .args(["-d", output_port, input_port])
        .output()
        .map_err(|e| PwSplitterError::LinkDestroyFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ignore errors about non-existent links
        if !stderr.contains("No such file") && !stderr.is_empty() {
            return Err(PwSplitterError::LinkDestroyFailed(stderr.to_string()));
        }
    }

    Ok(())
}

/// Get port name in pw-link format: "node_name:port_name"
pub fn get_port_link_name(node_name: &str, port_name: &str) -> String {
    format!("{}:{}", node_name, port_name)
}
