//! Translates a frontend DownloadSpec + AppSettings into a yt-dlp CLI argument vector.
//!
//! Everything is passed as discrete argv entries (never through a shell),
//! so URLs / templates / extra args cannot be used for command injection.

use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};

/// Progress lines are emitted on stdout prefixed with a marker we can grep for.
/// Using progress templates keeps parsing stable across yt-dlp versions.
pub const PROGRESS_TEMPLATE: &str =
    "download:[DSHPROG]|%(progress.status)s|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s";
pub const POSTPROCESS_TEMPLATE: &str =
    "postprocess:[DSHPP]|%(progress.status)s|%(progress.postprocessor)s";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSpec {
    pub url: String,
    /// "video" | "audio" | "subs"
    pub mode: String,
    /// Explicit yt-dlp format id selected from the format table (video/audio modes).
    pub format_id: Option<String>,
    /// Preset used when no explicit format id: "best" | "best1080" | "best720" | "worst".
    pub preset: Option<String>,
    /// Audio extraction target for mode=audio: "mp3" | "m4a" | "opus" | "flac" | "wav" | "best".
    pub audio_format: Option<String>,
    /// Audio quality passed to --audio-quality (e.g. "0", "192K"). Empty = yt-dlp default.
    pub audio_quality: Option<String>,
    /// Whether yt-dlp may treat the URL as a playlist.
    pub allow_playlist: bool,
    /// yt-dlp --playlist-items value, e.g. "1-10,15". Empty = all.
    pub playlist_items: Option<String>,
    /// Subtitle handling: "none" | "embed" | "file" | "both".
    pub sub_mode: String,
    /// Requested subtitle languages (e.g. ["en", "zh-Hans"]). Empty = site default.
    pub sub_langs: Vec<String>,
    /// Also fetch auto-generated subtitles (--write-auto-subs).
    pub write_auto_subs: bool,
    /// Convert subtitle files to srt (--convert-subs srt).
    pub convert_subs_to_srt: bool,
    /// Merge container when streams must be merged: "mp4" | "mkv" | "" (yt-dlp default).
    pub merge_format: Option<String>,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    /// --limit-rate value, e.g. "5M". Empty = unlimited.
    pub rate_limit: Option<String>,
    // ---- per-download overrides; fall back to settings when empty ----
    pub output_dir: String,
    pub filename_template: String,
    pub proxy: String,
    /// "none" | "file" | "browser"
    pub cookies_mode: String,
    pub cookies_file: String,
    pub cookies_browser: String,
    pub extra_args: String,
}

fn push_opt(args: &mut Vec<String>, flag: &str, value: &str) {
    if !value.trim().is_empty() {
        args.push(flag.to_string());
        args.push(value.trim().to_string());
    }
}

fn push_flag(args: &mut Vec<String>, cond: bool, flag: &str) {
    if cond {
        args.push(flag.to_string());
    }
}

/// Common network/auth options shared by info fetching and downloading.
pub fn network_args(
    args: &mut Vec<String>,
    proxy: &str,
    cookies_mode: &str,
    cookies_file: &str,
    cookies_browser: &str,
) {
    push_opt(args, "--proxy", proxy);
    match cookies_mode {
        "file" => push_opt(args, "--cookies", cookies_file),
        "browser" => push_opt(args, "--cookies-from-browser", cookies_browser),
        _ => {}
    }
}

pub fn build_info_args(url: &str, flat_playlist: bool, settings: &AppSettings) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--no-warnings".into(),
        "--no-call-home".into(),
        "--encoding".into(),
        "utf-8".into(),
        "-J".into(),
    ];
    if flat_playlist {
        args.push("--flat-playlist".into());
    }
    network_args(
        &mut args,
        &settings.proxy,
        &settings.cookies_mode,
        &settings.cookies_file,
        &settings.cookies_browser,
    );
    if !settings.extra_args.trim().is_empty() {
        if let Ok(extra) = shell_words::split(&settings.extra_args) {
            args.extend(extra);
        }
    }
    args.push(url.to_string());
    args
}

