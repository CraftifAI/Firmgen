use super::board_definition::BoardDefinition;
use super::config::ESP32Config;
use super::global_state;
use crate::call_validation::ContextFile;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

/// Lightweight text section extracted from board definition or config
/// for use as LLM context.
struct BoardContextSection {
    title: String,
    text: String,
}

fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let mut truncated = s.chars().take(max_chars.saturating_sub(3)).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn build_board_summary_section(board: &BoardDefinition) -> BoardContextSection {
    let mut lines = Vec::new();
    lines.push(format!(
        "Board: {}{}",
        board.name,
        board
            .variant
            .as_ref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default()
    ));
    lines.push(format!("Board ID: {}", board.board_id));
    if let Some(ver) = &board.board_version {
        lines.push(format!("Board version: {}", ver));
    }
    if let Some(desc) = &board.description {
        lines.push(format!("Description: {}", desc));
    }

    lines.push(format!("Chip type: {}", board.chip.chip_type));
    if let Some(min_rev) = board.chip.min_revision {
        lines.push(format!("Minimum supported chip revision: {}", min_rev));
    }

    if let Some(ident) = &board.identification {
        if let Some(range) = &ident.flash_size_range {
            lines.push(format!("Expected flash size range: {:?}", range));
        }
        if ident.psram_present {
            if let Some(size) = &ident.psram_size {
                lines.push(format!("PSRAM: present ({})", size));
            } else {
                lines.push("PSRAM: present".to_string());
            }
        }
    }

    let hw = &board.hardware.flash;
    lines.push(format!(
        "Flash: size={}, mode={}, freq={}",
        hw.size, hw.mode, hw.freq
    ));

    BoardContextSection {
        title: "Board summary".to_string(),
        text: lines.join("\n"),
    }
}

fn build_gpio_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    let gpio = board.gpio.as_ref()?;
    let mut lines = Vec::new();

    if let Some(led) = &gpio.led {
        lines.push(format!(
            "On-board LED: pin {}, type={}",
            led.pin, led.led_type
        ));
    }

    if let Some(button) = &gpio.button {
        lines.push(format!(
            "User/boot button: pin {}, type={}",
            button.pin, button.button_type
        ));
    }

    if let Some(safe) = &gpio.safe_pins {
        if !safe.is_empty() {
            lines.push(format!(
                "Safe GPIO pins for general IO: {:?}",
                safe
            ));
        }
    }
    if let Some(restricted) = &gpio.restricted_pins {
        if !restricted.is_empty() {
            lines.push(format!(
                "Restricted pins (avoid for general IO): {:?}",
                restricted
            ));
        }
    }
    if let Some(reasons) = &gpio.restricted_reasons {
        lines.push("Restricted pin reasons:".to_string());
        // Stable ordering for readability
        let mut keys: Vec<_> = reasons.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(reason) = reasons.get(k) {
                lines.push(format!("- {}: {}", k, reason));
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    Some(BoardContextSection {
        title: "GPIO constraints".to_string(),
        text: lines.join("\n"),
    })
}

fn build_pwm_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    let pwm = board.pwm.as_ref()?;
    let mut lines = Vec::new();
    lines.push("Default PWM configuration:".to_string());
    lines.push(format!(
        "- pin {} (timer {}, channel {}, freq {} Hz, resolution {} bits)",
        pwm.defaults.pin,
        pwm.defaults.timer,
        pwm.defaults.channel,
        pwm.defaults.freq_hz,
        pwm.defaults.duty_resolution
    ));
    if let Some(notes) = &pwm.defaults.notes {
        lines.push(format!("- notes: {}", notes));
    }
    if let Some(alts) = &pwm.alternate_pins {
        if !alts.is_empty() {
            lines.push(format!(
                "Alternate PWM-capable pins: {:?}",
                alts
            ));
        }
    }

    Some(BoardContextSection {
        title: "PWM defaults".to_string(),
        text: lines.join("\n"),
    })
}

fn build_adc_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    let adc = board.adc.as_ref()?;
    let mut lines = Vec::new();
    lines.push("ADC defaults:".to_string());
    lines.push(format!(
        "- pin {} (channel {}, attenuation {}, width {} bits)",
        adc.defaults.pin,
        adc.defaults.channel,
        adc.defaults.attenuation,
        adc.defaults.width
    ));
    if let Some(notes) = &adc.defaults.notes {
        lines.push(format!("- notes: {}", notes));
    }
    if let Some(adc1) = &adc.adc1_pins {
        lines.push(format!("ADC1 pins: {:?}", adc1));
    }
    if let Some(adc2) = &adc.adc2_pins {
        lines.push(format!("ADC2 pins: {:?}", adc2));
    }
    if let Some(notes) = &adc.notes {
        lines.push(format!("ADC notes: {}", notes));
    }

    Some(BoardContextSection {
        title: "ADC defaults".to_string(),
        text: lines.join("\n"),
    })
}

