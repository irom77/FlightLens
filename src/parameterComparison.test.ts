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
