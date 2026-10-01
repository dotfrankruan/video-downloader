// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import TestProbe from "./i18n/TestProbe.svelte";

// Mock the Tauri IPC layer before importing the store.
const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(async (cmd: string) => {
    if (cmd === "get_settings") {
      return {
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
    }
    if (cmd === "detect_tools") return { ytdlp: null, ffmpeg: null };
    return null;
  }),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

import { store } from "./stores.svelte";
import { getLocale } from "./i18n/i18n.svelte";

describe("store language wiring", () => {
  beforeEach(() => {
    invokeMock.mockClear();
  });

  it("saveSettings applies the selected language immediately", async () => {
    render(TestProbe);
    store.settings.language = "en";
    await store.saveSettings();
    await tick();
    expect(getLocale()).toBe("en");
    expect(screen.getByTestId("label").textContent).toBe("Queue");

    store.settings.language = "zh-CN";
    await store.saveSettings();
    await tick();
    expect(getLocale()).toBe("zh-CN");
    expect(screen.getByTestId("label").textContent).toBe("下载队列");

    expect(invokeMock).toHaveBeenCalledWith("save_settings", expect.anything());
  });
});
