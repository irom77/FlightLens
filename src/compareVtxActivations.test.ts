import { expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareVtxActivations } from "./compareVtxActivations";

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
const dims = [
  "vtxtable bands 8",
  "vtxtable channels 8",
  "vtxtable powerlevels 8",
];
const slot = "vtx 0 13 5 8 5 1001 1999";
const base = [...dims, slot];
const compare = (a: string[], b: string[]) =>
  compareVtxActivations(document(...a), document(...b));
const status = (b: string[]) =>
  compare(base, b).find((r) => r.slot === 0)?.status;

it("normalizes ranges and numeric spelling while retaining final sources and distinct slots", () => {
  const b = [...base, "vtx 00 013 05 08 05 1024 1975", "vtx", "vtxtable"];
  expect(status(b)).toBe("Equal");
  expect(compare(base, b)[0].b).toMatchObject({
    start: 1000,
    end: 1975,
    source: { line: 5 },
  });
  expect(status([...base, "vtx 0 12 5 8 5 1001 1999"])).toBe("Changed");
  const overlaps = ["vtx 9 0 0 0 0 0 65535", "vtx 0 0 0 0 0 900 2100"];
  expect(compare(overlaps, overlaps).map((r) => r.slot)).toEqual([0, 9]);
  expect(compare(overlaps, overlaps)[1].a).toMatchObject({
    start: 900,
    end: 2100,
  });
  expect(
    compare(["vtx 0 0 0 0 0 2000 1000"], ["vtx 0 0 0 0 0 1000 1000"])[0].status,
  ).toBe("Changed");
});

it("requires each nonzero selector's current dimension and respects both build bounds", () => {
  expect(status([slot, ...dims])).toBe("Unknown");
  for (const [key, selectors] of [
    ["bands", "1 0 0"],
    ["channels", "0 1 0"],
    ["powerlevels", "0 0 1"],
  ]) {
    const line = `vtx 0 0 ${selectors} 900 2100`;
    const valid = [`vtxtable ${key} 1`, line];
    expect(compare(valid, valid)[0].status).toBe("Equal");
    expect(compare(valid, [line])[0].status).toBe("Unknown");
    expect(compare(valid, [`vtxtable ${key} 0`, line])[0].status).toBe(
      "Unknown",
    );
  }
  const zero = ["vtx 0 0 0 0 0 900 2100"];
  expect(compare(zero, zero)[0].status).toBe("Equal");
  for (const selectors of ["6 8 5", "5 9 5", "5 8 6"]) {
    expect(status([...dims, `vtx 0 13 ${selectors} 1001 1999`])).toBe(
      "Unknown",
    );
  }
  expect(status(["vtxtable bands 4", ...dims.slice(1), slot])).toBe("Unknown");
  expect(
    status([
      ...base,
      "vtxtable bands 0",
      "vtxtable channels 0",
      "vtxtable powerlevels 0",
    ]),
  ).toBe("Equal");
});

it("clears unverified mutations and resets, with later explicit restoration", () => {
  for (const invalid of [
    "vtx 0",
    "vtx 10 0 0 0 0 900 2100",
    "vtx 0 14 0 0 0 900 2100",
    "vtx 0 0 0 0 0 -1 2100",
    "vtx 0 0 0 0 0 0 65536",
    "vtx 0 0 0 0 0 900.5 2100",
    "vtx 0 0 0 0 0 +900 2100",
    "vtx 0 0 0 0 0 900 2100 extra",
    "defaults nosave",
    "defaults invalid",
  ]) {
    expect(status([...base, invalid]), invalid).toBe("Unknown");
    expect(status([...base, invalid, ...base]), invalid).toBe("Equal");
  }
  expect(status([...base, "defaults nosave", slot])).toBe("Unknown");
  for (const invalid of [
    "vtxtable reset",
    "vtxtable band 1 RACE R FACTORY 5658",
    "vtxtable bands 9",
  ]) {
    expect(status([...base, invalid])).toBe("Equal");
    expect(status([...base, invalid, slot])).toBe("Unknown");
  }
  expect(compare([], [])).toEqual([]);
});

it("certifies only matching official 4.5 releases with expected packs", () => {
  const a = document(...base);
  for (let patch = 0; patch <= 5; patch++) {
    const b = { ...a, firmware: { ...a.firmware, version: `4.5.${patch}` } };
    expect(compareVtxActivations(b, b)[0].status).toBe("Equal");
  }
  for (const firmware of [
    { ...a.firmware, version: "4.5.1" },
    { ...a.firmware, version: "4.5.3.KAACK_V18" },
    {
      ...a.firmware,
      version: "2025.12.1",
      packId: "betaflight-2025.12.1-schema-1",
    },
    { ...a.firmware, version: null },
    { ...a.firmware, packId: null },
    { ...a.firmware, family: "INAV" },
  ])
    expect(compareVtxActivations(a, { ...a, firmware })[0].status).toBe(
      "Not comparable",
    );
  const newer = {
    ...a,
    firmware: {
      ...a.firmware,
      version: "2025.12.1",
      packId: "betaflight-2025.12.1-schema-1",
    },
  };
  expect(compareVtxActivations(newer, newer)[0].status).toBe("Not comparable");
});
