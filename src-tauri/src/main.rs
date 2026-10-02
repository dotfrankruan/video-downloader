#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Hidden diagnostic: `video-downloader --print-tools [out.json]` writes the
    // tool-resolution report (including every probe attempt and its error) as
    // JSON. It writes to a file because a Windows GUI-subsystem binary has no
    // console for stdout.
    if let Some(pos) = std::env::args().position(|a| a == "--print-tools") {
        let settings = video_downloader_lib::default_settings();
        #[cfg(target_os = "macos")]
        let data_dir = std::env::var_os("HOME").map(|h| {
            std::path::PathBuf::from(h).join("Library/Application Support/com.frankruan.videodownloader")
        });
        #[cfg(target_os = "linux")]
        let data_dir = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".config/com.frankruan.videodownloader"));
        #[cfg(target_os = "windows")]
        let data_dir = std::env::var_os("APPDATA")
            .map(|h| std::path::PathBuf::from(h).join("com.frankruan.videodownloader"));
        let resolved = video_downloader_lib::resolve_all(&settings, data_dir.as_deref());
        let json = serde_json::to_string_pretty(&resolved).unwrap();
        let out = std::env::args()
            .nth(pos + 1)
            .unwrap_or_else(|| "tools-report.json".to_string());
        match std::fs::write(&out, &json) {
            Ok(()) => eprintln!("tool report written to {out}"),
            Err(e) => eprintln!("failed to write {out}: {e}"),
        }
        #[cfg(not(windows))]
        println!("{json}");
        return;
    }
    video_downloader_lib::run()
}
