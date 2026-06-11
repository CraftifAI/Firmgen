//! Project manifest — root CMakeLists plus every file under `main/`.

use std::collections::HashMap;
use std::path::Path;

use walkdir::WalkDir;

use super::static_extract;
use super::super::schema::AnalysisFacts;

const MAIN_DIR: &str = "main";

const SDKCONFIG_CANDIDATES: &[&str] = &["sdkconfig.defaults", "sdkconfig"];

/// Kconfig / sdkconfig keys worth loading into context (avoids full sdkconfig token bloat).
const SDKCONFIG_KEY_PREFIXES: &[&str] = &[
    "CONFIG_IDF_TARGET",
    "CONFIG_ESP_WIFI_SSID",
    "CONFIG_ESP_WIFI_PASSWORD",
    "CONFIG_ESP_WIFI",
    "CONFIG_MQTT",
    "CONFIG_WIFI",
];

pub struct ProjectManifest {
    pub rel_paths: Vec<String>,
    pub hashes: HashMap<String, String>,
    /// Text file contents read during manifest build (single-pass I/O).
    pub contents: HashMap<String, String>,
}

pub fn build_manifest(project_root: &Path) -> Result<ProjectManifest, String> {
    let core_paths = resolve_core_paths(project_root);
    let mut manifest = read_manifest_files(project_root, &core_paths)?;

    let mut probe = AnalysisFacts {
        project_name: project_name(project_root),
        ..Default::default()
    };
    for rel in &core_paths {
        if let Some(content) = manifest.contents.get(rel) {
            static_extract::extract_from_file(rel, content, &mut probe);
        }
    }

    let sdkconfig_paths = sdkconfig_paths_if_needed(project_root, &core_paths, &probe);
    if !sdkconfig_paths.is_empty() {
        let extra = read_manifest_files(project_root, &sdkconfig_paths)?;
        for rel in &extra.rel_paths {
            manifest.rel_paths.push(rel.clone());
            manifest
                .hashes
                .insert(rel.clone(), extra.hashes[rel].clone());
            if let Some(c) = extra.contents.get(rel) {
                manifest.contents.insert(rel.clone(), c.clone());
            }
        }
        manifest.rel_paths.sort();
    }

    Ok(manifest)
}

/// Root `CMakeLists.txt` plus every regular file under `main/` (recursive).
pub fn resolve_core_paths(project_root: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    if project_root.join("CMakeLists.txt").is_file() {
        paths.push("CMakeLists.txt".to_string());
    }
    paths.extend(list_main_files(project_root));
    paths.sort();
    paths.dedup();
    paths
}

/// All regular files under `project_root/main/`, relative to project root.
pub fn list_main_files(project_root: &Path) -> Vec<String> {
    let main_dir = project_root.join(MAIN_DIR);
    if !main_dir.is_dir() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(&main_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if should_skip_under_main(path, project_root) {
            continue;
        }
        if let Some(rel) = relative_path(project_root, path) {
            paths.push(rel);
        }
    }
    paths.sort();
    paths
}

/// First `.c` / `.cpp` source under `main/` (for fallback snippets).
pub fn first_main_source_file(project_root: &Path) -> Option<String> {
    list_main_files(project_root)
        .into_iter()
        .find(|rel| rel.ends_with(".c") || rel.ends_with(".cpp") || rel.ends_with(".cc"))
}

/// True when `main/` contains at least one C/C++ translation unit.
pub fn main_dir_has_sources(project_root: &Path) -> bool {
    first_main_source_file(project_root).is_some()
}

fn should_skip_under_main(path: &Path, project_root: &Path) -> bool {
    let Some(rel) = path.strip_prefix(project_root).ok() else {
        return true;
    };
    for component in rel.components() {
        let name = component.as_os_str().to_string_lossy();
        if name.starts_with('.') || name == "__pycache__" || name == "build" {
            return true;
        }
    }
    false
}

