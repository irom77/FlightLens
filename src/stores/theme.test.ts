import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { preferredTheme, useTheme } from "./theme";
const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  });
  vi.stubGlobal("document", { documentElement: { dataset: {} } });
  useTheme.setState({ theme: "dark" });
});
afterEach(() => vi.unstubAllGlobals());
it("toggles between the two modes and remembers the choice", () => {
  useTheme.getState().toggle();
  expect(useTheme.getState().theme).toBe("light");
  expect(store.get("flightlens.theme")).toBe("light");
  expect(document.documentElement.dataset.theme).toBe("light");
  useTheme.getState().toggle();
  expect(useTheme.getState().theme).toBe("dark");
  expect(store.get("flightlens.theme")).toBe("dark");
});
it("prefers a stored choice and ignores unknown values", () => {
  store.set("flightlens.theme", "light");
  expect(preferredTheme()).toBe("light");
  store.set("flightlens.theme", "solarized");
  expect(preferredTheme()).toBe("dark");
});
it("falls back to dark when preference storage is unavailable", () => {
  vi.stubGlobal("localStorage", {
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("denied");
    },
  });
  expect(preferredTheme()).toBe("dark");
  expect(() => useTheme.getState().setTheme("light")).not.toThrow();
  expect(useTheme.getState().theme).toBe("light");
});
