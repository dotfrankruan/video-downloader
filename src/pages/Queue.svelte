<script lang="ts">
  import { openPath } from "@tauri-apps/plugin-opener";
  import { store, type Task } from "../lib/stores.svelte";
  import { t, fmtBytes } from "../lib/i18n/i18n.svelte";

  let expandedLogs = $state<Set<string>>(new Set());

  function toggleLog(id: string) {
    const next = new Set(expandedLogs);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedLogs = next;
  }

  function phaseLabel(phase: Task["phase"]): string {
    return t(`queue.${phase}`);
  }

  async function openFolder(task: Task) {
    const dir = task.spec.outputDir || store.settings.downloadDir;
    if (dir) {
      try {
        await openPath(dir);
      } catch {
        /* folder may not exist yet */
      }
    }
  }
</script>

<div>
  <div class="row" style="margin-bottom: 12px">
    <span class="muted grow">
      {t("queue.concurrent", { n: store.settings.maxConcurrent })} ·
      {t("queue.active")}: {store.activeCount}
    </span>
    <button onclick={() => store.clearFinished()}>{t("queue.clearFinished")}</button>
  </div>

  {#if store.tasks.length === 0}
    <p class="muted">{t("queue.empty")}</p>
  {/if}

  <div style="display: flex; flex-direction: column; gap: 10px">
    {#each [...store.tasks].reverse() as task (task.id)}
      <div class="card">
        <div class="row" style="margin-bottom: 6px">
          <span class="chip {task.phase}">{phaseLabel(task.phase)}</span>
          <span class="grow" style="font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
            {task.title}
          </span>
          <span class="muted">
            {#if task.phase === "downloading" && task.speed}
              {task.speed}{#if task.eta} · ETA {task.eta}{/if}
            {/if}
          </span>
        </div>

        <div
          class="progress"
          class:done={task.phase === "finished"}
          class:error={task.phase === "error"}
          style="margin-bottom: 6px"
        >
          <div style="width: {Math.min(100, task.percent)}%"></div>
        </div>

        <div class="row">
          <span class="muted">
            {task.percent.toFixed(1)}%
            {#if task.downloaded != null}
              · {fmtBytes(task.downloaded)}{#if task.total != null} / {fmtBytes(task.total)}{/if}
            {/if}
          </span>
          <span class="grow"></span>
          <button onclick={() => toggleLog(task.id)}>{t("queue.showLog")}</button>
          {#if task.phase === "downloading" || task.phase === "processing" || task.phase === "queued"}
            <button class="danger" onclick={() => store.cancel(task.id)}>{t("queue.cancel")}</button>
          {:else}
            {#if task.phase === "error" || task.phase === "cancelled"}
              <button onclick={() => store.retry(task.id)}>{t("queue.retry")}</button>
            {/if}
            {#if task.phase === "finished"}
              <button onclick={() => openFolder(task)}>{t("queue.openFolder")}</button>
            {/if}
            <button onclick={() => store.remove(task.id)}>{t("queue.remove")}</button>
          {/if}
        </div>

        {#if expandedLogs.has(task.id)}
          <div class="logbox" style="margin-top: 8px">{task.logs.join("\n") || "…"}</div>
        {/if}
      </div>
    {/each}
  </div>
</div>
