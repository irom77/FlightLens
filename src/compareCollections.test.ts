import { describe, expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareCollections } from "./compareCollections";

const collection = (name: string, raw: string, line = 1): SyntaxLine => ({
  line,
  start: 0,
  end: raw.length,
  raw,
  command: { kind: "collection", name, operands: raw.split(/\s+/).slice(1) },
});
const document = (syntax: SyntaxLine[]): ConfigDocument =>
  ({ syntax }) as ConfigDocument;
const status = (a: SyntaxLine[], b: SyntaxLine[]) =>
  compareCollections(document(a), document(b))[0].status;

describe("collection source-text comparison", () => {
  it("groups every recognized collection, with missing groups unknown in both directions", () => {
    const a = document([
      collection("vtx", "vtx 0 1"),
      collection("rxrange", "rxrange 0 1000 2000"),
    ]);
    const b = document([
      collection("rxfail", "rxfail 0 a"),
      collection("vtx", "vtx 0 1", 12),
    ]);
    expect(
      compareCollections(a, b).map((row) => [row.name, row.status]),
    ).toEqual([
      ["rxfail", "Unknown"],
      ["rxrange", "Unknown"],
      ["vtx", "Matching text"],
    ]);
    expect(compareCollections(b, a).map((row) => row.status)).toEqual(
      compareCollections(a, b).map((row) => row.status),
    );
    expect(compareCollections(document([]), document([]))).toEqual([]);
    for (const name of ["vtxtable", "vtx", "rxfail", "rxrange", "adjrange"]) {
      expect(
        status(
          [collection(name, `${name} 0`)],
          [collection(name, `${name} 1`)],
        ),
      ).toBe("Different text");
    }
  });
  it("retains whitespace, duplicates, order and pre-reset source instead of inferring final settings", () => {
    const first = collection("rxrange", "rxrange 0 1000 2000");
    const last = collection("rxrange", "rxrange 0 1050 1950", 3);
    expect(status([first, last], [last, first])).toBe("Different text");
    expect(status([first, first], [first])).toBe("Different text");
    expect(status([first], [{ ...first, raw: "rxrange  0 1000 2000" }])).toBe(
      "Different text",
    );
    const reset: SyntaxLine = {
      line: 2,
      start: 0,
      end: 15,
      raw: "defaults nosave",
      command: { kind: "defaults" },
    };
    const result = compareCollections(
      document([first, reset, last]),
      document([last]),
    )[0];
    expect(result.a).toEqual([first, last]);
    expect(result.status).toBe("Different text");
  });
  it("reports text matching without requiring or certifying firmware identity", () => {
    const a = document([collection("vtxtable", "vtxtable bands 5")]);
    const b = {
      ...a,
      firmware: { family: "Other", version: null, packId: null },
    } as ConfigDocument;
    expect(compareCollections(a, b)[0].status).toBe("Matching text");
  });
});
