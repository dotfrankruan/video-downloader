//! Persistent application settings, stored as JSON in the app config dir.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_FILENAME_TEMPLATE: &str = "%(title)s.%(ext)s";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    /// UI language: "en" | "zh-CN"
    pub language: String,
    /// User-overridden absolute path to a yt-dlp binary.
    pub ytdlp_path: String,
    /// User-overridden absolute path to an ffmpeg binary (or its directory).
    pub ffmpeg_path: String,
    /// Default output directory for downloads.
    pub download_dir: String,
    /// yt-dlp output template (the `-o` value, filename part only).
    pub filename_template: String,
    /// Proxy URL passed to yt-dlp, e.g. "http://127.0.0.1:7890" or "socks5://...".
    pub proxy: String,
    /// Path to a Netscape-format cookies.txt file.
    pub cookies_file: String,
    /// Browser to extract cookies from (--cookies-from-browser). E.g. "safari", "chrome".
    pub cookies_browser: String,
    /// How cookies are supplied: "none" | "file" | "browser".
    pub cookies_mode: String,
    /// Maximum number of parallel downloads (enforced by the frontend queue).
    pub max_concurrent: u32,
    /// Extra raw yt-dlp arguments appended to every download (escape hatch).
    pub extra_args: String,
    /// Base URL used to download the yt-dlp binary at runtime.
    /// Default "https://github.com"; users in restricted networks can set a mirror.
    pub ytdlp_mirror: String,
    /// Whether the runtime yt-dlp download should use the configured proxy.
    pub download_via_proxy: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "system".to_string(),
            ytdlp_path: String::new(),
            ffmpeg_path: String::new(),
            download_dir: String::new(),
            filename_template: DEFAULT_FILENAME_TEMPLATE.to_string(),
            proxy: String::new(),
            cookies_file: String::new(),
            cookies_browser: String::new(),
            cookies_mode: "none".to_string(),
            max_concurrent: 3,
            extra_args: String::new(),
            ytdlp_mirror: "https://github.com".to_string(),
            download_via_proxy: false,
        }
    }
}

impl AppSettings {
    pub fn load(path: &PathBuf) -> Self {
        match std::fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
                eprintln!("[settings] file at {:?} is invalid ({}); using defaults", path, e);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw)?;
        Ok(())
    }
}
