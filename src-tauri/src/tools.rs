//! Locating (and, for lite builds, downloading) the external yt-dlp / ffmpeg binaries.
//!
//! Resolution order for each tool:
//!   1. User-configured custom path (settings)
//!   2. Bundled sidecar next to the app executable   ("full" builds only)
//!   3. Previously downloaded copy in the app data dir
//!   4. PATH lookup, plus common bin dirs (a packaged macOS .app gets a
//!      minimal PATH that does not include /opt/homebrew/bin etc.)

use crate::settings::AppSettings;
use crate::state::{ProbeAttempt, ResolvedTools, ToolInfo, ToolResolution};
use anyhow::{anyhow, Context};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};

pub const YTDLP: &str = "yt-dlp";
pub const FFMPEG: &str = "ffmpeg";

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Extra directories searched beyond PATH (packaged apps often see a tiny PATH).
fn extra_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
        dirs.push(PathBuf::from("/snap/bin"));
    }
    if let Some(home) = dirs_next() {
        #[cfg(unix)]
        dirs.push(home.join(".local/bin"));
        #[cfg(windows)]
        {
            dirs.push(home.join("AppData/Local/Programs/yt-dlp"));
            dirs.push(home.join("scoop/shims"));
        }
    }
    dirs
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn is_executable_file(p: &Path) -> bool {
    p.is_file()
}

fn version_of(bin: &Path) -> Result<String, String> {
    // yt-dlp uses GNU-style "--version"; ffmpeg/ffprobe only accept the
    // single-dash "-version" (their parser strips one dash and then fails
    // on "--version" with exit code 8). Try both.
    let mut last_err = String::from("no version output");
    for flag in ["--version", "-version"] {
        let mut cmd = std::process::Command::new(bin);
        cmd.arg(flag)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        no_window_std(&mut cmd);
        match cmd.output() {
            Err(e) => last_err = format!("spawn failed: {e}"),
            Ok(out) => {
                if out.status.success() {
                    let first = String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !first.is_empty() {
                        return Ok(clean_version_line(&first));
                    }
                    last_err = "empty version output".into();
                } else {
                    let tail: Vec<String> = String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .rev()
                        .take(2)
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    last_err = format!("exit {:?}: {}", out.status.code(), tail.join(" | "));
                }
            }
        }
    }
    Err(last_err)
}

/// Tidy up version banners for display: ffmpeg prints
/// "ffmpeg version 9.0.2-https://www.martin-riedl.de ..." -> "9.0.2".
fn clean_version_line(line: &str) -> String {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    // "<tool> version <X>" pattern (ffmpeg, ffprobe)
    if tokens.len() >= 3 && tokens[1] == "version" {
        let v = tokens[2];
        // Cut source-url suffixes like "9.0.2-https://..."
        let v = match v.find("-http") {
            Some(i) => &v[..i],
            None => v,
        };
        return v.to_string();
    }
    line.to_string()
}

