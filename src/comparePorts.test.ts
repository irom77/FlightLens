import { describe, expect, it } from "vitest";
import type { ConfigDocument, Port } from "./bindings/core";
import { comparePorts } from "./comparePorts";
const port = (identifier = 0): Port => ({
  identifier,
  name: `Port ${identifier}`,
  mask: 0,
  functions: [],
  baud: [115200, 57600, 0, 115200],
  line: 1,
});
const document = (ports: Port[]): ConfigDocument =>
  ({
    ports,
    firmware: {
      family: "Betaflight",
      version: "4.5.0",
      packId: "betaflight-4.5.0-schema-1",
    },
  }) as ConfigDocument;
const status = (a: Port, b: Port) =>
  comparePorts(document([a]), document([b]))[0].status;
describe("serial allocation comparison", () => {
  it("matches identifiers independently of order and display names, keeping missing ports unknown", () => {
    const a = document([port(20), port(0)]),
      b = document([port(1), { ...port(0), name: "UART1" }]);
    expect(
      comparePorts(a, b).map((row) => [row.identifier, row.status]),
    ).toEqual([
      [0, "Equal"],
      [1, "Unknown"],
      [20, "Unknown"],
    ]);
    expect(comparePorts(b, a).map((row) => row.status)).toEqual(
      comparePorts(a, b).map((row) => row.status),
    );
    expect(comparePorts(document([]), document([]))).toEqual([]);
  });
  it("compares masks including unknown bits and every baud slot including AUTO", () => {
    expect(status(port(), port())).toBe("Equal");
    expect(status(port(), { ...port(), mask: 2147483648 })).toBe("Changed");
    for (let i = 0; i < 4; i++) {
      const b = port();
      b.baud[i] = i === 2 ? 9600 : 0;
      expect(status(port(), b)).toBe("Changed");
      expect(status(b, port())).toBe("Changed");
    }
  });
  it("requires matching firmware identity even when allocations are identical", () => {
    const a = document([port()]);
    for (const firmware of [
      { ...a.firmware, version: "4.5.1" },
      { ...a.firmware, version: "2025.12.1" },
      { ...a.firmware, version: "4.5.0.KAACK_V19" },
      { ...a.firmware, version: null },
      { ...a.firmware, family: "INAV" },
      { ...a.firmware, packId: null },
      { ...a.firmware, packId: "other" },
    ])
      expect(comparePorts(a, { ...a, firmware })[0].status).toBe(
        "Not comparable",
      );
  });
});
