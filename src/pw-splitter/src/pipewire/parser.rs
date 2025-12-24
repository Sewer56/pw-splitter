use crate::error::{PwSplitterError, Result};
use crate::pipewire::types::*;
use std::collections::HashMap;

/// Parse the JSON output from pw-dump
pub fn parse_pw_dump(json_str: &str) -> Result<Vec<PwObject>> {
    serde_json::from_str(json_str).map_err(|e| PwSplitterError::ParseError(e.to_string()))
}

/// Extract all audio sources (Stream/Output/Audio) from pw-dump objects
pub fn extract_audio_sources(objects: &[PwObject]) -> Vec<AudioSource> {
    objects
        .iter()
        .filter_map(|obj| {
            if let PwObject::Node(node) = obj {
                let info = node.info.as_ref()?;
                let props = info.props.as_ref()?;
                let media_class = props.media_class.as_ref()?;

                if media_class == "Stream/Output/Audio" {
                    return Some(AudioSource {
                        node_id: node.id,
                        node_name: props.node_name.clone().unwrap_or_default(),
                        application_name: props
                            .application_name
                            .clone()
                            .unwrap_or_else(|| props.node_name.clone().unwrap_or_default()),
                        media_name: props
                            .media_name
                            .clone()
                            .unwrap_or_else(|| "Audio".to_string()),
                    });
                }
            }
            None
        })
        .collect()
}

/// Extract all recording destinations (Stream/Input/Audio) from pw-dump objects
pub fn extract_recording_dests(objects: &[PwObject]) -> Vec<RecordingDest> {
    objects
        .iter()
        .filter_map(|obj| {
            if let PwObject::Node(node) = obj {
                let info = node.info.as_ref()?;
                let props = info.props.as_ref()?;
                let media_class = props.media_class.as_ref()?;

                if media_class == "Stream/Input/Audio" {
                    return Some(RecordingDest {
                        node_id: node.id,
                        node_name: props.node_name.clone().unwrap_or_default(),
                        application_name: props
                            .application_name
                            .clone()
                            .unwrap_or_else(|| props.node_name.clone().unwrap_or_default()),
                        media_name: props
                            .media_name
                            .clone()
                            .unwrap_or_else(|| "Audio".to_string()),
                    });
                }
            }
            None
        })
        .collect()
}

/// Extract all audio sinks (Audio/Sink) from pw-dump objects
pub fn extract_audio_sinks(objects: &[PwObject]) -> Vec<AudioSink> {
    objects
        .iter()
        .filter_map(|obj| {
            if let PwObject::Node(node) = obj {
                let info = node.info.as_ref()?;
                let props = info.props.as_ref()?;
                let media_class = props.media_class.as_ref()?;

                if media_class == "Audio/Sink" {
                    return Some(AudioSink {
                        node_id: node.id,
                        node_name: props.node_name.clone().unwrap_or_default(),
                        description: props
                            .node_description
                            .clone()
                            .unwrap_or_else(|| props.node_name.clone().unwrap_or_default()),
                    });
                }
            }
            None
        })
        .collect()
}

/// Extract all ports from pw-dump objects
pub fn extract_ports(objects: &[PwObject]) -> Vec<AudioPort> {
    objects
        .iter()
        .filter_map(|obj| {
            if let PwObject::Port(port) = obj {
                let info = port.info.as_ref()?;
                let props = info.props.as_ref()?;

                let direction = match info.direction.as_deref() {
                    Some("input") => PortDirection::Input,
                    Some("output") => PortDirection::Output,
                    _ => return None,
                };

                return Some(AudioPort {
                    port_id: port.id,
                    node_id: props.node_id?,
                    port_name: props.port_name.clone().unwrap_or_default(),
                    channel: props.audio_channel.clone().unwrap_or_default(),
                    direction,
                });
            }
            None
        })
        .collect()
}

/// Extract all links from pw-dump objects
pub fn extract_links(objects: &[PwObject]) -> Vec<AudioLink> {
    objects
        .iter()
        .filter_map(|obj| {
            if let PwObject::Link(link) = obj {
                let info = link.info.as_ref()?;
                return Some(AudioLink {
                    link_id: link.id,
                    output_node_id: info.output_node_id,
                    output_port_id: info.output_port_id,
                    input_node_id: info.input_node_id,
                    input_port_id: info.input_port_id,
                });
            }
            None
        })
        .collect()
}

/// Find all connections for a source node (what it's currently outputting to)
pub fn find_source_connections(source_node_id: u32, objects: &[PwObject]) -> Vec<SourceConnection> {
    let links = extract_links(objects);
    let sinks = extract_audio_sinks(objects);
    let recording_dests = extract_recording_dests(objects);

    // Build a map of node_id -> node_name for quick lookup
    let mut node_names: HashMap<u32, String> = HashMap::new();
    for sink in &sinks {
        node_names.insert(sink.node_id, sink.node_name.clone());
    }
    for dest in &recording_dests {
        node_names.insert(dest.node_id, dest.node_name.clone());
    }

    // Also check all nodes for names
    for obj in objects {
        if let PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && let Some(name) = &props.node_name
        {
            node_names.insert(node.id, name.clone());
        }
    }

    // Find links from our source
    let source_links: Vec<_> = links
        .iter()
        .filter(|link| link.output_node_id == source_node_id)
        .cloned()
        .collect();

    // Group by target node
    let mut connections_map: HashMap<u32, Vec<AudioLink>> = HashMap::new();
    for link in source_links {
        connections_map
            .entry(link.input_node_id)
            .or_default()
            .push(link);
    }

    connections_map
        .into_iter()
        .map(|(target_id, links)| SourceConnection {
            source_node_id,
            target_node_id: target_id,
            target_node_name: node_names
                .get(&target_id)
                .cloned()
                .unwrap_or_else(|| format!("Unknown({})", target_id)),
            links,
        })
        .collect()
}

/// Find a node by name
pub fn find_node_by_name(objects: &[PwObject], name: &str) -> Option<u32> {
    for obj in objects {
        if let PwObject::Node(node) = obj
            && let Some(info) = &node.info
            && let Some(props) = &info.props
            && props.node_name.as_deref() == Some(name)
        {
            return Some(node.id);
        }
    }
    None
}

/// Get node name by ID
pub fn get_node_name(objects: &[PwObject], node_id: u32) -> Option<String> {
    for obj in objects {
        if let PwObject::Node(node) = obj
            && node.id == node_id
            && let Some(info) = &node.info
            && let Some(props) = &info.props
        {
            return props.node_name.clone();
        }
    }
    None
}