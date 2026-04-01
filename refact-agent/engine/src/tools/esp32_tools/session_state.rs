use std::collections::HashMap;
use std::time::Instant;

/// Central state manager for ESP32 development session
#[derive(Clone, Debug)]
pub struct ESP32SessionState {
    // Hardware State (persists across tools)
    pub detected_devices: Vec<DetectedDevice>,
    pub active_device: Option<DeviceConnection>,
    pub last_hardware_check: Option<Instant>,
    
    // Board Definition (session-wide)
    pub board_id: Option<String>,
    pub board_verified: bool,
    
    // Project State
    pub current_project: Option<ProjectState>,
    pub project_history: Vec<ProjectAction>,
    
    // Build State
    pub last_build: Option<BuildResult>,
    pub build_artifacts: HashMap<String, ArtifactInfo>,
    pub pending_errors: Vec<ClassifiedError>,
    
    // Flash/Device State
    pub device_firmware: Option<FirmwareState>,
    pub ota_status: Option<OTAState>,
}

#[derive(Clone, Debug)]
pub struct DetectedDevice {
    pub port: String,
    pub chip: String,
    pub mac_address: Option<String>,
    pub flash_size: Option<String>,
    pub detected_at: Instant,
}

#[derive(Clone, Debug)]
pub struct DeviceConnection {
    pub port: String,
    pub chip: String,
    pub mac_address: String,
    pub flash_size: String,
    pub connected_at: Instant,
}

#[derive(Clone, Debug)]
pub struct ProjectState {
    pub name: String,
    pub path: std::path::PathBuf,
    pub target: String,
    pub config: ProjectConfig,
    pub last_modified: Instant,
    pub build_status: BuildStatus,
}

#[derive(Clone, Debug)]
pub enum BuildStatus {
    NotBuilt,
    Building,
    Built { config: String, timestamp: Instant, output_path: String },
    Failed { errors: Vec<ClassifiedError> },
}

#[derive(Clone, Debug)]
pub struct ProjectConfig {
    pub target: String,
    pub sdkconfig_path: Option<String>,
    pub partition_table: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectAction {
    pub action: String,
    pub timestamp: Instant,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    pub project_name: String,
    pub config: String,
    pub status: BuildStatus,
    pub output_path: Option<String>,
    pub build_time: Option<std::time::Duration>,
}

#[derive(Clone, Debug)]
pub struct ArtifactInfo {
    pub path: String,
    pub size: usize,
    pub created_at: Instant,
}

#[derive(Clone, Debug)]
pub struct FirmwareState {
    pub version: String,
    pub flashed_at: Instant,
    pub binary_path: String,
}

#[derive(Clone, Debug)]
pub struct OTAState {
    pub enabled: bool,
    pub partition_scheme: String,
    pub current_version: String,
}

#[derive(Clone, Debug)]
pub struct ClassifiedError {
    pub category: String,
    pub subcategory: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl ESP32SessionState {
    pub fn new() -> Self {
        Self {
            detected_devices: Vec::new(),
            active_device: None,
            last_hardware_check: None,
            board_id: None,
            board_verified: false,
            current_project: None,
            project_history: Vec::new(),
            last_build: None,
            build_artifacts: HashMap::new(),
            pending_errors: Vec::new(),
            device_firmware: None,
            ota_status: None,
        }
    }
    
    /// Set board ID for the session
    pub fn set_board_id(&mut self, board_id: String) {
        self.board_id = Some(board_id);
        self.board_verified = false; // Reset verification when board changes
    }
    
    /// Mark board as verified
    pub fn mark_board_verified(&mut self) {
        self.board_verified = true;
    }

    /// Returns a token-efficient summary for LLM context
    /// Target: <100 tokens for quick context
    pub fn get_context_summary(&self, max_tokens: usize) -> String {
        let mut summary = String::new();
        
        // Project state (1 line)
        if let Some(proj) = &self.current_project {
            summary.push_str(&format!(
                "PROJECT: {} [{}] {}\n",
                proj.name,
                proj.target,
                match &proj.build_status {
                    BuildStatus::Built { config, .. } => format!("built:{}", config),
                    BuildStatus::Failed { .. } => "FAILED".to_string(),
                    _ => "not_built".to_string(),
                }
            ));
        }
        
        // Device state (1 line)
        if let Some(dev) = &self.active_device {
            summary.push_str(&format!(
                "DEVICE: {} {} connected\n",
                dev.chip, dev.port
            ));
        } else {
            summary.push_str("DEVICE: none\n");
        }
        
        // Board state (1 line)
        if let Some(board) = &self.board_id {
            summary.push_str(&format!(
                "BOARD: {} [verified={}]\n",
                board,
                if self.board_verified { "yes" } else { "no" }
            ));
        }
        
        // Pending issues (if any)
        if !self.pending_errors.is_empty() {
            summary.push_str(&format!(
                "ISSUES: {} errors pending\n",
                self.pending_errors.len()
            ));
        }
        
        // Truncate if too long
        if summary.len() > max_tokens * 4 {  // Rough estimate: 4 chars per token
            summary.truncate(max_tokens * 4);
            summary.push_str("...");
        }
        
        summary
    }
    
    /// Quick facts the LLM can reference by key
    pub fn get_facts(&self) -> HashMap<String, String> {
        let mut facts = HashMap::new();
        
        if let Some(proj) = &self.current_project {
            facts.insert("project_name".into(), proj.name.clone());
            facts.insert("project_path".into(), proj.path.to_string_lossy().into());
            facts.insert("target".into(), proj.target.clone());
        }
        
        if let Some(dev) = &self.active_device {
            facts.insert("device_port".into(), dev.port.clone());
            facts.insert("device_chip".into(), dev.chip.clone());
        }
        
        if let Some(build) = &self.last_build {
            facts.insert("last_build_config".into(), build.config.clone());
            facts.insert("last_build_status".into(), format!("{:?}", build.status));
        }
        
        facts
    }
}

impl Default for ESP32SessionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents what changed between tool calls
#[derive(Debug, Clone)]
pub struct StateDelta {
    pub changed_fields: Vec<String>,
    pub summary: String,
}

impl StateDelta {
    pub fn none() -> Self {
        Self {
            changed_fields: vec![],
            summary: "No state changes".to_string(),
        }
    }
}

