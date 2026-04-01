//! Unified ESP-IDF Command Runner
//! 
//! Handles:
//! - Environment setup (IDF_PATH, tools in PATH)
//! - Consistent timeout handling
//! - Output parsing with error classification
//! - Cross-platform support

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

use super::config::ESP32Config;
use super::error_parser::ErrorParser;
use super::output_protocol::ClassifiedError;

/// Result of an IDF command execution
#[derive(Debug, Clone)]
pub struct IdfCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub errors: Vec<ClassifiedError>,
    pub summary: String,
}

/// Maximum number of lines to keep in truncated output
const MAX_OUTPUT_LINES: usize = 500;

impl IdfCommandResult {
    pub fn combined_output(&self) -> String {
        truncate_output(&format!("{}\n{}", self.stdout, self.stderr), MAX_OUTPUT_LINES)
    }
}

/// Truncate output to the last `max_lines` lines, preserving the most recent (useful) output.
/// If truncated, prepends a notice about how many lines were omitted.
pub fn truncate_output(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= max_lines {
        return output.to_string();
    }
    let skipped = lines.len() - max_lines;
    let kept: Vec<&str> = lines[skipped..].to_vec();
    format!("[... {} lines truncated ...]\n{}", skipped, kept.join("\n"))
}

/// Builder for IDF commands
pub struct IdfCommand {
    command: String,
    args: Vec<String>,
    project_path: Option<PathBuf>,
    timeout: Duration,
    env_vars: HashMap<String, String>,
    parse_errors: bool,
    /// For esptool: global options that must come BEFORE the subcommand
    esptool_global_args: Vec<String>,
    /// For esptool: the subcommand (stored separately to insert after global args)
    esptool_subcommand: Option<String>,
}

impl IdfCommand {
    /// Create a new idf.py command
    pub fn new(subcommand: &str) -> Self {
        Self {
            command: "idf.py".to_string(),
            args: vec![subcommand.to_string()],
            project_path: None,
            timeout: Duration::from_secs(300), // 5 minute default
            env_vars: HashMap::new(),
            parse_errors: true,
            esptool_global_args: vec![],
            esptool_subcommand: None,
        }
    }

