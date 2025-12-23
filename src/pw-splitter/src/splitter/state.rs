use crate::error::{PwSplitterError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const STATE_DIR: &str = "/tmp/pw-splitter";

/// Persistent state for an active split
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitState {
    /// Unique name for this split (based on source name)
    pub name: String,

    /// Source node info
    pub source_node_id: u32,
    pub source_node_name: String,
    pub source_application_name: String,

    /// Loopback names (for reconnecting on restart)
    pub recording_loopback_name: String,
    pub local_loopback_name: String,

    /// Recording destination info
    pub recording_dest_node_id: u32,
    pub recording_dest_media_name: String,
    pub recording_dest_application_name: String,

    /// Original output (for restoration)
    pub original_output_node_name: String,

    /// Original links that were disconnected (for restoration)
    pub original_links: Vec<SavedLink>,

    /// PIDs of loopback processes
    pub loopback_to_recording_pid: u32,
    pub loopback_to_local_pid: u32,

    /// Timestamp when split was created
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLink {
    pub output_port: String,
    pub input_port: String,
}

impl SplitState {
    /// Get the state file path for a split
    pub fn state_file_path(name: &str) -> PathBuf {
        PathBuf::from(STATE_DIR).join(format!("{}.json", name))
    }

    /// Save state to file
    pub fn save(&self) -> Result<()> {
        // Ensure state directory exists
        fs::create_dir_all(STATE_DIR).map_err(|e| {
            PwSplitterError::StateFileError(format!("Failed to create state dir: {}", e))
        })?;

        let path = Self::state_file_path(&self.name);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json).map_err(|e| {
            PwSplitterError::StateFileError(format!("Failed to write state file: {}", e))
        })?;

        Ok(())
    }

    /// Load state from file
    pub fn load(name: &str) -> Result<Self> {
        let path = Self::state_file_path(name);
        let json = fs::read_to_string(&path).map_err(|e| {
            PwSplitterError::StateFileError(format!("Failed to read state file: {}", e))
        })?;
        let state: SplitState = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Delete state file
    pub fn delete(&self) -> Result<()> {
        let path = Self::state_file_path(&self.name);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                PwSplitterError::StateFileError(format!("Failed to delete state file: {}", e))
            })?;
        }
        Ok(())
    }

    /// List all active splits
    pub fn list_all() -> Result<Vec<SplitState>> {
        let state_dir = PathBuf::from(STATE_DIR);
        if !state_dir.exists() {
            return Ok(vec![]);
        }

        let mut states = Vec::new();
        for entry in fs::read_dir(&state_dir).map_err(|e| {
            PwSplitterError::StateFileError(format!("Failed to read state dir: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                PwSplitterError::StateFileError(format!("Failed to read entry: {}", e))
            })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(json) = fs::read_to_string(&path)
                && let Ok(state) = serde_json::from_str::<SplitState>(&json)
            {
                states.push(state);
            }
        }

        Ok(states)
    }

    /// Check if a split with this name already exists
    pub fn exists(name: &str) -> bool {
        Self::state_file_path(name).exists()
    }

    /// Generate a unique split name
    pub fn generate_unique_name(base_name: &str) -> String {
        let mut name = base_name.to_string();
        let mut counter = 1;
        while Self::exists(&name) {
            name = format!("{}_{}", base_name, counter);
            counter += 1;
        }
        name
    }
}
