//! Shared application state: settings, resolved external tools, running downloads.

use crate::settings::AppSettings;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Cancellation info for a running download.
pub struct CancelHandle {
    /// PID of the yt-dlp process. On Unix we kill the whole process group
    /// (taking down any ffmpeg child); on Windows we use `taskkill /T /F`.
    pub pid: u32,
}

#[derive(Default)]
pub struct AppState {
    pub settings: RwLock<AppSettings>,
    pub settings_path: RwLock<Option<PathBuf>>,
    pub running: DashMap<String, CancelHandle>,
    /// Resolved tool paths, refreshed by `detect_tools`.
    pub tools: RwLock<ResolvedTools>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTools {
    pub ytdlp: Option<ToolInfo>,
    pub ffmpeg: Option<ToolInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub path: String,
    pub version: String,
    /// "custom" | "bundled" | "appdata" | "path"
    pub source: String,
}

pub type SharedState = Arc<AppState>;
