import { expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareRxRanges } from "./compareRxRanges";
const document = (...lines: string[]): ConfigDocument =>
  ({
    firmware: {
      family: "betaflight",
      version: "4.5.0",
      packId: "betaflight-4.5.0-schema-1",
    },
    syntax: lines.map(
      (raw, i): SyntaxLine => ({
        line: i + 1,
        start: 0,
        end: raw.length,
        raw,
        command:
          raw === "defaults nosave"
            ? { kind: "defaults" }
            : raw.startsWith("defaults")
              ? { kind: "malformed" }
              : {
                  kind: "collection",
                  name: "rxrange",
                  operands: raw.split(/\s+/).slice(1),
                },
      }),
    ),
  }) as ConfigDocument;
it("compares final numeric endpoints by channel, with source and unknown omissions", () => {
  const a = document("rxrange 1 1000 2000", "rxrange 0 1000 2000");
  const b = document(
    "rxrange 0 900 2100",
    "rxrange 0 01000 02000",
    "rxrange 2 750 2250",
  );
  const rows = compareRxRanges(a, b);
  expect(rows.map((row) => [row.channel, row.status])).toEqual([
    [0, "Equal"],
    [1, "Unknown"],
    [2, "Unknown"],
  ]);
  expect(rows[0].b?.source.line).toBe(2);
  expect(compareRxRanges(b, a).map((row) => row.status)).toEqual(
    rows.map((row) => row.status),
  );
  for (const changed of ["rxrange 0 1050 2000", "rxrange 0 1000 1950"])
    expect(compareRxRanges(a, document(changed))[0].status).toBe("Changed");
});
it("accepts inclusive endpoint bounds and preserves equal or reversed endpoints", () => {
  for (const line of [
    "rxrange 0 750 2250",
    "rxrange 3 2250 750",
    "rxrange 1 1000 1000",
  ]) {
    expect(compareRxRanges(document(line), document(line))[0].status).toBe(
      "Equal",
    );
  }
});
it("clears earlier knowledge on resets or unverified syntax, and recovers with explicit declarations", () => {
  const line = "rxrange 0 1000 2000";
  for (const reset of [
    "rxrange reset",
    "rxrange RESET",
    "defaults nosave",
    "defaults invalid",
    "rxrange 0 749 2000",
    "rxrange 0 1000 2251",
    "rxrange 4 1000 2000",
    "rxrange 0 nope 2000",
    "rxrange 0 1000 2000 extra",
  ]) {
    expect(
      compareRxRanges(document(line, reset), document(line))[0].status,
    ).toBe("Unknown");
    expect(
      compareRxRanges(document(line, reset, line), document(line))[0].status,
    ).toBe("Equal");
    expect(
      compareRxRanges(document(line, reset), document(line, reset)),
    ).toEqual([]);
  }
});
it("requires an exact verified release and pack on both sides", () => {
  const a = document("rxrange 0 1000 2000");
  for (const version of [
    "4.5.1",
    "4.5.0.KAACK_V19",
    "4.5.6",
    "2025.12.1",
    null,
  ]) {
    const b = { ...a, firmware: { ...a.firmware, version } };
    expect(compareRxRanges(a, b)[0].status).toBe("Not comparable");
  }
  for (const firmware of [
    { ...a.firmware, packId: null },
    { ...a.firmware, family: "INAV" },
  ])
    expect(compareRxRanges(a, { ...a, firmware })[0].status).toBe(
      "Not comparable",
    );
  for (const version of ["4.5.5", "2025.12.5"]) {
    const b = {
      ...a,
      firmware: {
        ...a.firmware,
        version,
        packId: version.startsWith("4.")
          ? "betaflight-4.5.0-schema-1"
          : "betaflight-2025.12.1-schema-1",
      },
    };
    expect(compareRxRanges(b, b)[0].status).toBe("Equal");
  }
});
