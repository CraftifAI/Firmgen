use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ESP32Config {
    pub esp_idf_path: String,
    pub projects_path: String,
    pub default_target: String,  // esp32, esp32s3, esp32c3, esp32c6, esp32p4
    pub default_serial_port: String,
    pub default_baud_rate: u32,
    /// Optional default baud rate for flashing.
    /// Falls back to `default_baud_rate` when not set.
    pub default_flash_baud_rate: Option<u32>,
    /// Optional default baud rate for serial monitor.
    /// Falls back to `default_baud_rate` when not set.
    pub default_monitor_baud_rate: Option<u32>,
    pub default_flash_mode: String,
    pub default_flash_freq: String,
    pub default_flash_size: String,
    
    // OTA Configuration
    pub ota_enabled: bool,
    pub ota_partition_scheme: String,
    
    // Cloud Configuration
    pub cloud_provider: String,  // aws, azure, gcp, none
    pub mqtt_broker: String,

    /// ESP-IDF version string (e.g. "5.3.0").
    /// Set this in esp32_tools.yaml to filter component registry searches by IDF compatibility.
    /// Run `idf.py --version` to find your version.
    #[serde(default)]
    pub esp_idf_version: Option<String>,
}

impl Default for ESP32Config {
    fn default() -> Self {
        Self {
            esp_idf_path: "/home/user/esp/esp-idf".to_string(),
            projects_path: "/home/user/esp/projects".to_string(),
            default_target: "esp32p4".to_string(),
            default_serial_port: "/dev/ttyUSB0".to_string(),
            default_baud_rate: 115200,
            default_flash_baud_rate: None,
            default_monitor_baud_rate: None,
            default_flash_mode: "dio".to_string(),
            default_flash_freq: "80m".to_string(),
            default_flash_size: "4MB".to_string(),
            ota_enabled: false,
            ota_partition_scheme: "default".to_string(),
            cloud_provider: "none".to_string(),
            mqtt_broker: String::new(),
            esp_idf_version: None,
        }
    }
}

impl ESP32Config {
    pub async fn load_from_api(api_url: &str) -> Result<Self, String> {
        // Try HTTP API first
        match Self::try_load_from_api(api_url).await {
            Ok(config) => Ok(config),
            Err(api_error) => {
                // Fallback to file-based loading
                let fallback_path = format!("{}/.cache/refact/esp32_tools.yaml", 
                    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
                match Self::load_from_file(&fallback_path).await {
                    Ok(config) => {
                        tracing::warn!("HTTP API failed ({}), using file fallback", api_error);
                        Ok(config)
                    },
                    Err(file_error) => Err(format!(
                        "Both HTTP API and file loading failed. API error: {}, File error: {}", 
                        api_error, file_error
                    ))
                }
            }
        }
    }

    async fn try_load_from_api(api_url: &str) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
        let response = client.get(api_url)
            .send()
            .await
            .map_err(|e| format!("Error fetching ESP32 config from API: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error status: {}", response.status()));
        }

        let config_json: serde_json::Value = response.json()
            .await
            .map_err(|e| format!("Error parsing API response JSON: {}", e))?;

        let esp32_config = config_json.get("esp32_config")
            .ok_or("Missing esp32_config section in API response")?;

