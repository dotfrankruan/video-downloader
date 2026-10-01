# Third-Party Notices / 第三方声明

This application aggregates (but does not link against) the following
third-party programs. "Full" builds bundle their official prebuilt
binaries; "lite" builds download or locate them at runtime instead.

本应用以「聚合」方式使用以下第三方程序（不作为库链接）。「完整版」
安装包内置其官方预编译二进制；「精简版」则在运行时下载或检测。

---

## yt-dlp

- Project / 项目: https://github.com/yt-dlp/yt-dlp
- License / 许可证: **The Unlicense** (public domain / 公有领域)
- Full text / 全文: `yt-dlp-LICENSE.txt`
- Invoked as a separate child process / 以独立子进程方式调用。

## ffmpeg / ffprobe

- Project / 项目: https://ffmpeg.org
- License / 许可证: **GNU GPLv3** (the prebuilt binaries we ship are
  GPL builds; full text in `ffmpeg-COPYING.GPLv3.txt`)
- Binary sources / 二进制来源:
  - Windows / Linux: https://github.com/yt-dlp/FFmpeg-Builds
  - macOS: https://ffmpeg.martin-riedl.de
- Corresponding source code / 对应源代码:
  - https://github.com/FFmpeg/FFmpeg (upstream)
  - Build scripts: https://github.com/yt-dlp/FFmpeg-Builds and
    https://ffmpeg.martin-riedl.de respectively
- ffmpeg is executed as a **separate child process** ("mere
  aggregation" per the GPL FAQ). It is not linked into this
  application, so the application's own code remains Apache-2.0.
  ffmpeg 以独立子进程方式执行（GPL FAQ 所称的「纯聚合」），
  未与本应用链接，因此本应用自有代码仍为 Apache-2.0。

## Frameworks / 框架

- Tauri — MIT / Apache-2.0 — https://tauri.app
- Svelte — MIT — https://svelte.dev
- Vite — MIT — https://vite.dev
