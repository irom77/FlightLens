import { beforeEach, expect, it } from "vitest";
import { useWorkspace } from "./workspace";
beforeEach(() =>
  useWorkspace.setState({ documents: [], activeId: null, tab: "Rates" }),
);
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
