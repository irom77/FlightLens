import { create } from "zustand";
export const themes = ["dark", "light"] as const;
export type Theme = (typeof themes)[number];
const storageKey = "flightlens.theme";
const isTheme = (value: unknown): value is Theme =>
  themes.some((t) => t === value);
// The first run follows the operating system; later runs follow the choice.
export const preferredTheme = (): Theme => {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(storageKey);
  } catch {
    /* Preference storage can be unavailable. */
  }
  if (isTheme(saved)) return saved;
  return typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
};
export const applyTheme = (theme: Theme) => {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.theme = theme;
  // Keep the window chrome hint on the palette itself, not a second copy of it.
  const meta = document.head?.querySelector('meta[name="theme-color"]');
  const background =
    typeof getComputedStyle === "function"
      ? getComputedStyle(root).getPropertyValue("--bg-app").trim()
      : "";
  if (meta && background) meta.setAttribute("content", background);
};
interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggle: () => void;
}
export const useTheme = create<ThemeState>((set, get) => ({
  theme: preferredTheme(),
  setTheme: (theme) => {
    try {
      localStorage.setItem(storageKey, theme);
    } catch {
      /* Preference storage can be unavailable. */
    }
    applyTheme(theme);
    set({ theme });
  },
  toggle: () => get().setTheme(get().theme === "dark" ? "light" : "dark"),
}));
