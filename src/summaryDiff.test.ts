import { expect, it } from "vitest";
import type { ConfigDocument, Parameter } from "./bindings/core";
import { summaryDiff, type SummarySlot } from "./summaryDiff";

function parameter(value: number, key = "p_roll", index = 0): Parameter {
  return {
    key,
    semanticKey: key,
    value: { kind: "integer", value },
    rawValue: String(value),
    scope: { kind: "pid", index },
    line: 2,
    valid: true,
    supported: true,
    unit: null,
    packId: "betaflight-4.5.0-schema-1",
  };
}
function slot(parameters: Parameter[] = [], lines: string[] = []): SummarySlot {
  return {
    profiles: { pid: 0, rate: 0 },
    document: {
      id: "private-id",
      title: "private-title",
      sourceId: "private-path",
      hash: "private-hash",
      craftName: null,
      pilotName: null,
      completeness: "partial",
      derivedNote: null,
      diagnostics: [],
      pidProfiles: [0, 1],
      rateProfiles: [0],
      selectedPid: 0,
      selectedRate: 0,
      firmware: {
        family: "betaflight",
        version: "4.5.0",
        packId: "betaflight-4.5.0-schema-1",
        boardName: null,
        header: null,
      },
      parameters: Object.fromEntries(parameters.map((p, i) => [String(i), p])),
      derived: {},
      features: {},
      ports: [],
      modes: [],
      syntax: lines.map((raw, i) => ({
        line: i + 1,
        start: 0,
        end: raw.length,
        raw,
        command: {
          kind: "collection",
          name: raw.split(" ")[0],
          operands: raw.split(/\s+/).slice(1),
        },
      })),
    } as ConfigDocument,
  };
}
it("preserves selected profiles, provenance and unknown values without inventing zero", () => {
  const a = slot([parameter(10), parameter(20, "p_roll", 1)]);
  const b = slot([parameter(30)]);
  a.profiles.pid = 1;
  expect(summaryDiff([a, b], null)[0]).toMatchObject({
    status: "changed",
    scope: "pid",
    values: ["20 · Declared", "30 · Declared"],
  });
  b.document.parameters["0"]!.valid = false;
  expect(summaryDiff([a, b], null)[0]).toMatchObject({
    status: "unknown",
    values: ["20 · Declared", null],
  });
  b.document.parameters = {};
  b.document.derived = {
    "pid:0:p_roll": {
      key: "p_roll",
      scope: { kind: "pid", index: 0 },
      value: { kind: "integer", value: 40 },
      rawValue: "40",
      sourceVersion: "4.5.0",
    } as NonNullable<ConfigDocument["derived"][string]>,
  };
  expect(summaryDiff([a, b], null)[0].values[1]).toContain("40 · Derived");
});
it("retains certified cross-version comparison and rejects uncertified equivalence", () => {
  const a = slot([parameter(14, "motor_poles"), parameter(10)]);
  const b = slot([parameter(12, "motor_poles"), parameter(20)]);
  for (const s of [a, b])
    s.document.parameters["0"]!.scope = { kind: "global" };
  b.document.firmware.version = "2025.12.1";
  b.document.firmware.packId = "betaflight-2025.12.1-schema-1";
  const rows = summaryDiff([a, b], null);
  expect(rows.find((r) => r.key === "motor_poles")?.status).toBe("changed");
  expect(rows.find((r) => r.key === "p_roll")?.status).toBe("not_comparable");
});
it("maps every baseline back to original slots and identifies one-sided changes", () => {
  const slots = [
    slot([parameter(10)]),
    slot([parameter(20)]),
    slot([parameter(10)]),
  ];
  for (const baseline of [0, 1, 2]) {
    const row = summaryDiff(slots, baseline)[0];
    expect(row.values).toEqual([
      "10 · Declared",
      "20 · Declared",
      "10 · Declared",
    ]);
    expect(row.status).toBe(baseline === 1 ? "changed" : "one_sided");
    expect(row.reason).toContain(baseline === 1 ? "same change" : "Backup B");
  }
  slots[2] = slot([parameter(30)]);
  expect(summaryDiff(slots, 2)[0].status).toBe("conflict");
  expect(() => summaryDiff(slots, null)).toThrow("baseline");
  expect(() => summaryDiff(slots, 3)).toThrow("baseline");
  expect(() => summaryDiff([slots[0]], null)).toThrow("two or three");
});
it("preserves explicit false and unknown collection slots", () => {
  const a = slot(),
    b = slot(),
    c = slot();
  a.document.features = { GPS: false };
  b.document.features = { GPS: true, OSD: false };
  c.document.features = { GPS: false, OSD: true };
  const rows = summaryDiff([a, b, c], 2);
  expect(rows.find((r) => r.key === "GPS")).toMatchObject({
    status: "one_sided",
    values: [
      "Disabled · Declared",
      "Enabled · Declared",
      "Disabled · Declared",
    ],
  });
  expect(rows.find((r) => r.key === "OSD")).toMatchObject({
    status: "unknown",
    values: [null, "Disabled · Declared", "Enabled · Declared"],
  });
});
it("separates semantic equality from source-text differences without disclosing source", () => {
  const a = slot([], ["rxrange 0 1000 2000"]);
  const b = slot([], ["rxrange 0 01000 2000"]);
  const c = slot([], ["rxrange 0 1000 02000"]);
  const rows = summaryDiff([a, b, c], 0);
  expect(rows).toHaveLength(1);
  expect(rows[0]).toMatchObject({
    section: "collections",
    status: "conflict",
    values: Array(3).fill("1 source lines (text withheld)"),
  });
  expect(rows[0].reason).toContain("does not establish equivalent behavior");
  const serialized = JSON.stringify(rows);
  for (const secret of ["private-", "01000", "02000", "rxrange 0"])
    expect(serialized).not.toContain(secret);
});
it("extracts all semantic collections using explicit fields without source metadata", () => {
  const a = slot(
    [],
    [
      "rxrange 0 1000 2000",
      "rxfail 0 s 1000",
      "vtxtable bands 1",
      "vtx 0 0 0 0 0 900 1200",
      "adjrange 0 0 0 900 1200 1 0",
    ],
  );
  const b = slot(
    [],
    [
      "rxrange 0 1100 2000",
      "rxfail 0 s 1100",
      "vtxtable bands 2",
      "vtx 0 0 0 0 0 900 1300",
      "adjrange 0 0 0 900 1300 1 0",
    ],
  );
  a.document.ports = [
    {
      identifier: 0,
      name: "private-port",
      mask: 1,
      baud: [1, 2, 3, 4],
      functions: [],
      line: 12345,
    },
  ];
  b.document.ports = [{ ...a.document.ports[0], mask: 2 }];
  a.document.modes = [
    {
      index: 0,
      modeId: 1,
      name: "private-mode",
      channel: 0,
      channelAssigned: true,
      start: 900,
      end: 1200,
      logic: 0,
      linked: 0,
      line: 12345,
    },
  ];
  b.document.modes = [{ ...a.document.modes[0], end: 1300 }];
  const rows = summaryDiff([a, b], null);
  for (const section of [
    "ports",
    "modes",
    "rxrange",
    "rxfail",
    "vtx",
    "adjrange",
    "collections",
  ])
    expect(
      rows.some((r) => r.section === section && r.status === "changed"),
    ).toBe(true);
  expect(rows.filter((r) => r.section === "vtx").map((r) => r.key)).toEqual([
    "table bands",
    "activation 0",
  ]);
  expect(JSON.stringify(rows)).not.toMatch(/private-|12345|"source"|"raw"/);
});
