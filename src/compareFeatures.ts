import type { ConfigDocument } from "./bindings/core";

export function compareFeatures(a: ConfigDocument, b: ConfigDocument) {
  const compatible = Boolean(
    a.firmware.packId &&
      a.firmware.packId === b.firmware.packId &&
      a.firmware.family === b.firmware.family &&
      a.firmware.version &&
      a.firmware.version === b.firmware.version,
  );
  // The parser's final map accounts for repeated declarations and resets.
  return [...new Set([...Object.keys(a.features), ...Object.keys(b.features)])]
    .sort()
    .map((name) => {
      const left = a.features[name];
      const right = b.features[name];
      return {
        name,
        a: left,
        b: right,
        status:
          left === undefined || right === undefined
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : left === right
                ? "Equal"
                : "Changed",
      };
    });
}
