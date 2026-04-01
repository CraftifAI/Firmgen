use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardDefinition {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub schema_version: Option<String>,
    pub board_id: String,
    pub board_version: Option<String>,
    pub name: String,
    pub variant: Option<String>,
    pub description: Option<String>,

    pub chip: ChipInfo,
    pub identification: Option<IdentificationInfo>,
    pub hardware: HardwareInfo,
    pub gpio: Option<GpioInfo>,
    pub pwm: Option<PwmInfo>,
    pub adc: Option<AdcInfo>,
    pub wifi: Option<WifiInfo>,
    pub config_presets: HashMap<String, ConfigPreset>,
    pub required_components: Vec<String>,
    pub conflicting_components: Vec<String>,
    pub recommended_components: Vec<String>,
    pub kb: Option<KnowledgeBaseInfo>,
    pub documentation: Option<DocumentationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipInfo {
    #[serde(rename = "type")]
    pub chip_type: String,
    pub min_revision: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentificationInfo {
    pub chip_type: String,
    pub flash_size_range: Option<Vec<String>>,
    pub psram_present: bool,
    pub psram_size: Option<String>,
    pub flash_manufacturer_ids: Option<Vec<String>>,
    pub flash_device_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub flash: FlashInfo,
    pub psram: Option<PsramInfo>,
    pub uart_console: Option<UartConsoleInfo>,
    pub usb_jtag: Option<UsbJtagInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashInfo {
    pub size: String,
    pub size_config: String,
    pub mode: String,
    pub mode_config: String,
    pub freq: String,
    pub freq_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsramInfo {
    pub enabled: bool,
    pub size: String,
    #[serde(rename = "type")]
    pub psram_type: String,
    pub mode: String,
    pub mode_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartConsoleInfo {
    pub tx: u32,
    pub rx: u32,
    pub uart: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbJtagInfo {
    pub supported: bool,
    pub d_minus: Option<u32>,
    pub d_plus: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioInfo {
    pub led: Option<LedInfo>,
    pub button: Option<ButtonInfo>,
    pub safe_pins: Option<Vec<u32>>,
    pub restricted_pins: Option<Vec<u32>>,
    pub restricted_reasons: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedInfo {
    pub pin: u32,
    #[serde(rename = "type")]
    pub led_type: String,
    pub driver: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonInfo {
    pub pin: u32,
    #[serde(rename = "type")]
    pub button_type: String,
    pub pull: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwmInfo {
    pub defaults: PwmDefaults,
    pub alternate_pins: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwmDefaults {
    pub pin: u32,
    pub timer: u32,
    pub channel: u32,
    pub freq_hz: u32,
    pub duty_resolution: u32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdcInfo {
    pub defaults: AdcDefaults,
    pub adc1_pins: Option<Vec<u32>>,
    pub adc2_pins: Option<Vec<u32>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdcDefaults {
    pub pin: u32,
    pub channel: String,
    pub attenuation: String,
    pub width: u32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiInfo {
    pub station: Option<WifiStationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStationInfo {
    pub sdkconfig: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPreset {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub use_cases: Option<Vec<String>>,
    pub extends: Option<String>,
    pub sdkconfig: HashMap<String, String>,
    pub requires_components: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseInfo {
    pub collection: String,
    pub pinout_doc: Option<String>,
    pub board_summary_doc: Option<String>,
    pub gpio_table_doc: Option<String>,
    pub kconfig_symbols: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationInfo {
    pub datasheet: Option<String>,
    pub schematic: Option<String>,
    pub user_guide: Option<String>,
}