fn build_wifi_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    let wifi = board.wifi.as_ref()?;
    let station = wifi.station.as_ref()?;
    let sdkconfig = &station.sdkconfig;
    if sdkconfig.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    lines.push("Wi-Fi station defaults (sdkconfig):".to_string());
    // show a stable subset of keys
    let mut keys: Vec<_> = sdkconfig.keys().collect();
    keys.sort();
    for key in keys.iter().take(16) {
        if let Some(v) = sdkconfig.get(*key) {
            lines.push(format!("- {} = {}", key, v));
        }
    }
    if sdkconfig.len() > 16 {
        lines.push(format!(
            "... ({} more Wi-Fi related sdkconfig entries omitted)",
            sdkconfig.len() - 16
        ));
    }

    Some(BoardContextSection {
        title: "Wi-Fi defaults".to_string(),
        text: lines.join("\n"),
    })
}

fn build_presets_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    if board.config_presets.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    lines.push("Board configuration presets:".to_string());
    // stable order
    let mut presets: Vec<(&String, &super::board_definition::ConfigPreset)> =
        board.config_presets.iter().collect();
    presets.sort_by(|a, b| a.0.cmp(b.0));
    for (name, preset) in presets {
        lines.push(format!("- preset: {}", name));
        if let Some(desc) = &preset.description {
            lines.push(format!("  description: {}", desc));
        }
        if let Some(tags) = &preset.tags {
            lines.push(format!("  tags: {:?}", tags));
        }
        if let Some(uses) = &preset.use_cases {
            lines.push(format!("  use_cases: {:?}", uses));
        }
        if let Some(ext) = &preset.extends {
            lines.push(format!("  extends: {}", ext));
        }
        lines.push(format!(
            "  sdkconfig keys ({} entries)",
            preset.sdkconfig.len()
        ));
    }

    Some(BoardContextSection {
        title: "Board presets".to_string(),
        text: lines.join("\n"),
    })
}

fn build_docs_section(board: &BoardDefinition) -> Option<BoardContextSection> {
    if board.kb.is_none() && board.documentation.is_none() {
        return None;
    }

    let mut lines = Vec::new();
    if let Some(kb) = &board.kb {
        lines.push("Knowledge base:".to_string());
        lines.push(format!("  collection: {}", kb.collection));
        if let Some(pinout) = &kb.pinout_doc {
            lines.push(format!("  pinout_doc: {}", pinout));
        }
        if let Some(summary) = &kb.board_summary_doc {
            lines.push(format!("  board_summary_doc: {}", summary));
        }
        if let Some(gpio) = &kb.gpio_table_doc {
            lines.push(format!("  gpio_table_doc: {}", gpio));
        }
        if let Some(kconfig) = &kb.kconfig_symbols {
            lines.push(format!("  kconfig_symbols: {}", kconfig));
        }
    }
    if let Some(doc) = &board.documentation {
        lines.push("Documentation:".to_string());
        if let Some(datasheet) = &doc.datasheet {
            lines.push(format!("  datasheet: {}", datasheet));
        }
        if let Some(schem) = &doc.schematic {
            lines.push(format!("  schematic: {}", schem));
        }
        if let Some(ug) = &doc.user_guide {
            lines.push(format!("  user_guide: {}", ug));
        }
    }

    Some(BoardContextSection {
        title: "Board documentation & KB".to_string(),
        text: lines.join("\n"),
    })
}

fn build_env_section(cfg: &ESP32Config) -> BoardContextSection {
    let mut lines = Vec::new();
    lines.push("ESP32 environment configuration:".to_string());
    lines.push(format!("ESP-IDF path: {}", cfg.esp_idf_path));
    lines.push(format!("Projects path: {}", cfg.projects_path));
    lines.push(format!("Default target: {}", cfg.default_target));
    lines.push(format!(
        "Default serial: {} @ {} baud",
        cfg.default_serial_port, cfg.default_baud_rate
    ));
    lines.push(format!(
        "Default flash: {} {} {}",
        cfg.default_flash_size, cfg.default_flash_mode, cfg.default_flash_freq
    ));
    lines.push(format!(
        "OTA: enabled={} (scheme={})",
        cfg.ota_enabled, cfg.ota_partition_scheme
    ));
    lines.push(format!(
        "Cloud: provider={}, mqtt_broker={}",
        cfg.cloud_provider,
        if cfg.mqtt_broker.is_empty() {
            "<not set>"
        } else {
            cfg.mqtt_broker.as_str()
        }
    ));

    BoardContextSection {
        title: "ESP32 tools config".to_string(),
        text: lines.join("\n"),
    }
}

