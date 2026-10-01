//! Spawning yt-dlp, parsing its output, and streaming progress events to the UI.

use crate::args::{build_download_args, build_info_args, DownloadSpec};
use crate::settings::AppSettings;
use crate::state::{AppState, CancelHandle};
use serde::Serialize;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub const EVENT_DOWNLOAD: &str = "dsh-download-event";
pub const EVENT_TOOL_DOWNLOAD: &str = "dsh-tool-download-event";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub id: String,
    /// "progress" | "status" | "log" | "error" | "finished"
    pub kind: String,
    /// Download phase: "queued" | "downloading" | "processing" | "finished" | "error" | "cancelled"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Exit code for finished events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

impl DownloadEvent {
    fn status(id: &str, status: &str, message: Option<String>) -> Self {
        Self {
            id: id.to_string(),
            kind: "status".into(),
            status: Some(status.into()),
            percent: None,
            speed: None,
            eta: None,
            downloaded: None,
            total: None,
            message,
            exit_code: None,
        }
    }
    fn simple(id: &str, kind: &str, message: String) -> Self {
        Self {
            id: id.to_string(),
            kind: kind.into(),
            status: None,
            percent: None,
            speed: None,
            eta: None,
            downloaded: None,
            total: None,
            message: Some(message),
            exit_code: None,
        }
    }
}

fn emit(app: &AppHandle, ev: &DownloadEvent) {
    if let Err(e) = app.emit(EVENT_DOWNLOAD, ev) {
        eprintln!("[ytdlp] failed to emit event: {e}");
    }
}

fn parse_percent(raw: &str) -> Option<f64> {
    raw.trim().trim_end_matches('%').trim().parse::<f64>().ok()
}

fn parse_bytes(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("na") || raw == "None" {
        return None;
    }
    raw.parse::<f64>().ok()
}

fn clean_field(raw: &str) -> Option<String> {
    let v = raw.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("na") || v == "None" || v == "unknown" {
        None
    } else {
        Some(v.to_string())
    }
}

/// Parse a `[DSHPROG]` line produced by PROGRESS_TEMPLATE.
fn parse_progress_line(id: &str, line: &str) -> Option<DownloadEvent> {
    let body = line.strip_prefix("[DSHPROG]|")?;
    let mut parts = body.split('|');
    let status = parts.next().unwrap_or("");
    let percent = parts.next().and_then(parse_percent);
    let speed = parts.next().and_then(clean_field);
    let eta = parts.next().and_then(clean_field);
    let downloaded = parts.next().and_then(parse_bytes);
    let total_direct = parts.next().and_then(parse_bytes);
    let total_estimate = parts.next().and_then(parse_bytes);
    let total = total_direct.or(total_estimate);
    let phase = match status {
        "finished" => "processing",
        _ => "downloading",
    };
    Some(DownloadEvent {
        id: id.to_string(),
        kind: "progress".into(),
        status: Some(phase.into()),
        percent,
        speed,
        eta,
        downloaded,
        total,
        message: None,
        exit_code: None,
    })
}

/// Human-readable phase transitions inferred from regular yt-dlp output lines.
fn infer_status_from_line(line: &str) -> Option<&'static str> {
    if line.starts_with("[Merger]")
        || line.starts_with("[ExtractAudio]")
        || line.starts_with("[EmbedSubtitle]")
        || line.starts_with("[VideoConvertor]")
        || line.starts_with("[VideoRemuxer]")
        || line.starts_with("[ThumbnailsConvertor]")
        || line.starts_with("[EmbedThumbnail]")
        || line.starts_with("[Metadata]")
        || line.starts_with("[Fixup")
    {
        Some("processing")
    } else if line.starts_with("[download] Destination:") || line.starts_with("[download] 100%") {
        Some("downloading")
    } else {
        None
    }
}

pub async fn fetch_info(
    ytdlp_path: &str,
    url: &str,
    flat_playlist: bool,
    settings: &AppSettings,
) -> Result<serde_json::Value, String> {
    let args = build_info_args(url, flat_playlist, settings);
    let output = Command::new(ytdlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to launch yt-dlp: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "yt-dlp exited with {:?}:\n{}",
            output.status.code(),
            tail
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("invalid yt-dlp JSON output: {e}"))
}

