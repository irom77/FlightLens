import { execFileSync } from "node:child_process";
import { beforeAll, expect, it } from "vitest";
import type { ConfigDocument } from "./bindings/core";
import {
  comparisonSyntax,
  sourceLine,
  type DocumentView,
} from "./documentView";
import { compareCollections } from "./compareCollections";
import { compareRxRanges } from "./compareRxRanges";
import { compareRxFails } from "./compareRxFails";
import { compareAdjustmentRanges } from "./compareAdjustmentRanges";
import { compareVtxActivations } from "./compareVtxActivations";
import { compareVtxTables } from "./compareVtxTables";
import { compareFeatures } from "./compareFeatures";
import { comparePorts } from "./comparePorts";
import { compareModes } from "./compareModes";
import { compareParameters } from "./compareParameters";
import { compareThreeParameters } from "./compareThreeParameters";

let cases: { document: ConfigDocument; view: DocumentView }[];
beforeAll(() => {
  // Exercise Rust's real parser/projection, not a duplicate JS implementation.
  // The binary accepts no backup input and emits built-in synthetic cases only.
  const fixtures: { document: ConfigDocument; view: DocumentView }[] =
    JSON.parse(
      execFileSync(
        process.env.CARGO ?? "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "flightlens-core",
          "--bin",
          "comparison_evidence",
        ],
        { encoding: "utf8", maxBuffer: 8 * 1024 * 1024, timeout: 120_000 },
      ),
    );
  cases = fixtures;
}, 120_000);

it("preserves comparison results and source evidence for every synthetic pair", () => {
  const comparators = [
    compareCollections,
    compareRxRanges,
    compareRxFails,
    compareAdjustmentRanges,
    compareVtxActivations,
    compareVtxTables,
    compareFeatures,
    comparePorts,
    compareModes,
  ];
  const statuses = new Set<string>();
  for (const a of cases) {
    expect(a.view).not.toHaveProperty("syntax");
    expect(a.view.sourceEvidence.lineCount).toBe(a.document.syntax.length);
    expect(a.view.sourceEvidence.hasDumpAll).toBe(true);
    expect(comparisonSyntax(a.view).length).toBeLessThan(
      a.document.syntax.length,
    );
    for (const b of cases) {
      for (const compare of comparators) {
        const full = compare(a.document, b.document);
        const compact = compare(a.view, b.view);
        expect(compact).toEqual(full);
        expect(compare(a.document, b.view)).toEqual(full);
        full.forEach((row) => statuses.add(row.status));
      }
      for (const pid of [0, 1]) {
        const profile = { pid, rate: pid };
        expect(compareParameters(a.view, b.view, profile, profile)).toEqual(
          compareParameters(a.document, b.document, profile, profile),
        );
      }
    }
  }
  for (const status of [
    "Equal",
    "Changed",
    "Unknown",
    "Not comparable",
    "Matching text",
    "Different text",
  ])
    expect(statuses.has(status)).toBe(true);
});

it("preserves source details used by parameter, port, mode and feature cells", () => {
  for (const { document, view } of cases) {
    const declarations = [
      ...Object.values(document.parameters),
      ...document.ports,
      ...document.modes,
    ];
    for (const value of declarations) {
      if (!value) continue;
      expect(sourceLine(view, value.line)).toEqual(
        sourceLine(document, value.line),
      );
      expect(sourceLine(view, value.line)).toBeDefined();
    }
    for (const name of Object.keys(document.features)) {
      const feature = (d: ConfigDocument | DocumentView) =>
        [...comparisonSyntax(d)]
          .reverse()
          .find(
            (line) =>
              line.command.kind === "feature" && line.command.name === name,
          );
      expect(feature(view)).toEqual(feature(document));
    }
  }
});

it("preserves three-document parameter comparisons across selected profiles", () => {
  for (let i = 0; i < cases.length - 2; i++) {
    const [a, b, c] = cases.slice(i, i + 3);
    const profiles = [
      { pid: 0, rate: 0 },
      { pid: 1, rate: 1 },
      { pid: 0, rate: 0 },
    ] as const;
    expect(
      compareThreeParameters([a.view, b.view, c.view], [...profiles]),
    ).toEqual(
      compareThreeParameters(
        [a.document, b.document, c.document],
        [...profiles],
      ),
    );
  }
});