        Ok(ESP32Config {
            esp_idf_path: esp32_config.get("esp_idf_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/esp/esp-idf")
                .to_string(),
            projects_path: esp32_config.get("projects_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/esp/projects")
                .to_string(),
            default_target: esp32_config.get("default_target")
                .and_then(|v| v.as_str())
                .unwrap_or("esp32p4")
                .to_string(),
            default_serial_port: esp32_config.get("default_serial_port")
                .and_then(|v| v.as_str())
                .unwrap_or("/dev/ttyUSB0")
                .to_string(),
            default_baud_rate: esp32_config.get("default_baud_rate")
                .and_then(|v| v.as_u64())
                .unwrap_or(115200) as u32,
            default_flash_baud_rate: esp32_config.get("default_flash_baud_rate")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            default_monitor_baud_rate: esp32_config.get("default_monitor_baud_rate")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            default_flash_mode: esp32_config.get("default_flash_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("dio")
                .to_string(),
            default_flash_freq: esp32_config.get("default_flash_freq")
                .and_then(|v| v.as_str())
                .unwrap_or("80m")
                .to_string(),
            default_flash_size: esp32_config.get("default_flash_size")
                .and_then(|v| v.as_str())
                .unwrap_or("4MB")
                .to_string(),
            ota_enabled: esp32_config.get("ota_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            ota_partition_scheme: esp32_config.get("ota_partition_scheme")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            cloud_provider: esp32_config.get("cloud_provider")
                .and_then(|v| v.as_str())
                .unwrap_or("none")
                .to_string(),
            mqtt_broker: esp32_config.get("mqtt_broker")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            esp_idf_version: esp32_config.get("esp_idf_version")
                .and_then(|v| v.as_str())
                .map(|s| s.trim_start_matches('v').to_string()),
        })
    }

    pub async fn load_from_file(config_path: &str) -> Result<Self, String> {
        let config_content = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| format!("Error reading ESP32 config file: {}", e))?;

        let config: serde_yaml::Value = serde_yaml::from_str(&config_content)
            .map_err(|e| format!("Error parsing ESP32 config file: {}", e))?;

        let esp32_config = config.get("esp32_config")
            .ok_or("Missing esp32_config section")?;

        Ok(ESP32Config {
            esp_idf_path: esp32_config.get("esp_idf_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/esp/esp-idf")
                .to_string(),
            projects_path: esp32_config.get("projects_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/user/esp/projects")
                .to_string(),
            default_target: esp32_config.get("default_target")
                .and_then(|v| v.as_str())
                .unwrap_or("esp32p4")
                .to_string(),
            default_serial_port: esp32_config.get("default_serial_port")
                .and_then(|v| v.as_str())
                .unwrap_or("/dev/ttyUSB0")
                .to_string(),
            default_baud_rate: esp32_config.get("default_baud_rate")
                .and_then(|v| v.as_u64())
                .unwrap_or(115200) as u32,
            default_flash_baud_rate: esp32_config.get("default_flash_baud_rate")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            default_monitor_baud_rate: esp32_config.get("default_monitor_baud_rate")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            default_flash_mode: esp32_config.get("default_flash_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("dio")
                .to_string(),
            default_flash_freq: esp32_config.get("default_flash_freq")
                .and_then(|v| v.as_str())
                .unwrap_or("80m")
                .to_string(),
            default_flash_size: esp32_config.get("default_flash_size")
                .and_then(|v| v.as_str())
                .unwrap_or("4MB")
                .to_string(),
            ota_enabled: esp32_config.get("ota_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            ota_partition_scheme: esp32_config.get("ota_partition_scheme")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            cloud_provider: esp32_config.get("cloud_provider")
                .and_then(|v| v.as_str())
                .unwrap_or("none")
                .to_string(),
            mqtt_broker: esp32_config.get("mqtt_broker")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            esp_idf_version: esp32_config.get("esp_idf_version")
                .and_then(|v| v.as_str())
                .map(|s| s.trim_start_matches('v').to_string()),
        })
    }

    pub fn validate_paths(&self) -> Result<(), String> {
        // Check if ESP-IDF path exists
        if !std::path::Path::new(&self.esp_idf_path).exists() {
            return Err(format!("ESP-IDF path does not exist: {}", self.esp_idf_path));
        }

        // Check if projects path exists (create if not)
        if !std::path::Path::new(&self.projects_path).exists() {
            std::fs::create_dir_all(&self.projects_path)
                .map_err(|e| format!("Failed to create projects directory: {}", e))?;
        }

        Ok(())
    }
}

