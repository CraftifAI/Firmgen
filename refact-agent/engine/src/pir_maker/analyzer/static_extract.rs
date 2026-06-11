//! Regex/heuristic extraction from ESP-IDF source and config files.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;

use serde_json::{json, Map, Value};

use super::super::schema::{AnalysisFacts, GpioFact, NetworkFact, PirComponent, TaskFact};

const DEBUG_LOG_PATH: &str = "debug-10e772.log";

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn debug_mode_log(
    run_id: &str,
    hypothesis_id: &str,
    location: &str,
    message: &str,
    data: serde_json::Value,
) {
    let payload = serde_json::json!({
        "sessionId": "10e772",
        "runId": run_id,
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": now_ms(),
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG_PATH)
    {
        let _ = writeln!(file, "{}", payload);
    }
}

lazy_static! {
    static ref RE_APP_MAIN: Regex = Regex::new(r"\bvoid\s+app_main\s*\(").unwrap();
    static ref RE_XTASK: Regex = Regex::new(
        r#"xTaskCreate\s*\(\s*([^,]+)\s*,\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*[^,]+,\s*(\d+)\s*,"#
    )
    .unwrap();
    static ref RE_GPIO_CONFIG_PIN: Regex =
        Regex::new(r"\.pin_bit_mask\s*=\s*(?:\(gpio_num_t\)\s*)?1(?:ULL|UL|U|LL|L)?\s*<<\s*(?:GPIO_NUM_)?(\d+)").unwrap();
    static ref RE_GPIO_NUM: Regex = Regex::new(r"GPIO_NUM_(\d+)").unwrap();
    static ref RE_GPIO_SET_PIN: Regex = Regex::new(r"gpio_set_level\s*\(\s*(\d+)\s*,").unwrap();
    static ref RE_WIFI: Regex = Regex::new(r"esp_wifi|esp_netif|wifi_init").unwrap();
    static ref RE_BLE: Regex =
        Regex::new(r"esp_ble|nimble|bluedroid|esp_bt_controller").unwrap();
    static ref RE_WIFI_SSID: Regex = Regex::new(r#"\.sta\.ssid\s*=\s*"([^"]+)""#).unwrap();
    static ref RE_WIFI_PASS: Regex = Regex::new(r#"\.sta\.password\s*=\s*"([^"]+)""#).unwrap();
    static ref RE_MQTT: Regex = Regex::new(r"esp_mqtt_client|mqtt_client").unwrap();
    static ref RE_HTTP: Regex = Regex::new(r"esp_http_client").unwrap();
    static ref RE_WEBSOCKET: Regex = Regex::new(r"esp_websocket_client").unwrap();
    static ref RE_I2C: Regex =
        Regex::new(r"i2c_(param_config|driver_install|master_[a-z_]+|new_master_bus|master_bus_add_device)").unwrap();
    static ref RE_SPI: Regex =
        Regex::new(r"spi_(bus_initialize|bus_add_device|device_transmit|device_polling_transmit|device_queue_trans)").unwrap();
    static ref RE_UART: Regex =
        Regex::new(r"uart_(driver_install|param_config|set_pin|write_bytes|read_bytes|wait_tx_done)").unwrap();
    static ref RE_ADC: Regex =
        Regex::new(r"adc(_oneshot|_continuous|1_|2_|_cali|_channel|_read)").unwrap();
    static ref RE_LEDC: Regex =
        Regex::new(r"ledc_(timer_config|channel_config|set_duty|update_duty|set_freq)").unwrap();
    static ref RE_OTA: Regex = Regex::new(r"esp_https_ota|esp_ota_(begin|write|end|set_boot_partition)").unwrap();
    static ref RE_STORAGE: Regex =
        Regex::new(r"nvs_flash_|spiffs|littlefs|fatfs|esp_vfs_").unwrap();
    static ref RE_EVENT_HANDLER: Regex =
        Regex::new(r"esp_event_(handler|loop)_|ESP_EVENT_ANY").unwrap();
    static ref RE_TIMER: Regex = Regex::new(r"esp_timer_|xTimer(Create|Start|Reset)").unwrap();
    static ref RE_CAMERA: Regex = Regex::new(r"esp_camera_|camera_init|esp_cam").unwrap();
    static ref RE_DISPLAY: Regex = Regex::new(r"ssd1306|st77|ili9|lvgl|esp_lcd").unwrap();
    static ref RE_LOGGER: Regex = Regex::new(r"ESP_LOG[EDWIV]").unwrap();
    static ref RE_DIAGNOSTICS: Regex =
        Regex::new(r"heap_caps_|esp_get_free_heap_size|uxTaskGetStackHighWaterMark").unwrap();
    static ref RE_MQTT_URI: Regex =
        Regex::new(r#"(?:uri|broker_url|broker)\s*=\s*"([^"]+)""#).unwrap();
    static ref RE_MQTT_TOPIC: Regex = Regex::new(r#"\.topic\s*=\s*"([^"]+)""#).unwrap();
    static ref RE_DEFINE_WIFI_SSID: Regex =
        Regex::new(r#"#define\s+WIFI_SSID\s+"([^"]+)""#).unwrap();
    static ref RE_DEFINE_WIFI_PASS: Regex =
        Regex::new(r#"#define\s+WIFI_PASSWORD\s+"([^"]+)""#).unwrap();
    static ref RE_IR_SENSOR: Regex =
        Regex::new(r"(?i)\b(ir_sensor|pir_sensor|pir\b|hc-?sr501|infrared|ir_recv)\b").unwrap();
    static ref RE_TARGET: Regex = Regex::new(r#"CONFIG_IDF_TARGET="?(\w+)"?"#).unwrap();
    static ref RE_COMPONENT: Regex =
        Regex::new(r#"idf_component_register\s*\(([^)]*)\)"#).unwrap();
    static ref RE_APP_DEFINE: Regex =
        Regex::new(r#"#define\s+(APP_[A-Z0-9_]+)\s+(.+)"#).unwrap();
    static ref RE_APP_GPIO_NUM: Regex =
        Regex::new(r"^(\d+|-1)\s*(?://.*)?$").unwrap();
    static ref RE_REQUIRES: Regex = Regex::new(r"REQUIRES\s+([^\)]+)").unwrap();
}

pub fn is_app_config_path(rel: &str) -> bool {
    let norm = rel.replace('\\', "/");
    norm.ends_with("app_config.h") || norm.ends_with("/main/app_config.h")
}

fn trim_define_rhs(raw: &str) -> String {
    raw.split("//")
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_matches('"')
        .to_string()
}

fn parse_define_pin(raw: &str) -> Option<u8> {
    let val_clean = raw.split("//").next().unwrap_or(raw).trim();
    let cap = RE_APP_GPIO_NUM.captures(val_clean)?;
    let pin = cap.get(1)?.as_str().parse::<i16>().ok()?;
    if !(0..=48).contains(&pin) {
        return None;
    }
    Some(pin as u8)
}

fn is_spi_transport_define(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.contains("_SPI_HOST")
        || upper.contains("_PIN_SCLK")
        || upper.contains("_PIN_MOSI")
        || upper.contains("_PIN_MISO")
        || upper.contains("_PIN_CS")
        || upper.contains("_PIN_DC")
        || upper.contains("_PIN_RST")
        || upper.contains("_SCLK_GPIO")
        || upper.contains("_MOSI_GPIO")
        || upper.contains("_MISO_GPIO")
        || upper.contains("_CS_GPIO")
        || upper.contains("_DC_GPIO")
        || upper.contains("_RST_GPIO")
}

fn is_grouped_peripheral_pin_define(name: &str) -> bool {
    let upper = name.to_uppercase();
    if is_spi_transport_define(&upper) {
        return true;
    }

    // Multi-pin bus/peripheral pin sets should stay attached to their component node.
    if upper.contains("I2C") && (upper.contains("SDA") || upper.contains("SCL")) {
        return true;
    }
    if upper.contains("UART")
        && (upper.contains("_TX")
            || upper.contains("_RX")
            || upper.contains("_RTS")
            || upper.contains("_CTS")
            || upper.contains("TX_PIN")
            || upper.contains("RX_PIN"))
    {
        return true;
    }
    if upper.contains("ADC")
        && (upper.contains("_PIN") || upper.contains("_GPIO") || upper.contains("_CHANNEL"))
    {
        return true;
    }
    if (upper.contains("DISPLAY")
        || upper.contains("OLED")
        || upper.contains("LCD")
        || upper.contains("SSD1306")
        || upper.contains("CAMERA"))
        && (upper.contains("_PIN") || upper.contains("_GPIO"))
    {
        return true;
    }
    false
}

fn insert_pin_binding(patch: &mut Map<String, Value>, binding_name: &str, pin: u8) {
    let entry = patch
        .entry("pin_bindings".to_string())
        .or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    if let Some(obj) = entry.as_object_mut() {
        obj.insert(binding_name.to_string(), json!(pin));
    }
}

fn insert_pin_property_with_binding(
    patch: &mut Map<String, Value>,
    property_name: &str,
    binding_name: &str,
    pin: u8,
) {
    patch.insert(property_name.to_string(), json!(pin));
    insert_pin_binding(patch, binding_name, pin);
}

fn parse_define_u32(raw: &str) -> Option<u32> {
    let trimmed = trim_define_rhs(raw);
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u32>().ok()
}

fn parse_define_f64(raw: &str) -> Option<f64> {
    trim_define_rhs(raw).parse::<f64>().ok()
}

fn parse_define_bool(raw: &str) -> Option<bool> {
    let v = trim_define_rhs(raw).to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" | "on" | "enable" | "enabled" => Some(true),
        "0" | "false" | "no" | "off" | "disable" | "disabled" => Some(false),
        _ => None,
    }
}

fn parse_define_string(raw: &str) -> Option<String> {
    let v = trim_define_rhs(raw);
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn collect_prefixes_for_suffixes(
    app_defines: &HashMap<String, String>,
    suffixes: &[&str],
) -> Vec<String> {
    let mut prefixes: Vec<String> = Vec::new();
    for key in app_defines.keys() {
        let Some(tail) = key.strip_prefix("APP_") else {
            continue;
        };
        for suffix in suffixes {
            let canonical = suffix.trim_start_matches('_');
            if tail == canonical {
                prefixes.push(String::new());
                break;
            }
            let with_prefix_sep = format!("_{canonical}");
            if let Some(prefix) = tail.strip_suffix(&with_prefix_sep) {
                prefixes.push(prefix.to_string());
                break;
            }
        }
    }
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn readable_prefix(prefix: &str) -> String {
    let readable = prefix.replace('_', " ").trim().to_string();
    if readable.is_empty() {
        "APP".to_string()
    } else {
        readable
    }
}

fn app_define_key(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        format!("APP_{suffix}")
    } else {
        format!("APP_{}_{}", prefix, suffix)
    }
}

/// Parse `main/app_config.h` — primary topology manifest written by the main coding agent.
pub fn extract_app_config(rel: &str, content: &str, facts: &mut AnalysisFacts) {
    if !is_app_config_path(rel) {
        return;
    }
    let mut app_defines: HashMap<String, String> = HashMap::new();
    let mut display_like_transport_passthrough_count = 0usize;
    let mut display_like_transport_passthrough_samples: Vec<String> = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let line_num = (line_no + 1) as u32;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let Some(cap) = RE_APP_DEFINE.captures(trimmed) else {
            continue;
        };
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let raw_val = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        let val = trim_define_rhs(raw_val);
        app_defines.insert(name.to_string(), raw_val.to_string());

        match name {
            "APP_WIFI_SSID" | "APP_WIFI_SSID_STR" => {
                let mut patch = Map::new();
                patch.insert("ssid".to_string(), json!(val));
                ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
            }
            "APP_WIFI_PASSWORD" | "APP_WIFI_PASS" => {
                let mut patch = Map::new();
                patch.insert("password".to_string(), json!(val));
                ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
            }
            "APP_MQTT_URI" | "APP_MQTT_BROKER_URL" => {
                let mut patch = Map::new();
                patch.insert("broker_url".to_string(), json!(val));
                ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
            }
            "APP_MQTT_USERNAME" => {
                let mut patch = Map::new();
                patch.insert("username".to_string(), json!(val));
                ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
            }
            "APP_MQTT_PASSWORD" => {
                let mut patch = Map::new();
                patch.insert("password".to_string(), json!(val));
                ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
            }
            "APP_MQTT_BASE_TOPIC"
            | "APP_MQTT_TOPIC"
            | "APP_MQTT_TELE_TOPIC"
            | "APP_MQTT_CMD_TOPIC" => {
                let mut patch = Map::new();
                patch.insert("topic".to_string(), json!(val));
                ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
            }
            n if is_spi_transport_define(n) => {
                // SPI transport defines are grouped into a single spi_device fact below.
            }
            n if is_grouped_peripheral_pin_define(n) => {
                // Keep multi-pin peripheral bindings attached to the component node.
            }
            n if n.contains("GPIO") || n.contains("ADC") || n.contains("PIN") => {
                let n_upper = n.to_ascii_uppercase();
                let display_like_transport = n_upper.contains("OLED")
                    || n_upper.contains("DISPLAY")
                    || n_upper.contains("LCD")
                    || n_upper.contains("SSD1306")
                    || n_upper.contains("CAMERA")
                    || n_upper.contains("SCLK")
                    || n_upper.contains("MOSI")
                    || n_upper.contains("MISO")
                    || n_upper.contains("_CS")
                    || n_upper.contains("_DC")
                    || n_upper.contains("_RST");
                if display_like_transport {
                    display_like_transport_passthrough_count += 1;
                    if display_like_transport_passthrough_samples.len() < 10 {
                        display_like_transport_passthrough_samples.push(format!(
                            "{}@{}",
                            n, line_num
                        ));
                    }
                }
                parse_app_config_gpio(rel, line_num, name, raw_val, facts);
            }
            n if n.ends_with("_INTERVAL_MS") || n.ends_with("_PERIOD_MS") => {
                if let Ok(ms) = val.parse::<f64>() {
                    ensure_app_config_task(facts, rel, ms);
                }
            }
            _ => {}
        }
    }
    // #region agent log
    debug_mode_log(
        "critical-bugs-2",
        "H9",
        "static_extract.rs:extract_app_config:grouped_define_routing",
        "tracked display-like define routing through gpio parser",
        serde_json::json!({
            "display_like_transport_passthrough_count": display_like_transport_passthrough_count,
            "display_like_transport_passthrough_samples": display_like_transport_passthrough_samples,
        }),
    );
    // #endregion
    extract_app_config_spi_devices(rel, &app_defines, facts);
    extract_app_config_i2c_devices(rel, &app_defines, facts);
    extract_app_config_uart_devices(rel, &app_defines, facts);
    extract_app_config_adc_readers(rel, &app_defines, facts);
    extract_app_config_display_outputs(rel, &app_defines, facts);
    extract_app_config_camera_capture(rel, &app_defines, facts);
    extract_app_config_http_clients(rel, &app_defines, facts);
    extract_app_config_websocket_clients(rel, &app_defines, facts);
    extract_app_config_ble(rel, &app_defines, facts);
    extract_app_config_storage(rel, &app_defines, facts);
    extract_app_config_ota(rel, &app_defines, facts);
    extract_app_config_logger(rel, &app_defines, facts);
    extract_app_config_timer_nodes(rel, &app_defines, facts);
    extract_app_config_event_handler(rel, &app_defines, facts);
}

fn parse_app_config_gpio(
    rel: &str,
    line: u32,
    define_name: &str,
    raw_val: &str,
    facts: &mut AnalysisFacts,
) {
    let val_clean = raw_val.split("//").next().unwrap_or(raw_val).trim();
    let Some(cap) = RE_APP_GPIO_NUM.captures(val_clean) else {
        return;
    };
    let pin_str = cap.get(1).map(|m| m.as_str()).unwrap_or("0");
    let pin: i16 = pin_str.parse().unwrap_or(-1);
    if pin < 0 {
        return;
    }
    let pin = pin as u8;
    let upper = define_name.to_uppercase();

    let has_servo_pwm_hint =
        upper.contains("SERVO") || upper.contains("LEDC") || upper.contains("PWM");
    let has_spi_display_hint = upper.contains("OLED")
        || upper.contains("SSD1306")
        || upper.contains("SCLK")
        || upper.contains("MOSI")
        || upper.contains("MISO")
        || upper.contains("_CS")
        || upper.contains("_DC")
        || upper.contains("_RST");

    let has_output_hint = upper.contains("OUTPUT")
        || upper.contains("_OUT")
        || upper.ends_with("_TX")
        || upper.contains("CTRL")
        || upper.contains("DRIVE")
        || upper.contains("ENABLE")
        || upper.contains("ACTUATOR")
        || upper.contains("MOTOR")
        || upper.contains("RELAY")
        || upper.contains("VALVE")
        || upper.contains("PWM")
        || upper.contains("SERVO")
        || upper.contains("LED");
    let has_input_hint = upper.contains("INPUT")
        || upper.contains("_IN")
        || upper.ends_with("_RX")
        || upper.contains("READ")
        || upper.contains("BUTTON")
        || upper.contains("SWITCH");
    let has_sensor_hint = upper.contains("SENSOR")
        || upper.contains("ADC")
        || upper.contains("MOISTURE")
        || upper.contains("TEMPERATURE")
        || upper.contains("PRESSURE")
        || upper.contains("IMU")
        || upper.contains("PIR")
        || upper.contains("IR");

    let node_type = if upper.contains("I2C") && (upper.contains("SDA") || upper.contains("SCL")) {
        "i2c_device"
    } else if has_servo_pwm_hint {
        "pwm_output"
    } else if has_sensor_hint {
        "sensor_input"
    } else if has_spi_display_hint {
        "gpio_output"
    } else if has_output_hint && !has_input_hint {
        "gpio_output"
    } else if has_input_hint && !has_output_hint {
        "gpio_input"
    } else if has_output_hint {
        "gpio_output"
    } else {
        "gpio_input"
    };
    if has_spi_display_hint {
        // #region agent log
        debug_mode_log(
            "critical-bugs-2",
            "H8",
            "static_extract.rs:parse_app_config_gpio:spi_display_hint",
            "classified app_config define with SPI/display hint",
            serde_json::json!({
                "define_name": define_name,
                "line": line,
                "node_type": node_type,
                "pin": pin,
                "raw_val": raw_val,
            }),
        );
        // #endregion
    }
    let label = define_name.trim_start_matches("APP_").replace('_', " ");

    let id_base = define_name
        .trim_start_matches("APP_")
        .to_lowercase()
        .replace("__", "_");
    let existing: Vec<String> = facts.gpio_facts.iter().map(|g| g.node_id.clone()).collect();
    let id = unique_id(&existing, &id_base);
    if facts
        .gpio_facts
        .iter()
        .any(|g| g.pin == pin && g.file == rel)
    {
        return;
    }
    facts.gpio_facts.push(GpioFact {
        node_id: id,
        node_type: node_type.to_string(),
        label: format!("{} (GPIO {}, {})", label, pin, define_name),
        pin,
        file: rel.to_string(),
        line: Some(line),
    });
}

fn ensure_app_config_task(facts: &mut AnalysisFacts, rel: &str, period_ms: f64) {
    if facts.task_facts.iter().any(|t| t.file == rel) {
        if let Some(t) = facts.task_facts.iter_mut().find(|t| t.file == rel) {
            if t.period_ms.is_none() || t.period_ms == Some(0.0) {
                t.period_ms = Some(period_ms);
            }
        }
        return;
    }
    let existing: Vec<String> = facts.task_facts.iter().map(|t| t.node_id.clone()).collect();
    facts.task_facts.push(TaskFact {
        node_id: unique_id(&existing, "app_task"),
        task_name: "app_task".to_string(),
        priority: Some(5),
        stack_size: Some(4096),
        period_ms: Some(period_ms),
        file: rel.to_string(),
        line: None,
    });
}

fn extract_app_config_spi_devices(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_SPI_HOST",
            "_PIN_SCLK",
            "_PIN_MOSI",
            "_PIN_MISO",
            "_PIN_CS",
            "_PIN_DC",
            "_PIN_RST",
            "_SCLK_GPIO",
            "_MOSI_GPIO",
            "_MISO_GPIO",
            "_CS_GPIO",
            "_DC_GPIO",
            "_RST_GPIO",
        ],
    );

    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let host = app_defines
            .get(&key("SPI_HOST"))
            .map(|v| trim_define_rhs(v))
            .filter(|v| !v.is_empty());
        let parse_spi_pin = |primary: &str, alt: &str| {
            app_defines
                .get(&key(primary))
                .or_else(|| app_defines.get(&key(alt)))
                .and_then(|v| parse_define_pin(v))
        };
        let sclk_from_gpio_alias = app_defines.get(&key("SCLK_GPIO")).is_some();
        let mosi_from_gpio_alias = app_defines.get(&key("MOSI_GPIO")).is_some();
        let miso_from_gpio_alias = app_defines.get(&key("MISO_GPIO")).is_some();
        let cs_from_gpio_alias = app_defines.get(&key("CS_GPIO")).is_some();
        let dc_from_gpio_alias = app_defines.get(&key("DC_GPIO")).is_some();
        let rst_from_gpio_alias = app_defines.get(&key("RST_GPIO")).is_some();
        let sclk = app_defines
            .get(&key("PIN_SCLK"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_SCLK", "SCLK_GPIO"));
        let mosi = app_defines
            .get(&key("PIN_MOSI"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_MOSI", "MOSI_GPIO"));
        let miso = app_defines
            .get(&key("PIN_MISO"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_MISO", "MISO_GPIO"));
        let cs = app_defines
            .get(&key("PIN_CS"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_CS", "CS_GPIO"));
        let dc = app_defines
            .get(&key("PIN_DC"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_DC", "DC_GPIO"));
        let rst = app_defines
            .get(&key("PIN_RST"))
            .and_then(|v| parse_define_pin(v))
            .or_else(|| parse_spi_pin("PIN_RST", "RST_GPIO"));

        if host.is_none()
            && sclk.is_none()
            && mosi.is_none()
            && miso.is_none()
            && cs.is_none()
            && dc.is_none()
            && rst.is_none()
        {
            continue;
        }
        // #region agent log
        debug_mode_log(
            "critical-bugs-3",
            "H2",
            "static_extract.rs:extract_app_config_spi_devices:key_variant_resolution",
            "resolved spi transport pin define variants",
            serde_json::json!({
                "prefix": prefix.as_str(),
                "sclk_from_gpio_alias": sclk_from_gpio_alias,
                "mosi_from_gpio_alias": mosi_from_gpio_alias,
                "miso_from_gpio_alias": miso_from_gpio_alias,
                "cs_from_gpio_alias": cs_from_gpio_alias,
                "dc_from_gpio_alias": dc_from_gpio_alias,
                "rst_from_gpio_alias": rst_from_gpio_alias,
                "resolved_sclk": sclk,
                "resolved_mosi": mosi,
                "resolved_miso": miso,
                "resolved_cs": cs,
                "resolved_dc": dc,
                "resolved_rst": rst,
            }),
        );
        // #endregion

        let mut patch = Map::new();
        if let Some(host) = host {
            patch.insert("host".to_string(), json!(host));
        }
        if let Some(pin) = sclk {
            insert_pin_property_with_binding(&mut patch, "sclk_pin", "sclk", pin);
        }
        if let Some(pin) = mosi {
            insert_pin_property_with_binding(&mut patch, "mosi_pin", "mosi", pin);
        }
        if let Some(pin) = miso {
            insert_pin_property_with_binding(&mut patch, "miso_pin", "miso", pin);
        }
        if let Some(pin) = cs {
            insert_pin_property_with_binding(&mut patch, "cs_pin", "cs", pin);
        }
        // Keep controller-specific pins so downstream prompts/diagrams can reference them.
        if let Some(pin) = dc {
            insert_pin_property_with_binding(&mut patch, "dc_pin", "dc", pin);
        }
        if let Some(pin) = rst {
            insert_pin_property_with_binding(&mut patch, "rst_pin", "rst", pin);
        }

        let readable = prefix.replace('_', " ");
        let label = format!("{} SPI Device", readable_prefix(&readable));
        ensure_component_node(
            facts,
            "spi_device",
            &label,
            rel,
            &format!("{}_spi", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_i2c_devices(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_I2C_SDA",
            "_I2C_SCL",
            "_I2C_ADDR",
            "_I2C_ADDRESS",
            "_I2C_CLOCK_HZ",
            "_I2C_FREQ_HZ",
            "_I2C_PORT",
        ],
    );

    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let sda = app_defines
            .get(&key("I2C_SDA"))
            .and_then(|v| parse_define_pin(v));
        let scl = app_defines
            .get(&key("I2C_SCL"))
            .and_then(|v| parse_define_pin(v));
        let address = app_defines
            .get(&key("I2C_ADDRESS"))
            .or_else(|| app_defines.get(&key("I2C_ADDR")))
            .and_then(|v| parse_define_string(v));
        let clock_hz = app_defines
            .get(&key("I2C_CLOCK_HZ"))
            .or_else(|| app_defines.get(&key("I2C_FREQ_HZ")))
            .and_then(|v| parse_define_u32(v));

        if sda.is_none() && scl.is_none() && address.is_none() && clock_hz.is_none() {
            continue;
        }

        let mut patch = Map::new();
        if let Some(pin) = sda {
            insert_pin_property_with_binding(&mut patch, "sda_pin", "sda", pin);
        }
        if let Some(pin) = scl {
            insert_pin_property_with_binding(&mut patch, "scl_pin", "scl", pin);
        }
        if let Some(address) = address {
            patch.insert("address".to_string(), json!(address));
        }
        if let Some(clock_hz) = clock_hz {
            patch.insert("clock_hz".to_string(), json!(clock_hz));
        }

        let label = format!("{} I2C Device", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "i2c_device",
            &label,
            rel,
            &format!("{}_i2c", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_uart_devices(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_UART_PORT",
            "_UART_BAUD",
            "_UART_BAUD_RATE",
            "_UART_TX",
            "_UART_TX_PIN",
            "_UART_RX",
            "_UART_RX_PIN",
        ],
    );

    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let port = app_defines
            .get(&key("UART_PORT"))
            .and_then(|v| parse_define_u32(v));
        let baud = app_defines
            .get(&key("UART_BAUD_RATE"))
            .or_else(|| app_defines.get(&key("UART_BAUD")))
            .and_then(|v| parse_define_u32(v));
        let tx = app_defines
            .get(&key("UART_TX_PIN"))
            .or_else(|| app_defines.get(&key("UART_TX")))
            .and_then(|v| parse_define_pin(v));
        let rx = app_defines
            .get(&key("UART_RX_PIN"))
            .or_else(|| app_defines.get(&key("UART_RX")))
            .and_then(|v| parse_define_pin(v));

        if port.is_none() && baud.is_none() && tx.is_none() && rx.is_none() {
            continue;
        }

        let mut patch = Map::new();
        if let Some(port) = port {
            patch.insert("port".to_string(), json!(port));
        }
        if let Some(baud) = baud {
            patch.insert("baud_rate".to_string(), json!(baud));
        }
        if let Some(pin) = tx {
            insert_pin_property_with_binding(&mut patch, "tx_pin", "tx", pin);
        }
        if let Some(pin) = rx {
            insert_pin_property_with_binding(&mut patch, "rx_pin", "rx", pin);
        }

        let label = format!("{} UART Device", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "uart_device",
            &label,
            rel,
            &format!("{}_uart", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_adc_readers(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_ADC_GPIO",
            "_ADC_PIN",
            "_ADC_CHANNEL",
            "_ADC_ATTENUATION",
            "_ADC_SAMPLE_RATE_HZ",
        ],
    );

    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let pin = app_defines
            .get(&key("ADC_GPIO"))
            .or_else(|| app_defines.get(&key("ADC_PIN")))
            .and_then(|v| parse_define_pin(v));
        let attenuation = app_defines
            .get(&key("ADC_ATTENUATION"))
            .and_then(|v| parse_define_string(v));
        let sample_rate_hz = app_defines
            .get(&key("ADC_SAMPLE_RATE_HZ"))
            .and_then(|v| parse_define_f64(v));
        let channel = app_defines
            .get(&key("ADC_CHANNEL"))
            .and_then(|v| parse_define_string(v));

        if pin.is_none() && attenuation.is_none() && sample_rate_hz.is_none() && channel.is_none() {
            continue;
        }

        let mut patch = Map::new();
        if let Some(pin) = pin {
            insert_pin_property_with_binding(&mut patch, "pin", "signal", pin);
        }
        if let Some(attenuation) = attenuation {
            patch.insert("attenuation".to_string(), json!(attenuation));
        }
        if let Some(rate) = sample_rate_hz {
            patch.insert("sample_rate_hz".to_string(), json!(rate));
        }
        if let Some(channel) = channel {
            patch.insert("channel".to_string(), json!(channel));
        }

        let label = format!("{} ADC Reader", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "adc_reader",
            &label,
            rel,
            &format!("{}_adc", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_display_outputs(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &["_DISPLAY_INTERFACE", "_DISPLAY_WIDTH", "_DISPLAY_HEIGHT"],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let interface = app_defines
            .get(&key("DISPLAY_INTERFACE"))
            .and_then(|v| parse_define_string(v));
        let width = app_defines
            .get(&key("DISPLAY_WIDTH"))
            .and_then(|v| parse_define_u32(v));
        let height = app_defines
            .get(&key("DISPLAY_HEIGHT"))
            .and_then(|v| parse_define_u32(v));
        if interface.is_none() && width.is_none() && height.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(interface) = interface {
            patch.insert("interface".to_string(), json!(interface.to_lowercase()));
        }
        if let Some(width) = width {
            patch.insert("width".to_string(), json!(width));
        }
        if let Some(height) = height {
            patch.insert("height".to_string(), json!(height));
        }
        let label = format!("{} Display", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "display_output",
            &label,
            rel,
            &format!("{}_display", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_camera_capture(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_CAMERA_INTERFACE",
            "_CAMERA_WIDTH",
            "_CAMERA_HEIGHT",
            "_CAMERA_FPS",
        ],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let interface = app_defines
            .get(&key("CAMERA_INTERFACE"))
            .and_then(|v| parse_define_string(v));
        let width = app_defines
            .get(&key("CAMERA_WIDTH"))
            .and_then(|v| parse_define_u32(v));
        let height = app_defines
            .get(&key("CAMERA_HEIGHT"))
            .and_then(|v| parse_define_u32(v));
        let fps = app_defines
            .get(&key("CAMERA_FPS"))
            .and_then(|v| parse_define_u32(v));
        if interface.is_none() && width.is_none() && height.is_none() && fps.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(interface) = interface {
            patch.insert("interface".to_string(), json!(interface.to_lowercase()));
        }
        if let Some(width) = width {
            patch.insert("frame_width".to_string(), json!(width));
        }
        if let Some(height) = height {
            patch.insert("frame_height".to_string(), json!(height));
        }
        if let Some(fps) = fps {
            patch.insert("fps".to_string(), json!(fps));
        }
        let label = format!("{} Camera", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "camera_capture",
            &label,
            rel,
            &format!("{}_camera", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_http_clients(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &["_HTTP_URL", "_REST_URL", "_HTTP_METHOD", "_HTTP_TIMEOUT_MS"],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let url = app_defines
            .get(&key("HTTP_URL"))
            .or_else(|| app_defines.get(&key("REST_URL")))
            .and_then(|v| parse_define_string(v));
        let method = app_defines
            .get(&key("HTTP_METHOD"))
            .and_then(|v| parse_define_string(v));
        let timeout = app_defines
            .get(&key("HTTP_TIMEOUT_MS"))
            .and_then(|v| parse_define_u32(v));
        if url.is_none() && method.is_none() && timeout.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(url) = url {
            patch.insert("url".to_string(), json!(url));
        }
        if let Some(method) = method {
            patch.insert("method".to_string(), json!(method.to_uppercase()));
        }
        if let Some(timeout) = timeout {
            patch.insert("timeout_ms".to_string(), json!(timeout));
        }
        let label = format!("{} HTTP Client", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "http_client",
            &label,
            rel,
            &format!("{}_http", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_websocket_clients(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_WS_URL",
            "_WEBSOCKET_URL",
            "_WS_RECONNECT_MS",
            "_WEBSOCKET_RECONNECT_MS",
        ],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let url = app_defines
            .get(&key("WEBSOCKET_URL"))
            .or_else(|| app_defines.get(&key("WS_URL")))
            .and_then(|v| parse_define_string(v));
        let reconnect = app_defines
            .get(&key("WEBSOCKET_RECONNECT_MS"))
            .or_else(|| app_defines.get(&key("WS_RECONNECT_MS")))
            .and_then(|v| parse_define_u32(v));
        if url.is_none() && reconnect.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(url) = url {
            patch.insert("url".to_string(), json!(url));
        }
        if let Some(reconnect) = reconnect {
            patch.insert("reconnect_ms".to_string(), json!(reconnect));
        }
        let label = format!("{} WebSocket Client", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "websocket_client",
            &label,
            rel,
            &format!("{}_websocket", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_ble(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &[
            "_BLE_DEVICE_NAME",
            "_BLE_ROLE",
            "_BT_DEVICE_NAME",
            "_BT_ROLE",
        ],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let device_name = app_defines
            .get(&key("BLE_DEVICE_NAME"))
            .or_else(|| app_defines.get(&key("BT_DEVICE_NAME")))
            .and_then(|v| parse_define_string(v));
        let role = app_defines
            .get(&key("BLE_ROLE"))
            .or_else(|| app_defines.get(&key("BT_ROLE")))
            .and_then(|v| parse_define_string(v));
        if device_name.is_none() && role.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(device_name) = device_name {
            patch.insert("device_name".to_string(), json!(device_name));
        }
        if let Some(role) = role {
            patch.insert("role".to_string(), json!(role.to_lowercase()));
        }
        let label = format!("{} BLE Manager", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "ble_manager",
            &label,
            rel,
            &format!("{}_ble", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_storage(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &["_STORAGE_BACKEND", "_STORAGE_NAMESPACE", "_NVS_NAMESPACE"],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let backend = app_defines
            .get(&key("STORAGE_BACKEND"))
            .and_then(|v| parse_define_string(v));
        let namespace = app_defines
            .get(&key("STORAGE_NAMESPACE"))
            .or_else(|| app_defines.get(&key("NVS_NAMESPACE")))
            .and_then(|v| parse_define_string(v));
        if backend.is_none() && namespace.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(backend) = backend {
            patch.insert("backend".to_string(), json!(backend.to_lowercase()));
        }
        if let Some(namespace) = namespace {
            patch.insert("namespace".to_string(), json!(namespace));
        }
        let label = format!("{} Storage", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "storage_manager",
            &label,
            rel,
            &format!("{}_storage", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_ota(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(
        app_defines,
        &["_OTA_URL", "_OTA_PARTITION", "_OTA_PARTITION_LABEL"],
    );
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let url = app_defines
            .get(&key("OTA_URL"))
            .and_then(|v| parse_define_string(v));
        let partition_label = app_defines
            .get(&key("OTA_PARTITION_LABEL"))
            .or_else(|| app_defines.get(&key("OTA_PARTITION")))
            .and_then(|v| parse_define_string(v));
        if url.is_none() && partition_label.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(url) = url {
            patch.insert("url".to_string(), json!(url));
        }
        if let Some(partition_label) = partition_label {
            patch.insert("partition_label".to_string(), json!(partition_label));
        }
        let label = format!("{} OTA Update", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "ota_update",
            &label,
            rel,
            &format!("{}_ota", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_logger(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(app_defines, &["_LOG_TAG", "_LOG_LEVEL"]);
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let tag = app_defines
            .get(&key("LOG_TAG"))
            .and_then(|v| parse_define_string(v));
        let level = app_defines
            .get(&key("LOG_LEVEL"))
            .and_then(|v| parse_define_string(v));
        if tag.is_none() && level.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(tag) = tag {
            patch.insert("tag".to_string(), json!(tag));
        }
        if let Some(level) = level {
            patch.insert("level".to_string(), json!(level.to_lowercase()));
        }
        let label = format!("{} Logger", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "logger",
            &label,
            rel,
            &format!("{}_logger", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_timer_nodes(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes =
        collect_prefixes_for_suffixes(app_defines, &["_TIMER_PERIOD_MS", "_TIMER_AUTO_RELOAD"]);
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let period_ms = app_defines
            .get(&key("TIMER_PERIOD_MS"))
            .and_then(|v| parse_define_f64(v));
        let auto_reload = app_defines
            .get(&key("TIMER_AUTO_RELOAD"))
            .and_then(|v| parse_define_bool(v));
        if period_ms.is_none() && auto_reload.is_none() {
            continue;
        }
        let mut patch = Map::new();
        patch.insert("timer_name".to_string(), json!(prefix.to_lowercase()));
        if let Some(period_ms) = period_ms {
            patch.insert("period_ms".to_string(), json!(period_ms));
        }
        if let Some(auto_reload) = auto_reload {
            patch.insert("auto_reload".to_string(), json!(auto_reload));
        }
        let label = format!("{} Timer", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "timer_node",
            &label,
            rel,
            &format!("{}_timer", prefix.to_lowercase()),
            patch,
        );
    }
}

fn extract_app_config_event_handler(
    rel: &str,
    app_defines: &HashMap<String, String>,
    facts: &mut AnalysisFacts,
) {
    let prefixes = collect_prefixes_for_suffixes(app_defines, &["_EVENT_BASE", "_EVENT_ID"]);
    for prefix in prefixes {
        let key = |suffix: &str| app_define_key(&prefix, suffix);
        let event_base = app_defines
            .get(&key("EVENT_BASE"))
            .and_then(|v| parse_define_string(v));
        let event_id = app_defines
            .get(&key("EVENT_ID"))
            .and_then(|v| parse_define_u32(v));
        if event_base.is_none() && event_id.is_none() {
            continue;
        }
        let mut patch = Map::new();
        if let Some(event_base) = event_base {
            patch.insert("event_base".to_string(), json!(event_base));
        }
        if let Some(event_id) = event_id {
            patch.insert("event_id".to_string(), json!(event_id));
        }
        let label = format!("{} Event Handler", readable_prefix(&prefix));
        ensure_component_node(
            facts,
            "event_handler",
            &label,
            rel,
            &format!("{}_event", prefix.to_lowercase()),
            patch,
        );
    }
}

pub fn detect_target_chip(project_root: &Path) -> Option<String> {
    for name in ["sdkconfig.defaults", "sdkconfig"] {
        let p = project_root.join(name);
        if let Ok(text) = std::fs::read_to_string(&p) {
            for line in text.lines() {
                if let Some(cap) = RE_TARGET.captures(line.trim()) {
                    return cap.get(1).map(|m| m.as_str().to_string());
                }
            }
        }
    }
    None
}

pub fn parse_cmake_components(project_root: &Path) -> Vec<PirComponent> {
    let mut components = Vec::new();
    for rel in ["CMakeLists.txt", "main/CMakeLists.txt"] {
        let path = project_root.join(rel);
        if !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains("idf_component_register") {
            continue;
        }
        let id = rel
            .trim_end_matches("/CMakeLists.txt")
            .rsplit('/')
            .next()
            .unwrap_or("main")
            .to_string();
        let mut requires = Vec::new();
        if let Some(cap) = RE_COMPONENT.captures(&text) {
            let block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(req) = RE_REQUIRES.captures(block) {
                requires = req
                    .get(1)
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
            }
        }
        components.push(PirComponent {
            id,
            path: rel.to_string(),
            requires,
            source_files: Vec::new(),
        });
    }
    components
}

pub fn extract_from_file(rel: &str, content: &str, facts: &mut AnalysisFacts) {
    if is_app_config_path(rel) {
        extract_app_config(rel, content, facts);
        return;
    }

    let norm = rel.replace('\\', "/");
    if norm == "sdkconfig" || norm == "sdkconfig.defaults" {
        extract_sdkconfig_lines(rel, content, facts);
        return;
    }

    if RE_APP_MAIN.is_match(content) {
        facts.has_app_main = true;
        facts.app_main_file = Some(rel.to_string());
    }

    if RE_WIFI.is_match(content) {
        let mut patch = Map::new();
        patch.insert("mode".to_string(), json!("station"));
        ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
    }
    if RE_MQTT.is_match(content) {
        ensure_network_node(facts, "mqtt_client", "MQTT", rel, Map::new());
    }
    if RE_BLE.is_match(content) {
        let mut patch = Map::new();
        patch.insert("role".to_string(), json!("peripheral"));
        ensure_network_node(facts, "ble_manager", "BLE", rel, patch);
    }
    if RE_HTTP.is_match(content) {
        ensure_network_node(facts, "http_client", "HTTP", rel, Map::new());
    }
    if RE_WEBSOCKET.is_match(content) {
        ensure_network_node(facts, "websocket_client", "WebSocket", rel, Map::new());
    }
    if RE_I2C.is_match(content) {
        ensure_network_node(facts, "i2c_device", "I2C Device", rel, Map::new());
    }
    if RE_SPI.is_match(content) {
        ensure_network_node(facts, "spi_device", "SPI Device", rel, Map::new());
    }
    if RE_UART.is_match(content) {
        ensure_network_node(facts, "uart_device", "UART Device", rel, Map::new());
    }
    if RE_ADC.is_match(content) {
        ensure_network_node(facts, "adc_reader", "ADC Reader", rel, Map::new());
    }
    if RE_LEDC.is_match(content) {
        ensure_network_node(facts, "pwm_output", "PWM Output", rel, Map::new());
    }
    if RE_OTA.is_match(content) {
        ensure_network_node(facts, "ota_update", "OTA Update", rel, Map::new());
    }
    if RE_STORAGE.is_match(content) {
        ensure_network_node(facts, "storage_manager", "Storage Manager", rel, Map::new());
    }
    if RE_EVENT_HANDLER.is_match(content) {
        ensure_network_node(facts, "event_handler", "Event Handler", rel, Map::new());
    }
    if RE_TIMER.is_match(content) {
        ensure_network_node(facts, "timer_node", "Timer", rel, Map::new());
    }
    if RE_CAMERA.is_match(content) {
        ensure_network_node(facts, "camera_capture", "Camera Capture", rel, Map::new());
    }
    if RE_DISPLAY.is_match(content) {
        ensure_network_node(facts, "display_output", "Display Output", rel, Map::new());
    }
    if RE_DIAGNOSTICS.is_match(content) {
        ensure_network_node(facts, "diagnostics", "Diagnostics", rel, Map::new());
    }
    if RE_LOGGER.is_match(content) {
        ensure_network_node(facts, "logger", "Logger", rel, Map::new());
    }

    for cap in RE_WIFI_SSID.captures_iter(content) {
        if let Some(ssid) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("ssid".to_string(), json!(ssid));
            ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
        }
    }
    for cap in RE_WIFI_PASS.captures_iter(content) {
        if let Some(pass) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("password".to_string(), json!(pass));
            ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
        }
    }
    for cap in RE_MQTT_URI.captures_iter(content) {
        if let Some(uri) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("broker_url".to_string(), json!(uri));
            ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
        }
    }
    for cap in RE_DEFINE_WIFI_SSID.captures_iter(content) {
        if let Some(ssid) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("ssid".to_string(), json!(ssid));
            ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
        }
    }
    for cap in RE_DEFINE_WIFI_PASS.captures_iter(content) {
        if let Some(pass) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("password".to_string(), json!(pass));
            ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
        }
    }
    for cap in RE_MQTT_TOPIC.captures_iter(content) {
        if let Some(topic) = cap.get(1).map(|m| m.as_str()) {
            let mut patch = Map::new();
            patch.insert("topic".to_string(), json!(topic));
            ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
        }
    }

    // app_config.h is the primary GPIO/peripheral pin manifest for generated projects.
    // When present, avoid noisy source-level GPIO scraping that can explode the graph
    // into one node per transport pin (SCLK/MOSI/SDA/TX/etc.).
    let app_config_pin_manifest_present =
        facts.analyzed_files.iter().any(|p| is_app_config_path(p))
            || facts.gpio_facts.iter().any(|g| is_app_config_path(&g.file))
            || facts
                .network_facts
                .iter()
                .any(|n| is_app_config_path(&n.file));

    for (line_no, line) in content.lines().enumerate() {
        let line_num = (line_no + 1) as u32;
        if let Some(cap) = RE_XTASK.captures(line) {
            let task_name = cap.get(2).map(|m| m.as_str()).unwrap_or("task").to_string();
            let stack: Option<u32> = cap.get(3).and_then(|m| m.as_str().parse().ok());
            let priority: Option<u8> = cap.get(4).and_then(|m| m.as_str().parse().ok());
            let existing: Vec<String> =
                facts.task_facts.iter().map(|t| t.node_id.clone()).collect();
            let id = unique_id(&existing, &task_name);
            facts.task_facts.push(TaskFact {
                node_id: id,
                task_name,
                priority,
                stack_size: stack,
                period_ms: None,
                file: rel.to_string(),
                line: Some(line_num),
            });
        }
        if app_config_pin_manifest_present {
            continue;
        }
        for cap in RE_GPIO_CONFIG_PIN.captures_iter(line) {
            if let Some(pin) = cap.get(1).and_then(|m| m.as_str().parse::<u8>().ok()) {
                push_gpio(facts, rel, line_num, pin, "gpio_output", "GPIO");
            }
        }
        for cap in RE_GPIO_SET_PIN.captures_iter(line) {
            if let Some(pin) = cap.get(1).and_then(|m| m.as_str().parse::<u8>().ok()) {
                push_gpio(facts, rel, line_num, pin, "gpio_output", "GPIO Output");
            }
        }
        for cap in RE_GPIO_NUM.captures_iter(line) {
            if let Some(pin) = cap.get(1).and_then(|m| m.as_str().parse::<u8>().ok()) {
                let is_ir = RE_IR_SENSOR.is_match(line) || RE_IR_SENSOR.is_match(content);
                let (nt, label) = if is_ir {
                    ("sensor_input", "IR/PIR Sensor")
                } else {
                    ("gpio_input", "GPIO Input")
                };
                push_gpio(facts, rel, line_num, pin, nt, label);
            }
        }
    }

    let is_app_main_file = facts.app_main_file.as_deref() == Some(rel);
    let has_task_for_file = facts.task_facts.iter().any(|t| t.file == rel);
    let has_inline_delay = content.contains("vTaskDelay(")
        || content.contains("vTaskDelayUntil(")
        || content.contains("HAL_Delay(")
        || content.contains("delay(")
        || content.contains("ets_delay_us(")
        || content.contains("esp_rom_delay_us(");
    let has_drive_calls = RE_GPIO_SET_PIN.is_match(content)
        || content.contains("gpio_set_level(")
        || RE_LEDC.is_match(content)
        || content.contains("led_strip_set_pixel(")
        || content.contains("led_strip_refresh(");
    let mut inserted_inline_task = false;
    if is_app_main_file && !has_task_for_file && has_inline_delay && has_drive_calls {
        let existing: Vec<String> = facts.task_facts.iter().map(|t| t.node_id.clone()).collect();
        facts.task_facts.push(TaskFact {
            node_id: unique_id(&existing, "app_main_loop_task"),
            task_name: "app_main_loop".to_string(),
            priority: Some(5),
            stack_size: Some(4096),
            period_ms: None,
            file: rel.to_string(),
            line: None,
        });
        inserted_inline_task = true;
    }
    // #region agent log
    debug_mode_log(
        "critical-bugs-3",
        "H3",
        "static_extract.rs:extract_from_file:inline_task_fallback",
        "evaluated inline app_main loop fallback task extraction",
        serde_json::json!({
            "file": rel,
            "is_app_main_file": is_app_main_file,
            "has_task_for_file": has_task_for_file,
            "has_inline_delay": has_inline_delay,
            "has_drive_calls": has_drive_calls,
            "inserted_inline_task": inserted_inline_task,
            "task_count_for_file_after": facts.task_facts.iter().filter(|t| t.file == rel).count(),
        }),
    );
    // #endregion

    if RE_IR_SENSOR.is_match(content) {
        let has_sensor = facts
            .gpio_facts
            .iter()
            .any(|g| g.node_type == "sensor_input");
        if !has_sensor {
            push_gpio(facts, rel, 0, 4, "sensor_input", "IR/PIR Sensor");
        }
    }
}

fn push_gpio(
    facts: &mut AnalysisFacts,
    rel: &str,
    line: u32,
    pin: u8,
    node_type: &str,
    label: &str,
) {
    let existing: Vec<String> = facts.gpio_facts.iter().map(|g| g.node_id.clone()).collect();
    let id = unique_id(&existing, &format!("{}_{}", node_type, pin));
    if facts
        .gpio_facts
        .iter()
        .any(|g| g.pin == pin && g.file == rel)
    {
        return;
    }
    facts.gpio_facts.push(GpioFact {
        node_id: id,
        node_type: node_type.to_string(),
        label: format!("{} (GPIO {})", label, pin),
        pin,
        file: rel.to_string(),
        line: Some(line),
    });
}

/// Parse filtered sdkconfig lines (WiFi/MQTT/target only — not full file).
fn extract_sdkconfig_lines(rel: &str, content: &str, facts: &mut AnalysisFacts) {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let val = trim_config_value(val);
            match key {
                "CONFIG_IDF_TARGET" => {
                    facts.target_chip = Some(val);
                }
                "CONFIG_ESP_WIFI_SSID" => {
                    let mut patch = Map::new();
                    patch.insert("ssid".to_string(), json!(val));
                    ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
                }
                "CONFIG_ESP_WIFI_PASSWORD" => {
                    let mut patch = Map::new();
                    patch.insert("password".to_string(), json!(val));
                    ensure_network_node(facts, "wifi_manager", "WiFi", rel, patch);
                }
                k if k.contains("MQTT") && k.contains("URI") => {
                    let mut patch = Map::new();
                    patch.insert("broker_url".to_string(), json!(val));
                    ensure_network_node(facts, "mqtt_client", "MQTT", rel, patch);
                }
                _ => {}
            }
        }
    }

    if let Some(wifi) = facts
        .network_facts
        .iter()
        .find(|n| n.node_type == "wifi_manager")
    {}
}

fn trim_config_value(raw: &str) -> String {
    let s = raw.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn ensure_component_node(
    facts: &mut AnalysisFacts,
    node_type: &str,
    label: &str,
    file: &str,
    id_base: &str,
    patch: Map<String, Value>,
) {
    if let Some(existing) = facts
        .network_facts
        .iter_mut()
        .find(|n| n.node_type == node_type && n.label == label)
    {
        if let Some(obj) = existing.properties.as_object_mut() {
            for (k, v) in patch {
                obj.insert(k, v);
            }
        } else {
            existing.properties = Value::Object(patch);
        }
        if is_app_config_path(file) {
            existing.file = file.to_string();
        } else if existing.file.is_empty()
            || existing.file == "sdkconfig"
            || existing.file.contains("sdkconfig")
        {
            existing.file = file.to_string();
        }
        return;
    }

    let existing_ids: Vec<String> = facts
        .network_facts
        .iter()
        .map(|n| n.node_id.clone())
        .collect();
    facts.network_facts.push(NetworkFact {
        node_id: unique_id(&existing_ids, id_base),
        node_type: node_type.to_string(),
        label: label.to_string(),
        file: file.to_string(),
        properties: Value::Object(patch),
    });
}

fn ensure_network_node(
    facts: &mut AnalysisFacts,
    node_type: &str,
    label: &str,
    file: &str,
    patch: Map<String, Value>,
) {
    if let Some(existing) = facts
        .network_facts
        .iter_mut()
        .find(|n| n.node_type == node_type)
    {
        if let Some(obj) = existing.properties.as_object_mut() {
            for (k, v) in patch {
                obj.insert(k, v);
            }
        } else {
            existing.properties = Value::Object(patch);
        }
        if is_app_config_path(file) {
            existing.file = file.to_string();
        } else if existing.file.is_empty()
            || existing.file == "sdkconfig"
            || existing.file.contains("sdkconfig")
        {
            existing.file = file.to_string();
        }
        return;
    }
    let existing_ids: Vec<String> = facts
        .network_facts
        .iter()
        .map(|n| n.node_id.clone())
        .collect();
    let base = match node_type {
        "wifi_manager" => "wifi",
        "mqtt_client" => "mqtt",
        _ => node_type,
    };
    facts.network_facts.push(NetworkFact {
        node_id: unique_id(&existing_ids, base),
        node_type: node_type.to_string(),
        label: label.to_string(),
        file: file.to_string(),
        properties: Value::Object(patch),
    });
}

pub fn unique_id_public(existing: &[String], base: &str) -> String {
    unique_id(existing, base)
}

fn unique_id(existing: &[String], base: &str) -> String {
    let mut id = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if id.is_empty() {
        id = "node".to_string();
    }
    let mut candidate = id.clone();
    let mut n = 0u32;
    while existing.contains(&candidate) {
        n += 1;
        candidate = format!("{}_{}", id, n);
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::{extract_app_config, extract_from_file};
    use crate::pir_maker::schema::AnalysisFacts;

    #[test]
    fn app_config_classifies_servo_and_spi_oled() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let app_config = r#"
#define APP_OLED_SPI_HOST SPI2_HOST
#define APP_SERVO_GPIO 18
#define APP_OLED_PIN_SCLK 11
#define APP_OLED_PIN_MOSI 10
#define APP_OLED_PIN_CS 9
#define APP_OLED_PIN_DC 8
#define APP_OLED_PIN_RST 13
#define APP_IR_GPIO 21
"#;

        extract_app_config("main/app_config.h", app_config, &mut facts);

        let servo = facts
            .gpio_facts
            .iter()
            .find(|g| g.label.contains("APP_SERVO_GPIO"))
            .expect("servo gpio should be extracted");
        assert_eq!(servo.node_type, "pwm_output");

        let oled_spi = facts
            .network_facts
            .iter()
            .find(|n| n.node_type == "spi_device")
            .expect("oled spi device should be extracted");
        let props = oled_spi
            .properties
            .as_object()
            .expect("spi device properties should be object");
        assert_eq!(
            props.get("host").and_then(|v| v.as_str()),
            Some("SPI2_HOST")
        );
        assert_eq!(props.get("sclk_pin").and_then(|v| v.as_u64()), Some(11));
        assert_eq!(props.get("mosi_pin").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(props.get("cs_pin").and_then(|v| v.as_u64()), Some(9));
        assert_eq!(props.get("dc_pin").and_then(|v| v.as_u64()), Some(8));
        assert_eq!(props.get("rst_pin").and_then(|v| v.as_u64()), Some(13));

        let ir = facts
            .gpio_facts
            .iter()
            .find(|g| g.label.contains("APP_IR_GPIO"))
            .expect("ir pin should be extracted");
        assert_eq!(ir.node_type, "sensor_input");
    }

    #[test]
    fn pin_bit_mask_with_macro_shift_does_not_create_gpio_one() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let content = r#"
gpio_config_t cfg = {
  .pin_bit_mask = 1ULL << APP_IR_GPIO,
};
"#;
        extract_from_file("main/main.c", content, &mut facts);
        assert!(
            facts.gpio_facts.is_empty(),
            "macro-based shift must not be misread as GPIO1"
        );
    }

    #[test]
    fn skips_gpio_num_fallback_when_app_config_gpio_exists() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        extract_app_config("main/app_config.h", "#define APP_IR_GPIO 21", &mut facts);
        extract_from_file(
            "main/main.c",
            "void f(){ gpio_num_t pin = GPIO_NUM_1; }",
            &mut facts,
        );
        assert!(
            !facts.gpio_facts.iter().any(|g| g.pin == 1),
            "fallback GPIO_NUM extraction must be suppressed when APP_* pins are present"
        );
    }

    #[test]
    fn skips_source_gpio_scraping_when_app_config_manifest_exists() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            analyzed_files: vec!["main/app_config.h".to_string(), "main/main.c".to_string()],
            ..Default::default()
        };
        let app_config = r#"
#define APP_OLED_SPI_HOST SPI2_HOST
#define APP_OLED_PIN_SCLK 11
#define APP_OLED_PIN_MOSI 10
#define APP_OLED_PIN_CS 9
#define APP_OLED_PIN_DC 8
#define APP_OLED_PIN_RST 13
"#;
        extract_app_config("main/app_config.h", app_config, &mut facts);
        extract_from_file(
            "main/main.c",
            r#"
void f(void) {
  gpio_set_level(11, 1);
  gpio_config_t cfg = { .pin_bit_mask = 1ULL << 10 };
  gpio_num_t p = GPIO_NUM_9;
}
"#,
            &mut facts,
        );
        assert!(
            facts.gpio_facts.is_empty(),
            "source GPIO scraping should stay disabled when app_config.h is present"
        );
    }

    #[test]
    fn app_config_extracts_component_matrix() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let app_config = r#"
#define APP_SENSOR_I2C_SDA 4
#define APP_SENSOR_I2C_SCL 5
#define APP_SENSOR_I2C_ADDR 0x3C
#define APP_GPS_UART_PORT 1
#define APP_GPS_UART_TX_PIN 17
#define APP_GPS_UART_RX_PIN 16
#define APP_GPS_UART_BAUD_RATE 9600
#define APP_TEMP_ADC_PIN 6
#define APP_TEMP_ADC_ATTENUATION 11db
#define APP_TEMP_ADC_SAMPLE_RATE_HZ 200
#define APP_PANEL_DISPLAY_INTERFACE spi
#define APP_PANEL_DISPLAY_WIDTH 128
#define APP_PANEL_DISPLAY_HEIGHT 64
#define APP_CAM_CAMERA_INTERFACE dvp
#define APP_CAM_CAMERA_WIDTH 640
#define APP_CAM_CAMERA_HEIGHT 480
#define APP_CAM_CAMERA_FPS 30
#define APP_HTTP_URL "https://example.com"
#define APP_HTTP_METHOD "POST"
#define APP_HTTP_TIMEOUT_MS 10000
#define APP_WS_URL "wss://example.com/ws"
#define APP_WS_RECONNECT_MS 3000
#define APP_BLE_DEVICE_NAME "gate"
#define APP_BLE_ROLE "peripheral"
#define APP_STORAGE_BACKEND "nvs"
#define APP_STORAGE_NAMESPACE "gate"
#define APP_OTA_URL "https://example.com/fw.bin"
#define APP_OTA_PARTITION_LABEL "ota_1"
#define APP_LOG_TAG "GATE"
#define APP_LOG_LEVEL "info"
#define APP_TIMER_PERIOD_MS 1000
#define APP_TIMER_AUTO_RELOAD 1
#define APP_EVENT_BASE "GATE_EVENT"
#define APP_EVENT_ID 7
"#;

        extract_app_config("main/app_config.h", app_config, &mut facts);

        let node_types: std::collections::HashSet<&str> = facts
            .network_facts
            .iter()
            .map(|n| n.node_type.as_str())
            .collect();
        for expected in [
            "i2c_device",
            "uart_device",
            "adc_reader",
            "display_output",
            "camera_capture",
            "http_client",
            "websocket_client",
            "ble_manager",
            "storage_manager",
            "ota_update",
            "logger",
            "timer_node",
            "event_handler",
        ] {
            assert!(
                node_types.contains(expected),
                "expected node_type `{expected}` to be extracted from app_config"
            );
        }
    }

    #[test]
    fn source_api_detection_extracts_component_matrix() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let content = r#"
void app_main(void) {
  i2c_param_config(0, NULL);
  spi_bus_initialize(0, NULL, 0);
  uart_driver_install(0, 256, 0, 0, NULL, 0);
  esp_http_client_init(NULL);
  esp_websocket_client_init(NULL);
  esp_ble_gap_start_advertising(NULL);
  adc_oneshot_read(NULL, 0, NULL);
  ledc_timer_config(NULL);
  nvs_flash_init();
  esp_event_handler_instance_register(0, 0, NULL, NULL, NULL);
  esp_timer_create(NULL, NULL);
  esp_camera_init(NULL);
  ssd1306_refresh(NULL);
  ESP_LOGI("TAG", "hello");
  heap_caps_get_free_size(0);
  esp_https_ota(NULL);
}
"#;

        extract_from_file("main/main.c", content, &mut facts);

        let node_types: std::collections::HashSet<&str> = facts
            .network_facts
            .iter()
            .map(|n| n.node_type.as_str())
            .collect();
        for expected in [
            "i2c_device",
            "spi_device",
            "uart_device",
            "http_client",
            "websocket_client",
            "ble_manager",
            "adc_reader",
            "pwm_output",
            "storage_manager",
            "event_handler",
            "timer_node",
            "camera_capture",
            "display_output",
            "logger",
            "diagnostics",
            "ota_update",
        ] {
            assert!(
                node_types.contains(expected),
                "expected node_type `{expected}` to be extracted from source APIs"
            );
        }
    }

    #[test]
    fn grouped_peripheral_pins_stay_on_component_nodes() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let app_config = r#"
#define APP_SENSOR_I2C_SDA 4
#define APP_SENSOR_I2C_SCL 5
#define APP_GPS_UART_TX_PIN 17
#define APP_GPS_UART_RX_PIN 16
#define APP_OLED_PIN_SCLK 11
#define APP_OLED_PIN_MOSI 10
"#;

        extract_app_config("main/app_config.h", app_config, &mut facts);
        assert!(
            facts.gpio_facts.iter().all(|g| {
                !g.label.contains("I2C")
                    && !g.label.contains("UART")
                    && !g.label.contains("SCLK")
                    && !g.label.contains("MOSI")
            }),
            "multi-pin peripheral transport lines should not become standalone GPIO nodes"
        );

        let has_pin_bindings = facts.network_facts.iter().any(|n| {
            n.properties
                .get("pin_bindings")
                .and_then(|v| v.as_object())
                .map(|o| !o.is_empty())
                .unwrap_or(false)
        });
        assert!(
            has_pin_bindings,
            "component nodes should carry pin_bindings metadata"
        );
    }

    #[test]
    fn grouped_spi_gpio_suffixes_stay_on_component_nodes() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let app_config = r#"
#define APP_OLED_SPI_HOST SPI2_HOST
#define APP_OLED_SCLK_GPIO 11
#define APP_OLED_MOSI_GPIO 10
#define APP_OLED_CS_GPIO 9
#define APP_OLED_DC_GPIO 8
#define APP_OLED_RST_GPIO 13
"#;

        extract_app_config("main/app_config.h", app_config, &mut facts);
        assert!(
            facts.gpio_facts.iter().all(|g| {
                !g.label.contains("SCLK")
                    && !g.label.contains("MOSI")
                    && !g.label.contains("CS")
                    && !g.label.contains("DC")
                    && !g.label.contains("RST")
            }),
            "SPI transport GPIO suffix defines should not become standalone GPIO facts"
        );

        let spi = facts
            .network_facts
            .iter()
            .find(|n| n.node_type == "spi_device")
            .expect("spi_device should be extracted from *_GPIO suffix pattern");
        let pin_bindings = spi
            .properties
            .get("pin_bindings")
            .and_then(|v| v.as_object())
            .expect("spi_device should include pin_bindings");
        assert_eq!(pin_bindings.get("sclk").and_then(|v| v.as_u64()), Some(11));
        assert_eq!(pin_bindings.get("mosi").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(pin_bindings.get("cs").and_then(|v| v.as_u64()), Some(9));
        assert_eq!(pin_bindings.get("dc").and_then(|v| v.as_u64()), Some(8));
        assert_eq!(pin_bindings.get("rst").and_then(|v| v.as_u64()), Some(13));
    }

    #[test]
    fn app_main_inline_loop_creates_synthetic_task_fact() {
        let mut facts = AnalysisFacts {
            project_name: "demo".to_string(),
            ..Default::default()
        };
        let content = r#"
void app_main(void) {
  while (1) {
    gpio_set_level(2, 1);
    vTaskDelay(pdMS_TO_TICKS(500));
    gpio_set_level(2, 0);
    vTaskDelay(pdMS_TO_TICKS(500));
  }
}
"#;
        extract_from_file("main/main.c", content, &mut facts);
        assert!(
            facts
                .task_facts
                .iter()
                .any(|t| t.file == "main/main.c" && t.task_name == "app_main_loop"),
            "inline app_main delay loop should produce synthetic runtime task fact"
        );
    }
}