    /// Create a raw command (not idf.py)
    pub fn raw(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: vec![],
            project_path: None,
            timeout: Duration::from_secs(60),
            env_vars: HashMap::new(),
            parse_errors: true,
            esptool_global_args: vec![],
            esptool_subcommand: None,
        }
    }

    /// Create an esptool command
    /// NOTE: esptool uses underscores in command names (chip_id, flash_id, etc.)
    /// Global options like --port must come BEFORE the subcommand
    pub fn esptool(subcommand: &str) -> Self {
        // Normalize command: convert hyphens to underscores for esptool compatibility
        let normalized_cmd = subcommand.replace('-', "_");
        Self {
            command: "python3".to_string(),
            args: vec!["-m".to_string(), "esptool".to_string()],
            project_path: None,
            timeout: Duration::from_secs(120),
            env_vars: HashMap::new(),
            parse_errors: true,
            esptool_global_args: vec![],
            esptool_subcommand: Some(normalized_cmd),
        }
    }

    /// Add an argument
    /// For esptool commands, options like --port are added as global args (before subcommand)
    pub fn arg(mut self, arg: &str) -> Self {
        if self.esptool_subcommand.is_some() && arg.starts_with('-') {
            // For esptool, options go before the subcommand
            self.esptool_global_args.push(arg.to_string());
        } else {
            self.args.push(arg.to_string());
        }
        self
    }

    /// Add multiple arguments
    /// For esptool commands, options like --port are added as global args (before subcommand)
    pub fn args(mut self, args: &[&str]) -> Self {
        for arg in args {
            if self.esptool_subcommand.is_some() && arg.starts_with('-') {
                // For esptool, options go before the subcommand
                self.esptool_global_args.push(arg.to_string());
            } else if self.esptool_subcommand.is_some() && self.esptool_global_args.last().map(|s| s.starts_with('-')).unwrap_or(false) {
                // This is likely a value for the previous option (e.g., port path after --port)
                self.esptool_global_args.push(arg.to_string());
            } else {
                self.args.push(arg.to_string());
            }
        }
        self
    }

    /// Set the working directory (project path)
    pub fn project_path(mut self, path: &Path) -> Self {
        self.project_path = Some(path.to_path_buf());
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set timeout in seconds
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Disable error parsing
    pub fn no_parse_errors(mut self) -> Self {
        self.parse_errors = false;
        self
    }

    /// Find the correct IDF Python virtual-environment path for the given IDF install.
    ///
    /// Scans all venvs matching the IDF version prefix and returns the one that
    /// actually has `esp_idf_monitor` in its site-packages — immune to system
    /// python3 version mismatches.
    fn find_python_env(idf_path: &str) -> Option<PathBuf> {
        // Derive IDF major.minor. Try multiple sources in order of reliability:

        // 1. tools/cmake/version.cmake — most reliable, present in most installs
        let idf_ver_key = (|| -> Option<String> {
            let cmake = PathBuf::from(idf_path)
                .join("tools").join("cmake").join("version.cmake");
            let content = std::fs::read_to_string(&cmake).ok()?;
            let major = content.lines()
                .find(|l| l.contains("IDF_VERSION_MAJOR"))?
                .split_whitespace().nth(1)?.trim_end_matches(')');
            let minor = content.lines()
                .find(|l| l.contains("IDF_VERSION_MINOR"))?
                .split_whitespace().nth(1)?.trim_end_matches(')');
            Some(format!("idf{}.{}", major, minor))
        })()
        // 2. version.txt — present in release tarballs
        .or_else(|| {
            let ver_file = PathBuf::from(idf_path).join("version.txt");
            let v = std::fs::read_to_string(&ver_file).ok()?;
            let v = v.trim().trim_start_matches('v');
            let mut p = v.splitn(3, '.');
            let major = p.next()?;
            let minor = p.next().unwrap_or("0");
            Some(format!("idf{}.{}", major, minor))
        })
        // 3. Last segment of the install path (e.g. "esp-idf-release-v5.5" → "idf5.5")
        .or_else(|| {
            let dir = PathBuf::from(idf_path).file_name()?
                .to_string_lossy().to_string();
            // Find "v5.5" or "5.5" pattern in the dir name
            let re = regex::Regex::new(r"v?(\d+)\.(\d+)").ok()?;
            let caps = re.captures(&dir)?;
            Some(format!("idf{}.{}", &caps[1], &caps[2]))
        })?;

        let home = std::env::var("HOME").ok()?;
        let env_base = PathBuf::from(&home).join(".espressif").join("python_env");

        let entries = std::fs::read_dir(&env_base).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&format!("{}_", idf_ver_key)) {
                continue;
            }
            let venv = entry.path();
            if !venv.join("bin").join("python").exists() {
                continue;
            }
            // Pick the venv that has esp_idf_monitor — avoids stale/incomplete venvs.
            let lib = venv.join("lib");
            let has_monitor = std::fs::read_dir(&lib)
                .ok()
                .map(|es| {
                    es.flatten().any(|e| {
                        e.path().join("site-packages").join("esp_idf_monitor").exists()
                    })
                })
                .unwrap_or(false);
            if has_monitor {
                return Some(venv);
            }
        }
        None
    }

    /// Execute the command with ESP-IDF environment.
    ///
    /// For `idf.py` commands:
    ///  - The script is invoked as `python3 /full/path/idf.py <args>` so PATH
    ///    is irrelevant for finding the script.
    ///  - `IDF_PYTHON_ENV_PATH` is set to the matching venv so idf.py activates
    ///    the correct Python environment regardless of what PATH contains.
    ///
    /// Non-idf.py commands (esptool, cp, …) are executed directly as before.
    pub async fn execute(self, config: &ESP32Config) -> Result<IdfCommandResult, String> {
        let start_time = std::time::Instant::now();

        let mut cmd = if self.command == "idf.py" {
            // Full absolute path to idf.py — no PATH lookup needed.
            let idf_py = format!("{}/tools/idf.py", config.esp_idf_path);

            let mut c = Command::new("python3");
            c.arg(&idf_py);
            c.args(&self.args);
            c
        } else {
            // esptool, cp, and other direct commands.
            let mut c = Command::new(&self.command);
            if let Some(ref subcommand) = self.esptool_subcommand {
                c.args(&self.args);
                c.args(&self.esptool_global_args);
                c.arg(subcommand);
            } else {
                c.args(&self.args);
            }
            c
        };

        // Working directory (applies to both idf.py and direct commands).
        if let Some(ref project_path) = self.project_path {
            cmd.current_dir(project_path);
        }

        // Always set IDF_PATH — idf.py needs it to locate components and tools.
        cmd.env("IDF_PATH", &config.esp_idf_path);

        // For idf.py: set IDF_PYTHON_ENV_PATH so the script activates the
        // correct venv (the one matching the current python3 + IDF version).
        // Without this idf.py guesses the venv name from sys.version_info of
        // whatever Python is on $PATH, which may differ from the installed venv.
        if self.command == "idf.py" {
            if let Some(env_path) = Self::find_python_env(&config.esp_idf_path) {
                cmd.env("IDF_PYTHON_ENV_PATH", env_path);
            }
        }

        // Custom per-call env vars (e.g. serial port overrides).
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| format!("Command timed out after {:?}", self.timeout))?
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let duration = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();
        let exit_code = output.status.code();

        // Parse errors if requested
        // Derive phase hint from command name for Cursor-style diagnostics
        let phase_hint = self.args.first().map(|s| s.as_str());
        let (errors, summary) = if self.parse_errors && !success {
            let parser = ErrorParser;
            let combined = format!("{}\n{}", stdout, stderr);
            let parsed = parser.parse_build_output(&combined, phase_hint);

            // Safety net: if parser found zero errors but the command definitely failed,
            // don't say "Build successful, no errors" — include stderr tail instead
            let summary = if parsed.errors.is_empty() {
                let stderr_tail: String = stderr.lines()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Command failed (no structured errors parsed). Last output:\n{}", stderr_tail)
            } else {
                parsed.summary
            };

            (parsed.errors, summary)
        } else if success {
            (vec![], "Command completed successfully".to_string())
        } else {
            (vec![], format!("Command failed with exit code {:?}", exit_code))
        };

        Ok(IdfCommandResult {
            success,
            exit_code,
            stdout,
            stderr,
            duration,
            errors,
            summary,
        })
    }

    /// Regenerate sdkconfig from defaults by removing the generated file first.
    pub async fn reconfigure_fresh(
        config: &ESP32Config,
        project_path: &Path,
        timeout_secs: u64,
    ) -> Result<IdfCommandResult, String> {
        let sdkconfig_path = project_path.join("sdkconfig");
        match tokio::fs::remove_file(&sdkconfig_path).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "Failed to remove autogenerated sdkconfig before reconfigure: {}",
                    e
                ));
            }
        }

        Self::new("reconfigure")
            .project_path(project_path)
            .timeout_secs(timeout_secs)
            .execute(config)
            .await
    }

    // setup_environment is intentionally removed — environment is now handled
    // directly inside execute(): idf.py commands source export.sh via bash,
    // and other commands inherit the process environment as-is.
}

