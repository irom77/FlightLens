import { expect, it } from "vitest";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareRxFails } from "./compareRxFails";
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
            : raw.startsWith("defaults") || raw === "rxfail"
              ? { kind: "malformed" }
              : {
                  kind: "collection",
                  name: "rxfail",
                  operands: raw.split(/\s+/).slice(1),
                },
      }),
    ),
  }) as ConfigDocument;
it("compares final modes and quantized set values with sources and unknown omissions", () => {
  const a = document("rxfail 0 s 1001", "rxfail 1 a", "rxfail 3 h");
  const b = document(
    "rxfail 0 h",
    "rxfail 0 s 1024",
    "rxfail",
    "rxfail 0",
    "rxfail 1 h",
    "rxfail 17 s 2250",
  );
  const rows = compareRxFails(a, b);
  expect(rows.map((r) => [r.channel, r.status])).toEqual([
    [0, "Equal"],
    [1, "Changed"],
    [3, "Unknown"],
    [17, "Unknown"],
  ]);
  expect(rows[0].b?.value).toBe(1000);
  expect(rows[0].b?.source.line).toBe(2);
  expect(compareRxFails(b, a).map((r) => r.status)).toEqual(
    rows.map((r) => r.status),
  );
  expect(compareRxFails(a, document("rxfail 0 s 1025"))[0].status).toBe(
    "Changed",
  );
});
it("accepts supported modes and inclusive quantization boundaries", () => {
  for (const line of [
    "rxfail 0 a",
    "rxfail 3 a",
    "rxfail 4 h",
    "rxfail 17 h",
    "rxfail 0 s 750",
    "rxfail 17 s 2250",
    "rxfail 00 s 00750",
  ]) {
    expect(compareRxFails(document(line), document(line))[0].status).toBe(
      "Equal",
    );
  }
  expect(
    compareRxFails(document("rxfail 0 s 774"), document("rxfail 0 s 750"))[0]
      .status,
  ).toBe("Equal");
  expect(
    compareRxFails(
      document("rxfail 0 s 1000", "rxfail 0 h"),
      document("rxfail 0 h"),
    )[0].status,
  ).toBe("Equal");
});
it("clears knowledge on defaults and unverified mutations and restores later declarations", () => {
  const line = "rxfail 0 s 1000";
  for (const invalid of [
    "defaults nosave",
    "defaults invalid",
    "rxfail reset",
    "rxfail 18",
    "rxfail -1 h",
    "rxfail 4 a",
    "rxfail 0 H",
    "rxfail 0 s",
    "rxfail 0 a 1000",
    "rxfail 0 h extra",
    "rxfail 0 s 749",
    "rxfail 0 s 2251",
    "rxfail 0 s 1000.5",
    "rxfail 0 s +1000",
    "rxfail 0 s 1000 extra",
  ]) {
    expect(
      compareRxFails(document(line, invalid), document(line))[0].status,
      invalid,
    ).toBe("Unknown");
    expect(
      compareRxFails(document(line, invalid, line), document(line))[0].status,
      invalid,
    ).toBe("Equal");
    expect(
      compareRxFails(document(line, invalid), document(line, invalid)),
    ).toEqual([]);
  }
});
it("requires an exact verified release and pack on both sides", () => {
  const a = document("rxfail 0 s 1000");
  for (const version of [
    "4.5.1",
    "4.5.0.KAACK_V19",
    "4.5.6",
    "2025.12.1",
    null,
  ]) {
    const b = { ...a, firmware: { ...a.firmware, version } };
    expect(compareRxFails(a, b)[0].status).toBe("Not comparable");
  }
  for (const firmware of [
    { ...a.firmware, packId: null },
    { ...a.firmware, family: "INAV" },
  ])
    expect(compareRxFails(a, { ...a, firmware })[0].status).toBe(
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
    expect(compareRxFails(b, b)[0].status).toBe("Equal");
  }
});