/// Try to locate the esp32_tools.yaml file on disk using the same
/// conventions as the Python API layer: REFACT_CACHE_DIR (or ~/.cache/refact),
/// with a final fallback to ./configs/esp32_tools.yaml.
fn find_esp32_tools_yaml() -> Option<PathBuf> {
    // 1) REFACT_CACHE_DIR/esp32_tools.yaml or ~/.cache/refact/esp32_tools.yaml
    let base_cache = std::env::var("REFACT_CACHE_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache").join("refact")
    });
    let cache_yaml = base_cache.join("esp32_tools.yaml");
    if cache_yaml.exists() {
        return Some(cache_yaml);
    }

    // 2) Project-local configs/esp32_tools.yaml
    if let Ok(cwd) = std::env::current_dir() {
        let local_yaml = cwd.join("configs").join("esp32_tools.yaml");
        if local_yaml.exists() {
            return Some(local_yaml);
        }
    }

    tracing::warn!("esp32_tools.yaml not found in cache dir or ./configs; ESP32 tools overview will be omitted from LLM context");
    None
}

/// Build a section that summarizes parts of the full esp32_tools.yaml
/// beyond what is represented in ESP32Config, such as tool enable flags,
/// example templates, and partition schemes.
fn build_full_yaml_section() -> Option<BoardContextSection> {
    let path = find_esp32_tools_yaml()?;
    tracing::info!("Building ESP32 tools overview from {}", path.display());

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to read esp32_tools.yaml at {}: {}", path.display(), e);
            return None;
        }
    };

    let yaml: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse esp32_tools.yaml at {}: {}", path.display(), e);
            return None;
        }
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "Source file: {}",
        path.to_string_lossy()
    ));

    // tools: enabled/disabled
    if let Some(tools_map) = yaml.get("tools").and_then(|v| v.as_mapping()) {
        let mut enabled = Vec::new();
        let mut disabled = Vec::new();
        for (k, v) in tools_map {
            let name = k.as_str().unwrap_or("").to_string();
            let enabled_flag = v
                .get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            if enabled_flag {
                enabled.push(name);
            } else {
                disabled.push(name);
            }
        }
        if !enabled.is_empty() {
            lines.push(format!("Enabled tools: {}", enabled.join(", ")));
        }
        if !disabled.is_empty() {
            lines.push(format!("Disabled/unknown tools: {}", disabled.join(", ")));
        }
    }

    // example_templates: names only, for brevity
    if let Some(templ_map) = yaml.get("example_templates").and_then(|v| v.as_mapping()) {
        let mut names: Vec<String> = templ_map
            .keys()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
        names.sort();
        if !names.is_empty() {
            lines.push(format!(
                "Example templates available (names): {}",
                names.join(", ")
            ));
        }
    }

    // partition_schemes: list scheme identifiers
    if let Some(ps_map) = yaml.get("partition_schemes").and_then(|v| v.as_mapping()) {
        let mut names: Vec<String> = ps_map
            .keys()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
        names.sort();
        if !names.is_empty() {
            lines.push(format!(
                "Partition schemes defined: {}",
                names.join(", ")
            ));
        }
    }

    if lines.len() <= 1 {
        // Only had the source file line and nothing meaningful.
        tracing::warn!(
            "esp32_tools.yaml at {} parsed successfully but no tools/templates/partition_schemes were found to summarize",
            path.display()
        );
        return None;
    }

    // Add a clear sanity marker so users can verify from the LLM side
    // that the esp32_tools overview was present in its context.
    lines.push("Sanity marker: ESP32_TOOLS_SUMMARY_OK".to_string());

    Some(BoardContextSection {
        title: "esp32_tools.yaml overview".to_string(),
        text: lines.join("\n"),
    })
}

