<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "./lib/stores.svelte";
  import { t } from "./lib/i18n/i18n.svelte";
  import NewDownload from "./pages/NewDownload.svelte";
  import Queue from "./pages/Queue.svelte";
  import Settings from "./pages/Settings.svelte";

  let ready = $state(false);

  onMount(async () => {
    await store.init();
    ready = true;
  });

  const pendingCount = $derived(
    store.tasks.filter((x) => x.phase === "downloading" || x.phase === "processing" || x.phase === "queued")
      .length,
  );
</script>

<div class="shell">
  <div class="topbar">
    <span class="brand">⬇ {t("app.title")}</span>
    <button class="tab" class:active={store.tab === "new"} onclick={() => (store.tab = "new")}>
      {t("nav.newDownload")}
    </button>
    <button class="tab" class:active={store.tab === "queue"} onclick={() => (store.tab = "queue")}>
      {t("nav.queue")}
      {#if pendingCount > 0}<span class="badge">{pendingCount}</span>{/if}
    </button>
    <button
      class="tab"
      class:active={store.tab === "settings"}
      onclick={() => (store.tab = "settings")}
    >
      {t("nav.settings")}
    </button>
  </div>

  <div class="content">
    <div class="page">
      {#if ready && !store.tools.ytdlp}
        <div class="banner">
          <span class="grow">{t("banner.noYtdlp")}</span>
          <button onclick={() => (store.tab = "settings")}>{t("banner.goSettings")}</button>
        </div>
      {/if}

      {#if store.tab === "new"}
        <NewDownload />
      {:else if store.tab === "queue"}
        <Queue />
      {:else}
        <Settings />
      {/if}
    </div>
  </div>
</div>
