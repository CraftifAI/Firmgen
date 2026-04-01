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

const FACTORY_FLASH_SH: &str = include_str!("../../../../resources/esp32_factory/factory_flash.sh");
const FACTORY_FLASH_BAT: &str = include_str!("../../../../resources/esp32_factory/factory_flash.bat");
const FACTORY_FLASH_PS1: &str = include_str!("../../../../resources/esp32_factory/factory_flash.ps1");

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

/// Build `{project}_release.zip` bytes (same layout as `package_esp32_release_factory.py` inner folder).
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

    let mut ordered: Vec<(String, String)> = flash_files
        .iter()
        .filter_map(|(off, v)| v.as_str().map(|rel| (off.clone(), rel.to_string())))
        .collect();
    ordered.sort_by_key(|(off, _)| {
        u64::from_str_radix(off.trim_start_matches("0x"), 16).unwrap_or(0)
    });

    let mut firmware_records = Vec::new();
    for (offset, rel) in &ordered {
        let src = build_dir.join(rel);
        if !src.is_file() {
            return Err(format!("Missing firmware file: {}", src.display()));
        }
        let data = std::fs::read(&src).map_err(|e| e.to_string())?;
        let arc_rel = rel.replace('\\', "/").trim_start_matches('/').to_string();
        if arc_rel.is_empty() {
            return Err(format!("Bad flash_files path: {}", rel));
        }
        let arc = format!("{}/firmware/{}", prefix, arc_rel);
        firmware_records.push(json!({
            "offset": offset,
            "source_relative_path": rel,
            "release_relative_path": format!("firmware/{}", arc_rel),
            "sha256": sha256_hex(&data),
            "size_bytes": data.len(),
        }));
        files.insert(arc, data);
    }

    files.insert(format!("{}/flasher_args.json", prefix), meta_str.into_bytes());
    let flash_args_path = build_dir.join("flash_args");
    if flash_args_path.is_file() {
        let fa = std::fs::read(&flash_args_path).map_err(|e| e.to_string())?;
        files.insert(format!("{}/flash_args", prefix), fa);
    }

    let chip = meta
        .pointer("/extra_esptool_args/chip")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let flash_mode = meta
        .pointer("/flash_settings/flash_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let flash_size = meta
        .pointer("/flash_settings/flash_size")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let flash_freq = meta
        .pointer("/flash_settings/flash_freq")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let readme = format!(
        r#"# {project_name} Factory Release

## Operator workflow

### Windows
Put the release `.zip` and `factory_flash.bat` in the same folder, then double-click `factory_flash.bat`.

### Linux / macOS
```bash
bash factory_flash.sh
```

The launcher extracts the zip to a temp directory, finds `flasher_args.json`, and flashes via esptool.

## Firmware target
- Chip: `{chip}`
- Flash mode: `{flash_mode}`
- Flash size: `{flash_size}`
- Flash freq: `{flash_freq}`

## Notes
- Requires Python and `esptool` on the host (unless you bundle esptool).
- USB drivers may be required on Windows.
"#,
        project_name = project_name,
        chip = chip,
        flash_mode = flash_mode,
        flash_size = flash_size,
        flash_freq = flash_freq,
    );
    files.insert(format!("{}/README.md", prefix), readme.into_bytes());

    let manifest = json!({
        "project_name": project_name,
        "release_type": "factory_flash_release",
        "chip": chip,
        "flash_settings": meta.get("flash_settings").cloned().unwrap_or(json!({})),
        "write_flash_args": meta.get("write_flash_args").cloned().unwrap_or(json!([])),
        "firmware_files": firmware_records,
    });
    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    files.insert(format!("{}/manifest.json", prefix), manifest_str.into_bytes());

    let mut checksum_lines: Vec<String> = Vec::new();
    for (path, bytes) in files.iter() {
        checksum_lines.push(format!("{}  {}", sha256_hex(bytes), path));
    }
    files.insert(
        format!("{}/SHA256SUMS.txt", prefix),
        checksum_lines.join("\n").into_bytes(),
    );

    files.insert(
        format!("{}/factory_flash.sh", prefix),
        FACTORY_FLASH_SH.as_bytes().to_vec(),
    );
    files.insert(
        format!("{}/factory_flash.bat", prefix),
        FACTORY_FLASH_BAT.as_bytes().to_vec(),
    );
    files.insert(
        format!("{}/factory_flash.ps1", prefix),
        FACTORY_FLASH_PS1.as_bytes().to_vec(),
    );

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