/// Spawn a download task. Returns immediately with the task id; progress is
/// streamed back through `EVENT_DOWNLOAD` events.
pub fn start_download(
    app: AppHandle,
    state: std::sync::Arc<AppState>,
    id: String,
    spec: DownloadSpec,
) -> Result<(), String> {
    let (settings, ytdlp, ffmpeg) = {
        let settings = state.settings.read().unwrap().clone();
        let tools = state.tools.read().unwrap().clone();
        let ytdlp = tools
            .ytdlp
            .ok_or_else(|| "yt-dlp is not available. Install it or download it from Settings.".to_string())?;
        (settings, ytdlp, tools.ffmpeg)
    };

    let args = build_download_args(&spec, &settings, ffmpeg.as_ref().map(|t| t.path.as_str()));

    let mut cmd = Command::new(&ytdlp.path);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Prevent a console window flash on Windows.
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(unix)]
    {
        // Own process group so we can kill yt-dlp together with ffmpeg children.
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to launch yt-dlp at {}: {e}", ytdlp.path))?;
    let pid = child
        .id()
        .ok_or_else(|| "could not obtain yt-dlp process id".to_string())?;

    state.running.insert(id.clone(), CancelHandle { pid });

    emit(&app, &DownloadEvent::status(&id, "downloading", None));
    emit(
        &app,
        &DownloadEvent::simple(&id, "log", format!("$ {} {}", ytdlp.path, args.join(" "))),
    );

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    tauri::async_runtime::spawn(async move {
        let app2 = app.clone();
        let id2 = id.clone();
        let stderr_task = tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            let mut recent: Vec<String> = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                recent.push(line.clone());
                if recent.len() > 12 {
                    recent.remove(0);
                }
                emit(&app2, &DownloadEvent::simple(&id2, "error", line));
            }
            recent
        });

        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim_end().to_string();
                    if let Some(ev) = parse_progress_line(&id, trimmed.trim_start()) {
                        emit(&app, &ev);
                        continue;
                    }
                    if trimmed.contains("[DSHPP]|") {
                        emit(&app, &DownloadEvent::status(&id, "processing", None));
                        continue;
                    }
                    if let Some(status) = infer_status_from_line(&trimmed) {
                        emit(&app, &DownloadEvent::status(&id, status, Some(trimmed.clone())));
                    } else if !trimmed.is_empty() {
                        emit(&app, &DownloadEvent::simple(&id, "log", trimmed));
                    }
                }
                Ok(None) => break, // EOF
                Err(_) => break,
            }
        }

        let stderr_tail = stderr_task.await.unwrap_or_default();
        let status = child.wait().await;
        state.running.remove(&id);

        let code = status.ok().and_then(|s| s.code());
        let success = matches!(code, Some(0));
        // Killed by signal (our cancel path) → code() is None on Unix.
        let cancelled = code.is_none() || code == Some(143) || code == Some(15);

        let ev = DownloadEvent {
            id: id.clone(),
            kind: "finished".into(),
            status: Some(
                if success {
                    "finished"
                } else if cancelled {
                    "cancelled"
                } else {
                    "error"
                }
                .into(),
            ),
            percent: if success { Some(100.0) } else { None },
            speed: None,
            eta: None,
            downloaded: None,
            total: None,
            message: if success {
                None
            } else {
                Some(stderr_tail.join("\n"))
            },
            exit_code: code,
        };
        emit(&app, &ev);
    });

    Ok(())
}

/// Kill a running download (whole process group on Unix, task tree on Windows).
pub fn cancel_download(state: &AppState, id: &str) -> Result<(), String> {
    let handle = state
        .running
        .get(id)
        .ok_or_else(|| "download is not running".to_string())?;
    let pid = handle.pid;
    #[cfg(unix)]
    unsafe {
        // Negative pid = the whole process group (yt-dlp + ffmpeg children).
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    Ok(())
}
