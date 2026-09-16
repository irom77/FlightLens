import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { savedActive, useWorkspace } from "./workspace";
const artifact = (id: string, title = id) => ({
  kind: "recognized" as const,
  document: { id, title, family: "betaflight", message: "fixture" },
});
const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  });
  useWorkspace.setState({
    documents: [],
    artifacts: new Map(),
    activeId: null,
    tab: "Rates",
  });
});
afterEach(() => vi.unstubAllGlobals());
it("deduplicates identical snapshots by hash", () => {
  const d = artifact("hash-a", "one");
  useWorkspace.getState().add(d);
  useWorkspace.getState().add(d);
  expect(useWorkspace.getState().documents).toHaveLength(1);
});
it("closing the active document selects an existing document", () => {
  for (const id of ["a", "b"]) useWorkspace.getState().add(artifact(id));
  useWorkspace.getState().close("b");
  expect(useWorkspace.getState().activeId).toBe("a");
  useWorkspace.getState().close("a");
  expect(useWorkspace.getState().activeId).toBeNull();
});
it("keeps sidebar summaries separate from in-memory artifacts", () => {
  useWorkspace.getState().add(artifact("a", "one"));
  expect(Object.keys(useWorkspace.getState().documents[0]).sort()).toEqual([
    "family",
    "id",
    "title",
  ]);
});
it("remembers which document was in front, and forgets it when closed", () => {
  for (const id of ["a", "b"]) useWorkspace.getState().add(artifact(id));
  expect(savedActive()).toBe("b");
  useWorkspace.getState().activate("a");
  expect(savedActive()).toBe("a");
  useWorkspace.getState().close("a");
  expect(savedActive()).toBe("b");
  useWorkspace.getState().close("b");
  expect(savedActive()).toBeNull();
});
it("persists an identifier only, never the backup behind it", () => {
  useWorkspace.getState().add(artifact("a", "one"));
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
  expect(() => useWorkspace.getState().add(artifact("a", "one"))).not.toThrow();
  expect(useWorkspace.getState().activeId).toBe("a");
  expect(savedActive()).toBeNull();
});

it("publishes replacement artifacts and closes them atomically with their summaries", () => {
  const snapshots: ReturnType<typeof useWorkspace.getState>[] = [];
  const unsubscribe = useWorkspace.subscribe((state) => snapshots.push(state));
  const original = artifact("a", "original");
  const replacement = artifact("a", "replacement");
  useWorkspace.getState().add(original);
  useWorkspace.getState().add(replacement);
  useWorkspace.getState().close("a");
  unsubscribe();
  expect(snapshots).toHaveLength(3);
  expect(snapshots[0].artifacts.get("a")).toBe(original);
  expect(snapshots[1].artifacts.get("a")).toBe(replacement);
  expect(snapshots[1].documents).toEqual(
    [replacement.document].map(({ id, title, family }) => ({
      id,
      title,
      family,
    })),
  );
  expect(snapshots[2].artifacts.size).toBe(0);
  expect(snapshots[2].documents).toEqual([]);
  expect(snapshots[2].activeId).toBeNull();
});