fn relative_path(project_root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(project_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// Include sdkconfig only when app_config.h and core sources leave gaps (chip, WiFi creds).
pub fn sdkconfig_paths_if_needed(
    project_root: &Path,
    core_paths: &[String],
    facts: &AnalysisFacts,
) -> Vec<String> {
    let has_app_cfg = core_paths
        .iter()
        .any(|p| static_extract::is_app_config_path(p));

    let uses_wifi = facts
        .network_facts
        .iter()
        .any(|n| n.node_type == "wifi_manager");

    let wifi_has_ssid = facts.network_facts.iter().any(|n| {
        n.node_type == "wifi_manager"
            && n.properties
                .as_object()
                .and_then(|o| o.get("ssid"))
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty())
    });

    if has_app_cfg && wifi_has_ssid && facts.target_chip.is_some() {
        return Vec::new();
    }

    if !uses_wifi && facts.target_chip.is_some() {
        return Vec::new();
    }

    let needs_chip = facts.target_chip.is_none();
    let needs_wifi_creds = uses_wifi && !wifi_has_ssid;
    if !needs_chip && !needs_wifi_creds {
        return Vec::new();
    }

    let mut out = Vec::new();
    if project_root.join("sdkconfig.defaults").is_file() {
        out.push("sdkconfig.defaults".to_string());
    } else if project_root.join("sdkconfig").is_file() {
        out.push("sdkconfig".to_string());
    }

    if facts.target_chip.is_none()
        && out.first().map(|s| s.as_str()) == Some("sdkconfig.defaults")
        && project_root.join("sdkconfig").is_file()
    {
        out.push("sdkconfig".to_string());
    }

    out
}

fn read_manifest_files(
    project_root: &Path,
    rel_paths: &[String],
) -> Result<ProjectManifest, String> {
    let mut hashes = HashMap::new();
    let mut contents = HashMap::new();
    let mut paths = Vec::new();

    for rel in rel_paths {
        let abs = project_root.join(rel);
        if !abs.is_file() {
            continue;
        }
        let (hash, text) = if is_sdkconfig_rel(rel) {
            read_sdkconfig_filtered(&abs)?
        } else {
            let raw = match std::fs::read_to_string(&abs) {
                Ok(s) => s,
                Err(e) => {
                    continue;
                }
            };
            let hash = format!("{:x}", md5::compute(raw.as_bytes()));
            (hash, raw)
        };
        hashes.insert(rel.clone(), hash);
        if !text.is_empty() {
            contents.insert(rel.clone(), text);
        }
        paths.push(rel.clone());
    }

    Ok(ProjectManifest {
        rel_paths: paths,
        hashes,
        contents,
    })
}

fn is_sdkconfig_rel(rel: &str) -> bool {
    let norm = rel.replace('\\', "/");
    norm == "sdkconfig" || norm == "sdkconfig.defaults"
}

fn read_sdkconfig_filtered(abs: &Path) -> Result<(String, String), String> {
    let raw = std::fs::read_to_string(abs).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", md5::compute(raw.as_bytes()));
    let filtered: Vec<String> = raw
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty()
                && !t.starts_with('#')
                && SDKCONFIG_KEY_PREFIXES
                    .iter()
                    .any(|prefix| t.starts_with(prefix))
        })
        .map(|s| s.to_string())
        .collect();
    Ok((hash, filtered.join("\n")))
}

fn project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("firmware")
        .to_string()
}

/// Priority for context assembly — higher = included first when truncating.
pub fn priority_for_path(rel: &str) -> u8 {
    let norm = rel.replace('\\', "/");
    if norm.ends_with("app_config.h") || norm.ends_with("/main/app_config.h") {
        return 110;
    }
    if norm.starts_with("main/") && (norm.ends_with(".c") || norm.ends_with(".cpp")) {
        return 100;
    }
    if norm.starts_with("main/") && norm.ends_with(".h") {
        return 88;
    }
    if norm == "main/Kconfig.projbuild" || norm.ends_with("/Kconfig.projbuild") {
        return 92;
    }
    if norm == "main/CMakeLists.txt"
        || (norm.starts_with("main/") && norm.ends_with("/CMakeLists.txt"))
    {
        return 96;
    }
    if norm == "CMakeLists.txt" {
        return 95;
    }
    if norm == "sdkconfig.defaults" {
        return 70;
    }
    if norm == "sdkconfig" {
        return 65;
    }
    if norm.starts_with("main/") {
        return 75;
    }
    50
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn list_main_files_includes_all_sources() {
        let tmp = std::env::temp_dir().join(format!("pir_main_manifest_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("main/drivers")).unwrap();
        fs::write(tmp.join("main/app_config.h"), "#define APP_LED_GPIO 2\n").unwrap();
        fs::write(
            tmp.join("main/sensor_task.c"),
            "void sensor_task(void) {}\n",
        )
        .unwrap();
        fs::write(
            tmp.join("main/drivers/i2c_bus.c"),
            "void i2c_init(void) {}\n",
        )
        .unwrap();
        fs::write(
            tmp.join("main/CMakeLists.txt"),
            "idf_component_register()\n",
        )
        .unwrap();

        let files = list_main_files(&tmp);
        assert!(files.contains(&"main/app_config.h".to_string()));
        assert!(files.contains(&"main/sensor_task.c".to_string()));
        assert!(files.contains(&"main/drivers/i2c_bus.c".to_string()));
        assert!(files.contains(&"main/CMakeLists.txt".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }
}
