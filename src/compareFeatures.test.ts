import { describe, expect, it } from "vitest";
import type { ConfigDocument } from "./bindings/core";
import { compareFeatures } from "./compareFeatures";

const document = (features: ConfigDocument["features"], version = "4.5.0") =>
  ({
    features,
    firmware: {
      family: "Betaflight",
      version,
      packId: "betaflight-4.5.0-schema-1",
    },
  }) as ConfigDocument;

describe("feature declaration comparison", () => {
  it("compares explicit false values and treats omissions as unknown", () => {
    const a = document({ OSD: false, GPS: true, "3D": false });
    const b = document({ OSD: false, GPS: false, TELEMETRY: true });
    expect(compareFeatures(a, b).map((row) => [row.name, row.status])).toEqual([
      ["3D", "Unknown"],
      ["GPS", "Changed"],
      ["OSD", "Equal"],
      ["TELEMETRY", "Unknown"],
    ]);
    expect(compareFeatures(b, a).map((row) => row.status)).toEqual(
      compareFeatures(a, b).map((row) => row.status),
    );
    expect(compareFeatures(document({}), document({}))).toEqual([]);
  });
  it("requires matching known firmware identity", () => {
    const a = document({ OSD: true });
    for (const version of ["4.5.1", "2025.12.1", "4.5.0.KAACK_V19"]) {
      expect(
        compareFeatures(a, document({ OSD: true }, version))[0].status,
      ).toBe("Not comparable");
    }
    for (const firmware of [
      { ...a.firmware, version: null },
      { ...a.firmware, packId: null },
      { ...a.firmware, packId: "other" },
      { ...a.firmware, family: "INAV" },
    ])
      expect(compareFeatures(a, { ...a, firmware })[0].status).toBe(
        "Not comparable",
      );
    expect(compareFeatures(a, document({}, "2025.12.1"))[0].status).toBe(
      "Unknown",
    );
  });
});
