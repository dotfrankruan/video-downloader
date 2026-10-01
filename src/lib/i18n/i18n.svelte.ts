import en from "./en.json";
import zhCN from "./zh-CN.json";

export type Locale = "en" | "zh-CN";

const dicts: Record<Locale, Record<string, string>> = { en, "zh-CN": zhCN };

export function detectSystemLocale(): Locale {
  const lang = (navigator.language || "en").toLowerCase();
  return lang.startsWith("zh") ? "zh-CN" : "en";
}

// Svelte 5 module-level state: components calling t() during render
// re-evaluate automatically when `current` changes.
let current = $state<Locale>(detectSystemLocale());

export function getLocale(): Locale {
  return current;
}

/** Apply a settings value: "system" | "en" | "zh-CN". */
export function applyLanguageSetting(value: string): void {
  current = value === "en" || value === "zh-CN" ? value : detectSystemLocale();
}

export function t(key: string, vars?: Record<string, string | number>): string {
  let s = dicts[current][key] ?? dicts.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}

/** Human-readable byte size. */
export function fmtBytes(n: number | null | undefined): string {
  if (n == null || !isFinite(n) || n <= 0) return t("common.bytes.na");
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

/** Seconds -> h:mm:ss */
export function fmtDuration(sec: number | null | undefined): string {
  if (sec == null || !isFinite(sec)) return "-";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return `${h > 0 ? h + ":" : ""}${mm}:${String(r).padStart(2, "0")}`;
}
