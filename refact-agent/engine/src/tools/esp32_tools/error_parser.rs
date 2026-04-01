use super::output_protocol::{ClassifiedError, ErrorCategory, ErrorSeverity, SuggestedPatch};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

/// Maximum number of unique errors to keep after cascade suppression
const MAX_UNIQUE_ERRORS: usize = 5;

// ─── ESP-IDF header → component mapping ────────────────────────────────────────
// Built-in table for auto-suggesting REQUIRES entries.
// Maps common ESP-IDF public headers to their owning component name.

lazy_static! {
    static ref HEADER_TO_COMPONENT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Wi-Fi & networking
        m.insert("esp_wifi.h", "esp_wifi");
        m.insert("esp_wifi_types.h", "esp_wifi");
        m.insert("esp_netif.h", "esp_netif");
        m.insert("esp_eth.h", "esp_eth");
        m.insert("esp_http_server.h", "esp_http_server");
        m.insert("esp_http_client.h", "esp_http_client");
        m.insert("esp_https_ota.h", "esp_https_ota");
        m.insert("mqtt_client.h", "mqtt");
        m.insert("esp_tls.h", "esp-tls");
        m.insert("esp_websocket_client.h", "esp_websocket_client");
        // Bluetooth
        m.insert("esp_bt.h", "bt");
        m.insert("esp_gap_ble_api.h", "bt");
        m.insert("esp_gatts_api.h", "bt");
        m.insert("esp_gattc_api.h", "bt");
        m.insert("esp_bt_main.h", "bt");
        m.insert("nimble/nimble_port.h", "bt");
        // System / core
        m.insert("nvs_flash.h", "nvs_flash");
        m.insert("nvs.h", "nvs_flash");
        m.insert("esp_event.h", "esp_event");
        m.insert("esp_log.h", "log");
        m.insert("esp_system.h", "esp_system");
        m.insert("esp_timer.h", "esp_timer");
        m.insert("esp_sleep.h", "esp_hw_support");
        m.insert("esp_pm.h", "esp_pm");
        m.insert("esp_ota_ops.h", "app_update");
        m.insert("esp_partition.h", "esp_partition");
        m.insert("esp_spiffs.h", "spiffs");
        m.insert("esp_littlefs.h", "esp_littlefs");
        m.insert("esp_vfs.h", "vfs");
        m.insert("esp_vfs_fat.h", "fatfs");
        // Drivers
        m.insert("driver/gpio.h", "driver");
        m.insert("driver/i2c.h", "driver");
        m.insert("driver/spi_master.h", "driver");
        m.insert("driver/spi_slave.h", "driver");
        m.insert("driver/uart.h", "driver");
        m.insert("driver/ledc.h", "driver");
        m.insert("driver/adc.h", "driver");
        m.insert("driver/dac.h", "driver");
        m.insert("driver/mcpwm.h", "driver");
        m.insert("driver/pcnt.h", "driver");
        m.insert("driver/rmt.h", "driver");
        m.insert("driver/i2s.h", "driver");
        m.insert("driver/timer.h", "driver");
        m.insert("driver/twai.h", "driver");
        m.insert("driver/sdmmc_host.h", "driver");
        // Provisioning
        m.insert("wifi_provisioning/manager.h", "wifi_provisioning");
        m.insert("wifi_provisioning/scheme_ble.h", "wifi_provisioning");
        m.insert("wifi_provisioning/scheme_softap.h", "wifi_provisioning");
        m.insert("network_provisioning/manager.h", "network_provisioning");
        // Console / protocols
        m.insert("esp_console.h", "console");
        m.insert("mdns.h", "mdns");
        m.insert("coap3/coap.h", "coap");
        m.insert("protocomm.h", "protocomm");
        m
    };
}

/// Look up which ESP-IDF component provides a given header.
/// Returns the component name if found.
fn lookup_component_for_header(header: &str) -> Option<&'static str> {
    // Direct lookup first
    if let Some(comp) = HEADER_TO_COMPONENT.get(header) {
        return Some(comp);
    }
    // Try stripping leading path segments for driver-style headers
    // e.g. "driver/gpio.h" might appear as just "gpio.h" in the error
    for (key, comp) in HEADER_TO_COMPONENT.iter() {
        if key.ends_with(header) {
            return Some(comp);
        }
    }
    // Heuristic: esp_xxx.h → esp_xxx component
    if header.starts_with("esp_") {
        let stem = header.strip_suffix(".h").unwrap_or(header);
        // We can't return a &'static str for a dynamically-constructed name,
        // but we can check if the stem matches a known component pattern
        // For now, we rely on the static table above
    }
    None
}

// ─── Regex patterns ────────────────────────────────────────────────────────────