/// Helper to infer project path from various sources
pub fn infer_project_path(explicit_path: Option<&str>, cwd: Option<&Path>) -> Option<PathBuf> {
    // 1. Use explicit path if provided
    if let Some(path) = explicit_path {
        let p = PathBuf::from(path);
        if is_esp_idf_project(&p) {
            return Some(p);
        }
    }

    // 2. Check current working directory
    if let Some(cwd) = cwd {
        if is_esp_idf_project(cwd) {
            return Some(cwd.to_path_buf());
        }
    }

    // 3. Check environment's current directory
    if let Ok(env_cwd) = std::env::current_dir() {
        if is_esp_idf_project(&env_cwd) {
            return Some(env_cwd);
        }
    }

    None
}

/// Check if a path is an ESP-IDF project
pub fn is_esp_idf_project(path: &Path) -> bool {
    path.join("CMakeLists.txt").exists() && path.join("main").exists()
}

/// Get project name from CMakeLists.txt or directory name
pub fn get_project_name(project_path: &Path) -> String {
    // Try to extract from CMakeLists.txt
    let cmake_path = project_path.join("CMakeLists.txt");
    if let Ok(content) = std::fs::read_to_string(&cmake_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("project(") {
                let start = "project(".len();
                if let Some(end) = line[start..].find(|c: char| c == ')' || c == ' ') {
                    return line[start..start + end].to_string();
                }
            }
        }
    }
    
    // Fallback to directory name
    project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_string()
}

/// List available serial ports (cross-platform)
pub fn list_serial_ports() -> Vec<String> {
    let mut ports = Vec::new();
    
    // Linux: /dev/ttyUSB*, /dev/ttyACM*
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/dev") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("ttyUSB") || name.starts_with("ttyACM") {
                        ports.push(format!("/dev/{}", name));
                    }
                }
            }
        }
    }
    
    // macOS: /dev/cu.*, /dev/tty.usbserial*, /dev/tty.usbmodem*
    #[cfg(target_os = "macos")]
    {
        if let Ok(entries) = std::fs::read_dir("/dev") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("cu.usbserial") || 
                       name.starts_with("cu.usbmodem") ||
                       name.starts_with("cu.SLAB") ||
                       name.starts_with("cu.wchusbserial") {
                        ports.push(format!("/dev/{}", name));
                    }
                }
            }
        }
    }
    
    // Windows: COM*
    #[cfg(target_os = "windows")]
    {
        // Check COM1 through COM256
        for i in 1..=256 {
            let port = format!("COM{}", i);
            // Simple existence check - try to get attributes
            let path = format!("\\\\.\\{}", port);
            if std::path::Path::new(&path).exists() {
                ports.push(port);
            }
        }
    }
    
    ports.sort();
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_esp_idf_project() {
        // This would need a mock filesystem for proper testing
        assert!(!is_esp_idf_project(Path::new("/nonexistent")));
    }

    #[test]
    fn test_list_serial_ports() {
        // Just ensure it doesn't panic
        let _ = list_serial_ports();
    }
}

