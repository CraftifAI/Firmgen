use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2000Config {
    pub ccs_path: String,
    pub ccs_cli_path: String,
    pub workspace_path: String,
    pub c2000ware_path: String,
    pub default_uart_device: String,
    pub default_uart_baud: u32,
    pub default_uart_parity: String,
}

impl Default for C2000Config {
    fn default() -> Self {
        Self {
            ccs_path: "/home/user/ti/ccs2020/ccs".to_string(),
            ccs_cli_path: "/home/user/ti/ccs2020/ccs/eclipse/ccs-server-cli.sh".to_string(),
            workspace_path: "/home/user/ti/ccs2020/ccs/example_workspace".to_string(),
            c2000ware_path: "/home/user/ti/C2000Ware_6_00_00_00".to_string(),
            default_uart_device: "/dev/ttyACM0".to_string(),
            default_uart_baud: 115200,
            default_uart_parity: "odd".to_string(),
        }
    }
}

impl C2000Config {
    pub async fn load_from_api(api_url: &str) -> Result<Self, String> {
        // Try HTTP API first
        match Self::try_load_from_api(api_url).await {
            Ok(config) => Ok(config),
            Err(api_error) => {
                // Fallback to file-based loading
                let fallback_path = "/home/shubham/.cache/refact/c2000_tools.yaml";
                match Self::load_from_file(fallback_path).await {
                    Ok(config) => {
                        println!("HTTP API failed ({}), using file fallback", api_error);
                        Ok(config)
                    },
                    Err(file_error) => Err(format!("Both HTTP API and file loading failed. API error: {}, File error: {}", api_error, file_error))
                }
            }
        }
    }

    async fn try_load_from_api(api_url: &str) -> Result<Self, String> {
        let response = reqwest::get(api_url)
            .await
            .map_err(|e| format!("Error fetching C2000 config from API: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error status: {}", response.status()));
        }

        let config_json: serde_json::Value = response.json()
            .await
            .map_err(|e| format!("Error parsing API response JSON: {}", e))?;

        let c2000_config = config_json.get("c2000_config")
            .ok_or("Missing c2000_config section in API response")?;

        let ccs_path = c2000_config.get("ccs_path")
            .and_then(|v| v.as_str())
            .unwrap_or("/home/user/ti/ccs2020/ccs")
            .to_string();

