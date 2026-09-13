import { expect, it } from "vitest";
import type { ConfigDocument } from "../bindings/core";
import { defaultProfiles, restoredProfiles, useSelections } from "./selections";

const doc = {
  id: "a",
  title: "Quad",
  rateProfiles: [0, 2],
  pidProfiles: [1, 3],
  parameters: {},
  derived: {},
} as unknown as ConfigDocument;

it("preserves valid nonzero profiles and reports unavailable saved profiles", () => {
  expect(restoredProfiles(doc, { rate: 2, pid: 3 })).toEqual({
    profiles: { rate: 2, pid: 3 },
    warnings: [],
  });
  const missing = restoredProfiles(doc, { rate: 7, pid: 9 });
  expect(missing.profiles).toEqual(defaultProfiles(doc));
  expect(missing.warnings).toHaveLength(2);
  expect(missing.warnings[0]).toContain(
    "profile 8 unavailable; showing profile 1",
  );
});

it("keeps independent profiles per backup across comparison changes", () => {
  useSelections.setState({ profiles: {}, comparison: null });
  useSelections.getState().setProfile(doc, "rate", 2);
  useSelections.getState().setProfile(doc, "pid", 3);
  useSelections.getState().setProfile({ ...doc, id: "b" }, "rate", 0);
  useSelections
    .getState()
    .setComparison({ documents: ["b", "a"], baseline: null });
  useSelections.getState().setComparison(null);
  expect(useSelections.getState().profiles).toEqual({
    a: { rate: 2, pid: 3 },
    b: { rate: 0, pid: 1 },
  });
});
