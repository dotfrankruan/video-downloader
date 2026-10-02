mod args;
mod settings;
mod state;
mod tools;
mod ytdlp;

pub use tools::resolve_all;

/// Exposed for the --print-tools diagnostic entry point.
pub fn default_settings() -> settings::AppSettings {
    settings::AppSettings::default()
}

use settings::AppSettings;
use state::{AppState, SharedState};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use ytdlp::EVENT_TOOL_DOWNLOAD;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDownloadProgress {
    tool: String,
    downloaded: u64,
    total: Option<u64>,
    done: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    /// "full" when yt-dlp/ffmpeg sidecars are bundled, otherwise "lite".
    variant: String,
    /// Directory where runtime-downloaded tools are stored.
    tools_dir: String,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    let tools_dir = app
        .path()
        .app_data_dir()
        .map(|d| tools::tools_dir(&d).to_string_lossy().to_string())
        .unwrap_or_default();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        variant: if tools::has_bundled_tools() { "full" } else { "lite" }.to_string(),
        tools_dir,
    }
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, SharedState>) -> AppSettings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: tauri::State<'_, SharedState>, settings: AppSettings) -> Result<(), String> {
    let path = state.settings_path.read().unwrap().clone();
    settings
        .save(&path.ok_or_else(|| "settings path not initialised".to_string())?)
        .map_err(|e| e.to_string())?;
    *state.settings.write().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn detect_tools(app: AppHandle, state: tauri::State<'_, SharedState>) -> state::ResolvedTools {
    let app_data_dir = app.path().app_data_dir().ok();
    let settings = state.settings.read().unwrap().clone();
    let resolved = tools::resolve_all(&settings, app_data_dir.as_deref());
    *state.tools.write().unwrap() = resolved.clone();
    resolved
}

#[tauri::command]
async fn download_ytdlp(app: AppHandle, state: tauri::State<'_, SharedState>) -> Result<state::ToolInfo, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    let settings = state.settings.read().unwrap().clone();
    let proxy = if settings.download_via_proxy {
        Some(settings.proxy.as_str())
    } else {
        None
    };
    let app2 = app.clone();
    let info = tools::download_ytdlp(
        &app_data_dir,
        &settings.ytdlp_mirror,
        proxy,
        move |downloaded, total| {
            let _ = app2.emit(
                EVENT_TOOL_DOWNLOAD,
                ToolDownloadProgress {
                    tool: "yt-dlp".into(),
                    downloaded,
                    total,
                    done: false,
                    error: None,
                },
            );
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    // Refresh the tool cache so subsequent downloads use the new binary.
    let resolved = tools::resolve_all(&settings, Some(&app_data_dir));
    *state.tools.write().unwrap() = resolved;
    let _ = app.emit(
        EVENT_TOOL_DOWNLOAD,
        ToolDownloadProgress {
            tool: "yt-dlp".into(),
            downloaded: 0,
            total: None,
            done: true,
            error: None,
        },
    );
    Ok(info)
}

#[tauri::command]
async fn fetch_info(
    state: tauri::State<'_, SharedState>,
    url: String,
    flat_playlist: bool,
) -> Result<serde_json::Value, String> {
    let (settings, ytdlp) = {
        let s = state.settings.read().unwrap().clone();
        let t = state.tools.read().unwrap().clone();
        (s, t.ytdlp.info)
    };
    let ytdlp = ytdlp
        .ok_or_else(|| "yt-dlp is not available. Install it or download it from Settings.".to_string())?;
    ytdlp::fetch_info(&ytdlp.path, &url, flat_playlist, &settings).await
}

/// Spawns yt-dlp via tokio::process. MUST stay `async`: sync Tauri commands
/// run on the main thread inside an FFI callback, where there is no tokio
/// runtime context — tokio's Command::spawn then panics in Handle::current
/// and the unwind across the FFI boundary aborts the whole app.
/// (Crash report: start_download -> tokio build_child -> Handle::current.)
#[tauri::command]
async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    id: String,
    spec: args::DownloadSpec,
) -> Result<(), String> {
    ytdlp::start_download(app, state.inner().clone(), id, spec)
}

#[tauri::command]
fn cancel_download(state: tauri::State<'_, SharedState>, id: String) -> Result<(), String> {
    ytdlp::cancel_download(&state, &id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared: SharedState = Arc::new(AppState {
        settings: RwLock::new(AppSettings::default()),
        settings_path: RwLock::new(None),
        running: dashmap::DashMap::new(),
        tools: RwLock::new(state::ResolvedTools::default()),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared)
        .setup(|app| {
            // Load persisted settings, then resolve external tools once at startup.
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join("settings.json");
            let settings = AppSettings::load(&settings_path);
            let app_data_dir = app.path().app_data_dir().ok();
            let resolved = tools::resolve_all(&settings, app_data_dir.as_deref());

            let state = app.state::<SharedState>();
            *state.settings.write().unwrap() = settings;
            *state.settings_path.write().unwrap() = Some(settings_path);
            *state.tools.write().unwrap() = resolved;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_settings,
            save_settings,
            detect_tools,
            download_ytdlp,
            fetch_info,
            start_download,
            cancel_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running video-downloader");
}
