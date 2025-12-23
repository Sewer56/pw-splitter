#![allow(dead_code)]
use serde::Deserialize;

/// Represents a PipeWire object from pw-dump
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PwObject {
    #[serde(rename = "PipeWire:Interface:Node")]
    Node(PwNode),
    #[serde(rename = "PipeWire:Interface:Port")]
    Port(PwPort),
    #[serde(rename = "PipeWire:Interface:Link")]
    Link(PwLink),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PwNode {
    pub id: u32,
    pub info: Option<NodeInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct NodeInfo {
    pub state: Option<String>,
    pub props: Option<NodeProps>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NodeProps {
    #[serde(rename = "node.name")]
    pub node_name: Option<String>,
    #[serde(rename = "node.description")]
    pub node_description: Option<String>,
    #[serde(rename = "application.name")]
    pub application_name: Option<String>,
    #[serde(rename = "media.name")]
    pub media_name: Option<String>,
    #[serde(rename = "media.class")]
    pub media_class: Option<String>,
    #[serde(rename = "object.id")]
    pub object_id: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PwPort {
    pub id: u32,
    pub info: Option<PortInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PortInfo {
    pub direction: Option<String>,
    pub props: Option<PortProps>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PortProps {
    #[serde(rename = "node.id")]
    pub node_id: Option<u32>,
    #[serde(rename = "port.id")]
    pub port_id: Option<u32>,
    #[serde(rename = "port.name")]
    pub port_name: Option<String>,
    #[serde(rename = "audio.channel")]
    pub audio_channel: Option<String>,
    #[serde(rename = "object.id")]
    pub object_id: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PwLink {
    pub id: u32,
    pub info: Option<LinkInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct LinkInfo {
    pub output_node_id: u32,
    pub output_port_id: u32,
    pub input_node_id: u32,
    pub input_port_id: u32,
    pub state: Option<String>,
    pub props: Option<LinkProps>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LinkProps {
    #[serde(rename = "link.output.node")]
    pub link_output_node: Option<u32>,
    #[serde(rename = "link.output.port")]
    pub link_output_port: Option<u32>,
    #[serde(rename = "link.input.node")]
    pub link_input_node: Option<u32>,
    #[serde(rename = "link.input.port")]
    pub link_input_port: Option<u32>,
}

// Simplified types for our application

/// An audio source (application producing audio)
#[derive(Debug, Clone)]
pub struct AudioSource {
    pub node_id: u32,
    pub node_name: String,
    pub application_name: String,
    pub media_name: String,
}

impl AudioSource {
    pub fn display_name(&self) -> String {
        format!("{} [{}]", self.application_name, self.media_name)
    }

    /// Generate a safe name for use in PipeWire object names
    pub fn safe_name(&self) -> String {
        self.application_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
    }
}

/// A recording destination (application capturing audio)
#[derive(Debug, Clone)]
pub struct RecordingDest {
    pub node_id: u32,
    pub node_name: String,
    pub application_name: String,
    pub media_name: String,
}

impl RecordingDest {
    pub fn display_name(&self) -> String {
        format!("{} [{}]", self.application_name, self.media_name)
    }
}

/// An audio sink (speaker/output device)
#[derive(Debug, Clone)]
pub struct AudioSink {
    pub node_id: u32,
    pub node_name: String,
    pub description: String,
}

/// A port on a node
#[derive(Debug, Clone)]
pub struct AudioPort {
    pub port_id: u32,
    pub node_id: u32,
    pub port_name: String,
    pub channel: String,
    pub direction: PortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

/// An existing link between ports
#[derive(Debug, Clone)]
pub struct AudioLink {
    pub link_id: u32,
    pub output_node_id: u32,
    pub output_port_id: u32,
    pub input_node_id: u32,
    pub input_port_id: u32,
}

/// Connection info for a source
#[derive(Debug, Clone)]
pub struct SourceConnection {
    pub source_node_id: u32,
    pub target_node_id: u32,
    pub target_node_name: String,
    pub links: Vec<AudioLink>,
}