/// Hide the console window that would otherwise flash for every spawned
/// process on Windows (GUI app + console tools = cmd popups without this).
#[cfg(windows)]
fn no_window_std(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

fn probe(path: &Path, source: &str) -> Result<ToolInfo, String> {
    if !is_executable_file(path) {
        return Err("file does not exist".into());
    }
    version_of(path).map(|version| ToolInfo {
        path: path.to_string_lossy().to_string(),
        version,
        source: source.to_string(),
    })
}

fn attempt(source: &str, path: &Path, result: &Result<ToolInfo, String>) -> ProbeAttempt {
    ProbeAttempt {
        source: source.to_string(),
        path: path.to_string_lossy().to_string(),
        ok: result.is_ok(),
        detail: match result {
            Ok(info) => info.version.clone(),
            Err(e) => e.clone(),
        },
    }
}

/// Directory next to the current executable where Tauri places `externalBin` sidecars.
fn bundled_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Sidecar files shipped in "full" builds are placed NEXT TO the app
/// executable. Tauri strips the target-triple suffix at bundle time, so an
/// installed app has clean names (`ffmpeg.exe`); only during plain
/// `cargo run` would the raw `ffmpeg-<triple>` names appear.
/// Returns (path, is_clean_name).
fn bundled_candidate(base: &str) -> Option<(PathBuf, bool)> {
    let dir = bundled_dir()?;
    let triple = target_triple();
    let with_triple = if cfg!(windows) {
        format!("{base}-{triple}.exe")
    } else {
        format!("{base}-{triple}")
    };
    let p = dir.join(&with_triple);
    if p.is_file() {
        return Some((p, false));
    }
    let p = dir.join(exe_name(base));
    if p.is_file() {
        return Some((p, true));
    }
    None
}

/// Materialize triple-suffixed sidecars into the app data dir under their
/// CLEAN names — only needed for the dev/`cargo run` case, because yt-dlp
/// locates ffprobe next to the ffmpeg binary by exact name. Properly bundled
/// apps already have clean names next to the exe and are used in place,
/// avoiding execution from %APPDATA% (which some Windows AV/policies block)
/// and avoiding a ~160 MB per-tool copy.
fn materialize_bundled(app_data_dir: &Path) {
    let bin_dir = appdata_bin_dir(app_data_dir);
    for base in [YTDLP, FFMPEG, "ffprobe"] {
        let Some((sidecar, clean)) = bundled_candidate(base) else {
            continue;
        };
        if clean {
            continue;
        }
        if std::fs::create_dir_all(&bin_dir).is_err() {
            return;
        }
        let dest = bin_dir.join(exe_name(base));
        #[cfg(unix)]
        {
            // Re-link if missing or pointing elsewhere (e.g. after an app update).
            let stale = match std::fs::read_link(&dest) {
                Ok(target) => target != sidecar,
                Err(_) => dest.exists(), // non-symlink file in the way
            };
            if !dest.exists() || stale {
                let _ = std::fs::remove_file(&dest);
                let _ = std::os::unix::fs::symlink(&sidecar, &dest);
            }
        }
        #[cfg(windows)]
        {
            // Symlinks need privileges on Windows; copy instead. Re-copy when the
            // size differs (cheap staleness check after app updates).
            let stale = match (std::fs::metadata(&dest), std::fs::metadata(&sidecar)) {
                (Ok(d), Ok(s)) => d.len() != s.len(),
                _ => true,
            };
            if !dest.exists() || stale {
                let _ = std::fs::copy(&sidecar, &dest);
            }
        }
    }
}

/// The path to a bundled tool, ready to execute.
fn bundled_tool_path(base: &str, app_data_dir: &Path) -> Option<PathBuf> {
    let (sidecar, clean) = bundled_candidate(base)?;
    if clean {
        return Some(sidecar);
    }
    // Triple-suffixed: prefer the materialized clean-name copy; fall back to
    // the sidecar itself if materialization failed.
    let p = appdata_bin_dir(app_data_dir).join(exe_name(base));
    Some(if p.exists() { p } else { sidecar })
}

pub fn target_triple() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "x86_64-pc-windows-msvc";
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "aarch64-pc-windows-msvc";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "x86_64-unknown-linux-gnu";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "aarch64-unknown-linux-gnu";
}

fn path_lookup(base: &str) -> Option<PathBuf> {
    let name = exe_name(base);
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push(dir.join(&name));
        }
    }
    for dir in extra_search_dirs() {
        candidates.push(dir.join(&name));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn appdata_bin_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("bin")
}