pub fn build_download_args(
    spec: &DownloadSpec,
    settings: &AppSettings,
    ffmpeg_path: Option<&str>,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--newline".into(),
        "--no-color".into(),
        "--no-call-home".into(),
        "--encoding".into(),
        "utf-8".into(),
        "--progress-template".into(),
        PROGRESS_TEMPLATE.into(),
        "--progress-template".into(),
        POSTPROCESS_TEMPLATE.into(),
        "--no-simulate".into(),
        "--progress".into(),
    ];

    // --- format selection ---
    match spec.mode.as_str() {
        "audio" => {
            let f = spec
                .format_id
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "bestaudio/best".to_string());
            push_opt(&mut args, "-f", &f);
            args.push("-x".into());
            let af = spec
                .audio_format
                .clone()
                .filter(|s| !s.is_empty() && s != "best")
                .unwrap_or_else(|| "mp3".to_string());
            if af != "best" {
                push_opt(&mut args, "--audio-format", &af);
            }
            if let Some(q) = spec.audio_quality.clone().filter(|s| !s.trim().is_empty()) {
                push_opt(&mut args, "--audio-quality", &q);
            }
        }
        "subs" => {
            args.push("--skip-download".into());
        }
        _ => {
            // video
            if let Some(id) = spec.format_id.clone().filter(|s| !s.trim().is_empty()) {
                // Pair a video-only format with best audio; keep audio-only ids as-is.
                push_opt(&mut args, "-f", &format!("{id}+bestaudio/{id}"));
            } else {
                let preset = spec.preset.clone().unwrap_or_else(|| "best".into());
                let f = match preset.as_str() {
                    "best1080" => "bv*[height<=1080]+ba/b[height<=1080]",
                    "best720" => "bv*[height<=720]+ba/b[height<=720]",
                    "worst" => "wv+wa/w",
                    _ => "bv*+ba/b",
                };
                push_opt(&mut args, "-f", f);
            }
            if let Some(mf) = spec.merge_format.clone().filter(|s| !s.trim().is_empty()) {
                push_opt(&mut args, "--merge-output-format", &mf);
            }
        }
    }

    // --- subtitles ---
    let want_subs = spec.mode == "subs" || spec.sub_mode != "none";
    if want_subs {
        args.push("--write-subs".into());
        push_flag(&mut args, spec.write_auto_subs, "--write-auto-subs");
        if !spec.sub_langs.is_empty() {
            push_opt(&mut args, "--sub-langs", &spec.sub_langs.join(","));
        }
        if spec.convert_subs_to_srt {
            args.push("--convert-subs".into());
            args.push("srt".into());
        }
        if spec.mode != "subs" && (spec.sub_mode == "embed" || spec.sub_mode == "both") {
            args.push("--embed-subs".into());
        }
    }

    // --- playlist ---
    if spec.allow_playlist {
        args.push("--yes-playlist".into());
        args.push("--ignore-errors".into());
        if let Some(items) = spec.playlist_items.clone().filter(|s| !s.trim().is_empty()) {
            push_opt(&mut args, "--playlist-items", &items);
        }
    } else {
        args.push("--no-playlist".into());
    }

    // --- output ---
    let out_dir = if spec.output_dir.trim().is_empty() {
        settings.download_dir.trim()
    } else {
        spec.output_dir.trim()
    };
    push_opt(&mut args, "-P", out_dir);
    let tmpl = if spec.filename_template.trim().is_empty() {
        settings.filename_template.trim()
    } else {
        spec.filename_template.trim()
    };
    push_opt(&mut args, "-o", tmpl);

    // --- metadata / extras ---
    push_flag(&mut args, spec.embed_thumbnail, "--embed-thumbnail");
    push_flag(&mut args, spec.embed_metadata, "--embed-metadata");
    push_flag(&mut args, spec.embed_chapters, "--embed-chapters");
    if let Some(rl) = spec.rate_limit.clone().filter(|s| !s.trim().is_empty()) {
        push_opt(&mut args, "--limit-rate", &rl);
    }

    // --- network / auth ---
    let proxy = if spec.proxy.trim().is_empty() {
        settings.proxy.trim()
    } else {
        spec.proxy.trim()
    };
    let (cmode, cfile, cbrowser) = if spec.cookies_mode.trim().is_empty() {
        (
            settings.cookies_mode.as_str(),
            settings.cookies_file.as_str(),
            settings.cookies_browser.as_str(),
        )
    } else {
        (
            spec.cookies_mode.as_str(),
            spec.cookies_file.as_str(),
            spec.cookies_browser.as_str(),
        )
    };
    network_args(&mut args, proxy, cmode, cfile, cbrowser);

    if let Some(ff) = ffmpeg_path.filter(|s| !s.trim().is_empty()) {
        push_opt(&mut args, "--ffmpeg-location", ff.trim());
    }

    // --- user escape hatch (per-download first, then global) ---
    for raw in [&spec.extra_args, &settings.extra_args] {
        if !raw.trim().is_empty() {
            if let Ok(extra) = shell_words::split(raw) {
                args.extend(extra);
            }
        }
    }

    args.push(spec.url.clone());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;

    fn base_spec() -> DownloadSpec {
        DownloadSpec {
            url: "https://youtu.be/abc".into(),
            mode: "video".into(),
            format_id: None,
            preset: Some("best".into()),
            audio_format: None,
            audio_quality: None,
            allow_playlist: false,
            playlist_items: None,
            sub_mode: "none".into(),
            sub_langs: vec![],
            write_auto_subs: false,
            convert_subs_to_srt: false,
            merge_format: None,
            embed_thumbnail: false,
            embed_metadata: false,
            embed_chapters: false,
            rate_limit: None,
            output_dir: "/tmp/out".into(),
            filename_template: "%(title)s.%(ext)s".into(),
            proxy: "http://127.0.0.1:7890".into(),
            cookies_mode: "file".into(),
            cookies_file: "/tmp/cookies.txt".into(),
            cookies_browser: String::new(),
            extra_args: String::new(),
        }
    }

    #[test]
    fn video_preset_and_network_args() {
        let s = AppSettings::default();
        let a = build_download_args(&base_spec(), &s, Some("/opt/ffmpeg"));
        let joined = a.join("\u{1f}");
        assert!(joined.contains("-f\u{1f}bv*+ba/b"));
        assert!(joined.contains("--proxy\u{1f}http://127.0.0.1:7890"));
        assert!(joined.contains("--cookies\u{1f}/tmp/cookies.txt"));
        assert!(joined.contains("--no-playlist"));
        assert!(joined.contains("--ffmpeg-location\u{1f}/opt/ffmpeg"));
        assert!(joined.contains("-P\u{1f}/tmp/out"));
        assert!(joined.contains("-o\u{1f}%(title)s.%(ext)s"));
        assert!(a.last().unwrap() == "https://youtu.be/abc");
    }

    #[test]
    fn explicit_format_pairs_bestaudio() {
        let s = AppSettings::default();
        let mut spec = base_spec();
        spec.format_id = Some("137".into());
        let a = build_download_args(&spec, &s, None);
        assert!(a.join("\u{1f}").contains("-f\u{1f}137+bestaudio/137"));
    }

    #[test]
    fn subs_only_mode() {
        let s = AppSettings::default();
        let mut spec = base_spec();
        spec.mode = "subs".into();
        spec.sub_langs = vec!["en".into(), "zh-Hans".into()];
        spec.convert_subs_to_srt = true;
        let a = build_download_args(&spec, &s, None);
        let j = a.join("\u{1f}");
        assert!(j.contains("--skip-download"));
        assert!(j.contains("--write-subs"));
        assert!(j.contains("--sub-langs\u{1f}en,zh-Hans"));
        assert!(j.contains("--convert-subs\u{1f}srt"));
        assert!(!j.contains("--embed-subs"));
    }

    #[test]
    fn embed_subs_and_playlist_range() {
        let s = AppSettings::default();
        let mut spec = base_spec();
        spec.sub_mode = "embed".into();
        spec.allow_playlist = true;
        spec.playlist_items = Some("1-10,15".into());
        let a = build_download_args(&spec, &s, None);
        let j = a.join("\u{1f}");
        assert!(j.contains("--embed-subs"));
        assert!(j.contains("--yes-playlist"));
        assert!(j.contains("--playlist-items\u{1f}1-10,15"));
    }

    #[test]
    fn extra_args_split_like_a_shell() {
        let s = AppSettings::default();
        let mut spec = base_spec();
        spec.extra_args = "--sleep-requests 1 --impersonate \"chrome\"".into();
        let a = build_download_args(&spec, &s, None);
        let j = a.join("\u{1f}");
        assert!(j.contains("--sleep-requests\u{1f}1"));
        assert!(j.contains("--impersonate\u{1f}chrome"));
    }

    #[test]
    fn settings_fallbacks_used_when_spec_empty() {
        let mut s = AppSettings::default();
        s.download_dir = "/default/dir".into();
        s.proxy = "socks5://127.0.0.1:1080".into();
        let mut spec = base_spec();
        spec.output_dir = String::new();
        spec.proxy = String::new();
        spec.cookies_mode = String::new();
        s.cookies_mode = "browser".into();
        s.cookies_browser = "safari".into();
        let a = build_download_args(&spec, &s, None);
        let j = a.join("\u{1f}");
        assert!(j.contains("-P\u{1f}/default/dir"));
        assert!(j.contains("--proxy\u{1f}socks5://127.0.0.1:1080"));
        assert!(j.contains("--cookies-from-browser\u{1f}safari"));
    }
}
