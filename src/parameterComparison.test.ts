import { describe, expect, it } from "vitest";
import type { ConfigDocument, Parameter } from "./bindings/core";
import { compareParameters } from "./compareParameters";

const parameter = (value: number, index = 0): Parameter => ({
  key: "p_roll",
  semanticKey: "p_roll",
  value: { kind: "integer", value },
  rawValue: String(value),
  scope: { kind: "pid", index },
  line: 2,
  valid: true,
  supported: true,
  unit: null,
  packId: "pack",
});
const document = (parameters: Parameter[]): ConfigDocument =>
  ({
    firmware: {
      family: "Betaflight",
      version: "4.5.0",
      packId: "pack",
      boardName: null,
      header: null,
    },
    parameters: Object.fromEntries(parameters.map((p, i) => [String(i), p])),
    derived: {},
  }) as ConfigDocument;
const profiles = { pid: 0, rate: 0 };

describe("parameter comparison", () => {
  it("maps PID profiles independently and compares typed values, including zero", () => {
    const a = document([parameter(0), parameter(30, 1)]);
    const b = document([parameter(30), parameter(10, 1)]);
    expect(
      compareParameters(a, b, { ...profiles, pid: 1 }, profiles)[0].status,
    ).toBe("Equal");
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
      "Changed",
    );
    b.parameters["0"]!.rawValue = "030";
    expect(
      compareParameters(a, b, { ...profiles, pid: 1 }, profiles)[0].status,
    ).toBe("Equal");
  });
  it("keeps missing, invalid and unsupported values unknown", () => {
    const a = document([parameter(0)]);
    expect(
      compareParameters(a, document([]), profiles, profiles)[0].status,
    ).toBe("Unknown");
    for (const property of ["valid", "supported"] as const) {
      const b = document([{ ...parameter(0), [property]: false }]);
      expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
        "Unknown",
      );
    }
  });
  it("uses certified derived DTO values without replacing invalid declarations", () => {
    const a = document([parameter(30)]);
    const b = document([]);
    b.derived = {
      p: {
        key: "p_roll",
        scope: { kind: "pid", index: 0 },
        value: { kind: "integer", value: 30 },
        rawValue: "30",
        sourceVersion: "4.5.0",
      },
    };
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe("Equal");
    b.parameters = { p: { ...parameter(30), valid: false } };
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
      "Unknown",
    );
  });
  it("does not equate shared keys across uncertified firmware versions or unknown scopes", () => {
    const a = document([parameter(30)]);
    const b = document([parameter(30)]);
    b.firmware.version = "4.5.1";
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
      "Not comparable",
    );
    b.firmware.version = a.firmware.version;
    a.firmware.packId = null;
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
      "Not comparable",
    );
    a.parameters["0"]!.scope = { kind: "unknown" };
    b.parameters["0"]!.scope = { kind: "unknown" };
    expect(compareParameters(a, b, profiles, profiles)[0].status).toBe(
      "Unknown",
    );
  });
});

describe("certified motor pole comparison", () => {
  function motor(version: string, value = 14) {
    const d = document([
      {
        ...parameter(value),
        key: "motor_poles",
        semanticKey: "motor_poles",
        scope: { kind: "global" },
      },
    ]);
    d.firmware.version = version;
    d.firmware.packId = version.startsWith("4.5.")
      ? "betaflight-4.5.0-schema-1"
      : "betaflight-2025.12.1-schema-1";
    return d;
  }
  const status = (a: ConfigDocument, b: ConfigDocument) =>
    compareParameters(a, b, profiles, profiles)[0].status;
  it("compares verified releases symmetrically", () => {
    for (const version of ["4.5.0", "4.5.5", "2025.12.1", "2025.12.5"]) {
      expect(status(motor(version), motor("4.5.2"))).toBe("Equal");
      expect(status(motor("4.5.2", 12), motor(version))).toBe("Changed");
    }
  });
  it("rejects uncertified versions, families, packs and scopes", () => {
    for (const version of [
      "4.5.6",
      "2025.12.6",
      "4.5.3.KAACK_V19",
      "2025.12.3-alpha.KAACK_V19",
    ]) {
      expect(status(motor("4.5.0"), motor(version))).toBe("Not comparable");
    }
    for (const change of [
      (d: ConfigDocument) => {
        d.firmware.family = "INAV";
      },
      (d: ConfigDocument) => {
        d.firmware.packId = "other";
      },
      (d: ConfigDocument) => {
        d.parameters["0"]!.scope = { kind: "pid", index: 0 };
      },
    ]) {
      const a = motor("4.5.0"),
        b = motor("2025.12.1");
      change(a);
      change(b);
      expect(status(a, b)).toBe("Not comparable");
    }
  });
  it("preserves unknown values and rejects values outside the mapping", () => {
    const a = motor("4.5.0"),
      b = motor("2025.12.1");
    b.parameters["0"]!.valid = false;
    expect(status(a, b)).toBe("Unknown");
    expect(status(a, motor("2025.12.1", 256))).toBe("Not comparable");
    b.parameters = {};
    expect(status(a, b)).toBe("Unknown");
  });
});

describe("certified battery voltage comparison", () => {
  const make = (key: string, value: number, version: string) => {
    const d = document([
      { ...parameter(value), key, semanticKey: key, scope: { kind: "global" } },
    ]);
    d.firmware.version = version;
    d.firmware.packId = version.startsWith("4.5.")
      ? "betaflight-4.5.0-schema-1"
      : "betaflight-2025.12.1-schema-1";
    return d;
  };
  const status = (a: ConfigDocument, b: ConfigDocument) =>
    compareParameters(a, b, profiles, profiles)[0].status;
  for (const key of [
    "vbat_max_cell_voltage",
    "vbat_min_cell_voltage",
    "vbat_warning_cell_voltage",
  ]) {
    it(`compares ${key} in hundredths of a volt within certified bounds`, () => {
      for (const version of ["4.5.0", "4.5.5", "2025.12.1", "2025.12.5"]) {
        for (const value of [100, 350, 500]) {
          const a = make(key, value, version),
            b = make(key, value, "4.5.2");
          expect(status(a, b)).toBe("Equal");
          expect(status(b, a)).toBe("Equal");
          expect(
            status(a, make(key, value === 500 ? 499 : value + 1, "4.5.2")),
          ).toBe("Changed");
        }
      }
      const a = make(key, 350, "4.5.0");
      for (const value of [99, 501])
        expect(status(a, make(key, value, "2025.12.1"))).toBe("Not comparable");
      for (const version of [
        "4.5.3.KAACK_V19",
        "2025.12.3-alpha.KAACK_V19",
        "2025.12.6",
      ])
        expect(status(a, make(key, 350, version))).toBe("Not comparable");
      const b = make(key, 350, "2025.12.1");
      b.parameters["0"]!.value = { kind: "text", value: "350" };
      expect(status(a, b)).toBe("Not comparable");
      b.parameters["0"]!.valid = false;
      expect(status(a, b)).toBe("Unknown");
      b.parameters = {};
      expect(status(a, b)).toBe("Unknown");
    });
  }
});
