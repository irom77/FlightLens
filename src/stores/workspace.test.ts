import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { savedActive, useWorkspace } from "./workspace";
const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  });
  useWorkspace.setState({ documents: [], activeId: null, tab: "Rates" });
});
afterEach(() => vi.unstubAllGlobals());
it("deduplicates identical snapshots by hash", () => {
  const d = { id: "hash-a", title: "one", family: "betaflight" };
  useWorkspace.getState().add(d);
  useWorkspace.getState().add(d);
  expect(useWorkspace.getState().documents).toHaveLength(1);
});
it("closing the active document selects an existing document", () => {
  for (const id of ["a", "b"])
    useWorkspace.getState().add({ id, title: id, family: "betaflight" });
  useWorkspace.getState().close("b");
  expect(useWorkspace.getState().activeId).toBe("a");
  useWorkspace.getState().close("a");
  expect(useWorkspace.getState().activeId).toBeNull();
});
it("stores only navigation metadata, not backup text or settings", () => {
  useWorkspace.getState().add({ id: "a", title: "one", family: "betaflight" });
  expect(Object.keys(useWorkspace.getState().documents[0]).sort()).toEqual([
    "family",
    "id",
    "title",
  ]);
});
it("remembers which document was in front, and forgets it when closed", () => {
  for (const id of ["a", "b"])
    useWorkspace.getState().add({ id, title: id, family: "betaflight" });
  expect(savedActive()).toBe("b");
  useWorkspace.getState().activate("a");
  expect(savedActive()).toBe("a");
  useWorkspace.getState().close("a");
  expect(savedActive()).toBe("b");
  useWorkspace.getState().close("b");
  expect(savedActive()).toBeNull();
});
it("persists an identifier only, never the backup behind it", () => {
  useWorkspace.getState().add({ id: "a", title: "one", family: "betaflight" });
  expect([...store]).toEqual([["flightlens.active", "a"]]);
});
it("survives preference storage being unavailable", () => {
  vi.stubGlobal("localStorage", {
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("denied");
    },
    removeItem: () => {
      throw new Error("denied");
    },
  });
  expect(() =>
    useWorkspace
      .getState()
      .add({ id: "a", title: "one", family: "betaflight" }),
  ).not.toThrow();
  expect(useWorkspace.getState().activeId).toBe("a");
  expect(savedActive()).toBeNull();
});
