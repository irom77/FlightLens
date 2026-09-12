import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import compatibility from "./rxfailCompatibility.json";

export type RxFail = {
  channel: number;
  mode: "a" | "h" | "s";
  value?: number;
  source: SyntaxLine;
};

function failsafes(document: ConfigDocument) {
  const values = new Map<number, RxFail>();
  for (const source of document.syntax) {
    const command = source.command;
    if (
      command.kind === "defaults" ||
      (command.kind === "malformed" &&
        source.raw.trim().split(/\s+/)[0] === "defaults")
    ) {
      values.clear();
    }
    if (command.kind === "malformed" && /^rxfail\s+/.test(source.raw.trim()))
      values.clear();
    if (command.kind !== "collection" || command.name !== "rxfail") continue;
    const [channelText, mode, valueText] = command.operands;
    const channel = Number(channelText);
    const validChannel =
      /^\d+$/.test(channelText ?? "") && channel < compatibility.channels;
    if (validChannel && command.operands.length === 1) continue;
    const value = Number(valueText);
    if (
      !validChannel ||
      !(
        ((mode === "h" || (mode === "a" && channel < 4)) &&
          command.operands.length === 2) ||
        (mode === "s" &&
          command.operands.length === 3 &&
          /^\d+$/.test(valueText) &&
          value >= compatibility.min &&
          value <= compatibility.max)
      )
    ) {
      values.clear();
      continue;
    }
    values.set(channel, {
      channel,
      mode: mode as RxFail["mode"],
      value:
        mode === "s"
          ? compatibility.min +
            Math.floor((value - compatibility.min) / 25) * 25
          : undefined,
      source,
    });
  }
  return values;
}

export function compareRxFails(a: ConfigDocument, b: ConfigDocument) {
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
  const left = failsafes(a),
    right = failsafes(b);
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
              : x.mode === y.mode && x.value === y.value
                ? "Equal"
                : "Changed",
      };
    });
}
