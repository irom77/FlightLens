import type { DiffRowInput } from "./bindings/core";
import type { ComparisonDocument } from "./documentView";
import {
  compareParameters,
  type ComparisonValue,
  type ProfileSelection,
} from "./compareParameters";
import { compareThreeParameters } from "./compareThreeParameters";
import { compareThreeRows, type PairRow } from "./compareThreeRows";
import { compareFeatures } from "./compareFeatures";
import { comparePorts } from "./comparePorts";
import { compareModes } from "./compareModes";
import { compareRxRanges } from "./compareRxRanges";
import { compareRxFails } from "./compareRxFails";
import { compareVtxTables } from "./compareVtxTables";
import { compareVtxActivations } from "./compareVtxActivations";
import { compareAdjustmentRanges } from "./compareAdjustmentRanges";
import { compareCollections } from "./compareCollections";

export type SummarySlot = {
  document: ComparisonDocument;
  profiles: ProfileSelection;
};

function parameter(value: ComparisonValue | undefined): string | null {
  if (value?.declared) {
    const p = value.declared;
    return p.valid && p.supported
      ? `${p.rawValue}${p.unit ? ` ${p.unit}` : ""} · Declared`
      : null;
  }
  const p = value?.derived;
  return p
    ? `${p.rawValue} · Derived · Betaflight ${p.sourceVersion} default`
    : null;
}

function status(value: string): string | null {
  switch (value) {
    case "Equal":
    case "Matching text":
      return null;
    case "Changed":
    case "Different text":
    case "Same change":
      return "changed";
    case "Left only":
    case "Right only":
      return "one_sided";
    case "Conflict":
      return "conflict";
    case "Not comparable":
      return "not_comparable";
    default:
      return "unknown";
  }
}

/** Uses the same classifiers as Compare. Values always retain A/B/C slot order.
 * Raw source, titles, paths and line numbers are deliberately never serialized.
 * Rust must still validate sensitive keys and bound every field before preview.
 */
export function summaryDiff(
  slots: SummarySlot[],
  baseline: number | null,
): DiffRowInput[] {
  if (slots.length !== 2 && slots.length !== 3)
    throw new Error("Select two or three backups.");
  if (
    baseline !== null &&
    (!Number.isInteger(baseline) || baseline < 0 || baseline >= slots.length)
  )
    throw new Error("Select a valid baseline.");
  if (slots.length === 3 && baseline === null)
    throw new Error("Choose a baseline to summarize three backups.");
  const order =
    slots.length === 3
      ? [baseline!, ...[0, 1, 2].filter((i) => i !== baseline)]
      : [0, 1];
  const ordered = order.map((i) => slots[i]);
  const [a, b, c] = ordered.map((s) => s.document);
  const output: DiffRowInput[] = [];
  const add = (
    section: string,
    key: string,
    scope: string,
    result: string,
    values: (string | null)[],
    detail?: string,
  ) => {
    const normalized = status(result);
    if (!normalized) return;
    const slotted = slots.map((): string | null => null);
    order.forEach((slot, i) => {
      slotted[slot] = values[i];
    });
    const explanation =
      result === "Left only"
        ? `Only Backup ${"ABC"[order[1]]} differs from baseline.`
        : result === "Right only"
          ? `Only Backup ${"ABC"[order[2]]} differs from baseline.`
          : result === "Same change"
            ? "Both non-baseline backups have the same change."
            : result === "Not comparable"
              ? "Compare does not certify equivalence for these values and firmware versions."
              : result === "Unknown"
                ? "Missing, invalid or unsupported values remain unknown; absence is not a change."
                : null;
    output.push({
      section,
      key,
      scope,
      status: normalized,
      values: slotted,
      reason: [detail, explanation].filter(Boolean).join(" ") || null,
    });
  };
  if (c) {
    for (const row of compareThreeParameters(
      [a, b, c],
      ordered.map((s) => s.profiles) as [
        ProfileSelection,
        ProfileSelection,
        ProfileSelection,
      ],
    ))
      add(
        "parameters",
        row.key,
        row.scope,
        row.status,
        row.values.map(parameter),
      );
  } else {
    for (const row of compareParameters(
      a,
      b,
      ordered[0].profiles,
      ordered[1].profiles,
    ))
      add("parameters", row.key, row.scope, row.status, [
        parameter(row.a),
        parameter(row.b),
      ]);
  }
  function collection<T, R extends PairRow<T>>(
    section: string,
    compare: (a: ComparisonDocument, b: ComparisonDocument) => R[],
    key: (row: R) => string,
    value: (v: NonNullable<R["a"]>) => string,
    detail?: string,
  ) {
    const render = (v: R["a"]) => (v == null ? null : value(v));
    if (c) {
      for (const row of compareThreeRows(
        [compare(a, b), compare(a, c), compare(b, c)],
        key,
      ))
        add(
          section,
          row.key,
          "global",
          row.status,
          row.values.map(render),
          detail,
        );
    } else {
      for (const row of compare(a, b))
        add(
          section,
          key(row),
          "global",
          row.status,
          [render(row.a), render(row.b)],
          detail,
        );
    }
  }
  collection(
    "features",
    compareFeatures,
    (r) => r.name,
    (v) => (v ? "Enabled · Declared" : "Disabled · Declared"),
  );
  collection(
    "ports",
    comparePorts,
    (r) => String(r.identifier),
    (v) => JSON.stringify({ mask: v.mask, baud: v.baud }),
  );
  collection(
    "modes",
    compareModes,
    (r) => String(r.index),
    (v) =>
      JSON.stringify({
        modeId: v.modeId,
        channel: v.channel,
        start: v.start,
        end: v.end,
        logic: v.logic,
        linked: v.linked,
      }),
  );
  collection(
    "rxrange",
    compareRxRanges,
    (r) => String(r.channel),
    (v) => JSON.stringify({ min: v.min, max: v.max }),
  );
  collection(
    "rxfail",
    compareRxFails,
    (r) => String(r.channel),
    (v) => JSON.stringify({ mode: v.mode, value: v.value ?? null }),
  );
  collection(
    "vtx",
    compareVtxTables,
    (r) => `table ${r.key}`,
    (v) => v.value,
  );
  collection(
    "vtx",
    compareVtxActivations,
    (r) => `activation ${r.slot}`,
    (v) =>
      JSON.stringify({
        aux: v.aux,
        band: v.band,
        channel: v.channel,
        power: v.power,
        start: v.start,
        end: v.end,
      }),
  );
  collection(
    "adjrange",
    compareAdjustmentRanges,
    (r) => String(r.slot),
    (v) =>
      JSON.stringify({
        aux: v.aux,
        functionId: v.functionId,
        selectAux: v.selectAux,
        center: v.center,
        scale: v.scale,
        start: v.start,
        end: v.end,
      }),
  );
  // Text comparisons may contain arbitrary user text. Keep classification and
  // counts only; never transmit the underlying CLI lines or their operands.
  collection(
    "collections",
    compareCollections,
    (r) => r.name,
    (v) => `${v.length} source lines (text withheld)`,
    "Source-text comparison only; matching or differing text does not establish equivalent behavior or final settings. Source text is withheld.",
  );
  return output;
}
