# Video Downloader

一个基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的跨平台桌面下载器（图形界面），
使用 **Tauri 2 + Rust + Svelte** 构建，界面轻量、中英双语。

A cross-platform desktop GUI for [yt-dlp](https://github.com/yt-dlp/yt-dlp),
built with **Tauri 2 + Rust + Svelte**. Lightweight, bilingual (中文 / English).

---

## 功能 · Features

- 下载前获取媒体信息（标题、时长、缩略图、全部格式列表）
  Fetch full media info before downloading (title, duration, thumbnail, all formats)
- 播放列表解析（快速扁平扫描）、条目勾选或范围选择（如 `1-10,15`）
  Playlist parsing (fast flat scan), per-item checkboxes or range selection
- 视频格式表 + 快捷预设（最佳画质 / ≤1080p / ≤720p / 最小体积）、纯音频提取（mp3/m4a/opus/flac/wav）
  Format table + quality presets, audio-only extraction
- 字幕：列出可用字幕语言、单独下载字幕文件、嵌入视频、自动字幕、一键转 SRT
  Subtitles: list languages, download separately, embed into video, auto captions, convert to SRT
- Cookie 配置：`cookies.txt` 文件 或 直接从浏览器提取（YouTube 高码率必需）
  Cookies: `cookies.txt` file or extract from browser (required for YouTube high-bitrate formats)
- 代理服务器（http / https / socks5）
  Proxy support (http / https / socks5)
- 输出目录选择 + 文件名模板（`-o` template）
  Output directory picker + filename template
- 下载队列：实时进度（百分比 / 速度 / ETA / 已下载大小）、并行数可配、取消 / 重试 / 打开文件夹
  Download queue with live progress, configurable concurrency, cancel / retry / reveal in folder
- 高级：限速、嵌入封面 / 元数据 / 章节、自定义附加 yt-dlp 参数（覆盖全部 yt-dlp 能力）
  Advanced: rate limit, embed thumbnail/metadata/chapters, custom extra yt-dlp args

## 下载 · Download

前往 [Releases](https://github.com/dotfrankruan/video-downloader/releases)。
每个平台 × 架构提供两种变体：

Get binaries from [Releases](https://github.com/dotfrankruan/video-downloader/releases).
Two variants per platform/architecture:

| 变体 Variant | 说明 |
|---|---|
| **lite（精简版）** | 不内置任何二进制。首次运行自动检测系统中的 yt-dlp/ffmpeg，缺失时可一键从 GitHub 下载 yt-dlp（支持自定义镜像与代理，适应大陆网络）。ffmpeg 需自行安装（如 `brew install ffmpeg`）。<br>Bundles nothing. Detects system yt-dlp/ffmpeg; can download yt-dlp at runtime (custom mirror + proxy supported). |
| **full（完整版）** | 内置 yt-dlp + ffmpeg + ffprobe 官方预编译二进制，开箱即用、零依赖，适合无法访问 GitHub 或离线环境。<br>Bundles official prebuilt yt-dlp + ffmpeg + ffprobe. Zero dependencies, works offline. |

支持平台：macOS（arm64 / x86_64）、Windows（x86_64 / arm64）、Linux（x86_64 / arm64，AppImage + .deb）。

### ⚠️ macOS 用户必读（未签名应用）· macOS: unsigned app

本应用仅使用 **ad-hoc 签名**（dmg 内附 `请先阅读 READ-ME-FIRST.txt`）。
首次打开被 Gatekeeper 拦截时，任选其一：

The app is **ad-hoc signed only** (the dmg contains a bilingual
`READ-ME-FIRST.txt`). When Gatekeeper blocks the first launch, pick ONE:

1. 系统设置 → 隐私与安全性 → 「仍要打开」
   System Settings → Privacy & Security → **Open Anyway**
2. 终端移除隔离属性 / remove quarantine:
   ```bash
   xattr -dr com.apple.quarantine "/Applications/Video Downloader.app"
   ```
3. 自行重新签名 / re-sign yourself:
   ```bash
   codesign --force --deep --sign - "/Applications/Video Downloader.app"
   ```

## 使用提示 · Usage tips

- **YouTube 高码率 / 机器人验证**：设置 → Cookie → 选择浏览器自动提取，或导出 `cookies.txt`。
  Settings → Cookies → extract from your browser, or provide a `cookies.txt`.
- **大陆网络下载 yt-dlp 失败**：设置 → 下载镜像地址，填写镜像（需镜像 `github.com` 路径的 gh-proxy 类服务），或勾选「使用代理」。
  If downloading yt-dlp fails behind restricted networks, set a mirror base URL and/or enable the proxy for tool downloads.
- **嵌入字幕需要封装合并**：选字幕「嵌入」时建议合并格式选 `mp4` 或 `mkv`。
  Embedding subtitles requires merging; pick `mp4` or `mkv` as merge container.

## 从源码构建 · Build from source

```bash
# 依赖 / prerequisites: Rust (rustup), Node.js + pnpm, Xcode CLT (macOS)
pnpm install

# 本地快速迭代（推荐，无需等 CI）/ Fast local loop (no CI needed):
make help        # 列出所有目标 / list targets
make test        # cargo test + 前端测试 + svelte-check
make dev         # 开发模式（热更新）/ dev mode with hot reload
make dmg         # lite .app + dmg（含未签名告知文件）
make dmg-full    # full .app + dmg（内置 yt-dlp/ffmpeg/ffprobe）
make print-tools # 验证 full 版的工具检测 / verify tool detection
```

手工命令 / manual equivalent:

```bash
# 开发模式 / dev mode (hot reload)
pnpm tauri dev

# 本机 lite 构建 / local lite build
pnpm tauri build

# 本机 full 构建（先下载预编译工具链，不编译 ffmpeg）
# local full build (downloads prebuilt tools first, nothing compiled)
scripts/fetch-tools.sh aarch64-apple-darwin   # 或你的目标 triple / or your target triple
pnpm tauri build --config src-tauri/tauri.full.conf.json

# macOS dmg（含未签名告知文件）/ dmg with the unsigned-app notice
scripts/make-dmg.sh "src-tauri/target/release/bundle/macos/Video Downloader.app" "Video-Downloader.dmg"
```

Release 由 GitHub Actions 矩阵构建（三平台 × 双架构 × lite/full），见
[.github/workflows/release.yml](.github/workflows/release.yml)。

Releases are built by a GitHub Actions matrix (3 platforms × 2 archs × lite/full),
see [.github/workflows/release.yml](.github/workflows/release.yml)。

## 许可证 · License

- 本项目代码：**[Apache-2.0](LICENSE)**
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)：**Unlicense**（公有领域）
- [ffmpeg](https://ffmpeg.org)：**GPLv3**（预编译二进制）。本应用将其作为**独立子进程**调用
  （GPL FAQ 所称 "mere aggregation"），不构成衍生作品，故本应用代码保持 Apache-2.0。
  完整版安装包内附 GPL 全文与源码获取方式（`LICENSES/` 目录）。
- 详见 / details: [src-tauri/LICENSES/THIRD-PARTY-NOTICES.md](src-tauri/LICENSES/THIRD-PARTY-NOTICES.md)

## 免责 · Disclaimer

请遵守目标网站的服务条款与当地法律，仅下载你有权下载的内容。
Respect the terms of service of the sites you download from and your local
laws. Only download content you have the rights to.
