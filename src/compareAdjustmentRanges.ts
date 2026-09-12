import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import compatibility from "./vtxTableCompatibility.json";

export type AdjustmentRange = {
  aux: number;
  functionId: number;
  selectAux: number;
  center: number;
  scale: number;
  start: number;
  end: number;
  source: SyntaxLine;
};

function ranges(document: ConfigDocument) {
  const values = new Map<number, AdjustmentRange>();
  // Unknown/vendor versions retain syntactically understood records, but cannot
  // pass the comparison gate. Official 4.5 has the narrower function enum.
  const maxFunction = /^4\.5\.[0-5]$/.test(document.firmware.version ?? "")
    ? 34
    : 35;
  const range = (value: number) =>
    900 + Math.floor((Math.max(900, Math.min(2100, value)) - 900) / 25) * 25;
  for (const source of document.syntax) {
    const command = source.command;
    if (
      command.kind === "defaults" ||
      (command.kind === "malformed" &&
        source.raw.trim().split(/\s+/)[0] === "defaults")
    )
      values.clear();
    if (command.kind === "malformed" && /^adjrange\s+/.test(source.raw.trim()))
      values.clear();
    if (command.kind !== "collection" || command.name !== "adjrange") continue;
    const [
      slot,
      unused,
      aux,
      start,
      end,
      functionId,
      selectAux,
      center = 0,
      scale = 0,
    ] = command.operands.map(Number);
    if (
      ![7, 8, 9].includes(command.operands.length) ||
      !command.operands.every(
        (value) => /^\d+$/.test(value) && Number(value) <= 65535,
      ) ||
      slot > 29 ||
      unused !== 0 ||
      aux > 13 ||
      selectAux > 13 ||
      functionId > maxFunction
    ) {
      values.clear();
      continue;
    }
    values.set(slot, {
      aux,
      functionId,
      selectAux,
      center,
      scale,
      start: range(start),
      end: range(end),
      source,
    });
  }
  return values;
}

export function compareAdjustmentRanges(a: ConfigDocument, b: ConfigDocument) {
  const releases: Record<string, string | undefined> = compatibility.releases;
  const certified = ({ firmware }: ConfigDocument) =>
    Boolean(
      firmware.family === compatibility.family &&
        /^(4\.5\.[0-5]|2025\.12\.[1-5])$/.test(firmware.version ?? "") &&
        firmware.version &&
        firmware.packId &&
        releases[firmware.version] === firmware.packId,
    );
  const compatible =
    certified(a) && certified(b) && a.firmware.version === b.firmware.version;
  const left = ranges(a),
    right = ranges(b);
  const fields = [
    "aux",
    "functionId",
    "selectAux",
    "center",
    "scale",
    "start",
    "end",
  ] as const;
  return [...new Set([...left.keys(), ...right.keys()])]
    .sort((a, b) => a - b)
    .map((slot) => {
      const x = left.get(slot),
        y = right.get(slot);
      return {
        slot,
        a: x,
        b: y,
        status:
          !x || !y
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : fields.every((key) => x[key] === y[key])
                ? "Equal"
                : "Changed",
      };
    });
}