lazy_static! {
    // === Generic GCC/Clang diagnostic ===
    // Matches: /path/file.c:14:10: fatal error: message
    // Matches: /path/file.c:14:10: error: message
    // Matches: /path/file.c:14:10: warning: message
    // Matches: /path/file.c:14:10: note: message
    static ref GCC_DIAGNOSTIC_RE: Regex = Regex::new(
        r"^(.+?):(\d+):(\d+):\s+(fatal error|error|warning|note):\s+(.*)$"
    ).unwrap();

    // === Specialised sub-patterns applied to the message portion ===

    // "nonexistent.h: No such file or directory"
    static ref MISSING_HEADER_MSG_RE: Regex = Regex::new(
        r"^(\S+):\s+No such file or directory$"
    ).unwrap();

    // "implicit declaration of function 'foo'"  (with optional "did you mean 'bar'?")
    static ref IMPLICIT_DECL_MSG_RE: Regex = Regex::new(
        r"implicit declaration of function\s+'(\w+)'(?:;\s+did you mean\s+'(\w+)')?"
    ).unwrap();

    // "'foo' undeclared" or "use of undeclared identifier 'foo'"
    static ref UNDECLARED_ID_MSG_RE: Regex = Regex::new(
        r"'(\w+)'\s+undeclared|use of undeclared identifier\s+'(\w+)'"
    ).unwrap();

    // "unknown type name 'foo'"
    static ref UNKNOWN_TYPE_MSG_RE: Regex = Regex::new(
        r"unknown type name\s+'(\w+)'"
    ).unwrap();

    // "redefinition of 'foo'" or "previous definition"
    static ref REDEFINITION_MSG_RE: Regex = Regex::new(
        r"redefinition of\s+'(\w+)'"
    ).unwrap();

    // "too many arguments to function 'foo'" / "too few arguments"
    static ref ARG_COUNT_MSG_RE: Regex = Regex::new(
        r"too (?:many|few) arguments to function\s+'(\w+)'"
    ).unwrap();

    // "'struct_type' has no member named 'field'"
    static ref STRUCT_MEMBER_RE: Regex = Regex::new(
        r"'(\w+)' has no member named '(\w+)'"
    ).unwrap();

    // "assignment to expression with array type" / "lvalue required"
    static ref LVALUE_RE: Regex = Regex::new(
        r"lvalue required as|assignment to expression with array type"
    ).unwrap();

    // === Linker patterns (not file:line:col format) ===

    // "undefined reference to `symbol'"
    static ref UNDEF_REF_RE: Regex = Regex::new(
        r"undefined reference to [`'](\w+)'"
    ).unwrap();

    // "multiple definition of `symbol'"
    static ref MULTI_DEF_RE: Regex = Regex::new(
        r"multiple definition of [`'](\w+)'"
    ).unwrap();

    // === ESP-IDF / CMake / build-system patterns ===

    static ref TARGET_MISMATCH_RE: Regex = Regex::new(
        r"Project sdkconfig.*was generated for target '(\w+)', but environment variable IDF_TARGET is set to '(\w+)'"
    ).unwrap();

    static ref CMAKE_COMPONENT_NOT_FOUND_RE: Regex = Regex::new(
        r"(?i)CMake\s+Error\s+at\s+([^:]+):(\d+):\s+Component\s+([^\s]+)\s+not\s+found"
    ).unwrap();

    // "Failed to resolve component 'xxx' required by component 'main': unknown name."
    // This is the actual ESP-IDF build.cmake error format
    static ref FAILED_RESOLVE_COMPONENT_RE: Regex = Regex::new(
        r"Failed to resolve component\s+'(\w+)'"
    ).unwrap();

    // "HINT: The component 'xxx' could not be found."
    // User-facing hint line from ESP-IDF build system
    static ref HINT_COMPONENT_NOT_FOUND_RE: Regex = Regex::new(
        r"HINT:\s+The component\s+'(\w+)'\s+could not be found"
    ).unwrap();

    static ref COMPONENT_NOT_FOUND_RE: Regex = Regex::new(
        r"Component\s+(\w+)\s+not\s+found"
    ).unwrap();

    static ref PARTITION_OVERFLOW_RE: Regex = Regex::new(
        r"ERROR:\s+(\S+)\.bin binary size\s+0x([0-9a-fA-F]+)\s+bytes\.\s+Smallest app partition is\s+0x([0-9a-fA-F]+)\s+bytes\."
    ).unwrap();

    static ref FLASH_TIMEOUT_RE: Regex = Regex::new(
        r"A fatal error occurred:\s+Failed to connect to ESP32:\s+Timed out waiting for packet header"
    ).unwrap();

    static ref FLASH_WRONG_CHIP_RE: Regex = Regex::new(
        r"A fatal error occurred:\s+Wrong chip type!\s+Expected (\S+), but got (\S+)"
    ).unwrap();

    static ref FLASH_PORT_BUSY_RE: Regex = Regex::new(
        r"A fatal error occurred:\s+Serial port (\S+) is busy"
    ).unwrap();

    // Phase-detection heuristics from context lines
    static ref PHASE_LINKER_RE: Regex = Regex::new(
        r"(?:ld|collect2|xtensa-\w+-elf-ld)"
    ).unwrap();

    static ref PHASE_CMAKE_RE: Regex = Regex::new(
        r"(?i)cmake\s+error"
    ).unwrap();
}

// ─── Parser ────────────────────────────────────────────────────────────────────

/// Parse build/output errors into classified errors (Cursor-style pipeline)
pub struct ErrorParser;

impl ErrorParser {
    /// Parse build output into classified, deduplicated, ranked errors.
    ///
    /// `phase_hint` is an optional hint from the caller (e.g. "build", "flash")
    /// that helps set the `phase` field when it can't be inferred from the log.
    pub fn parse_build_output(&self, output: &str, phase_hint: Option<&str>) -> ParsedBuildResult {
        let mut result = ParsedBuildResult::default();
        let default_phase = phase_hint.unwrap_or("build").to_string();

        // Track current inferred phase from context lines
        let mut current_phase = default_phase.clone();

        for line in output.lines() {
            // Update phase heuristic from context
            if PHASE_CMAKE_RE.is_match(line) {
                current_phase = "cmake".to_string();
            } else if PHASE_LINKER_RE.is_match(line) {
                current_phase = "link".to_string();
            }

            // 1. Try generic GCC/Clang diagnostic (highest priority — captures everything)
            if let Some(error) = self.try_gcc_diagnostic(line, &current_phase) {
                result.errors.push(error);
                continue;
            }

            // 2. Try standalone linker patterns (not file:line:col format)
            if let Some(error) = self.try_linker_patterns(line, &current_phase) {
                result.errors.push(error);
                continue;
            }

            // 3. Try ESP-IDF / CMake / build-system patterns
            if let Some(error) = self.try_esp_idf_patterns(line, &current_phase) {
                result.errors.push(error);
                continue;
            }

            // 4. LAST-RESORT catch-all: capture any line that looks like an error
            //    but didn't match any specific pattern. This ensures we never
            //    silently drop errors and show "Build successful, no errors".
            if let Some(error) = self.try_catch_all(line, &current_phase) {
                result.errors.push(error);
            }
        }

        // Deduplicate by message
        result.errors = self.deduplicate_errors(result.errors);

        // Sort: fatal/error first, then warning, then info
        result.errors.sort_by_key(|e| match e.severity {
            ErrorSeverity::Critical => 0,
            ErrorSeverity::Error => 1,
            ErrorSeverity::Warning => 2,
            ErrorSeverity::Info => 3,
        });

        // Cascade suppression: keep at most MAX_UNIQUE_ERRORS
        if result.errors.len() > MAX_UNIQUE_ERRORS {
            result.errors.truncate(MAX_UNIQUE_ERRORS);
        }

        // Remove pure GCC notes — they are always cascade context, never the root cause.
        // (e.g. "each undeclared identifier is reported only once for each function it appears in")
        result.errors.retain(|e| !matches!(e.severity, ErrorSeverity::Info));

        // Generate summary
        result.summary = self.generate_summary(&result.errors);

        result
    }

