import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import compatibility from "./rxrangeCompatibility.json";

export type RxRange = {
  channel: number;
  min: number;
  max: number;
  source: SyntaxLine;
};

function ranges(document: ConfigDocument) {
  const values = new Map<number, RxRange>();
  for (const source of document.syntax) {
    const command = source.command;
    if (
      command.kind === "defaults" ||
      (command.kind === "malformed" &&
        source.raw.trim().split(/\s+/)[0] === "defaults")
    ) {
      values.clear();
    }
    if (command.kind === "malformed" && /^rxrange\s+/.test(source.raw.trim()))
      values.clear();
    if (command.kind !== "collection" || command.name !== "rxrange") continue;
    const operands = command.operands;
    if (
      operands.length !== 3 ||
      operands.some((value) => !/^\d+$/.test(value))
    ) {
      // Resets and unverified syntax invalidate previous explicit knowledge.
      values.clear();
      continue;
    }
    const [channel, min, max] = operands.map(Number);
    if (
      channel >= compatibility.channels ||
      min < compatibility.min ||
      min > compatibility.max ||
      max < compatibility.min ||
      max > compatibility.max
    ) {
      values.clear();
      continue;
    }
    values.set(channel, { channel, min, max, source });
  }
  return values;
}

export function compareRxRanges(a: ConfigDocument, b: ConfigDocument) {
  const releases: Record<string, string | undefined> = compatibility.releases;
  const certified = (document: ConfigDocument) =>
    Boolean(
      document.firmware.family === compatibility.family &&
        document.firmware.version &&
        document.firmware.packId &&
        releases[document.firmware.version] === document.firmware.packId,
    );
  const compatible =
    certified(a) && certified(b) && a.firmware.version === b.firmware.version;
  const left = ranges(a),
    right = ranges(b);
  return [...new Set([...left.keys(), ...right.keys()])]
    .sort((a, b) => a - b)
    .map((channel) => {
      const x = left.get(channel),
        y = right.get(channel);
      return {
        channel,
        a: x,
        b: y,
        status:
          !x || !y
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : x.min === y.min && x.max === y.max
                ? "Equal"
                : "Changed",
      };
    });
}
