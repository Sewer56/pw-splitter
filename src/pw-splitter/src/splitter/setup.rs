use crate::error::{PwSplitterError, Result};
use crate::pipewire::{self, AudioSource, PwObject, RecordingDest, SourceConnection};
use crate::splitter::state::{SavedLink, SplitState};
use std::process::Child;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Information needed to set up a split
pub struct SplitConfig {
    pub source: AudioSource,
    pub recording_dest: RecordingDest,
    pub original_connections: Vec<SourceConnection>,
}

/// Result of setting up a split
pub struct SplitResult {
    pub state: SplitState,
    pub loopback_to_recording: Child,
    pub loopback_to_local: Child,
}

/// Set up the audio split
///
/// This creates two loopback streams that both capture from the source:
/// - One sends to the recording destination (OBS) at full volume
/// - One sends to the local speakers with adjustable volume
pub fn setup_split(config: SplitConfig) -> Result<SplitResult> {
    let source_safe_name = config.source.safe_name();
    let split_name = SplitState::generate_unique_name(&format!("{}_Split", source_safe_name));

    // Find the primary output connection (usually a sink)
    let primary_connection = find_primary_output(&config.original_connections)?;

    // Step 1: Spawn loopback to recording destination (full volume)
    // No autoconnect on either side - we'll manually link everything
    let recording_loopback_name = format!("{}_to_Recording", source_safe_name);
    let recording_loopback_desc = format!(
        "{} -> {}",
        config.source.application_name, config.recording_dest.application_name
    );

    let loopback_to_recording =
        pipewire::spawn_loopback_no_target(&recording_loopback_name, &recording_loopback_desc)?;

    // Step 2: Spawn loopback to local/original output (adjustable volume)
    let local_loopback_name = format!("{}_to_Local", source_safe_name);
    let local_loopback_desc = format!("{} -> Local", config.source.application_name);

    let loopback_to_local =
        pipewire::spawn_loopback_no_target(&local_loopback_name, &local_loopback_desc)?;

    // Wait for loopbacks to initialize and create their ports
    thread::sleep(Duration::from_millis(500));

    // Step 3: Disconnect source from all current outputs
    let mut saved_links = Vec::new();
    let objects = pipewire::get_pw_objects()?;

    for conn in &config.original_connections {
        if let Some(links) = disconnect_source_from_target(&config.source, conn, &objects) {
            saved_links.extend(links);
        }
    }

    // Step 4: Connect source to both loopback capture inputs
    connect_source_to_loopback(&config.source, &recording_loopback_name)?;
    connect_source_to_loopback(&config.source, &local_loopback_name)?;

    // Step 5: Connect loopback playback outputs to destinations
    // Recording loopback -> OBS (by port ID to avoid ambiguity)
    pipewire::connect_loopback_to_recording_dest(
        &recording_loopback_name,
        config.recording_dest.node_id,
    )?;

    // Local loopback -> speakers
    connect_loopback_to_sink(&local_loopback_name, &primary_connection.target_node_name)?;

    // Create the state
    let state = SplitState {
        name: split_name,
        source_node_id: config.source.node_id,
        source_node_name: config.source.node_name.clone(),
        source_application_name: config.source.application_name.clone(),
        recording_loopback_name: recording_loopback_name.clone(),
        local_loopback_name: local_loopback_name.clone(),
        recording_dest_node_id: config.recording_dest.node_id,
        recording_dest_media_name: config.recording_dest.media_name.clone(),
        recording_dest_application_name: config.recording_dest.application_name.clone(),
        original_output_node_name: primary_connection.target_node_name.clone(),
        original_links: saved_links,
        loopback_to_recording_pid: loopback_to_recording.id(),
        loopback_to_local_pid: loopback_to_local.id(),
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    // Save state
    state.save()?;

    Ok(SplitResult {
        state,
        loopback_to_recording,
        loopback_to_local,
    })
}

/// Find the primary output connection (prefer Audio/Sink over recording inputs)
fn find_primary_output(connections: &[SourceConnection]) -> Result<&SourceConnection> {
    if connections.is_empty() {
        return Err(PwSplitterError::NoActiveConnection);
    }

    // Try to find an Audio/Sink connection first
    let objects = pipewire::get_pw_objects()?;
    let sinks = pipewire::extract_audio_sinks(&objects);

    for conn in connections {
        if sinks.iter().any(|s| s.node_id == conn.target_node_id) {
            return Ok(conn);
        }
    }

    // Fall back to first connection
    Ok(&connections[0])
}

/// Disconnect source from a target, returning the saved links
fn disconnect_source_from_target(
    source: &AudioSource,
    connection: &SourceConnection,
    objects: &[PwObject],
) -> Option<Vec<SavedLink>> {
    let ports = pipewire::extract_ports(objects);
    let mut saved_links = Vec::new();

    // Get source output ports (FL, FR)
    let source_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == source.node_id
                && p.direction == pipewire::PortDirection::Output
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    // Get target input ports (FL, FR)
    let target_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == connection.target_node_id
                && p.direction == pipewire::PortDirection::Input
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    // Get node names for pw-link
    let source_node_name = pipewire::get_node_name(objects, source.node_id)?;
    let target_node_name = pipewire::get_node_name(objects, connection.target_node_id)?;

    // Disconnect each link
    for src_port in &source_ports {
        for tgt_port in &target_ports {
            if src_port.channel == tgt_port.channel {
                let output_port =
                    pipewire::get_port_link_name(&source_node_name, &src_port.port_name);
                let input_port =
                    pipewire::get_port_link_name(&target_node_name, &tgt_port.port_name);

                if pipewire::destroy_link(&output_port, &input_port).is_ok() {
                    saved_links.push(SavedLink {
                        output_port,
                        input_port,
                    });
                }
            }
        }
    }

    if saved_links.is_empty() {
        None
    } else {
        Some(saved_links)
    }
}

/// Connect source output to a loopback's capture input
fn connect_source_to_loopback(source: &AudioSource, loopback_name: &str) -> Result<()> {
    let objects = pipewire::get_pw_objects()?;
    let ports = pipewire::extract_ports(&objects);

    // Find the loopback capture node (it has "input" in description and has input ports)
    // The capture side of pw-loopback has input ports
    let loopback_node_id = find_loopback_capture_node(&objects, loopback_name);

    let loopback_node_id = loopback_node_id.ok_or_else(|| {
        PwSplitterError::NodeNotFound(format!("loopback capture {}", loopback_name))
    })?;

    // Get source output ports (FL, FR)
    let source_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == source.node_id
                && p.direction == pipewire::PortDirection::Output
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    // Get loopback capture input ports (FL, FR)
    let loopback_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == loopback_node_id
                && p.direction == pipewire::PortDirection::Input
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    if source_ports.is_empty() || loopback_ports.is_empty() {
        return Err(PwSplitterError::LinkCreationFailed(format!(
            "Could not find ports: source={}, loopback={}",
            source_ports.len(),
            loopback_ports.len()
        )));
    }

    let source_node_name = pipewire::get_node_name(&objects, source.node_id)
        .ok_or_else(|| PwSplitterError::NodeNotFound(format!("source node {}", source.node_id)))?;

    let loopback_node_name =
        pipewire::get_node_name(&objects, loopback_node_id).ok_or_else(|| {
            PwSplitterError::NodeNotFound(format!("loopback node {}", loopback_node_id))
        })?;

    // Create links for FL and FR
    for src_port in &source_ports {
        for lb_port in &loopback_ports {
            if src_port.channel == lb_port.channel {
                let output_port =
                    pipewire::get_port_link_name(&source_node_name, &src_port.port_name);
                let input_port =
                    pipewire::get_port_link_name(&loopback_node_name, &lb_port.port_name);
                pipewire::create_link(&output_port, &input_port)?;
            }
        }
    }

    Ok(())
}

/// Connect loopback playback output to a sink
fn connect_loopback_to_sink(loopback_name: &str, sink_name: &str) -> Result<()> {
    let objects = pipewire::get_pw_objects()?;
    let ports = pipewire::extract_ports(&objects);

    // Find the loopback playback node (has output ports)
    let loopback_node_id =
        find_loopback_playback_node(&objects, loopback_name).ok_or_else(|| {
            PwSplitterError::NodeNotFound(format!("loopback playback {}", loopback_name))
        })?;

    let sink_node_id = pipewire::find_node_by_name(&objects, sink_name)
        .ok_or_else(|| PwSplitterError::NodeNotFound(sink_name.to_string()))?;

    // Get loopback playback output ports (FL, FR)
    let loopback_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == loopback_node_id
                && p.direction == pipewire::PortDirection::Output
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    // Get sink input ports (FL, FR)
    let sink_ports: Vec<_> = ports
        .iter()
        .filter(|p| {
            p.node_id == sink_node_id
                && p.direction == pipewire::PortDirection::Input
                && (p.channel == "FL" || p.channel == "FR")
        })
        .collect();

    if loopback_ports.is_empty() || sink_ports.is_empty() {
        return Err(PwSplitterError::LinkCreationFailed(format!(
            "Could not find ports: loopback={}, sink={}",
            loopback_ports.len(),
            sink_ports.len()
        )));
    }

    let loopback_node_name =
        pipewire::get_node_name(&objects, loopback_node_id).ok_or_else(|| {
            PwSplitterError::NodeNotFound(format!("loopback node {}", loopback_node_id))
        })?;

    // Create links for FL and FR
    for lb_port in &loopback_ports {
        for sink_port in &sink_ports {
            if lb_port.channel == sink_port.channel {
                let output_port =
                    pipewire::get_port_link_name(&loopback_node_name, &lb_port.port_name);
                let input_port = pipewire::get_port_link_name(sink_name, &sink_port.port_name);
                pipewire::create_link(&output_port, &input_port)?;
            }
        }
    }

    Ok(())
}

/// Find the capture side of a loopback (the node with input ports)
fn find_loopback_capture_node(objects: &[PwObject], loopback_name: &str) -> Option<u32> {
    let ports = pipewire::extract_ports(objects);

    for obj in objects {
        if let pipewire::PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && let Some(desc) = &props.node_description
            && desc.contains(loopback_name)
            && desc.contains("input")
        {
            return Some(node.id);
        }
    }

    // Fallback: find by checking which node has input ports
    for obj in objects {
        if let pipewire::PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && let Some(name) = &props.node_name
            && name.contains(loopback_name)
        {
            // Check if this node has input ports
            let has_input = ports
                .iter()
                .any(|p| p.node_id == node.id && p.direction == pipewire::PortDirection::Input);
            if has_input {
                return Some(node.id);
            }
        }
    }

    None
}

/// Find the playback side of a loopback (the node with output ports)
fn find_loopback_playback_node(objects: &[PwObject], loopback_name: &str) -> Option<u32> {
    let ports = pipewire::extract_ports(objects);

    for obj in objects {
        if let pipewire::PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && let Some(desc) = &props.node_description
            && desc.contains(loopback_name)
            && desc.contains("output")
        {
            return Some(node.id);
        }
    }

    // Fallback: find by checking which node has output ports
    for obj in objects {
        if let pipewire::PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && let Some(name) = &props.node_name
            && name.contains(loopback_name)
        {
            // Check if this node has output ports
            let has_output = ports
                .iter()
                .any(|p| p.node_id == node.id && p.direction == pipewire::PortDirection::Output);
            if has_output {
                return Some(node.id);
            }
        }
    }

    None
}