    // ── Generic GCC/Clang diagnostic ───────────────────────────────────────────

    fn try_gcc_diagnostic(&self, line: &str, current_phase: &str) -> Option<ClassifiedError> {
        let caps = GCC_DIAGNOSTIC_RE.captures(line)?;

        let file = caps.get(1)?.as_str().to_string();
        let line_num: usize = caps.get(2)?.as_str().parse().ok()?;
        let col: usize = caps.get(3)?.as_str().parse().ok()?;
        let severity_str = caps.get(4)?.as_str();
        let message = caps.get(5)?.as_str().to_string();

        let severity = match severity_str {
            "fatal error" => ErrorSeverity::Critical,
            "error" => ErrorSeverity::Error,
            "warning" => ErrorSeverity::Warning,
            "note" => ErrorSeverity::Info,
            _ => ErrorSeverity::Info,
        };

        // Now run specialised classifiers on the message to enrich the diagnostic
        let classified = self.classify_gcc_message(&file, line_num, col, &severity, &message, current_phase);
        Some(classified)
    }

    /// Specialised classification of a GCC/Clang diagnostic message.
    /// Returns an enriched ClassifiedError with kind, likely_cause, suggested_patch, etc.
    fn classify_gcc_message(
        &self,
        file: &str,
        line_num: usize,
        col: usize,
        severity: &ErrorSeverity,
        message: &str,
        current_phase: &str,
    ) -> ClassifiedError {
        // ── Missing header ─────────────────────────────────────────────────────
        if let Some(caps) = MISSING_HEADER_MSG_RE.captures(message) {
            let header = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let component = lookup_component_for_header(header);

            let likely_cause = if component.is_some() {
                format!(
                    "Component dependency not declared — '{}' is provided by the '{}' component, but it is not in REQUIRES",
                    header,
                    component.unwrap()
                )
            } else {
                format!(
                    "Header '{}' not found in include paths. The owning component may not be in REQUIRES, or the header path may be wrong.",
                    header
                )
            };

            let suggested_patch = component.map(|comp| SuggestedPatch {
                file: "main/CMakeLists.txt".to_string(),
                action: format!("Add '{}' to REQUIRES", comp),
                search_text: Some("REQUIRES".to_string()),
                replace_text: None, // LLM should do the actual splice
            });

            let mut fix_hints = vec![];
            if let Some(comp) = component {
                fix_hints.push(format!("Add '{}' to REQUIRES in main/CMakeLists.txt", comp));
            }
            fix_hints.push(format!("Check if '{}' path is in COMPONENT_INCLUDES", header));
            fix_hints.push("Run 'idf.py reconfigure' after CMakeLists.txt changes".to_string());

            return ClassifiedError {
                category: ErrorCategory::Build("missing_header".to_string()),
                subcategory: "missing_header".to_string(),
                severity: severity.clone(),
                message: format!("{}: No such file or directory", header),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints,
                related_docs: vec![],
                search_keywords: vec!["missing header".to_string(), "include path".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "missing_header".to_string(),
                likely_cause,
                suggested_patch,
            };
        }

        // ── Implicit declaration of function ───────────────────────────────────
        if let Some(caps) = IMPLICIT_DECL_MSG_RE.captures(message) {
            let func = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let suggested = caps.get(2).map(|m| m.as_str());

            let likely_cause = if let Some(s) = suggested {
                format!("Function '{}' is not declared; compiler suggests '{}'. Likely a typo or missing #include.", func, s)
            } else {
                format!("Function '{}' is used without a prior declaration. Add the correct #include for its header.", func)
            };

            let mut fix_hints = vec![];
            if let Some(s) = suggested {
                fix_hints.push(format!("Did you mean '{}'? Check for typos.", s));
            }
            fix_hints.push("Include the correct header or add a forward declaration.".to_string());

            return ClassifiedError {
                category: ErrorCategory::Build("implicit_declaration".to_string()),
                subcategory: "implicit_declaration".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints,
                related_docs: vec![],
                search_keywords: vec!["implicit declaration".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "implicit_declaration".to_string(),
                likely_cause,
                suggested_patch: None,
            };
        }

        // ── Undeclared identifier ──────────────────────────────────────────────
        if let Some(caps) = UNDECLARED_ID_MSG_RE.captures(message) {
            let id = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");
            return ClassifiedError {
                category: ErrorCategory::Build("undeclared_identifier".to_string()),
                subcategory: "undeclared_identifier".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints: vec![
                    format!("Declare '{}' or include the header that declares it.", id),
                    "Check for typos in the identifier name.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["undeclared".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "undeclared_identifier".to_string(),
                likely_cause: format!("Identifier '{}' is used but not declared — missing #include or declaration.", id),
                suggested_patch: None,
            };
        }

        // ── Unknown type name ──────────────────────────────────────────────────
        if let Some(caps) = UNKNOWN_TYPE_MSG_RE.captures(message) {
            let type_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            return ClassifiedError {
                category: ErrorCategory::Build("unknown_type".to_string()),
                subcategory: "unknown_type".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints: vec![
                    format!("Include the header that defines type '{}'.", type_name),
                    "Check for typos in the type name.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["unknown type".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "unknown_type".to_string(),
                likely_cause: format!("Type '{}' is not defined — likely a missing #include.", type_name),
                suggested_patch: None,
            };
        }

        // ── Redefinition ───────────────────────────────────────────────────────
        if let Some(caps) = REDEFINITION_MSG_RE.captures(message) {
            let symbol = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            return ClassifiedError {
                category: ErrorCategory::Build("redefinition".to_string()),
                subcategory: "redefinition".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints: vec![
                    format!("Check for duplicate definition of '{}'.", symbol),
                    "Add include guards (#ifndef/#define/#endif) if missing.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["redefinition".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "redefinition".to_string(),
                likely_cause: format!("Symbol '{}' is defined more than once — possibly a missing include guard or duplicate source.", symbol),
                suggested_patch: None,
            };
        }

        // ── Argument count mismatch ────────────────────────────────────────────
        if let Some(caps) = ARG_COUNT_MSG_RE.captures(message) {
            let func = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            return ClassifiedError {
                category: ErrorCategory::Build("argument_count".to_string()),
                subcategory: "argument_count".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints: vec![
                    format!("Check the function signature of '{}' and fix the call site.", func),
                ],
                related_docs: vec![],
                search_keywords: vec!["argument count".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "argument_count".to_string(),
                likely_cause: format!("Function '{}' is called with the wrong number of arguments.", func),
                suggested_patch: None,
            };
        }

        // ── Struct member not found ────────────────────────────────────────────
        // Common pattern: API version mismatch or field renamed between component versions
        if let Some(caps) = STRUCT_MEMBER_RE.captures(message) {
            let struct_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let field = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            return ClassifiedError {
                category: ErrorCategory::Build("struct_member_missing".to_string()),
                subcategory: "struct_member_missing".to_string(),
                severity: severity.clone(),
                message: message.to_string(),
                file: Some(file.to_string()),
                line: Some(line_num),
                column: Some(col),
                fix_hints: vec![
                    format!("Search the component header for the current definition of '{}'.", struct_type),
                    format!("The field '{}' may have been renamed or removed in a newer ESP-IDF/component version.", field),
                    "Check the component's CHANGELOG or migration guide for API changes.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["struct member".to_string(), "api change".to_string()],
                phase: current_phase.to_string(),
                tool: "gcc".to_string(),
                kind: "struct_member_missing".to_string(),
                likely_cause: format!(
                    "Field '{}' does not exist in '{}'. This usually means an ESP-IDF or component API change — the field was renamed or the struct was restructured in a newer version.",
                    field, struct_type
                ),
                suggested_patch: None,
            };
        }

        // ── Fallback: generic GCC error/warning ────────────────────────────────
        ClassifiedError {
            category: ErrorCategory::Build("compiler_diagnostic".to_string()),
            subcategory: "generic".to_string(),
            severity: severity.clone(),
            message: message.to_string(),
            file: Some(file.to_string()),
            line: Some(line_num),
            column: Some(col),
            fix_hints: vec![format!("Review the error at {}:{}:{}", file, line_num, col)],
            related_docs: vec![],
            search_keywords: vec![],
            phase: current_phase.to_string(),
            tool: "gcc".to_string(),
            kind: "generic_error".to_string(),
            likely_cause: String::new(),
            suggested_patch: None,
        }
    }

    // ── Standalone linker patterns ─────────────────────────────────────────────

    fn try_linker_patterns(&self, line: &str, current_phase: &str) -> Option<ClassifiedError> {
        // undefined reference to `symbol'
        if let Some(caps) = UNDEF_REF_RE.captures(line) {
            let symbol = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Build("linker_error".to_string()),
                subcategory: "undefined_reference".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Undefined reference to '{}'", symbol),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("Add the library/component containing '{}' to REQUIRES in CMakeLists.txt", symbol),
                    format!("Check if function '{}' is declared in a header and its source is compiled", symbol),
                    "Verify component dependencies in CMakeLists.txt".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["undefined reference".to_string(), "linker error".to_string()],
                phase: "link".to_string(),
                tool: "ld".to_string(),
                kind: "undefined_reference".to_string(),
                likely_cause: format!("Symbol '{}' is declared (header included) but not defined — the implementing library/component is not linked.", symbol),
                suggested_patch: Some(SuggestedPatch {
                    file: "main/CMakeLists.txt".to_string(),
                    action: format!("Add the component containing '{}' to REQUIRES", symbol),
                    search_text: Some("REQUIRES".to_string()),
                    replace_text: None,
                }),
            });
        }

        // multiple definition of `symbol'
        if let Some(caps) = MULTI_DEF_RE.captures(line) {
            let symbol = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Build("linker_error".to_string()),
                subcategory: "multiple_definition".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Multiple definition of '{}'", symbol),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("Check for duplicate definitions of '{}' across source files", symbol),
                    "Use 'static' keyword for file-local symbols.".to_string(),
                    "Use 'extern' in headers and define in exactly one .c file.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["multiple definition".to_string(), "linker error".to_string()],
                phase: "link".to_string(),
                tool: "ld".to_string(),
                kind: "multiple_definition".to_string(),
                likely_cause: format!("Symbol '{}' is defined in more than one translation unit.", symbol),
                suggested_patch: None,
            });
        }

        None
    }

    // ── ESP-IDF / CMake / flash patterns ───────────────────────────────────────

    fn try_esp_idf_patterns(&self, line: &str, current_phase: &str) -> Option<ClassifiedError> {
        // Target mismatch
        if let Some(caps) = TARGET_MISMATCH_RE.captures(line) {
            let sdkconfig_target = caps.get(1)?.as_str();
            let env_target = caps.get(2)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Config("target_mismatch".to_string()),
                subcategory: "target_mismatch".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("SDK config target mismatch: sdkconfig was generated for '{}', but IDF_TARGET is '{}'", sdkconfig_target, env_target),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("Run 'idf.py set-target {}' to regenerate sdkconfig for the correct target", env_target),
                    format!("Or set IDF_TARGET to '{}' to match the existing sdkconfig", sdkconfig_target),
                ],
                related_docs: vec![],
                search_keywords: vec!["target mismatch".to_string(), "IDF_TARGET".to_string(), "set-target".to_string()],
                phase: "cmake".to_string(),
                tool: "cmake".to_string(),
                kind: "target_mismatch".to_string(),
                likely_cause: format!("The sdkconfig was generated for '{}' but the build is targeting '{}'. Run set-target to fix.", sdkconfig_target, env_target),
                suggested_patch: None,
            });
        }

        // CMake component not found (detailed pattern)
        if let Some(caps) = CMAKE_COMPONENT_NOT_FOUND_RE.captures(line) {
            let file = caps.get(1)?.as_str().to_string();
            let line_num = caps.get(2)?.as_str().parse().ok();
            let component = caps.get(3)?.as_str();

            return Some(ClassifiedError {
                category: ErrorCategory::Build("missing_component".to_string()),
                subcategory: "cmake_component_not_found".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("CMake: Component '{}' not found", component),
                file: Some(file),
                line: line_num,
                column: None,
                fix_hints: vec![
                    format!("Install component: idf.py add-dependency {}", component),
                    format!("Check if '{}' is in ESP-IDF components/ or managed_components/", component),
                    "Verify IDF_PATH is set correctly".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["component not found".to_string(), "cmake error".to_string()],
                phase: "cmake".to_string(),
                tool: "cmake".to_string(),
                kind: "missing_component".to_string(),
                likely_cause: format!("Component '{}' is in REQUIRES but does not exist in IDF or managed_components. Install it or fix the spelling.", component),
                suggested_patch: Some(SuggestedPatch {
                    file: "main/idf_component.yml".to_string(),
                    action: format!("Add '{}' as a dependency, or install via idf.py add-dependency", component),
                    search_text: None,
                    replace_text: None,
                }),
            });
        }

        // "Failed to resolve component 'xxx'" — ESP-IDF build.cmake format
        if let Some(caps) = FAILED_RESOLVE_COMPONENT_RE.captures(line) {
            let component = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Build("missing_component".to_string()),
                subcategory: "failed_resolve_component".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Failed to resolve component '{}': unknown name", component),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("The component '{}' does not exist. Check spelling in CMakeLists.txt REQUIRES.", component),
                    format!("Search the ESP Component Registry: idf.py add-dependency {}", component),
                    format!("Check if '{}' is in ESP-IDF components/ or managed_components/", component),
                ],
                related_docs: vec![],
                search_keywords: vec!["failed to resolve component".to_string(), "unknown name".to_string()],
                phase: "cmake".to_string(),
                tool: "cmake".to_string(),
                kind: "missing_component".to_string(),
                likely_cause: format!(
                    "Component '{}' is listed in REQUIRES but does not exist. The name may be misspelled, or the component needs to be installed via idf.py add-dependency.",
                    component
                ),
                suggested_patch: Some(SuggestedPatch {
                    file: "main/CMakeLists.txt".to_string(),
                    action: format!("Fix the component name '{}' in REQUIRES, or install it via idf.py add-dependency", component),
                    search_text: Some(component.to_string()),
                    replace_text: None,
                }),
            });
        }

        // "HINT: The component 'xxx' could not be found." — user-friendly ESP-IDF hint
        if let Some(caps) = HINT_COMPONENT_NOT_FOUND_RE.captures(line) {
            let component = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Build("missing_component".to_string()),
                subcategory: "hint_component_not_found".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Component '{}' could not be found", component),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("Search the ESP Component Registry: idf.py add-dependency {}", component),
                    "Component may have been moved, renamed, or is not supported by this target.".to_string(),
                    "Check the ESP-IDF migration guide for moved components.".to_string(),
                ],
                related_docs: vec!["https://components.espressif.com".to_string()],
                search_keywords: vec!["component not found".to_string()],
                phase: "cmake".to_string(),
                tool: "cmake".to_string(),
                kind: "missing_component".to_string(),
                likely_cause: format!(
                    "Component '{}' could not be found. It may have been misspelled, moved to the IDF component manager, or is not supported by the selected target.",
                    component
                ),
                suggested_patch: Some(SuggestedPatch {
                    file: "main/CMakeLists.txt".to_string(),
                    action: format!("Fix the component name '{}' in REQUIRES, or install it", component),
                    search_text: Some(component.to_string()),
                    replace_text: None,
                }),
            });
        }

        // Component not found (fallback simpler pattern)
        if let Some(caps) = COMPONENT_NOT_FOUND_RE.captures(line) {
            let component = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Build("missing_component".to_string()),
                subcategory: "missing_component".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Component not found: {}", component),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    format!("Install component: idf.py add-dependency {}", component),
                    format!("Check if '{}' is in ESP-IDF components/", component),
                    "Verify IDF_PATH is set correctly".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["component not found".to_string()],
                phase: "cmake".to_string(),
                tool: "cmake".to_string(),
                kind: "missing_component".to_string(),
                likely_cause: format!("Component '{}' is referenced but not installed.", component),
                suggested_patch: None,
            });
        }

        // Partition overflow
        if let Some(caps) = PARTITION_OVERFLOW_RE.captures(line) {
            let _binary = caps.get(1)?.as_str();
            let bin_size_hex = caps.get(2)?.as_str();
            let part_size_hex = caps.get(3)?.as_str();
            let bin_size = u64::from_str_radix(bin_size_hex, 16).ok();
            let part_size = u64::from_str_radix(part_size_hex, 16).ok();

            return Some(ClassifiedError {
                category: ErrorCategory::Build("partition_overflow".to_string()),
                subcategory: "partition_overflow".to_string(),
                severity: ErrorSeverity::Error,
                message: match (bin_size, part_size) {
                    (Some(b), Some(p)) => format!("Binary size (0x{:x}) exceeds smallest app partition (0x{:x})", b, p),
                    _ => "Application binary does not fit into configured app partition".to_string(),
                },
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    "Increase application partition size in partitions.csv".to_string(),
                    "Enable size optimisation (CONFIG_COMPILER_OPTIMIZATION_SIZE)".to_string(),
                    "Review flash size configuration and selected partition scheme".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["partition overflow".to_string(), "binary size".to_string()],
                phase: current_phase.to_string(),
                tool: "esptool".to_string(),
                kind: "partition_overflow".to_string(),
                likely_cause: "The compiled binary is larger than the app partition. Increase partition size or enable compiler size optimisation.".to_string(),
                suggested_patch: Some(SuggestedPatch {
                    file: "partitions.csv".to_string(),
                    action: "Increase the factory/ota app partition size".to_string(),
                    search_text: None,
                    replace_text: None,
                }),
            });
        }

        // Flash connection timeout
        if FLASH_TIMEOUT_RE.is_match(line) {
            return Some(ClassifiedError {
                category: ErrorCategory::Flash("connection_timeout".to_string()),
                subcategory: "connection_timeout".to_string(),
                severity: ErrorSeverity::Error,
                message: "Failed to connect to ESP32: Timed out waiting for packet header".to_string(),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    "Hold BOOT button while powering or resetting the board.".to_string(),
                    "Check USB cable and port; try a different cable/port.".to_string(),
                    "Try a lower baud rate (e.g. 115200).".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["flash timeout".to_string(), "failed to connect".to_string()],
                phase: "flash".to_string(),
                tool: "esptool".to_string(),
                kind: "connection_timeout".to_string(),
                likely_cause: "The board did not respond during flash. It may not be in download mode.".to_string(),
                suggested_patch: None,
            });
        }

        // Flash wrong chip type
        if let Some(caps) = FLASH_WRONG_CHIP_RE.captures(line) {
            let expected = caps.get(1)?.as_str();
            let got = caps.get(2)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Flash("wrong_chip".to_string()),
                subcategory: "wrong_chip".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Wrong chip type: expected {}, but got {}", expected, got),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    "Check that IDF_TARGET matches the connected chip.".to_string(),
                    "Ensure you are using the correct firmware image for this chip.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["wrong chip type".to_string()],
                phase: "flash".to_string(),
                tool: "esptool".to_string(),
                kind: "wrong_chip".to_string(),
                likely_cause: format!("Firmware was built for {} but the connected chip is {}.", expected, got),
                suggested_patch: None,
            });
        }

        // Flash port busy
        if let Some(caps) = FLASH_PORT_BUSY_RE.captures(line) {
            let port = caps.get(1)?.as_str();
            return Some(ClassifiedError {
                category: ErrorCategory::Flash("port_busy".to_string()),
                subcategory: "port_busy".to_string(),
                severity: ErrorSeverity::Error,
                message: format!("Serial port {} is busy (may be in use by another program)", port),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    "Close any serial terminal or monitor using this port.".to_string(),
                    "Ensure no other idf.py monitor/flash process is running.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec!["serial port busy".to_string(), "flash port busy".to_string()],
                phase: "flash".to_string(),
                tool: "esptool".to_string(),
                kind: "port_busy".to_string(),
                likely_cause: format!("Serial port {} is locked by another process.", port),
                suggested_patch: None,
            });
        }

        None
    }

    // ── Last-resort catch-all ──────────────────────────────────────────────

    /// Catches lines that look like errors but didn't match any specific pattern.
    /// This ensures we never silently drop build errors.
    fn try_catch_all(&self, line: &str, current_phase: &str) -> Option<ClassifiedError> {
        lazy_static! {
            // Matches lines containing common error indicators
            // Excludes lines that are just context/noise (e.g., "-- Configuring incomplete, errors occurred!")
            static ref CATCH_ALL_ERROR_RE: Regex = Regex::new(
                r"(?i)^(?:.*(?:CMake Error|FAILED:|fatal error|Error:|error:|\berror\b.*failed|ninja: error).*)"
            ).unwrap();

            // Lines to exclude from catch-all (noise that contains "error" but isn't actionable)
            static ref CATCH_ALL_EXCLUDE_RE: Regex = Regex::new(
                r"(?i)(?:errors occurred|error log|error output|CMakeOutput\.log|stderr_output|See also)"
            ).unwrap();
        }

        if CATCH_ALL_ERROR_RE.is_match(line) && !CATCH_ALL_EXCLUDE_RE.is_match(line) {
            let trimmed = line.trim();
            // Skip empty or very short lines
            if trimmed.len() < 10 {
                return None;
            }
            return Some(ClassifiedError {
                category: ErrorCategory::Build("unrecognized_error".to_string()),
                subcategory: "unrecognized".to_string(),
                severity: ErrorSeverity::Error,
                message: trimmed.to_string(),
                file: None,
                line: None,
                column: None,
                fix_hints: vec![
                    "This error was not recognized by the parser — read the message carefully.".to_string(),
                ],
                related_docs: vec![],
                search_keywords: vec![],
                phase: current_phase.to_string(),
                tool: "unknown".to_string(),
                kind: "unrecognized_error".to_string(),
                likely_cause: String::new(),
                suggested_patch: None,
            });
        }

        None
    }

    // ── Deduplication ──────────────────────────────────────────────────────────

    fn deduplicate_errors(&self, errors: Vec<ClassifiedError>) -> Vec<ClassifiedError> {
        let mut seen = std::collections::HashSet::new();
        errors.into_iter()
            .filter(|e| seen.insert(e.message.clone()))
            .collect()
    }

    // ── Summary generation ─────────────────────────────────────────────────────

    fn generate_summary(&self, errors: &[ClassifiedError]) -> String {
        if errors.is_empty() {
            return "Build successful, no errors".to_string();
        }

        let error_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Error | ErrorSeverity::Critical)).count();
        let warning_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Warning)).count();

        let mut parts = vec![];
        if error_count > 0 {
            parts.push(format!("{} error(s)", error_count));
        }
        if warning_count > 0 {
            parts.push(format!("{} warning(s)", warning_count));
        }

        let first_error = errors.iter()
            .find(|e| matches!(e.severity, ErrorSeverity::Error | ErrorSeverity::Critical));

        if let Some(first) = first_error {
            format!("{} — first: {} [{}]", parts.join(", "), first.message, first.kind)
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Default)]
pub struct ParsedBuildResult {
    pub errors: Vec<ClassifiedError>,
    pub warnings: Vec<ClassifiedError>,
    pub summary: String,
    pub build_time: Option<std::time::Duration>,
    pub output_size: Option<usize>,
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> ErrorParser {
        ErrorParser
    }

