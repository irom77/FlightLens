import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { applyVtxTableLine, type VtxTableValue } from "./compareVtxTables";
import compatibility from "./vtxTableCompatibility.json";

export type VtxActivation = {
  aux: number;
  band: number;
  channel: number;
  power: number;
  start: number;
  end: number;
  source: SyntaxLine;
};

function activations(document: ConfigDocument) {
  const values = new Map<number, VtxActivation>();
  const table = new Map<string, VtxTableValue>();
  const selector = (value: number, key: string, fallback: number) =>
    value === 0 ||
    (table.has(key) &&
      value <= Math.min(Number(table.get(key)!.value), fallback));
  const range = (value: number) =>
    900 + Math.floor((Math.max(900, Math.min(2100, value)) - 900) / 25) * 25;
  for (const source of document.syntax) {
    applyVtxTableLine(table, source);
    const command = source.command;
    if (
      command.kind === "defaults" ||
      (command.kind === "malformed" &&
        source.raw.trim().split(/\s+/)[0] === "defaults")
    )
      values.clear();
    if (command.kind === "malformed" && /^vtx\s+/.test(source.raw.trim()))
      values.clear();
    if (command.kind !== "collection" || command.name !== "vtx") continue;
    const [slot, aux, band, channel, power, start, end] =
      command.operands.map(Number);
    if (
      command.operands.length !== 7 ||
      !command.operands.every(
        (value) => /^\d+$/.test(value) && Number(value) <= 65535,
      ) ||
      slot > 9 ||
      aux > 13 ||
      !selector(band, "bands", 5) ||
      !selector(channel, "channels", 8) ||
      !selector(power, "powerlevels", 5)
    ) {
      values.clear();
      continue;
    }
    values.set(slot, {
      aux,
      band,
      channel,
      power,
      start: range(start),
      end: range(end),
      source,
    });
  }
  return values;
}

export function compareVtxActivations(a: ConfigDocument, b: ConfigDocument) {
  const releases: Record<string, string | undefined> = compatibility.releases;
  const certified = ({ firmware }: ConfigDocument) =>
    Boolean(
      firmware.family === compatibility.family &&
        /^4\.5\.[0-5]$/.test(firmware.version ?? "") &&
        firmware.version &&
        firmware.packId &&
        releases[firmware.version] === firmware.packId,
    );
  const compatible =
    certified(a) && certified(b) && a.firmware.version === b.firmware.version;
  const left = activations(a),
    right = activations(b);
  const fields = ["aux", "band", "channel", "power", "start", "end"] as const;
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