/// Resolve one tool using the documented order, recording every attempt.
fn resolve_tool(base: &str, custom: &str, app_data_dir: Option<&Path>) -> ToolResolution {
    let mut attempts: Vec<ProbeAttempt> = Vec::new();
    let mut custom_invalid = false;

    // 1. custom override
    if !custom.trim().is_empty() {
        let p = PathBuf::from(custom.trim());
        // Allow pointing either at the binary itself or at a directory containing it.
        let candidate = if p.is_dir() { p.join(exe_name(base)) } else { p };
        let result = probe(&candidate, "custom");
        attempts.push(attempt("custom", &candidate, &result));
        if let Ok(info) = result {
            return ToolResolution { info: Some(info), custom_invalid: false, attempts };
        }
        custom_invalid = true;
    }

    // 2. bundled sidecar (full builds), used in place next to the exe when
    //    it already has a clean name; materialized otherwise.
    if let Some(dir) = app_data_dir {
        match bundled_tool_path(base, dir) {
            Some(p) => {
                let result = probe(&p, "bundled");
                attempts.push(attempt("bundled", &p, &result));
                if let Ok(info) = result {
                    return ToolResolution { info: Some(info), custom_invalid, attempts };
                }
            }
            None => attempts.push(ProbeAttempt {
                source: "bundled".into(),
                path: String::new(),
                ok: false,
                detail: "no bundled sidecar in this build".into(),
            }),
        }
    }

    // 3. downloaded copy in app data dir
    if let Some(dir) = app_data_dir {
        let p = appdata_bin_dir(dir).join(exe_name(base));
        if p.exists() {
            let result = probe(&p, "appdata");
            attempts.push(attempt("appdata", &p, &result));
            if let Ok(info) = result {
                return ToolResolution { info: Some(info), custom_invalid, attempts };
            }
        }
    }

    // 4. PATH + common dirs
    match path_lookup(base) {
        Some(p) => {
            let result = probe(&p, "path");
            attempts.push(attempt("path", &p, &result));
            if let Ok(info) = result {
                return ToolResolution { info: Some(info), custom_invalid, attempts };
            }
        }
        None => attempts.push(ProbeAttempt {
            source: "path".into(),
            path: String::new(),
            ok: false,
            detail: "not found in PATH or common bin dirs".into(),
        }),
    }

    ToolResolution { info: None, custom_invalid, attempts }
}

/// True when bundled sidecar binaries ship inside this build ("full" variant).
pub fn has_bundled_tools() -> bool {
    bundled_candidate(YTDLP).is_some()
}

/// Directory where runtime-downloaded tools are stored.
pub fn tools_dir(app_data_dir: &Path) -> PathBuf {
    appdata_bin_dir(app_data_dir)
}

pub fn resolve_all(settings: &AppSettings, app_data_dir: Option<&Path>) -> ResolvedTools {
    // Materialize bundled sidecars (clean names) before probing, so that
    // yt-dlp can find ffprobe next to ffmpeg in "full" builds.
    if let Some(dir) = app_data_dir {
        materialize_bundled(dir);
    }
    ResolvedTools {
        ytdlp: resolve_tool(YTDLP, &settings.ytdlp_path, app_data_dir),
        ffmpeg: resolve_tool(FFMPEG, &settings.ffmpeg_path, app_data_dir),
    }
}

/// Which yt-dlp release asset fits this platform (mirrors the official release names).
pub fn ytdlp_asset_name() -> anyhow::Result<&'static str> {
    #[cfg(target_os = "macos")]
    {
        Ok("yt-dlp_macos") // universal2 build
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Ok("yt-dlp.exe")
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        Ok("yt-dlp_arm64.exe")
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok("yt-dlp_linux")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Ok("yt-dlp_linux_aarch64")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(anyhow!("unsupported platform for runtime yt-dlp download"))
    }
}

/// Download the yt-dlp binary into the app data dir. Returns the new ToolInfo.
/// `mirror` is a base URL like "https://github.com" (the release path is appended).
pub async fn download_ytdlp(
    app_data_dir: &Path,
    mirror: &str,
    proxy: Option<&str>,
    on_progress: impl Fn(u64, Option<u64>) + Send,
) -> anyhow::Result<ToolInfo> {
    let asset = ytdlp_asset_name()?;
    let base = if mirror.trim().is_empty() {
        "https://github.com"
    } else {
        mirror.trim().trim_end_matches('/')
    };
    let url = format!("{base}/yt-dlp/yt-dlp/releases/latest/download/{asset}");

    let mut builder = reqwest::Client::builder()
        .user_agent("video-downloader/0.1 (+https://github.com/dotfrankruan/video-downloader)");
    if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
        builder = builder.proxy(
            reqwest::Proxy::all(p.trim()).context("invalid proxy URL for yt-dlp download")?,
        );
    }
    let client = builder.build()?;

    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed to request {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("download failed: HTTP {} for {url}", resp.status()));
    }
    let total = resp.content_length();

    let bin_dir = appdata_bin_dir(app_data_dir);
    tokio::fs::create_dir_all(&bin_dir).await?;
    let dest = bin_dir.join(exe_name(YTDLP));
    let tmp = bin_dir.join(format!("{}.part", exe_name(YTDLP)));

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&tmp, &dest).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    probe(&dest, "appdata")
        .map_err(|e| anyhow!("downloaded yt-dlp at {:?} failed to run: {e}", dest))
}