    #[test]
    fn test_missing_header_with_component_mapping() {
        let log = r#"/home/user/project/main/wifi_station_main.c:5:10: fatal error: esp_wifi.h: No such file or directory"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "missing_header");
        assert_eq!(err.file.as_deref(), Some("/home/user/project/main/wifi_station_main.c"));
        assert_eq!(err.line, Some(5));
        assert_eq!(err.column, Some(10));
        assert!(err.likely_cause.contains("esp_wifi"));
        assert!(err.suggested_patch.is_some());
        let patch = err.suggested_patch.as_ref().unwrap();
        assert_eq!(patch.file, "main/CMakeLists.txt");
        assert!(patch.action.contains("esp_wifi"));
    }

    #[test]
    fn test_missing_header_unknown_component() {
        let log = r#"/home/user/project/main/app.c:3:10: fatal error: my_custom_lib.h: No such file or directory"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "missing_header");
        assert!(err.suggested_patch.is_none()); // unknown header → no auto-patch
        assert!(err.likely_cause.contains("my_custom_lib.h"));
    }

    #[test]
    fn test_undefined_reference() {
        let log = r#"main.c.obj: undefined reference to `esp_wifi_init'"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "undefined_reference");
        assert_eq!(err.phase, "link");
        assert!(err.likely_cause.contains("esp_wifi_init"));
    }

