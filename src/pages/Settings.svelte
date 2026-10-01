<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api, onToolDownloadEvent, type ToolInfo } from "../lib/api";
  import { store } from "../lib/stores.svelte";
  import { t, applyLanguageSetting } from "../lib/i18n/i18n.svelte";

  let toolDlPercent = $state<number | null>(null);
  let toolDlDone = $state("");
  let toolDlError = $state("");

  const browsers = ["safari", "chrome", "firefox", "edge", "brave", "opera", "vivaldi", "chromium"];

  onMount(() => {
    let unlisten: (() => void) | undefined;
    onToolDownloadEvent((ev) => {
      if (ev.done) {
        toolDlPercent = null;
        void store.refreshTools();
        return;
      }
      if (ev.total) {
        toolDlPercent = Math.round((ev.downloaded / ev.total) * 100);
      }
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  async function downloadYtdlp() {
    toolDlError = "";
    toolDlDone = "";
    toolDlPercent = 0;
    try {
      const info: ToolInfo = await api.downloadYtdlp();
      toolDlDone = t("settings.tools.downloadDone", { path: info.path });
    } catch (e) {
      toolDlError = String(e);
    } finally {
      toolDlPercent = null;
    }
  }

  async function browseFile(target: "cookies" | "ytdlp" | "ffmpeg") {
    const path = await openDialog({ directory: false, multiple: false });
    if (typeof path !== "string") return;
    if (target === "cookies") store.settings.cookiesFile = path;
    else if (target === "ytdlp") store.settings.ytdlpPath = path;
    else store.settings.ffmpegPath = path;
  }

  async function browseDir() {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir === "string") store.settings.downloadDir = dir;
  }

  function toolStatus(info: ToolInfo | null): string {
    if (!info) return t("settings.tools.notFound");
    return `${info.version} (${info.source})`;
  }
</script>

<div>
  <div class="form-grid">
    <label>{t("settings.language")}</label>
    <select
      bind:value={store.settings.language}
      style="width: 220px"
      onchange={() => applyLanguageSetting(store.settings.language)}
    >
      <option value="system">{t("settings.language.system")}</option>
      <option value="en">English</option>
      <option value="zh-CN">简体中文</option>
    </select>
  </div>

  <div class="section-title">{t("settings.tools")}</div>
  <fieldset>
    <legend>{t("settings.tools")}</legend>
    <div class="form-grid">
      <label>{t("settings.tools.ytdlp")}</label>
      <span class="muted">{toolStatus(store.tools.ytdlp)}</span>

      <label>{t("settings.tools.ffmpeg")}</label>
      <span class="muted">{toolStatus(store.tools.ffmpeg)}</span>

      <label>{t("settings.tools.path")} · yt-dlp</label>
      <div class="row">
        <input class="grow" type="text" bind:value={store.settings.ytdlpPath} placeholder="/opt/homebrew/bin/yt-dlp" />
        <button onclick={() => browseFile("ytdlp")}>{t("settings.browse")}</button>
      </div>

      <label>{t("settings.tools.path")} · ffmpeg</label>
      <div class="row">
        <input class="grow" type="text" bind:value={store.settings.ffmpegPath} placeholder="/opt/homebrew/bin/ffmpeg" />
        <button onclick={() => browseFile("ffmpeg")}>{t("settings.browse")}</button>
      </div>

      <label>{t("settings.tools.mirror")}</label>
      <div>
        <input style="width: 100%" type="text" bind:value={store.settings.ytdlpMirror} placeholder="https://github.com" />
        <div class="muted" style="margin-top: 4px">{t("settings.tools.mirrorHint")}</div>
      </div>

      <label></label>
      <div class="row wrap">
        <button onclick={() => store.refreshTools()}>{t("settings.tools.detect")}</button>
        <button class="primary" onclick={downloadYtdlp} disabled={toolDlPercent !== null}>
          {toolDlPercent !== null
            ? t("settings.tools.downloading", { percent: toolDlPercent })
            : t("settings.tools.download")}
        </button>
        <label class="row" style="gap: 4px">
          <input type="checkbox" bind:checked={store.settings.downloadViaProxy} />
          {t("settings.tools.viaProxy")}
        </label>
      </div>
    </div>
    {#if toolDlDone}<div class="muted" style="margin-top: 8px; color: var(--green)">{toolDlDone}</div>{/if}
    {#if toolDlError}<div class="muted" style="margin-top: 8px; color: var(--red)">{toolDlError}</div>{/if}
    <div class="muted" style="margin-top: 8px">{t("settings.tools.ffmpegHint")}</div>
  </fieldset>

  <div class="section-title">{t("settings.network")}</div>
  <fieldset>
    <legend>{t("settings.network")}</legend>
    <div class="form-grid">
      <label>{t("settings.proxy")}</label>
      <div>
        <input style="width: 100%" type="text" bind:value={store.settings.proxy} placeholder="http://127.0.0.1:7890" />
        <div class="muted" style="margin-top: 4px">{t("settings.proxyHint")}</div>
      </div>
    </div>
  </fieldset>

  <fieldset>
    <legend>{t("settings.cookies")}</legend>
    <div class="form-grid">
      <label>{t("settings.cookies.mode")}</label>
      <div class="radio-row">
        <label><input type="radio" bind:group={store.settings.cookiesMode} value="none" /> {t("settings.cookies.none")}</label>
        <label><input type="radio" bind:group={store.settings.cookiesMode} value="file" /> {t("settings.cookies.file")}</label>
        <label><input type="radio" bind:group={store.settings.cookiesMode} value="browser" /> {t("settings.cookies.browser")}</label>
      </div>

      {#if store.settings.cookiesMode === "file"}
        <label>{t("settings.cookies.filePath")}</label>
        <div class="row">
          <input class="grow" type="text" bind:value={store.settings.cookiesFile} placeholder="/path/to/cookies.txt" />
          <button onclick={() => browseFile("cookies")}>{t("settings.browse")}</button>
        </div>
      {/if}

      {#if store.settings.cookiesMode === "browser"}
        <label>{t("settings.cookies.browserName")}</label>
        <select bind:value={store.settings.cookiesBrowser} style="width: 220px">
          <option value="">—</option>
          {#each browsers as b}
            <option value={b}>{b}</option>
          {/each}
        </select>
      {/if}
    </div>
    <div class="muted" style="margin-top: 8px">{t("settings.cookies.hint")}</div>
  </fieldset>

  <div class="section-title">{t("settings.output")}</div>
  <fieldset>
    <legend>{t("settings.output")}</legend>
    <div class="form-grid">
      <label>{t("settings.output.dir")}</label>
      <div class="row">
        <input class="grow" type="text" bind:value={store.settings.downloadDir} />
        <button onclick={browseDir}>{t("settings.browse")}</button>
      </div>

      <label>{t("settings.output.template")}</label>
      <div>
        <div class="row">
          <input class="grow" type="text" bind:value={store.settings.filenameTemplate} />
          <button onclick={() => (store.settings.filenameTemplate = "%(title)s.%(ext)s")}>
            {t("settings.output.templateReset")}
          </button>
        </div>
        <div class="muted" style="margin-top: 4px">{t("settings.output.templateHint")}</div>
      </div>
    </div>
  </fieldset>

  <div class="section-title">{t("settings.download")}</div>
  <fieldset>
    <legend>{t("settings.download")}</legend>
    <div class="form-grid">
      <label>{t("settings.concurrent")}</label>
      <input
        type="number"
        min="1"
        max="10"
        style="width: 90px"
        bind:value={store.settings.maxConcurrent}
      />

      <label>{t("settings.extraArgs")}</label>
      <div>
        <input style="width: 100%" type="text" bind:value={store.settings.extraArgs} placeholder="--sleep-requests 1" />
        <div class="muted" style="margin-top: 4px">{t("settings.extraArgsHint")}</div>
      </div>
    </div>
  </fieldset>

  <div class="row" style="margin-top: 16px">
    <button class="primary" onclick={() => store.saveSettings()}>{t("settings.save")}</button>
    {#if store.settingsSavedFlash}
      <span style="color: var(--green)">{t("settings.saved")}</span>
    {/if}
    {#if store.settingsError}
      <span style="color: var(--red)">{store.settingsError}</span>
    {/if}
  </div>
</div>
