import { expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareAdjustmentRanges } from "./compareAdjustmentRanges";

const document = (...lines: string[]): ConfigDocument =>
  ({
    firmware: {
      family: "betaflight",
      version: "4.5.0",
      packId: "betaflight-4.5.0-schema-1",
    },
    syntax: lines.map((raw, i): SyntaxLine => {
      const [name, ...operands] = raw.split(/\s+/);
      return {
        line: i + 1,
        start: 0,
        end: raw.length,
        raw,
        command:
          raw === "defaults nosave"
            ? { kind: "defaults" }
            : name === "defaults" || !operands.length
              ? { kind: "malformed" }
              : { kind: "collection", name, operands },
      };
    }),
  }) as ConfigDocument;
const base = "adjrange 0 0 13 1001 1999 34 13";
const compare = (a: string[], b: string[]) =>
  compareAdjustmentRanges(document(...a), document(...b));
const status = (...lines: string[]) => compare([base], lines)[0]?.status;

it("normalizes stored fields, optional defaults and final source", () => {
  const lines = [
    base + " 10 20",
    "adjrange 00 00 013 1024 1975 034 013",
    "adjrange",
  ];
  expect(status(...lines)).toBe("Equal");
  expect(compare([base], lines)[0].b).toMatchObject({
    center: 0,
    scale: 0,
    start: 1000,
    end: 1975,
    source: { line: 2 },
  });
  expect(status(base + " 0")).toBe("Equal");
  expect(status(base + " 0 0")).toBe("Equal");
  expect(status(base + " 65535 65535")).toBe("Changed");
  expect(compare([base], [base + " 10 20", base + " 10"])[0].b?.scale).toBe(0);
  for (const changed of [
    "0 0 12 1000 1975 34 13",
    "0 0 13 1025 1975 34 13",
    "0 0 13 1000 2000 34 13",
    "0 0 13 1000 1975 33 13",
    "0 0 13 1000 1975 34 12",
  ])
    expect(status("adjrange " + changed)).toBe("Changed");
});

it("preserves separate slots, zero records and reversed ranges", () => {
  const lines = ["adjrange 29 0 0 0 65535 0 0", "adjrange 0 0 0 900 900 0 0"];
  expect(compare(lines, lines).map((r) => r.slot)).toEqual([0, 29]);
  expect(compare(lines, lines)[1].a).toMatchObject({
    start: 900,
    end: 2100,
    functionId: 0,
  });
  expect(
    compare(
      ["adjrange 0 0 0 2000 1000 0 0"],
      ["adjrange 0 0 0 1000 1000 0 0"],
    )[0].status,
  ).toBe("Changed");
  expect(compare([], [])).toEqual([]);
});

it("invalidates unverified mutations and restores only explicitly assigned slots", () => {
  for (const invalid of [
    "adjrange 0",
    "adjrange 30 0 0 900 2100 0 0",
    "adjrange 0 1 0 900 2100 0 0",
    "adjrange 0 0 14 900 2100 0 0",
    "adjrange 0 0 0 900 2100 0 14",
    "adjrange 0 0 0 900 2100 35 0",
    "adjrange 0 0 0 -1 2100 0 0",
    "adjrange 0 0 0 +900 2100 0 0",
    "adjrange 0 0 0 900.5 2100 0 0",
    "adjrange 0 0 0 0 65536 0 0",
    base + " 65536",
    base + " 0 65536",
    base + " 0 0 0",
    "defaults nosave",
    "defaults invalid",
  ]) {
    expect(status(base, invalid), invalid).toBe("Unknown");
    expect(status(base, invalid, base), invalid).toBe("Equal");
  }
  const second = "adjrange 1 0 0 900 2100 0 0";
  expect(
    compare([base, second], [base, second, "adjrange 1", base]).map(
      (r) => r.status,
    ),
  ).toEqual(["Equal", "Unknown"]);
});

it("certifies verified matching releases and uses family-specific function bounds", () => {
  const a = document(base);
  for (const version of [
    ...Array.from({ length: 6 }, (_, i) => `4.5.${i}`),
    ...Array.from({ length: 5 }, (_, i) => `2025.12.${i + 1}`),
  ]) {
    const newer = version.startsWith("2025");
    const b = document("adjrange 0 0 0 900 2100 35 0");
    b.firmware = {
      ...a.firmware,
      version,
      packId: newer ? "betaflight-2025.12.1-schema-1" : a.firmware.packId,
    };
    expect(compareAdjustmentRanges(b, b)).toHaveLength(newer ? 1 : 0);
    const c = { ...a, firmware: b.firmware };
    expect(compareAdjustmentRanges(c, c)[0].status).toBe("Equal");
  }
  for (const firmware of [
    { ...a.firmware, version: "4.5.1" },
    { ...a.firmware, version: "4.5.3.KAACK_V18" },
    { ...a.firmware, version: "2025.12.6" },
    { ...a.firmware, version: null },
    { ...a.firmware, packId: null },
    { ...a.firmware, family: "INAV" },
  ])
    expect(compareAdjustmentRanges(a, { ...a, firmware })[0].status).toBe(
      "Not comparable",
    );
});
