<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api, type DownloadSpec, type YtFormat, type YtInfo } from "../lib/api";
  import { store } from "../lib/stores.svelte";
  import { t, fmtBytes, fmtDuration } from "../lib/i18n/i18n.svelte";

  // ---------- fetch state ----------
  let url = $state("");
  let flatPlaylist = $state(true);
  let fetching = $state(false);
  let fetchError = $state("");
  let info = $state<YtInfo | null>(null);

  // ---------- selection state ----------
  let isPlaylist = $derived(!!info && info._type === "playlist" && !!info.entries?.length);
  let selectedEntries = $state<Set<number>>(new Set());
  let rangeText = $state("");

  let mode = $state<"video" | "audio" | "subs">("video");
  let preset = $state("best");
  let selectedFormatId = $state("");
  let audioFormat = $state("mp3");
  let audioQuality = $state("");
  let mergeFormat = $state("");

  let subMode = $state<"none" | "embed" | "file" | "both">("none");
  let subLangs = $state<Set<string>>(new Set());
  let writeAutoSubs = $state(false);
  let convertSrt = $state(true);

  let embedThumbnail = $state(false);
  let embedMetadata = $state(true);
  let embedChapters = $state(false);
  let rateLimit = $state("");
  let extraArgs = $state("");

  let outputDir = $state("");
  let template = $state("%(title)s.%(ext)s");

  let addedFlash = $state("");

  const videoFormats = $derived(
    (info?.formats ?? []).filter((f) => f.vcodec && f.vcodec !== "none"),
  );
  const audioFormats = $derived(
    (info?.formats ?? []).filter((f) => (!f.vcodec || f.vcodec === "none") && f.acodec && f.acodec !== "none"),
  );
  const manualSubLangs = $derived(Object.keys(info?.subtitles ?? {}));
  const autoSubLangs = $derived(Object.keys(info?.automatic_captions ?? {}));

  async function fetchInfo() {
    if (!url.trim() || fetching) return;
    fetching = true;
    fetchError = "";
    info = null;
    try {
      const result = await api.fetchInfo(url.trim(), flatPlaylist);
      info = result;
      selectedEntries = new Set((result.entries ?? []).map((_, i) => i + 1));
      rangeText = "";
      subLangs = new Set();
      selectedFormatId = "";
      if (result.playlist_count && result.entries && result.entries.length < result.playlist_count && flatPlaylist) {
        // flat scan was truncated by yt-dlp; nothing to do, entries shown as-is
      }
    } catch (e) {
      fetchError = String(e);
    } finally {
      fetching = false;
    }
  }

  function toggleEntry(idx: number) {
    const next = new Set(selectedEntries);
    if (next.has(idx)) next.delete(idx);
    else next.add(idx);
    selectedEntries = next;
    rangeText = "";
  }

  function selectAll(all: boolean) {
    const n = info?.entries?.length ?? 0;
    selectedEntries = all ? new Set(Array.from({ length: n }, (_, i) => i + 1)) : new Set();
    rangeText = "";
  }

  /** Parse "1-10,15" into the selection set. */
  function applyRange() {
    const n = info?.entries?.length ?? 0;
    const next = new Set<number>();
    for (const part of rangeText.split(",")) {
      const p = part.trim();
      if (!p) continue;
      const m = p.match(/^(\d+)(?:-(\d+))?$/);
      if (!m) continue;
      const a = parseInt(m[1], 10);
      const b = m[2] ? parseInt(m[2], 10) : a;
      for (let i = Math.max(1, Math.min(a, b)); i <= Math.min(n, Math.max(a, b)); i++) next.add(i);
    }
    selectedEntries = next;
  }

  function toggleLang(lang: string) {
    const next = new Set(subLangs);
    if (next.has(lang)) next.delete(lang);
    else next.add(lang);
    subLangs = next;
  }

  async function browseOutput() {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir === "string") outputDir = dir;
  }

  function fmtResolution(f: YtFormat): string {
    if (f.resolution) return f.resolution;
    if (f.width && f.height) return `${f.width}x${f.height}`;
    return f.format_note ?? "-";
  }

  function buildSpec(): DownloadSpec {
    const entries = info?.entries ?? [];
    const usePlaylist = isPlaylist && selectedEntries.size > 0;
    const allSelected = selectedEntries.size === entries.length;
    return {
      url: info?.webpage_url ?? url.trim(),
      mode,
      formatId: selectedFormatId || null,
      preset: selectedFormatId ? null : preset,
      audioFormat,
      audioQuality: audioQuality || null,
      allowPlaylist: usePlaylist,
      playlistItems: usePlaylist && !allSelected ? [...selectedEntries].sort((a, b) => a - b).join(",") : null,
      subMode,
      subLangs: [...subLangs],
      writeAutoSubs,
      convertSubsToSrt: convertSrt,
      mergeFormat: mergeFormat || null,
      embedThumbnail,
      embedMetadata,
      embedChapters,
      rateLimit: rateLimit || null,
      outputDir,
      filenameTemplate: template,
      proxy: "",
      cookiesMode: "",
      cookiesFile: "",
      cookiesBrowser: "",
      extraArgs,
    };
  }

  function addToQueue() {
    if (!info) return;
    const spec = buildSpec();
    const title = info.title ?? spec.url;
    store.addTasks([{ title, url: spec.url, spec }]);
    addedFlash = t("new.added", { count: 1 });
    setTimeout(() => (addedFlash = ""), 3000);
    store.tab = "queue";
  }
