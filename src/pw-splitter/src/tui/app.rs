use crate::error::Result;
use crate::pipewire::{self, AudioSource, RecordingDest, SourceConnection};
use crate::splitter::{self, SplitConfig, SplitState};

/// Application state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    SelectSource,
    SelectDestination,
    Confirm,
    Active,
    Error(String),
    Done,
}

/// Main application
pub struct App {
    pub state: AppState,
    pub sources: Vec<AudioSource>,
    pub destinations: Vec<RecordingDest>,
    pub selected_source_idx: usize,
    pub selected_dest_idx: usize,
    pub selected_source: Option<AudioSource>,
    pub selected_dest: Option<RecordingDest>,
    pub source_connections: Vec<SourceConnection>,
    pub active_split: Option<SplitState>,
    pub status_message: String,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let objects = pipewire::get_pw_objects()?;
        let sources = pipewire::extract_audio_sources(&objects);
        let destinations = pipewire::extract_recording_dests(&objects);

        Ok(Self {
            state: AppState::SelectSource,
            sources,
            destinations,
            selected_source_idx: 0,
            selected_dest_idx: 0,
            selected_source: None,
            selected_dest: None,
            source_connections: Vec::new(),
            active_split: None,
            status_message: String::new(),
            should_quit: false,
        })
    }

    /// Refresh the list of sources and destinations
    pub fn refresh(&mut self) -> Result<()> {
        let objects = pipewire::get_pw_objects()?;
        self.sources = pipewire::extract_audio_sources(&objects);
        self.destinations = pipewire::extract_recording_dests(&objects);

        // Reset indices if out of bounds
        if self.selected_source_idx >= self.sources.len() {
            self.selected_source_idx = self.sources.len().saturating_sub(1);
        }
        if self.selected_dest_idx >= self.destinations.len() {
            self.selected_dest_idx = self.destinations.len().saturating_sub(1);
        }

        Ok(())
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        match self.state {
            AppState::SelectSource => {
                if self.selected_source_idx > 0 {
                    self.selected_source_idx -= 1;
                }
            }
            AppState::SelectDestination => {
                if self.selected_dest_idx > 0 {
                    self.selected_dest_idx -= 1;
                }
            }
            _ => {}
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        match self.state {
            AppState::SelectSource => {
                if !self.sources.is_empty() && self.selected_source_idx < self.sources.len() - 1 {
                    self.selected_source_idx += 1;
                }
            }
            AppState::SelectDestination => {
                if !self.destinations.is_empty()
                    && self.selected_dest_idx < self.destinations.len() - 1
                {
                    self.selected_dest_idx += 1;
                }
            }
            _ => {}
        }
    }

    /// Confirm current selection and move to next state
    pub fn confirm_selection(&mut self) {
        match self.state {
            AppState::SelectSource => {
                if self.sources.is_empty() {
                    self.status_message = "No audio sources available".to_string();
                    return;
                }

                let source = self.sources[self.selected_source_idx].clone();

                // Find current connections for this source
                if let Ok(objects) = pipewire::get_pw_objects() {
                    self.source_connections =
                        pipewire::find_source_connections(source.node_id, &objects);
                }

                if self.source_connections.is_empty() {
                    self.status_message = "Warning: Source has no active connections".to_string();
                }

                self.selected_source = Some(source);
                self.state = AppState::SelectDestination;
                self.status_message.clear();
            }
            AppState::SelectDestination => {
                if self.destinations.is_empty() {
                    self.status_message = "No recording destinations available".to_string();
                    return;
                }

                self.selected_dest = Some(self.destinations[self.selected_dest_idx].clone());
                self.state = AppState::Confirm;
                self.status_message.clear();
            }
            AppState::Confirm => {
                self.execute_split();
            }
            AppState::Active => {
                // Stop the split
                if let Some(state) = &self.active_split {
                    match splitter::teardown_split(state) {
                        Ok(()) => {
                            self.status_message = "Split stopped successfully".to_string();
                            self.active_split = None;
                            self.state = AppState::Done;
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to stop split: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Go back to previous state
    pub fn go_back(&mut self) {
        match self.state {
            AppState::SelectDestination => {
                self.selected_source = None;
                self.source_connections.clear();
                self.state = AppState::SelectSource;
            }
            AppState::Confirm => {
                self.selected_dest = None;
                self.state = AppState::SelectDestination;
            }
            AppState::Active => {
                // Don't go back from active state - must stop first
            }
            _ => {}
        }
        self.status_message.clear();
    }

    /// Execute the split setup
    fn execute_split(&mut self) {
        let source = match &self.selected_source {
            Some(s) => s.clone(),
            None => {
                self.state = AppState::Error("No source selected".to_string());
                return;
            }
        };

        let dest = match &self.selected_dest {
            Some(d) => d.clone(),
            None => {
                self.state = AppState::Error("No destination selected".to_string());
                return;
            }
        };

        // If source has no connections, we still proceed but warn
        let connections = if self.source_connections.is_empty() {
            // Try to get default output
            if let Ok(objects) = pipewire::get_pw_objects() {
                let sinks = pipewire::extract_audio_sinks(&objects);
                if let Some(default_sink) = sinks.first() {
                    vec![SourceConnection {
                        source_node_id: source.node_id,
                        target_node_id: default_sink.node_id,
                        target_node_name: default_sink.node_name.clone(),
                        links: Vec::new(),
                    }]
                } else {
                    self.state = AppState::Error("No output sinks available".to_string());
                    return;
                }
            } else {
                self.state = AppState::Error("Failed to query PipeWire".to_string());
                return;
            }
        } else {
            self.source_connections.clone()
        };

        let config = SplitConfig {
            source,
            recording_dest: dest,
            original_connections: connections,
        };

        match splitter::setup_split(config) {
            Ok(result) => {
                self.active_split = Some(result.state);
                self.state = AppState::Active;
                self.status_message = "Split active! Adjust volume in pwvucontrol".to_string();

                // Forget the child processes so they keep running
                std::mem::forget(result.loopback_to_recording);
                std::mem::forget(result.loopback_to_local);
            }
            Err(e) => {
                self.state = AppState::Error(format!("Failed to create split: {}", e));
            }
        }
    }

    /// Check if loopback processes are still running and restart if needed
    pub fn check_and_restart_loopbacks(&mut self) {
        if let Some(state) = &mut self.active_split {
            let (recording_running, local_running) = splitter::check_loopbacks_running(state);

            if !recording_running {
                self.status_message = "Recording loopback crashed, restarting...".to_string();
                if let Err(e) = splitter::restart_loopback_to_recording(state) {
                    self.status_message = format!("Failed to restart recording loopback: {}", e);
                } else {
                    self.status_message = "Recording loopback restarted".to_string();
                }
            }

            if !local_running {
                self.status_message = "Local loopback crashed, restarting...".to_string();
                if let Err(e) = splitter::restart_loopback_to_local(state) {
                    self.status_message = format!("Failed to restart local loopback: {}", e);
                } else {
                    self.status_message = "Local loopback restarted".to_string();
                }
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::SelectSource,
            sources: Vec::new(),
            destinations: Vec::new(),
            selected_source_idx: 0,
            selected_dest_idx: 0,
            selected_source: None,
            selected_dest: None,
            source_connections: Vec::new(),
            active_split: None,
            status_message: String::new(),
            should_quit: false,
        }
    }
}
