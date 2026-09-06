import { describe, expect, it } from "vitest";
import { defaultProfile } from "./App";

describe("defaultProfile", () => {
  it("opens profile 1 when it is present", () => {
    expect(defaultProfile([0, 1, 2])).toBe(0);
  });

  it("uses the first discovered profile when profile 1 is absent", () => {
    expect(defaultProfile([1, 2])).toBe(1);
    expect(defaultProfile([])).toBe(0);
  });
});
