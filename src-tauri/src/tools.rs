//! Locating (and, for lite builds, downloading) the external yt-dlp / ffmpeg binaries.
//!
//! Resolution order for each tool:
//!   1. User-configured custom path (settings)
//!   2. Bundled sidecar next to the app executable   ("full" builds only)
//!   3. Previously downloaded copy in the app data dir
//!   4. PATH lookup, plus common bin dirs (a packaged macOS .app gets a
//!      minimal PATH that does not include /opt/homebrew/bin etc.)

use crate::settings::AppSettings;
use crate::state::{ResolvedTools, ToolInfo};
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

fn version_of(bin: &Path) -> Option<String> {
    let out = std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

fn probe(path: &Path, source: &str) -> Option<ToolInfo> {
    if !is_executable_file(path) {
        return None;
    }
    version_of(path).map(|version| ToolInfo {
        path: path.to_string_lossy().to_string(),
        version,
        source: source.to_string(),
    })
}

/// Directory next to the current executable where Tauri places `externalBin` sidecars.
fn bundled_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Sidecar files shipped in "full" builds are named e.g. `yt-dlp-aarch64-apple-darwin`
/// (target-triple suffixed) by Tauri's externalBin mechanism.
fn bundled_candidate(base: &str) -> Option<PathBuf> {
    let dir = bundled_dir()?;
    let triple = target_triple();
    let with_triple = if cfg!(windows) {
        format!("{base}-{triple}.exe")
    } else {
        format!("{base}-{triple}")
    };
    let plain = exe_name(base);
    for name in [with_triple, plain] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Copy/symlink a bundled sidecar into the app data dir under its CLEAN name
/// (`yt-dlp`, `ffmpeg`, `ffprobe`). yt-dlp locates ffprobe/avconv next to the
/// ffmpeg binary by exact name, which the triple-suffixed sidecar names break,
/// so all bundled tools are materialized before use.
fn materialize_bundled(app_data_dir: &Path) {
    let bin_dir = appdata_bin_dir(app_data_dir);
    for base in [YTDLP, FFMPEG, "ffprobe"] {
        let Some(sidecar) = bundled_candidate(base) else {
            continue;
        };
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

/// Materialized clean-name path of a bundled tool, if it exists.
fn bundled_materialized(base: &str, app_data_dir: &Path) -> Option<PathBuf> {
    bundled_candidate(base)?; // only when a real sidecar exists
    let p = appdata_bin_dir(app_data_dir).join(exe_name(base));
    if p.exists() {
        Some(p)
    } else {
        None
    }
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

/// Resolve one tool using the documented order. `custom` is the settings override.
fn resolve_tool(
    base: &str,
    custom: &str,
    app_data_dir: Option<&Path>,
) -> Option<ToolInfo> {
    // 1. custom override
    if !custom.trim().is_empty() {
        let p = PathBuf::from(custom.trim());
        // Allow pointing either at the binary itself or at a directory containing it.
        let candidate = if p.is_dir() { p.join(exe_name(base)) } else { p };
        if let Some(info) = probe(&candidate, "custom") {
            return Some(info);
        }
    }
    // 2. bundled sidecar (full builds), materialized under its clean name
    if let Some(dir) = app_data_dir {
        if let Some(p) = bundled_materialized(base, dir) {
            if let Some(info) = probe(&p, "bundled") {
                return Some(info);
            }
        }
    }
    // 3. downloaded copy in app data dir
    if let Some(dir) = app_data_dir {
        let p = appdata_bin_dir(dir).join(exe_name(base));
        if let Some(info) = probe(&p, "appdata") {
            return Some(info);
        }
    }
    // 4. PATH + common dirs
    if let Some(p) = path_lookup(base) {
        if let Some(info) = probe(&p, "path") {
            return Some(info);
        }
    }
    None
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
        Ok("yt-dlp_win_arm64.exe")
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok("yt-dlp")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Ok("yt-dlp_aarch64")
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

    probe(&dest, "appdata").ok_or_else(|| anyhow!("downloaded yt-dlp failed to run at {:?}", dest))
}