        Ok(C2000Config {
            ccs_path: ccs_path.clone(),
            ccs_cli_path: format!("{}/eclipse/ccs-server-cli.sh", ccs_path),
            workspace_path: c2000_config.get("workspace_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/ti/ccs2020/ccs/example_workspace")
                .to_string(),
            c2000ware_path: c2000_config.get("c2000ware_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/ti/C2000Ware_6_00_00_00")
                .to_string(),
            default_uart_device: c2000_config.get("default_uart_device")
                .and_then(|v| v.as_str())
                .unwrap_or("/dev/ttyACM0")
                .to_string(),
            default_uart_baud: c2000_config.get("default_uart_baud")
                .and_then(|v| v.as_u64())
                .unwrap_or(115200) as u32,
            default_uart_parity: c2000_config.get("default_uart_parity")
                .and_then(|v| v.as_str())
                .unwrap_or("odd")
                .to_string(),
        })
    }

    // Keep the old method for backward compatibility
    pub async fn load_from_file(config_path: &str) -> Result<Self, String> {
        let config_content = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| format!("Error reading C2000 config file: {}", e))?;

        let config: serde_yaml::Value = serde_yaml::from_str(&config_content)
            .map_err(|e| format!("Error parsing C2000 config file: {}", e))?;

        let c2000_config = config.get("c2000_config")
            .ok_or("Missing c2000_config section")?;

        let ccs_path = c2000_config.get("ccs_path")
            .and_then(|v| v.as_str())
            .unwrap_or("/home/user/ti/ccs2020/ccs")
            .to_string();

        Ok(C2000Config {
            ccs_path: ccs_path.clone(),
            ccs_cli_path: format!("{}/eclipse/ccs-server-cli.sh", ccs_path),
            workspace_path: c2000_config.get("workspace_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/ti/ccs2020/ccs/example_workspace")
                .to_string(),
            c2000ware_path: c2000_config.get("c2000ware_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/ti/C2000Ware_6_00_00_00")
                .to_string(),
            default_uart_device: c2000_config.get("default_uart_device")
                .and_then(|v| v.as_str())
                .unwrap_or("/dev/ttyACM0")
                .to_string(),
            default_uart_baud: c2000_config.get("default_uart_baud")
                .and_then(|v| v.as_u64())
                .unwrap_or(115200) as u32,
            default_uart_parity: c2000_config.get("default_uart_parity")
                .and_then(|v| v.as_str())
                .unwrap_or("odd")
                .to_string(),
        })
    }

    pub fn resolve_path_variables(&self, path: &str) -> String {
        let mut resolved = path.to_string();
        
        // Replace common variables
        resolved = resolved.replace("$CCS", &self.ccs_path);
        resolved = resolved.replace("$WS", &self.workspace_path);
        resolved = resolved.replace("$C2000WARE", &self.c2000ware_path);
        
        // Replace home directory
        if let Some(home) = std::env::var("HOME").ok() {
            resolved = resolved.replace("~", &home);
        }

        resolved
    }

    pub fn get_ccxml_path(&self, project_name: &str) -> PathBuf {
        PathBuf::from(&self.workspace_path)
            .join(project_name)
            .join("targetConfigs")
            .join("TMS320F28P650DK9.ccxml")
    }

    pub fn get_output_path(&self, project_name: &str, configuration: &str) -> PathBuf {
        PathBuf::from(&self.workspace_path)
            .join(project_name)
            .join(configuration)
            .join(format!("{}.out", project_name))
    }

    pub fn validate_paths(&self) -> Result<(), String> {
        // Check if CCS path exists
        if !std::path::Path::new(&self.ccs_path).exists() {
            return Err(format!("CCS path does not exist: {}", self.ccs_path));
        }

        // Check if CCS CLI exists
        let ccs_cli_path = format!("{}/eclipse/ccs-server-cli.sh", self.ccs_path);
        if !std::path::Path::new(&ccs_cli_path).exists() {
            return Err(format!("CCS CLI not found: {}", ccs_cli_path));
        }

        // Check if workspace path exists (create if not)
        if !std::path::Path::new(&self.workspace_path).exists() {
            std::fs::create_dir_all(&self.workspace_path)
                .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
        }

        // Check if C2000Ware path exists
        if !std::path::Path::new(&self.c2000ware_path).exists() {
            return Err(format!("C2000Ware path does not exist: {}", self.c2000ware_path));
        }

        Ok(())
    }

    /// Check if a file path is within the CCS workspace (where modifications are allowed)
    pub fn is_in_ccs_workspace(&self, file_path: &str) -> bool {
        let path = PathBuf::from(file_path);
        let ccs_workspace = PathBuf::from(&self.workspace_path);
        
        // Try canonical paths for more accurate comparison
        let path_canonical = path.canonicalize().ok();
        let workspace_canonical = ccs_workspace.canonicalize().ok();
        
        if let (Some(p), Some(w)) = (path_canonical, workspace_canonical) {
            return p.starts_with(&w);
        }
        
        // Fallback to string comparison if canonicalization fails
        path.starts_with(&ccs_workspace)
    }

    /// Get safe path for modification - copy to CCS workspace if needed
    /// Returns the path to use for modification (either original if already in workspace, or copied path)
    pub fn get_safe_modification_path(&self, source_path: &str) -> Result<PathBuf, String> {
        let source = PathBuf::from(source_path);
        
        if !source.exists() {
            return Err(format!("Source file does not exist: {}", source_path));
        }
        
        // Already safe - return as-is
        if self.is_in_ccs_workspace(source_path) {
            return Ok(source);
        }
        
        // Need to copy to workspace
        let file_name = source.file_name()
            .ok_or("Invalid path: no filename")?
            .to_string_lossy()
            .to_string();
        
        // Try to preserve some directory structure for SDK files
        let relative_path = if source_path.contains("C2000Ware") || source_path.contains("ti_c2000_sdk") {
            // Extract meaningful path components (e.g., examples/spi/CCS/file.syscfg)
            if let Some(parts) = self.extract_sdk_relative_path(source_path) {
                PathBuf::from("sdk_copies").join(parts)
            } else {
                PathBuf::from("sdk_copies").join(&file_name)
            }
        } else {
            // For other files outside workspace, use a generic location
            PathBuf::from("workspace_copies").join(&file_name)
        };
        
        let dest = PathBuf::from(&self.workspace_path).join(relative_path);
        
        // Create directory if needed
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
        
        // Copy file
        std::fs::copy(&source, &dest)
            .map_err(|e| format!("Failed to copy file from {} to {}: {}", 
                source.display(), dest.display(), e))?;
        
        Ok(dest)
    }

    /// Extract meaningful relative path from SDK file path
    /// Returns path like "examples/spi/CCS/file.syscfg" or None if can't extract
    fn extract_sdk_relative_path(&self, full_path: &str) -> Option<PathBuf> {
        // Look for common SDK directory patterns
        let patterns = vec![
            "examples/",
            "driverlib/",
            "device_support/",
            "training/",
        ];
        
        for pattern in patterns {
            if let Some(pos) = full_path.find(pattern) {
                let after_pattern = &full_path[pos + pattern.len()..];
                // Take up to 3-4 path components to preserve structure
                let components: Vec<&str> = after_pattern.split('/')
                    .take(4)
                    .collect();
                if !components.is_empty() {
                    return Some(PathBuf::from_iter(components));
                }
            }
        }
        
        None
    }
}

// Shared utility function for modifying projectspec files
pub fn replace_project_name_in_projectspec(content: &str, new_name: &str) -> Result<String, String> {
    // Find the <project> tag and replace the name attribute
    let proj_open_idx = match content.find("<project") {
        Some(i) => i,
        None => return Err("Invalid projectspec: missing <project tag".to_string()),
    };
    
    let tag_close_rel = match content[proj_open_idx..].find('>') {
        Some(i) => i,
        None => return Err("Invalid projectspec: unterminated <project tag".to_string()),
    };
    
    let tag_close_idx = proj_open_idx + tag_close_rel;

    // Work within the tag header only
    let header = &content[proj_open_idx..tag_close_idx];
    let name_attr = "name=\"";
    let name_pos_rel = match header.find(name_attr) {
        Some(i) => i,
        None => return Err("Invalid projectspec: <project> missing name=\"...\" attribute".to_string()),
    };
    
    let name_val_start = proj_open_idx + name_pos_rel + name_attr.len();
    let rest = &content[name_val_start..tag_close_idx];
    let name_end_rel = match rest.find('"') {
        Some(i) => i,
        None => return Err("Invalid projectspec: unterminated name attribute".to_string()),
    };
    
    let name_val_end = name_val_start + name_end_rel;

    // Build the modified content
    let mut result = String::with_capacity(content.len() + new_name.len());
    result.push_str(&content[..name_val_start]);
    result.push_str(new_name);
    result.push_str(&content[name_val_end..]);

    Ok(result)
}


