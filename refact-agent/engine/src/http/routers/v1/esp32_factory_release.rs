//! GET /v1/esp32/factory-release?chat_id=... — factory flash ZIP for the chat's ESP-IDF project.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::io::Write;
use std::path::Path;

use axum::extract::Query;
use hyper::{Body, Response, StatusCode};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::custom_error::ScratchError;
use crate::progressbar;

#[derive(Deserialize)]
pub struct FactoryReleaseQuery {
    pub chat_id: String,
}

fn sha256_hex(data: &[u8]) -> String {
    let out = Sha256::digest(data);
    out.iter().map(|b| format!("{:02x}", b)).collect()
}

fn content_disposition_for(filename: &str) -> String {
    let ascii_fallback: String = filename
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
        .collect();

    // Minimal RFC 5987 encoding for UTF-8 bytes.
    let encoded = filename
        .as_bytes()
        .iter()
        .map(|b| match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => (*b as char).to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect::<String>();

    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, encoded
    )
}

/// Extract just the filename from a relative path (e.g. "bootloader/bootloader.bin" → "bootloader.bin").
/// If there's a collision, keep original relative path to avoid clobbering.
fn flatten_filename(rel: &str) -> String {
    let normalized = rel.replace('\\', "/");
    normalized
        .rsplit('/')
        .next()
        .unwrap_or(&normalized)
        .to_string()
}

