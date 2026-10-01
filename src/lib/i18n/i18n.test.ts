// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import TestProbe from "./TestProbe.svelte";
import { applyLanguageSetting, t, getLocale } from "./i18n.svelte";

describe("i18n", () => {
  it("translates in both locales", () => {
    applyLanguageSetting("en");
    expect(t("nav.queue")).toBe("Queue");
    applyLanguageSetting("zh-CN");
    expect(t("nav.queue")).toBe("下载队列");
  });

  it("re-renders components when the locale changes", async () => {
    applyLanguageSetting("en");
    render(TestProbe);
    expect(screen.getByTestId("label").textContent).toBe("Queue");

    applyLanguageSetting("zh-CN");
    await tick();
    expect(screen.getByTestId("label").textContent).toBe("下载队列");
  });

  it("falls back to system detection for 'system'", () => {
    applyLanguageSetting("system");
    expect(["en", "zh-CN"]).toContain(getLocale());
  });
});
