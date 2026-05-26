use std::fs;
use std::path::PathBuf;

const LARGE_FILE_SIZE_THRESHOLD: u64 = 4096*1024; // 4Mb files
const SMALL_FILE_SIZE_THRESHOLD: u64 = 5;        // 5 Bytes

pub const SOURCE_FILE_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cc", "h", "hpp", "cs", "java", "py", "rb", "go", "rs", "swift",
    "php", "js", "jsx", "ts", "tsx", "lua", "pl", "r", "sh", "bat", "cmd", "ps1",
    "m", "kt", "kts", "groovy", "dart", "fs", "fsx", "fsi", "html", "htm", "css",
    "scss", "sass", "less", "json", "xml", "yml", "yaml", "md", "sql", "cfg",
    "conf", "ini", "toml", "dockerfile", "ipynb", "rmd", "xml", "kt", "xaml",
    "unity", "gd", "uproject", "asm", "s", "tex", "makefile", "mk", "cmake",
    "gradle", "liquid"
];

pub fn is_valid_file(path: &PathBuf, allow_hidden_folders: bool, ignore_size_thresholds: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Single metadata() call — avoids the extra syscall that path.is_file() would make
    // before the metadata() call below (on Windows each is a separate GetFileAttributesW).
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Err("Unable to access file metadata".into()),
    };

    if !metadata.is_file() {
        return Err("Path is not a file".into());
    }

    if !allow_hidden_folders && path.ancestors().any(|ancestor| {
        ancestor.file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }) {
        return Err("Parent dir starts with a dot".into());
    }

    let file_size = metadata.len();
    if !ignore_size_thresholds && file_size < SMALL_FILE_SIZE_THRESHOLD {
        return Err("File size is too small".into());
    }
    if !ignore_size_thresholds && file_size > LARGE_FILE_SIZE_THRESHOLD {
        return Err("File size is too large".into());
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o400 == 0 {
            return Err("File has no read permissions".into());
        }
    }
    Ok(())
}
