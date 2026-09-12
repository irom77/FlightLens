import { describe, expect, it } from "vitest";
import type { ConfigDocument, Mode } from "./bindings/core";
import { compareModes } from "./compareModes";
const mode = (index = 0): Mode => ({
  index,
  name: `Mode ${index}`,
  modeId: 0,
  channel: 0,
  channelAssigned: true,
  start: 1700,
  end: 2100,
  logic: 0,
  linked: 0,
  line: 1,
});
const document = (modes: Mode[]): ConfigDocument =>
  ({
    modes,
    firmware: {
      family: "Betaflight",
      version: "4.5.0",
      packId: "betaflight-4.5.0-schema-1",
    },
  }) as ConfigDocument;
const status = (a: Mode, b: Mode) =>
  compareModes(document([a]), document([b]))[0].status;
describe("mode-assignment comparison", () => {
  it("matches indexs independently of order and display names, keeping missing modes unknown", () => {
    const a = document([mode(20), mode(0)]),
      b = document([mode(1), { ...mode(0), name: "ARM" }]);
    expect(compareModes(a, b).map((row) => [row.index, row.status])).toEqual([
      [0, "Equal"],
      [1, "Unknown"],
      [20, "Unknown"],
    ]);
    expect(compareModes(b, a).map((row) => row.status)).toEqual(
      compareModes(a, b).map((row) => row.status),
    );
    expect(compareModes(document([]), document([]))).toEqual([]);
  });
  it("compares each declared field, preserves empty ranges and unassigned channels", () => {
    expect(status(mode(), mode())).toBe("Equal");
    for (const field of [
      "modeId",
      "channel",
      "start",
      "end",
      "logic",
      "linked",
    ] as const) {
      const b = { ...mode(), [field]: mode()[field]! + 1 };
      expect(status(mode(), b)).toBe("Changed");
      expect(status(b, mode())).toBe("Changed");
    }
    const empty = {
      ...mode(),
      channel: 14,
      channelAssigned: false,
      start: 900,
      end: 900,
    };
    expect(status(empty, empty)).toBe("Equal");
    for (const field of ["logic", "linked"] as const) {
      const omitted = { ...mode(), [field]: null };
      expect(status(mode(), omitted)).toBe("Unknown");
      expect(status(omitted, mode())).toBe("Unknown");
      expect(status(omitted, omitted)).toBe("Unknown");
    }
  });
  it("requires matching firmware identity even when allocations are identical", () => {
    const a = document([mode()]);
    for (const firmware of [
      { ...a.firmware, version: "4.5.1" },
      { ...a.firmware, version: "2025.12.1" },
      { ...a.firmware, version: "4.5.0.KAACK_V19" },
      { ...a.firmware, version: null },
      { ...a.firmware, family: "INAV" },
      { ...a.firmware, packId: null },
      { ...a.firmware, packId: "other" },
    ])
      expect(compareModes(a, { ...a, firmware })[0].status).toBe(
        "Not comparable",
      );
  });
});
