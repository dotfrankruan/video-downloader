import {
  api,
  defaultSettings,
  onDownloadEvent,
  type AppInfo,
  type AppSettings,
  type DownloadEvent,
  type DownloadSpec,
  type ResolvedTools,
  type TaskPhase,
} from "./api";
import { applyLanguageSetting } from "./i18n/i18n.svelte";

export interface Task {
  id: string;
  title: string;
  url: string;
  spec: DownloadSpec;
  phase: TaskPhase;
  percent: number;
  speed: string;
  eta: string;
  downloaded: number | null;
  total: number | null;
  logs: string[];
  addedAt: number;
}

const MAX_LOG_LINES = 200;

class AppStore {
  settings = $state<AppSettings>({ ...defaultSettings });
  tools = $state<ResolvedTools>({
    ytdlp: { info: null, customInvalid: false, attempts: [] },
    ffmpeg: { info: null, customInvalid: false, attempts: [] },
  });
  appInfo = $state<AppInfo>({ version: "", variant: "lite", toolsDir: "" });
  tasks = $state<Task[]>([]);
  tab = $state<"new" | "queue" | "settings">("new");
  settingsSavedFlash = $state(false);
  settingsError = $state("");

  get activeCount(): number {
    return this.tasks.filter((t) => t.phase === "downloading" || t.phase === "processing").length;
  }

  async init(): Promise<void> {
    try {
      this.settings = await api.getSettings();
    } catch {
      /* keep defaults */
    }
    applyLanguageSetting(this.settings.language);
    try {
      this.appInfo = await api.appInfo();
    } catch {
      /* keep defaults */
    }
    await this.refreshTools();
    await onDownloadEvent((ev) => this.handleEvent(ev));
  }

  async refreshTools(): Promise<void> {
    try {
      this.tools = await api.detectTools();
    } catch {
      /* ignore */
    }
  }

  async saveSettings(): Promise<void> {
    this.settingsError = "";
    try {
      await api.saveSettings($state.snapshot(this.settings) as AppSettings);
    } catch (e) {
      this.settingsError = String(e);
      return;
    }
    applyLanguageSetting(this.settings.language);
    await this.refreshTools();
    this.settingsSavedFlash = true;
    setTimeout(() => (this.settingsSavedFlash = false), 2000);
  }

  addTasks(items: Array<{ title: string; url: string; spec: DownloadSpec }>): void {
    for (const item of items) {
      this.tasks.push({
        id: crypto.randomUUID(),
        title: item.title,
        url: item.url,
        spec: item.spec,
        phase: "queued",
        percent: 0,
        speed: "",
        eta: "",
        downloaded: null,
        total: null,
        logs: [],
        addedAt: Date.now(),
      });
    }
    this.pumpQueue();
  }

  /** Start queued tasks until the concurrency limit is reached. */
  pumpQueue(): void {
    const limit = Math.max(1, this.settings.maxConcurrent || 1);
    let running = this.activeCount;
    for (const task of this.tasks) {
      if (running >= limit) break;
      if (task.phase !== "queued") continue;
      task.phase = "downloading";
      running++;
      api.startDownload(task.id, $state.snapshot(task.spec) as DownloadSpec).catch((e) => {
        task.phase = "error";
        this.pushLog(task, String(e));
        this.pumpQueue();
      });
    }
  }

  private pushLog(task: Task, line: string): void {
    task.logs.push(line);
    if (task.logs.length > MAX_LOG_LINES) {
      task.logs.splice(0, task.logs.length - MAX_LOG_LINES);
    }
  }

  private handleEvent(ev: DownloadEvent): void {
    const task = this.tasks.find((t) => t.id === ev.id);
    if (!task) return;
    switch (ev.kind) {
      case "progress":
        if (ev.percent != null) task.percent = ev.percent;
        if (ev.speed) task.speed = ev.speed;
        if (ev.eta) task.eta = ev.eta;
        if (ev.downloaded != null) task.downloaded = ev.downloaded;
        if (ev.total != null) task.total = ev.total;
        if (ev.status) task.phase = ev.status;
        break;
      case "status":
        if (ev.status) task.phase = ev.status;
        if (ev.message) this.pushLog(task, ev.message);
        break;
      case "log":
        if (ev.message) this.pushLog(task, ev.message);
        break;
      case "error":
        if (ev.message) this.pushLog(task, `ERROR: ${ev.message}`);
        break;
      case "finished": {
        task.phase = ev.status ?? (ev.exitCode === 0 ? "finished" : "error");
        if (task.phase === "finished") task.percent = 100;
        if (ev.message && task.phase === "error") {
          this.pushLog(task, ev.message);
        }
        this.pumpQueue();
        break;
      }
    }
  }

  async cancel(id: string): Promise<void> {
    const task = this.tasks.find((t) => t.id === id);
    if (!task) return;
    if (task.phase === "queued") {
      task.phase = "cancelled";
      return;
    }
    try {
      await api.cancelDownload(id);
    } catch {
      /* process may have already exited */
    }
  }

  retry(id: string): void {
    const task = this.tasks.find((t) => t.id === id);
    if (!task) return;
    task.phase = "queued";
    task.percent = 0;
    task.speed = "";
    task.eta = "";
    task.logs = [];
    this.pumpQueue();
  }

  remove(id: string): void {
    const task = this.tasks.find((t) => t.id === id);
    if (task && (task.phase === "downloading" || task.phase === "processing")) {
      void this.cancel(id);
    }
    this.tasks = this.tasks.filter((t) => t.id !== id);
  }

  clearFinished(): void {
    this.tasks = this.tasks.filter(
      (t) => t.phase === "downloading" || t.phase === "processing" || t.phase === "queued",
    );
  }
}

export const store = new AppStore();
