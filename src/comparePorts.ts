import type { ConfigDocument } from "./bindings/core";

export function comparePorts(a: ConfigDocument, b: ConfigDocument) {
  const compatible = Boolean(
    a.firmware.packId &&
      a.firmware.packId === b.firmware.packId &&
      a.firmware.family === b.firmware.family &&
      a.firmware.version &&
      a.firmware.version === b.firmware.version,
  );
  const left = new Map(a.ports.map((port) => [port.identifier, port]));
  const right = new Map(b.ports.map((port) => [port.identifier, port]));
  return [...new Set([...left.keys(), ...right.keys()])]
    .sort((a, b) => a - b)
    .map((identifier) => {
      const x = left.get(identifier),
        y = right.get(identifier);
      return {
        identifier,
        a: x,
        b: y,
        status:
          !x || !y
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : x.mask === y.mask &&
                  x.baud.every((baud, index) => baud === y.baud[index])
                ? "Equal"
                : "Changed",
      };
    });
}
