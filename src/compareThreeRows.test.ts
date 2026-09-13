import { expect, it } from "vitest";
import { compareThreeRows } from "./compareThreeRows";
import { compareFeatures } from "./compareFeatures";
import { compareRxRanges } from "./compareRxRanges";
import { compareCollections } from "./compareCollections";
import type { ConfigDocument } from "./bindings/core";

function document(
  features: Record<string, boolean>,
  ...lines: string[]
): ConfigDocument {
  return {
    firmware: {
      family: "betaflight",
      version: "4.5.0",
      packId: "betaflight-4.5.0-schema-1",
    },
    features,
    syntax: lines.map((raw, i) => ({
      line: i + 1,
      start: 0,
      end: raw.length,
      raw,
      command: {
        kind: "collection",
        name: "rxrange",
        operands: raw.split(/\s+/).slice(1),
      },
    })),
  } as ConfigDocument;
}

it("keeps explicit false values and entries present only in the non-baseline pair", () => {
  const a = document({ GPS: false });
  const b = document({ GPS: true, OSD: false });
  const c = document({ GPS: false, OSD: true });
  const rows = compareThreeRows(
    [compareFeatures(a, b), compareFeatures(a, c), compareFeatures(b, c)],
    (row) => row.name,
  );
  expect(rows[0].values).toEqual([false, true, false]);
  expect(rows[0].status).toBe("Left only");
  expect(rows[1].values).toEqual([undefined, false, true]);
  expect(rows[1].status).toBe("Unknown");
});

it("preserves semantic normalization, final source lines, reset uncertainty and release gates", () => {
  const a = document({}, "rxrange 0 1000 2000");
  const b = document({}, "rxrange 0 900 2000", "rxrange 0 01000 02000");
  const c = document({}, "rxrange 0 1100 2000");
  const compare = () =>
    compareThreeRows(
      [compareRxRanges(a, b), compareRxRanges(a, c), compareRxRanges(b, c)],
      (row) => String(row.channel),
    )[0];
  expect(compare().status).toBe("Right only");
  expect(compare().values[1]?.source.line).toBe(2);
  b.syntax.push(...document({}, "rxrange reset").syntax);
  expect(compare().status).toBe("Unknown");
  b.syntax = a.syntax;
  c.firmware.version = "4.5.0.VENDOR";
  expect(compare().status).toBe("Not comparable");
  expect(compare().left).toBe("Equal");
});

it("classifies conflicting text separately from semantically equal declarations", () => {
  const a = document({}, "rxrange 0 1000 2000");
  const b = document({}, "rxrange 0 01000 2000");
  const c = document({}, "rxrange 0 1000 02000");
  const semantic = compareThreeRows(
    [compareRxRanges(a, b), compareRxRanges(a, c), compareRxRanges(b, c)],
    (row) => String(row.channel),
  )[0];
  const text = compareThreeRows(
    [
      compareCollections(a, b),
      compareCollections(a, c),
      compareCollections(b, c),
    ],
    (row) => row.name,
  )[0];
  expect(semantic.status).toBe("Equal");
  expect(text.status).toBe("Conflict");
  expect(text.left).toBe("Different text");
});
