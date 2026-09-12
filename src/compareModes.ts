import type { ConfigDocument } from "./bindings/core";

export function compareModes(a: ConfigDocument, b: ConfigDocument) {
  const compatible = Boolean(
    a.firmware.packId &&
      a.firmware.packId === b.firmware.packId &&
      a.firmware.family === b.firmware.family &&
      a.firmware.version &&
      a.firmware.version === b.firmware.version,
  );
  const left = new Map(a.modes.map((mode) => [mode.index, mode]));
  const right = new Map(b.modes.map((mode) => [mode.index, mode]));
  return [...new Set([...left.keys(), ...right.keys()])]
    .sort((a, b) => a - b)
    .map((index) => {
      const x = left.get(index),
        y = right.get(index);
      return {
        index,
        a: x,
        b: y,
        status:
          !x || !y
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : x.logic === null ||
                  y.logic === null ||
                  x.linked === null ||
                  y.linked === null
                ? "Unknown"
                : x.modeId === y.modeId &&
                    x.channel === y.channel &&
                    x.start === y.start &&
                    x.end === y.end &&
                    x.logic === y.logic &&
                    x.linked === y.linked
                  ? "Equal"
                  : "Changed",
      };
    });
}
