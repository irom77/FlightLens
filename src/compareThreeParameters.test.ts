import { describe, expect, it } from "vitest";
import type { ConfigDocument, Parameter } from "./bindings/core";
import { compareThreeParameters } from "./compareThreeParameters";

const parameter = (value: number, index = 0): Parameter => ({
  key: "p_roll",
  semanticKey: "p_roll",
  value: { kind: "integer", value },
  rawValue: String(value),
  scope: { kind: "pid", index },
  line: 2,
  valid: true,
  supported: true,
  unit: null,
  packId: "pack",
});
const document = (parameters: Parameter[]): ConfigDocument =>
  ({
    firmware: {
      family: "betaflight",
      version: "4.5.0",
      packId: "pack",
      boardName: null,
      header: null,
    },
    parameters: Object.fromEntries(parameters.map((p, i) => [String(i), p])),
    derived: {},
  }) as ConfigDocument;
const profiles = { pid: 0, rate: 0 };

describe("baseline parameter comparison", () => {
  const compare = (a: ConfigDocument, b: ConfigDocument, c: ConfigDocument) =>
    compareThreeParameters([a, b, c], [profiles, profiles, profiles]);
  it.each([
    [10, 10, 10, "Equal"],
    [10, 20, 10, "Left only"],
    [10, 10, 20, "Right only"],
    [10, 20, 20, "Same change"],
    [10, 20, 30, "Conflict"],
  ])("classifies %s / %s / %s as %s", (a, b, c, expected) => {
    expect(
      compare(
        document([parameter(Number(a))]),
        document([parameter(Number(b))]),
        document([parameter(Number(c))]),
      )[0].status,
    ).toBe(expected);
  });
  it("keeps missing, invalid and unsupported inputs unknown", () => {
    const a = document([parameter(10)]);
    for (const c of [
      document([]),
      document([{ ...parameter(20), valid: false }]),
      document([{ ...parameter(20), supported: false }]),
    ]) {
      expect(compare(a, document([parameter(30)]), c)[0].status).toBe(
        "Unknown",
      );
    }
    expect(compare(document([]), a, a)[0].status).toBe("Unknown");
  });
  it("retains compatibility gates and independent pair statuses", () => {
    const a = document([parameter(10)]);
    const c = document([parameter(20)]);
    c.firmware.version = "2025.12.1";
    const row = compare(a, a, c)[0];
    expect(row.status).toBe("Not comparable");
    expect(row.left).toBe("Equal");
    expect(row.right).toBe("Not comparable");
  });
  it("uses independent profiles and changes the classification with the baseline", () => {
    const a = document([parameter(10), parameter(20, 1)]);
    const b = document([parameter(20)]);
    const c = document([parameter(10)]);
    expect(compare(a, b, c)[0].status).toBe("Left only");
    expect(compare(b, a, c)[0].status).toBe("Same change");
    expect(
      compareThreeParameters(
        [a, b, c],
        [{ pid: 1, rate: 0 }, profiles, profiles],
      )[0].status,
    ).toBe("Right only");
  });
});
