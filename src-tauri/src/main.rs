#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Hidden diagnostic: `video-downloader --print-tools` prints the tool
    // resolution result as JSON and exits (useful for debugging detection).
    if std::env::args().any(|a| a == "--print-tools") {
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
        println!("{}", serde_json::to_string_pretty(&resolved).unwrap());
        return;
    }
    video_downloader_lib::run()
}