    #[test]
    fn test_multiple_definition() {
        let log = r#"multiple definition of `app_main'"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "multiple_definition");
        assert_eq!(err.phase, "link");
    }

    #[test]
    fn test_implicit_declaration() {
        let log = r#"/home/user/main/app.c:42:5: error: implicit declaration of function 'esp_wifi_start'; did you mean 'esp_wifi_stop'?"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "implicit_declaration");
        assert!(err.likely_cause.contains("esp_wifi_start"));
        assert!(err.likely_cause.contains("esp_wifi_stop"));
    }

    #[test]
    fn test_implicit_declaration_no_suggestion() {
        let log = r#"/home/user/main/app.c:10:5: error: implicit declaration of function 'my_func'"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "implicit_declaration");
        assert!(err.likely_cause.contains("my_func"));
    }

    #[test]
    fn test_target_mismatch() {
        let log = "Project sdkconfig '/home/user/project/sdkconfig' was generated for target 'esp32', but environment variable IDF_TARGET is set to 'esp32s3'";
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "target_mismatch");
        assert_eq!(err.phase, "cmake");
        assert!(err.likely_cause.contains("esp32"));
        assert!(err.likely_cause.contains("esp32s3"));
    }

    #[test]
    fn test_cmake_component_not_found() {
        let log = r#"CMake Error at /opt/esp/idf/tools/cmake/component.cmake:245: Component nonexistent not found"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "missing_component");
        assert!(err.suggested_patch.is_some());
    }

    #[test]
    fn test_partition_overflow() {
        let log = "ERROR: blink.bin binary size 0x1e640 bytes. Smallest app partition is 0x100000 bytes.";
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "partition_overflow");
        assert!(err.suggested_patch.is_some());
    }

    #[test]
    fn test_cascade_suppression() {
        // Generate 10 distinct errors from the same file
        let mut log_lines = Vec::new();
        for i in 1..=10 {
            log_lines.push(format!("/home/user/main/app.c:{}:1: error: some_error_{}", i, i));
        }
        let log = log_lines.join("\n");
        let result = parser().parse_build_output(&log, Some("build"));
        assert!(result.errors.len() <= MAX_UNIQUE_ERRORS,
            "Expected at most {} errors, got {}", MAX_UNIQUE_ERRORS, result.errors.len());
    }

    #[test]
    fn test_generic_gcc_error() {
        let log = r#"/home/user/main/app.c:20:5: error: expected ';' before '}' token"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "generic_error");
        assert_eq!(err.file.as_deref(), Some("/home/user/main/app.c"));
        assert_eq!(err.line, Some(20));
        assert_eq!(err.column, Some(5));
    }

    #[test]
    fn test_warning_captured() {
        let log = r#"/home/user/main/app.c:15:10: warning: unused variable 'x' [-Wunused-variable]"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert!(matches!(err.severity, ErrorSeverity::Warning));
    }

    #[test]
    fn test_empty_build_output() {
        let result = parser().parse_build_output("", Some("build"));
        assert!(result.errors.is_empty());
        assert_eq!(result.summary, "Build successful, no errors");
    }

    #[test]
    fn test_clean_build_output() {
        let log = "[100%] Built target app\n[100%] Built target bootloader";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(result.errors.is_empty());
        assert_eq!(result.summary, "Build successful, no errors");
    }

    #[test]
    fn test_deduplication() {
        let log = "/home/user/main/app.c:5:10: error: something wrong\n\
                   /home/user/main/app.c:5:10: error: something wrong";
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_severity_ordering() {
        let log = "/home/user/main/app.c:15:10: warning: unused variable\n\
                   /home/user/main/app.c:5:10: fatal error: esp_wifi.h: No such file or directory\n\
                   /home/user/main/app.c:20:5: error: expected ';'";
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 3);
        // Fatal should be first (Critical)
        assert!(matches!(result.errors[0].severity, ErrorSeverity::Critical));
        // Error second
        assert!(matches!(result.errors[1].severity, ErrorSeverity::Error));
        // Warning last
        assert!(matches!(result.errors[2].severity, ErrorSeverity::Warning));
    }

    #[test]
    fn test_header_to_component_lookup() {
        assert_eq!(lookup_component_for_header("esp_wifi.h"), Some("esp_wifi"));
        assert_eq!(lookup_component_for_header("nvs_flash.h"), Some("nvs_flash"));
        assert_eq!(lookup_component_for_header("esp_http_server.h"), Some("esp_http_server"));
        assert_eq!(lookup_component_for_header("nonexistent_header.h"), None);
    }

    #[test]
    fn test_flash_timeout() {
        let log = "A fatal error occurred: Failed to connect to ESP32: Timed out waiting for packet header";
        let result = parser().parse_build_output(log, Some("flash"));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, "connection_timeout");
        assert_eq!(result.errors[0].phase, "flash");
    }

    #[test]
    fn test_flash_wrong_chip() {
        let log = "A fatal error occurred: Wrong chip type! Expected ESP32-S3, but got ESP32";
        let result = parser().parse_build_output(log, Some("flash"));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, "wrong_chip");
    }

    #[test]
    fn test_flash_port_busy() {
        let log = "A fatal error occurred: Serial port /dev/ttyUSB0 is busy";
        let result = parser().parse_build_output(log, Some("flash"));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, "port_busy");
    }

    #[test]
    fn test_unknown_type() {
        let log = r#"/home/user/main/app.c:8:1: error: unknown type name 'wifi_config_t'"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, "unknown_type");
        assert!(result.errors[0].likely_cause.contains("wifi_config_t"));
    }

    #[test]
    fn test_summary_format() {
        let log = "/home/user/main/app.c:5:10: fatal error: esp_wifi.h: No such file or directory\n\
                   /home/user/main/app.c:15:10: warning: unused variable";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(result.summary.contains("error"));
        assert!(result.summary.contains("warning"));
        assert!(result.summary.contains("missing_header"));
    }

    #[test]
    fn test_phase_detection_from_context() {
        let log = "some cmake error output\n\
                   CMake Error at something\n\
                   /home/user/main/app.c:5:10: error: something wrong";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(!result.errors.is_empty());
        // The error after "CMake Error" context should have cmake phase
        let err = &result.errors[0];
        assert_eq!(err.phase, "cmake");
    }

    #[test]
    fn test_struct_member_missing() {
        let log = r#"/home/user/project/main/blink.c:40:10: error: 'led_strip_config_t' has no member named 'led_pixel_format'"#;
        let result = parser().parse_build_output(log, Some("build"));
        assert_eq!(result.errors.len(), 1);
        let err = &result.errors[0];
        assert_eq!(err.kind, "struct_member_missing");
        assert!(err.likely_cause.contains("led_pixel_format"));
        assert!(err.likely_cause.contains("led_strip_config_t"));
        assert!(err.likely_cause.contains("API change"));
    }

    #[test]
    fn test_notes_are_suppressed() {
        // GCC notes should not appear in the error output
        let log = "/home/user/main/app.c:40:29: note: each undeclared identifier is reported only once for each function it appears in";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(result.errors.is_empty(), "GCC notes should be suppressed");
    }

    #[test]
    fn test_mixed_errors_notes_filtered() {
        let log = "/home/user/main/app.c:40:10: error: 'led_strip_config_t' has no member named 'led_pixel_format'\n\
                   /home/user/main/app.c:40:29: error: 'LED_PIXEL_FORMAT_GRB' undeclared (first use in this function)\n\
                   /home/user/main/app.c:40:29: note: each undeclared identifier is reported only once for each function it appears in";
        let result = parser().parse_build_output(log, Some("build"));
        // Note should be filtered out, leaving 2 errors
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors.iter().all(|e| !matches!(e.severity, ErrorSeverity::Info)));
    }

    #[test]
    fn test_failed_resolve_component() {
        // Exact format from ESP-IDF v5.5 build.cmake
        let log = "  Failed to resolve component 'wifi_provisioning_ble' required by component\n  'main': unknown name.";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(!result.errors.is_empty(), "Should have caught 'Failed to resolve component'");
        let err = &result.errors[0];
        assert_eq!(err.kind, "missing_component");
        assert!(err.message.contains("wifi_provisioning_ble"));
        assert!(err.suggested_patch.is_some());
    }

    #[test]
    fn test_hint_component_not_found() {
        let log = "HINT: The component 'wifi_provisioning_ble' could not be found. This could be because: component name was misspelled";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(!result.errors.is_empty(), "Should have caught HINT component line");
        let err = &result.errors[0];
        assert_eq!(err.kind, "missing_component");
        assert!(err.message.contains("wifi_provisioning_ble"));
    }

    #[test]
    fn test_full_cmake_resolve_log() {
        // Simulate the full log output the user reported
        let log = r#"Executing action: all (aliases: build)
Running ninja in directory /home/user/project/build
CMake Error at /opt/esp/idf/tools/cmake/build.cmake:328 (message):
  Failed to resolve component 'wifi_provisioning_ble' required by component
  'main': unknown name.
-- Configuring incomplete, errors occurred!
HINT: The component 'wifi_provisioning_ble' could not be found."#;
        let result = parser().parse_build_output(log, Some("build"));
        // Should capture the component error (deduplicated — both lines match same component)
        assert!(!result.errors.is_empty(), "Full cmake log should produce errors");
        assert!(result.errors.iter().any(|e| e.kind == "missing_component"));
        assert!(result.summary.contains("error"));
    }

    #[test]
    fn test_catch_all_ninja_error() {
        // A ninja error that doesn't match any specific pattern
        let log = "ninja: error: rebuilding 'build.ninja': subcommand failed";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(!result.errors.is_empty(), "Catch-all should capture ninja errors");
        assert_eq!(result.errors[0].kind, "unrecognized_error");
    }

    #[test]
    fn test_catch_all_excludes_noise() {
        // Lines containing "error" but that are just noise
        let log = "-- Configuring incomplete, errors occurred!\nSee also \"/home/user/project/build/CMakeFiles/CMakeOutput.log\".";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(result.errors.is_empty(), "Noise lines should be excluded by catch-all");
    }

    #[test]
    fn test_catch_all_unknown_cmake_error() {
        // A completely new CMake error format we've never seen
        let log = "CMake Error: Some brand new error format that nobody thought of";
        let result = parser().parse_build_output(log, Some("build"));
        assert!(!result.errors.is_empty(), "Unknown CMake errors should be caught");
    }
}