/// Build a single concatenated context string from the board definition
/// and optionally global ESP32 config + esp32_tools.yaml overview.
/// When `include_config_and_yaml` is false, only board-specific sections are included
/// (used when config + yaml are injected separately via context_file).
pub async fn build_board_context_string(
    max_chars: usize,
    include_config_and_yaml: bool,
) -> Option<String> {
    let state = global_state::get_state().await;
    let board_id = match &state.session.board_id {
        Some(b) => b.clone(),
        None => return None,
    };
    let cache = &state.cache;

    // API URL for board definitions (same as tools use)
    let api_url = std::env::var("REFACT_ESP32_CONFIG_URL")
        .unwrap_or_else(|_| "http://localhost:8002".to_string());
    let board_url = format!("{}/v1/boards/{}", api_url, board_id);

    // Reuse the shared cache logic, including file cache + local folder fallback.
    let board_def = match cache
        .get_board_definition(&board_id, async {
            let client = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
            let response = client
                .get(&board_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch board definition: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Server returned error: {}", response.status()));
            }

            let board_def: BoardDefinition = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse board definition: {}", e))?;
            Ok(board_def)
        })
        .await
    {
        Ok(def) => def,
        Err(e) => {
            tracing::warn!("Failed to build ESP32 board context: {}", e);
            return None;
        }
    };

    drop(state);

    // Collect sections
    let mut sections = Vec::new();
    sections.push(build_board_summary_section(&board_def));
    if let Some(sec) = build_gpio_section(&board_def) {
        sections.push(sec);
    }
    if let Some(sec) = build_pwm_section(&board_def) {
        sections.push(sec);
    }
    if let Some(sec) = build_adc_section(&board_def) {
        sections.push(sec);
    }
    if let Some(sec) = build_wifi_section(&board_def) {
        sections.push(sec);
    }
    if let Some(sec) = build_presets_section(&board_def) {
        sections.push(sec);
    }
    if let Some(sec) = build_docs_section(&board_def) {
        sections.push(sec);
    }

    if include_config_and_yaml {
        if let Ok(cfg) = global_state::get_config().await {
            sections.push(build_env_section(&cfg));
        }
        if let Some(sec) = build_full_yaml_section() {
            sections.push(sec);
        }
    }

    if sections.is_empty() {
        return None;
    }

    // Simple concatenation with headings; could be extended to scoring-based selection later.
    let mut out = String::new();
    for s in sections {
        out.push_str(&format!("# {}\n{}\n\n", s.title, s.text));
    }

    Some(truncate_to_chars(&out, max_chars))
}

/// Build a context string with ONLY esp32_tools config + yaml overview.
/// Does NOT require board_id. Use when no board is selected but ESP32 tools are available.
pub async fn build_esp32_tools_only_context_string(max_chars: usize) -> Option<String> {
    let mut sections = Vec::new();

    // Environment config (from API / esp32_tools.yaml esp32_config section)
    if let Ok(cfg) = global_state::get_config().await {
        sections.push(build_env_section(&cfg));
    }

    // esp32_tools.yaml overview (tools, templates, partition_schemes)
    if let Some(sec) = build_full_yaml_section() {
        sections.push(sec);
    }

    if sections.is_empty() {
        tracing::warn!("build_esp32_tools_only_context: no config or yaml overview available");
        return None;
    }

    let mut out = String::new();
    out.push_str("# ESP32 config and esp32_tools.yaml overview\n\n");
    for s in sections {
        out.push_str(&format!("# {}\n{}\n\n", s.title, s.text));
    }
    out.push_str("Sanity marker: ESP32_TOOLS_SUMMARY_OK\n");

    Some(truncate_to_chars(&out, max_chars))
}

/// Build a ContextFile with esp32_tools config + yaml overview only (no board required).
pub async fn build_esp32_tools_only_context_file(max_chars: usize) -> Option<ContextFile> {
    let content = build_esp32_tools_only_context_string(max_chars).await?;

    Some(ContextFile {
        file_name: "esp32_tools_config_and_overview.txt".to_string(),
        file_content: content,
        line1: 0,
        line2: 0,
        symbols: Vec::new(),
        gradient_type: -1,
        usefulness: 80.0,
    })
}

/// Build a `ContextFile` that can be attached as a `context_file` chat message.
pub async fn build_board_context_file(max_chars: usize) -> Option<ContextFile> {
    let state = global_state::get_state().await;
    let board_id = match &state.session.board_id {
        Some(b) => b.clone(),
        None => return None,
    };
    drop(state);

    let content = match build_board_context_string(max_chars, false).await {
        Some(c) => c,
        None => return None,
    };

    Some(ContextFile {
        file_name: format!("board_definitions/{}.json (LLM summary)", board_id),
        file_content: content,
        line1: 0,
        line2: 0,
        symbols: Vec::new(),
        gradient_type: -1,
        usefulness: 80.0,
    })
}