</script>

<div>
  <div class="row">
    <input
      class="grow"
      type="url"
      placeholder={t("new.urlPlaceholder")}
      bind:value={url}
      onkeydown={(e) => e.key === "Enter" && fetchInfo()}
    />
    <label class="row" style="gap:4px; white-space:nowrap">
      <input type="checkbox" bind:checked={flatPlaylist} />
      {t("new.flatPlaylist")}
    </label>
    <button class="primary" onclick={fetchInfo} disabled={fetching || !url.trim()}>
      {fetching ? t("new.fetching") : t("new.fetch")}
    </button>
  </div>

  {#if fetchError}
    <div class="banner" style="border-color: var(--red); color: var(--red); background: rgba(239,68,68,0.1); margin-top: 12px">
      <span class="grow" style="white-space: pre-wrap">{t("new.error")}: {fetchError}</span>
    </div>
  {/if}

  {#if !info && !fetchError}
    <p class="muted" style="margin-top: 24px">{t("new.needInfo")}</p>
  {/if}

  {#if info}
    <!-- media info -->
    <div class="card row" style="align-items: flex-start; margin-top: 14px; gap: 16px">
      {#if info.thumbnail}
        <img class="thumb" src={info.thumbnail} alt="" referrerpolicy="no-referrer" />
      {/if}
      <div class="grow">
        <div style="font-weight: 600; font-size: 15px; margin-bottom: 6px">{info.title}</div>
        <div class="muted">
          {t("new.info.uploader")}: {info.uploader ?? info.channel ?? "-"} ·
          {t("new.info.extractor")}: {info.extractor_key ?? "-"}
          {#if info.duration != null} · {t("new.info.duration")}: {fmtDuration(info.duration)}{/if}
          {#if info.view_count != null} · {t("new.info.views")}: {info.view_count.toLocaleString()}{/if}
          {#if info.upload_date} · {t("new.info.uploadDate")}: {info.upload_date}{/if}
        </div>
      </div>
    </div>

    <!-- playlist picker -->
    {#if isPlaylist}
      <fieldset>
        <legend>{t("new.playlist.title", { count: info.entries?.length ?? 0 })}</legend>
        <div class="row" style="margin-bottom: 8px">
          <button onclick={() => selectAll(true)}>{t("new.playlist.selectAll")}</button>
          <button onclick={() => selectAll(false)}>{t("new.playlist.selectNone")}</button>
          <input
            class="grow"
            type="text"
            placeholder={t("new.playlist.range")}
            bind:value={rangeText}
            onchange={applyRange}
          />
          <span class="muted">
            {t("new.playlist.selected", { selected: selectedEntries.size, total: info.entries?.length ?? 0 })}
          </span>
        </div>
        <div class="playlist-scroll">
          {#each info.entries ?? [] as entry, i}
            <label>
              <input
                type="checkbox"
                checked={selectedEntries.has(i + 1)}
                onchange={() => toggleEntry(i + 1)}
              />
              <span class="muted">{i + 1}.</span>
              <span class="grow" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                {entry.title ?? entry.id}
              </span>
              {#if entry.duration != null}<span class="muted">{fmtDuration(entry.duration)}</span>{/if}
            </label>
          {/each}
        </div>
      </fieldset>
    {/if}

    <!-- mode -->
    <fieldset>
      <legend>{t("new.mode")}</legend>
      <div class="radio-row">
        <label><input type="radio" bind:group={mode} value="video" /> {t("new.mode.video")}</label>
        <label><input type="radio" bind:group={mode} value="audio" /> {t("new.mode.audio")}</label>
        <label><input type="radio" bind:group={mode} value="subs" /> {t("new.mode.subs")}</label>
      </div>
    </fieldset>

    <!-- format -->
    {#if mode !== "subs"}
      <fieldset>
        <legend>{t("new.format")}</legend>
        {#if mode === "video"}
          <div class="radio-row" style="margin-bottom: 10px">
            {#each [["best", "new.format.best"], ["best1080", "new.format.best1080"], ["best720", "new.format.best720"], ["worst", "new.format.worst"]] as [value, key]}
              <label>
                <input
                  type="radio"
                  bind:group={preset}
                  {value}
                  onchange={() => (selectedFormatId = "")}
                />
                {t(key)}
              </label>
            {/each}
            <span class="muted">·</span>
            <select bind:value={mergeFormat}>
              <option value="">{t("new.mergeFormat")}: yt-dlp</option>
              <option value="mp4">{t("new.mergeFormat")}: mp4</option>
              <option value="mkv">{t("new.mergeFormat")}: mkv</option>
            </select>
          </div>
          <div class="table-scroll">
            <table class="fmt">
              <thead>
                <tr>
                  <th>{t("new.format.table.id")}</th>
                  <th>{t("new.format.table.res")}</th>
                  <th>{t("new.format.table.ext")}</th>
                  <th>{t("new.format.table.fps")}</th>
                  <th>{t("new.format.table.vcodec")}</th>
                  <th>{t("new.format.table.acodec")}</th>
                  <th>{t("new.format.table.size")}</th>
                  <th>{t("new.format.table.tbr")}</th>
                </tr>
              </thead>
              <tbody>
                {#each videoFormats as f}
                  <tr
                    class:selected={selectedFormatId === f.format_id}
                    onclick={() => (selectedFormatId = selectedFormatId === f.format_id ? "" : f.format_id)}
                  >
                    <td>{f.format_id}</td>
                    <td>{fmtResolution(f)}</td>
                    <td>{f.ext ?? "-"}</td>
                    <td>{f.fps ?? "-"}</td>
                    <td>{f.vcodec ?? "-"}</td>
                    <td>{f.acodec ?? "-"}</td>
                    <td>{fmtBytes(f.filesize ?? f.filesize_approx)}</td>
                    <td>{f.tbr ? `${Math.round(f.tbr)}k` : "-"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="row wrap" style="margin-bottom: 10px">
            <label>{t("new.format.audioFormat")}</label>
            <select bind:value={audioFormat}>
              {#each ["mp3", "m4a", "opus", "flac", "wav", "best"] as af}
                <option value={af}>{af}</option>
              {/each}
            </select>
            <label>{t("new.format.audioQuality")}</label>
            <input type="text" style="width: 90px" bind:value={audioQuality} placeholder="0 / 192K" />
          </div>
          <div class="table-scroll">
            <table class="fmt">
              <thead>
                <tr>
                  <th>{t("new.format.table.id")}</th>
                  <th>{t("new.format.table.ext")}</th>
                  <th>{t("new.format.table.acodec")}</th>
                  <th>{t("new.format.table.size")}</th>
                  <th>{t("new.format.table.tbr")}</th>
                </tr>
              </thead>
              <tbody>
                {#each audioFormats as f}
                  <tr
                    class:selected={selectedFormatId === f.format_id}
                    onclick={() => (selectedFormatId = selectedFormatId === f.format_id ? "" : f.format_id)}
                  >
                    <td>{f.format_id}</td>
                    <td>{f.ext ?? "-"}</td>
                    <td>{f.acodec ?? "-"}</td>
                    <td>{fmtBytes(f.filesize ?? f.filesize_approx)}</td>
                    <td>{f.abr ?? f.tbr ? `${Math.round(f.abr ?? f.tbr ?? 0)}k` : "-"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </fieldset>
    {/if}

    <!-- subtitles -->
    {#if mode !== "audio"}
      <fieldset>
        <legend>{t("new.subs")}</legend>
        <div class="radio-row" style="margin-bottom: 10px">
          {#if mode !== "subs"}
            {#each [["none", "new.subs.none"], ["embed", "new.subs.embed"], ["file", "new.subs.file"], ["both", "new.subs.both"]] as [value, key]}
              <label><input type="radio" bind:group={subMode} {value} /> {t(key)}</label>
            {/each}
          {:else}
            <span class="muted">{t("new.subs.file")}</span>
          {/if}
        </div>
        {#if manualSubLangs.length > 0}
          <div class="muted" style="margin-bottom: 4px">{t("new.subs.available")}</div>
          <div class="chips" style="margin-bottom: 8px">
            {#each manualSubLangs as lang}
              <button class="chip-toggle" class:on={subLangs.has(lang)} onclick={() => toggleLang(lang)}>
                {lang}
              </button>
            {/each}
          </div>
        {/if}
        <div class="row wrap">
          <label class="row" style="gap: 4px">
            <input type="checkbox" bind:checked={writeAutoSubs} />
            {t("new.subs.auto")}
            {#if autoSubLangs.length > 0}<span class="muted">({autoSubLangs.length})</span>{/if}
          </label>
          <label class="row" style="gap: 4px">
            <input type="checkbox" bind:checked={convertSrt} />
            {t("new.subs.srt")}
          </label>
        </div>
      </fieldset>
    {/if}

    <!-- advanced -->
    <fieldset>
      <legend>{t("new.advanced")}</legend>
      <div class="row wrap" style="margin-bottom: 8px">
        <label class="row" style="gap: 4px"><input type="checkbox" bind:checked={embedThumbnail} /> {t("new.advanced.thumbnail")}</label>
        <label class="row" style="gap: 4px"><input type="checkbox" bind:checked={embedMetadata} /> {t("new.advanced.metadata")}</label>
        <label class="row" style="gap: 4px"><input type="checkbox" bind:checked={embedChapters} /> {t("new.advanced.chapters")}</label>
      </div>
      <div class="row wrap">
        <label>{t("new.advanced.rateLimit")}</label>
        <input type="text" style="width: 90px" bind:value={rateLimit} placeholder="5M" />
        <label class="grow">{t("new.advanced.extraArgs")}</label>
        <input type="text" class="grow" style="min-width: 260px" bind:value={extraArgs} placeholder="--sleep-requests 1" />
      </div>
    </fieldset>

    <!-- output -->
    <fieldset>
      <legend>{t("new.output")}</legend>
      <div class="row" style="margin-bottom: 8px">
        <label>{t("new.output.dir")}</label>
        <input class="grow" type="text" bind:value={outputDir} placeholder={store.settings.downloadDir || ""} />
        <button onclick={browseOutput}>{t("new.output.browse")}</button>
      </div>
      <div class="row">
        <label>{t("new.output.template")}</label>
        <input class="grow" type="text" bind:value={template} />
      </div>
    </fieldset>

    <div class="row">
      <button class="primary" onclick={addToQueue} disabled={isPlaylist && selectedEntries.size === 0}>
        {t("new.addToQueue")}
      </button>
      {#if addedFlash}<span class="muted">{addedFlash}</span>{/if}
    </div>
  {/if}
</div>
