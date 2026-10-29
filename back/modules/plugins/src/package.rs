use std::io::{Cursor, Read};

use sha2::{Digest, Sha256};

use crate::manifest::{INVALID_MANIFEST, PluginManifest};
use aphel_kernel::errors::AppError;

pub const MANIFEST_FILE: &str = "plugin.toml";
pub const BACKEND_FILE: &str = "backend.wasm";
pub const MAX_PACKAGE_BYTES: usize = 50 * 1024 * 1024;
const MAX_FILES: usize = 500;
const INVALID_PACKAGE: &str = "plugin_package_invalid";

#[derive(Debug, Clone)]
pub struct PackageFile {
    pub path: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub checksum: String,
    pub files: Vec<PackageFile>,
}

impl PluginPackage {
    pub fn backend(&self) -> Option<&PackageFile> {
        self.files.iter().find(|file| file.path == BACKEND_FILE)
    }
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::validation_code(INVALID_PACKAGE, message)
}

pub fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        "toml" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 255
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub fn read_package(bytes: &[u8]) -> Result<PluginPackage, AppError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(invalid("the package is larger than 50 MB"));
    }
    let checksum: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("the package is not a valid zip file"))?;
    if archive.len() > MAX_FILES {
        return Err(invalid("the package has too many files"));
    }
    let mut files = Vec::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| invalid("the package has an unreadable entry"))?;
        if entry.is_dir() {
            continue;
        }
        let path = entry
            .name()
            .map_err(|_| invalid("the package has an entry with an invalid name"))?
            .trim_start_matches("./")
            .to_owned();
        if !safe_path(&path) {
            return Err(invalid(format!("the package has an unsafe path: {path}")));
        }
        let mut content = Vec::new();
        entry
            .by_ref()
            .take((MAX_PACKAGE_BYTES - total + 1) as u64)
            .read_to_end(&mut content)
            .map_err(|_| invalid(format!("could not read {path}")))?;
        total += content.len();
        if total > MAX_PACKAGE_BYTES {
            return Err(invalid("the unpacked package is larger than 50 MB"));
        }
        files.push(PackageFile {
            content_type: content_type(&path).to_owned(),
            path,
            content,
        });
    }
    let manifest = files
        .iter()
        .find(|file| file.path == MANIFEST_FILE)
        .ok_or_else(|| {
            AppError::validation_code(INVALID_MANIFEST, "the package has no plugin.toml")
        })?;
    let text = std::str::from_utf8(&manifest.content)
        .map_err(|_| AppError::validation_code(INVALID_MANIFEST, "plugin.toml is not UTF-8"))?;
    let manifest = PluginManifest::parse(text)?;
    if let Some(web) = &manifest.web
        && !files.iter().any(|file| file.path == web.entry)
    {
        return Err(invalid(format!("the web entry {} is missing", web.entry)));
    }
    Ok(PluginPackage {
        manifest,
        checksum,
        files,
    })
}