/// Build `{project}_release.zip` — a clean, self-contained firmware archive.
///
/// Structure:
/// ```text
/// {project}_release/
/// ├── firmware/
/// │   ├── bootloader.bin
/// │   ├── partition-table.bin
/// │   └── {project}.bin
/// ├── flash_config.json    ← single metadata file
/// ├── flash.bat            ← simple hardcoded esptool command
/// ├── flash.sh             ← simple hardcoded esptool command
/// ├── SHA256SUMS.txt
/// └── README.md
/// ```
fn build_factory_zip(project_dir: &Path) -> Result<Vec<u8>, String> {
    let project_name = project_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid project path".to_string())?;

    let build_dir = project_dir.join("build");
    let flasher_path = build_dir.join("flasher_args.json");
    let meta_str = std::fs::read_to_string(&flasher_path).map_err(|e| e.to_string())?;
    let meta: serde_json::Value = serde_json::from_str(&meta_str).map_err(|e| e.to_string())?;

    let flash_files = meta
        .get("flash_files")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "flasher_args.json: missing flash_files".to_string())?;

    let prefix = format!("{}_release", project_name);
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    // Sort flash_files by offset (numerically).
    let mut ordered: Vec<(String, String)> = flash_files
        .iter()
        .filter_map(|(off, v)| v.as_str().map(|rel| (off.clone(), rel.to_string())))
        .collect();
    ordered.sort_by_key(|(off, _)| {
        u64::from_str_radix(off.trim_start_matches("0x"), 16).unwrap_or(0)
    });

    // Flatten firmware filenames. If two files would collide, fall back to full relative path.
    let flat_names: Vec<String> = ordered.iter().map(|(_, rel)| flatten_filename(rel)).collect();
    let mut final_names: Vec<String> = Vec::with_capacity(ordered.len());
    for (i, flat) in flat_names.iter().enumerate() {
        let has_collision = flat_names.iter().enumerate().any(|(j, other)| j != i && other == flat);
        if has_collision {
            // Keep the relative path, just normalize slashes
            final_names.push(ordered[i].1.replace('\\', "/").trim_start_matches('/').to_string());
        } else {
            final_names.push(flat.clone());
        }
    }

    // Read firmware files and build flash_config entries.
    let mut flash_config_files = Vec::new();
    for (idx, (offset, rel)) in ordered.iter().enumerate() {
        let src = build_dir.join(rel);
        if !src.is_file() {
            return Err(format!("Missing firmware file: {}", src.display()));
        }
        let data = std::fs::read(&src).map_err(|e| e.to_string())?;
        let firmware_name = &final_names[idx];
        let arc = format!("{}/firmware/{}", prefix, firmware_name);

        flash_config_files.push(json!({
            "offset": offset,
            "file": format!("firmware/{}", firmware_name),
        }));

        files.insert(arc, data);
    }

    // Extract chip and flash settings from ESP-IDF metadata.
    let chip = meta
        .pointer("/extra_esptool_args/chip")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    let flash_mode = meta
        .pointer("/flash_settings/flash_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("dio");
    let flash_size = meta
        .pointer("/flash_settings/flash_size")
        .and_then(|v| v.as_str())
        .unwrap_or("detect");
    let flash_freq = meta
        .pointer("/flash_settings/flash_freq")
        .and_then(|v| v.as_str())
        .unwrap_or("40m");

    // Build flash_config.json — single, clean metadata file.
    let flash_config = json!({
        "project_name": project_name,
        "chip": chip,
        "flash_mode": flash_mode,
        "flash_size": flash_size,
        "flash_freq": flash_freq,
        "flash_files": flash_config_files,
    });
    let flash_config_str = serde_json::to_string_pretty(&flash_config).map_err(|e| e.to_string())?;
    files.insert(format!("{}/flash_config.json", prefix), flash_config_str.into_bytes());

    // Build the esptool write_flash argument string for scripts and README.
    let mut esptool_file_args = String::new();
    let mut bat_file_args = String::new();
    for entry in &flash_config_files {
        let offset = entry["offset"].as_str().unwrap_or("0x0");
        let file = entry["file"].as_str().unwrap_or("firmware/unknown.bin");
        esptool_file_args.push_str(&format!("  {} {} \\\n", offset, file));
        // BAT uses backslash paths and ^ for continuation
        let bat_file = file.replace('/', "\\");
        bat_file_args.push_str(&format!("  {} {} ^\n", offset, bat_file));
    }
    // Trim trailing continuation characters
    let esptool_file_args = esptool_file_args.trim_end_matches(" \\\n");
    let bat_file_args = bat_file_args.trim_end_matches(" ^\n");

    // Generate flash.sh — simple, hardcoded, no JSON parsing.
    let flash_sh = format!(
        r#"#!/usr/bin/env bash
# Factory flash script for {project_name}
# Usage: bash flash.sh [PORT]
#   PORT defaults to /dev/ttyUSB0 (override with first argument or FLASH_PORT env var)
set -euo pipefail

PORT="${{{flash_port_env}:-${{1:-/dev/ttyUSB0}}}}"

echo "[INFO] Flashing {project_name} to $PORT ..."
esptool.py --chip {chip} --port "$PORT" --baud 460800 write_flash \
  --flash_mode {flash_mode} --flash_size {flash_size} --flash_freq {flash_freq} \
{esptool_file_args}

echo "[OK] Flash complete."
"#,
        project_name = project_name,
        chip = chip,
        flash_mode = flash_mode,
        flash_size = flash_size,
        flash_freq = flash_freq,
        esptool_file_args = esptool_file_args,
        flash_port_env = "FLASH_PORT",
    );
    files.insert(format!("{}/flash.sh", prefix), flash_sh.into_bytes());

    // Generate flash.bat — simple, hardcoded, no JSON parsing or PowerShell.
    let flash_bat = format!(
        r#"@echo off
REM Factory flash script for {project_name}
REM Usage: flash.bat [COM_PORT]
REM   COM_PORT defaults to COM3 (override with first argument or FLASH_PORT env var)
setlocal

if not "%~1"=="" (
  set PORT=%~1
) else if defined FLASH_PORT (
  set PORT=%FLASH_PORT%
) else (
  set PORT=COM3
)

echo [INFO] Flashing {project_name} to %PORT% ...
esptool.py --chip {chip} --port %PORT% --baud 460800 write_flash ^
  --flash_mode {flash_mode} --flash_size {flash_size} --flash_freq {flash_freq} ^
{bat_file_args}

if %ERRORLEVEL% NEQ 0 (
  echo.
  echo [ERR] Flash failed with exit code %ERRORLEVEL%.
  pause
  exit /b %ERRORLEVEL%
)

echo [OK] Flash complete.
pause
"#,
        project_name = project_name,
        chip = chip,
        flash_mode = flash_mode,
        flash_size = flash_size,
        flash_freq = flash_freq,
        bat_file_args = bat_file_args,
    );
    files.insert(format!("{}/flash.bat", prefix), flash_bat.into_bytes());

    // Generate README.md
    let mut readme_flash_args = String::new();
    for entry in &flash_config_files {
        let offset = entry["offset"].as_str().unwrap_or("0x0");
        let file = entry["file"].as_str().unwrap_or("firmware/unknown.bin");
        readme_flash_args.push_str(&format!("  {} {} \\\n", offset, file));
    }
    let readme_flash_args = readme_flash_args.trim_end_matches(" \\\n");

    let readme = format!(
        r#"# {project_name} — Factory Release

## Quick Start

1. Extract this ZIP
2. Connect your {chip} board via USB
3. Run the flash script:

### Windows
```
flash.bat COM3
```

### Linux / macOS
```bash
bash flash.sh /dev/ttyUSB0
```

Replace the port with your actual serial port.

## Manual Flash Command

If you prefer to run esptool directly:

```bash
esptool.py --chip {chip} --port PORT --baud 460800 write_flash \
  --flash_mode {flash_mode} --flash_size {flash_size} --flash_freq {flash_freq} \
{readme_flash_args}
```

## Firmware Details

| Setting    | Value        |
|------------|--------------|
| Chip       | `{chip}`     |
| Flash mode | `{flash_mode}` |
| Flash size | `{flash_size}` |
| Flash freq | `{flash_freq}` |

## Prerequisites

- **esptool** must be installed and on your PATH
  - Install via pip: `pip install esptool`
  - Or download standalone: https://github.com/espressif/esptool/releases
- USB drivers for your board (CP210x, CH340, FTDI, etc.)

## Files

- `flash_config.json` — Machine-readable flash configuration
- `firmware/` — Binary firmware files
- `flash.bat` — Windows flash script
- `flash.sh` — Linux/macOS flash script
- `SHA256SUMS.txt` — File integrity checksums
"#,
        project_name = project_name,
        chip = chip,
        flash_mode = flash_mode,
        flash_size = flash_size,
        flash_freq = flash_freq,
        readme_flash_args = readme_flash_args,
    );
    files.insert(format!("{}/README.md", prefix), readme.into_bytes());

    // SHA256SUMS.txt
    let mut checksum_lines: Vec<String> = Vec::new();
    for (path, bytes) in files.iter() {
        // Use relative path within the release folder for checksums
        let rel_path = path.strip_prefix(&format!("{}/", prefix)).unwrap_or(path);
        checksum_lines.push(format!("{}  {}", sha256_hex(bytes), rel_path));
    }
    files.insert(
        format!("{}/SHA256SUMS.txt", prefix),
        checksum_lines.join("\n").into_bytes(),
    );

    // Write ZIP
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (path, data) in files {
            zip.start_file(path, opts).map_err(|e| e.to_string())?;
            zip.write_all(&data).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}

pub async fn handle_v1_esp32_factory_release(
    Query(q): Query<FactoryReleaseQuery>,
) -> Result<Response<Body>, ScratchError> {
    if q.chat_id.is_empty() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            "chat_id is required".to_string(),
        ));
    }

    let Some(project_path) = progressbar::esp32_project_path_for_chat(&q.chat_id).await else {
        return Err(ScratchError::new(
            StatusCode::NOT_FOUND,
            "No ESP32 project found for this chat. Create a project or run a build in this chat first."
                .to_string(),
        ));
    };

    if !project_path.is_dir() {
        return Err(ScratchError::new(
            StatusCode::NOT_FOUND,
            format!("Project path no longer exists: {}", project_path.display()),
        ));
    }

    let cmake = project_path.join("CMakeLists.txt");
    if !cmake.is_file() {
        return Err(ScratchError::new(
            StatusCode::BAD_REQUEST,
            format!(
                "Path does not look like an ESP-IDF project (no CMakeLists.txt): {}",
                project_path.display()
            ),
        ));
    }

    let flasher_json = project_path.join("build").join("flasher_args.json");
    if !flasher_json.is_file() {
        return Err(ScratchError::new(
            StatusCode::CONFLICT,
            "No build artifacts yet (build/flasher_args.json missing). Run idf.py build first."
                .to_string(),
        ));
    }

    let zip_bytes = build_factory_zip(&project_path).map_err(|e| {
        ScratchError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build factory zip: {}", e),
        )
    })?;

    let project_name = project_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let filename = format!("{}_release.zip", project_name);

    Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/zip")
        .header(
            hyper::header::CONTENT_DISPOSITION,
            content_disposition_for(&filename),
        )
        .body(Body::from(zip_bytes))
        .map_err(|e| {
            ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("response build error: {}", e),
            )
        })
}
