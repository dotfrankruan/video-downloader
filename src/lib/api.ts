import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ---------- Settings (mirrors src-tauri/src/settings.rs, camelCase) ----------

export interface AppSettings {
  language: string; // "system" | "en" | "zh-CN"
  ytdlpPath: string;
  ffmpegPath: string;
  downloadDir: string;
  filenameTemplate: string;
  proxy: string;
  cookiesFile: string;
  cookiesBrowser: string;
  cookiesMode: "none" | "file" | "browser";
  maxConcurrent: number;
  extraArgs: string;
  ytdlpMirror: string;
  downloadViaProxy: boolean;
}

export const defaultSettings: AppSettings = {
  language: "system",
  ytdlpPath: "",
  ffmpegPath: "",
  downloadDir: "",
  filenameTemplate: "%(title)s.%(ext)s",
  proxy: "",
  cookiesFile: "",
  cookiesBrowser: "",
  cookiesMode: "none",
  maxConcurrent: 3,
  extraArgs: "",
  ytdlpMirror: "https://github.com",
  downloadViaProxy: false,
};

// ---------- Tools ----------

export interface ToolInfo {
  path: string;
  version: string;
  source: "custom" | "bundled" | "appdata" | "path";
}

export interface ResolvedTools {
  ytdlp: ToolInfo | null;
  ffmpeg: ToolInfo | null;
}

export interface ToolDownloadProgress {
  tool: string;
  downloaded: number;
  total: number | null;
  done: boolean;
  error: string | null;
}

// ---------- Download spec (mirrors src-tauri/src/args.rs DownloadSpec) ----------

export interface DownloadSpec {
  url: string;
  mode: "video" | "audio" | "subs";
  formatId: string | null;
  preset: string | null;
  audioFormat: string | null;
  audioQuality: string | null;
  allowPlaylist: boolean;
  playlistItems: string | null;
  subMode: "none" | "embed" | "file" | "both";
  subLangs: string[];
  writeAutoSubs: boolean;
  convertSubsToSrt: boolean;
  mergeFormat: string | null;
  embedThumbnail: boolean;
  embedMetadata: boolean;
  embedChapters: boolean;
  rateLimit: string | null;
  outputDir: string;
  filenameTemplate: string;
  proxy: string;
  cookiesMode: "none" | "file" | "browser" | "";
  cookiesFile: string;
  cookiesBrowser: string;
  extraArgs: string;
}

// ---------- Download events (mirrors src-tauri/src/ytdlp.rs DownloadEvent) ----------

export type TaskPhase =
  | "queued"
  | "downloading"
  | "processing"
  | "finished"
  | "error"
  | "cancelled";

export interface DownloadEvent {
  id: string;
  kind: "progress" | "status" | "log" | "error" | "finished";
  status?: TaskPhase;
  percent?: number;
  speed?: string;
  eta?: string;
  downloaded?: number;
  total?: number;
  message?: string;
  exitCode?: number;
}

// ---------- yt-dlp info JSON (subset we render) ----------

export interface YtFormat {
  format_id: string;
  format_note?: string;
  ext?: string;
  resolution?: string;
  width?: number | null;
  height?: number | null;
  fps?: number | null;
  vcodec?: string;
  acodec?: string;
  filesize?: number | null;
  filesize_approx?: number | null;
  tbr?: number | null;
  abr?: number | null;
  vbr?: number | null;
  format?: string;
}

export interface YtSubEntry {
  ext: string;
  url?: string;
  name?: string;
}

export interface YtPlaylistEntry {
  id: string;
  title?: string;
  url?: string;
  duration?: number | null;
  webpage_url?: string;
}

export interface YtInfo {
  id: string;
  title: string;
  _type?: string; // "playlist" when playlist
  webpage_url?: string;
  uploader?: string;
  channel?: string;
  duration?: number | null;
  thumbnail?: string;
  description?: string;
  view_count?: number | null;
  upload_date?: string;
  extractor_key?: string;
  formats?: YtFormat[];
  subtitles?: Record<string, YtSubEntry[]>;
  automatic_captions?: Record<string, YtSubEntry[]>;
  entries?: YtPlaylistEntry[];
  playlist_count?: number | null;
}

// ---------- invoke wrappers ----------

export const api = {
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => invoke<void>("save_settings", { settings }),
  detectTools: () => invoke<ResolvedTools>("detect_tools"),
  downloadYtdlp: () => invoke<ToolInfo>("download_ytdlp"),
  fetchInfo: (url: string, flatPlaylist: boolean) =>
    invoke<YtInfo>("fetch_info", { url, flatPlaylist }),
  startDownload: (id: string, spec: DownloadSpec) =>
    invoke<void>("start_download", { id, spec }),
  cancelDownload: (id: string) => invoke<void>("cancel_download", { id }),
};

export function onDownloadEvent(cb: (ev: DownloadEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadEvent>("dsh-download-event", (e) => cb(e.payload));
}

export function onToolDownloadEvent(cb: (ev: ToolDownloadProgress) => void): Promise<UnlistenFn> {
  return listen<ToolDownloadProgress>("dsh-tool-download-event", (e) => cb(e.payload));
}
