import { expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareVtxTables } from "./compareVtxTables";
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
            : raw.startsWith("defaults") || raw === "vtxtable"
              ? { kind: "malformed" }
              : {
                  kind: "collection",
                  name: "vtxtable",
                  operands: raw.split(/\s+/).slice(1),
                },
      }),
    ),
  }) as ConfigDocument;
const base = [
  "vtxtable bands 1",
  "vtxtable channels 2",
  "vtxtable band 1 RACE R FACTORY 5658 5695",
  "vtxtable powerlevels 2",
  "vtxtable powervalues 25 100",
  "vtxtable powerlabels 25 100",
];
const status = (a: string[], b: string[], key = "band 1") =>
  compareVtxTables(document(...a), document(...b)).find(
    (row) => row.key === key,
  )?.status;
it("normalizes final explicit values, preserves queries and source lines", () => {
  const b = [...base, "vtxtable band 01 race r factory 05658 5695", "vtxtable"];
  expect(status(base, b)).toBe("Equal");
  expect(
    compareVtxTables(document(...base), document(...b)).find(
      (r) => r.key === "band 1",
    )?.b?.source.line,
  ).toBe(7);
  expect(
    status(base, [...base, "vtxtable band 1 RACE R CUSTOM 5658 5695"]),
  ).toBe("Changed");
  expect(
    status(base, [...base, "vtxtable powerlabels low hi"], "powerlabels"),
  ).toBe("Changed");
  expect(
    status(
      [...base, "vtxtable powerlabels low hi"],
      [...base, "vtxtable powerlabels LOW HI"],
      "powerlabels",
    ),
  ).toBe("Equal");
});
it("invalidates dependencies when dimensions change and permits explicit restoration", () => {
  for (const key of ["bands", "channels"]) {
    const changed = [
      ...base,
      `vtxtable ${key} 0`,
      `vtxtable ${key} ${key === "bands" ? 1 : 2}`,
    ];
    expect(status(base, changed)).toBe("Unknown");
    expect(status(base, [...changed, base[2]])).toBe("Equal");
    expect(
      status(base, [...base, `vtxtable ${key} ${key === "bands" ? 1 : 2}`]),
    ).toBe("Equal");
  }
  expect(
    status(
      base,
      [...base, "vtxtable powerlevels 1", "vtxtable powerlevels 2"],
      "powervalues",
    ),
  ).toBe("Unknown");
  expect(status(base, [base[2], ...base.slice(0, 2)])).toBe("Unknown");
  expect(status(base, [base[4], base[3]], "powervalues")).toBe("Unknown");
});
it("clears unverified syntax and defaults without inferring initial values", () => {
  for (const invalid of [
    "defaults nosave",
    "defaults invalid",
    "vtxtable reset",
    "vtxtable bands 9",
    "vtxtable bands -1",
    "vtxtable channels 2 extra",
    "vtxtable band 0 RACE R FACTORY 5658 5695",
    "vtxtable band 1 TOOLONGGG R FACTORY 5658 5695",
    "vtxtable band 1 RACE RR FACTORY 5658 5695",
    "vtxtable band 1 RACE R 5658 5695",
    "vtxtable band 1 RACE R FACTORY 65536 5695",
    "vtxtable powervalues +25 100",
    "vtxtable powervalues 25.5 100",
    "vtxtable powervalues 25",
    "vtxtable powerlabels LONG 100",
    "vtxtable powerlabels é 100",
  ]) {
    expect(status(base, [...base, invalid]), invalid).toBe("Unknown");
    expect(status(base, [...base, invalid, ...base]), invalid).toBe("Equal");
  }
  expect(compareVtxTables(document(), document())).toEqual([]);
  const empty = [
    "vtxtable bands 0",
    "vtxtable channels 0",
    "vtxtable powerlevels 0",
    "vtxtable powervalues",
    "vtxtable powerlabels",
  ];
  expect(
    compareVtxTables(document(...empty), document(...empty)).map(
      (r) => r.status,
    ),
  ).toEqual(Array(5).fill("Equal"));
  expect(
    status(base, [
      "vtxtable bands 1",
      "vtxtable channels 0",
      "vtxtable band 1 RACE R FACTORY",
    ]),
  ).toBe("Unknown");
  expect(
    status(
      [...base, "vtxtable powervalues 0 65535"],
      [...base, "vtxtable powervalues 000 065535"],
      "powervalues",
    ),
  ).toBe("Equal");
});
it("certifies only matching reviewed official versions and packs", () => {
  const a = document(...base);
  for (const firmware of [
    { ...a.firmware, version: "4.5.1" },
    { ...a.firmware, version: "4.5.0.KAACK_V19" },
    { ...a.firmware, version: null },
    { ...a.firmware, packId: null },
    { ...a.firmware, family: "INAV" },
  ]) {
    expect(
      compareVtxTables(a, { ...a, firmware }).every(
        (r) => r.status === "Not comparable",
      ),
    ).toBe(true);
  }
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
    expect(compareVtxTables(b, b).every((r) => r.status === "Equal")).toBe(
      true,
    );
  }
});
