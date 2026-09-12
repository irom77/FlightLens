import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import compatibility from "./vtxTableCompatibility.json";

export type VtxTableValue = { value: string; source: SyntaxLine };
const dimensions = ["bands", "channels", "powerlevels"];
const unsigned = (text: string | undefined, max: number) =>
  /^\d+$/.test(text ?? "") && Number(text) <= max;
const token = (text: string | undefined, max: number) =>
  Boolean(text && text.length <= max && /^[!-~]+$/.test(text));

function table(document: ConfigDocument) {
  const values = new Map<string, VtxTableValue>();
  const count = (key: string) => {
    const entry = values.get(key);
    return entry ? Number(entry.value) : undefined;
  };
  for (const source of document.syntax) {
    const command = source.command;
    if (
      command.kind === "defaults" ||
      (command.kind === "malformed" &&
        source.raw.trim().split(/\s+/)[0] === "defaults")
    )
      values.clear();
    if (command.kind === "malformed" && /^vtxtable\s+/.test(source.raw.trim()))
      values.clear();
    if (command.kind !== "collection" || command.name !== "vtxtable") continue;
    const [key, ...args] = command.operands;
    const set = (name: string, value: string) =>
      values.set(name, { value, source });
    if (dimensions.includes(key) && args.length === 1 && unsigned(args[0], 8)) {
      if (count(key) !== Number(args[0])) {
        for (const name of values.keys()) {
          if (
            (key === "bands" || key === "channels") &&
            name.startsWith("band ")
          )
            values.delete(name);
          if (
            key === "powerlevels" &&
            (name === "powervalues" || name === "powerlabels")
          )
            values.delete(name);
        }
      }
      set(key, String(Number(args[0])));
      continue;
    }
    const bands = count("bands"),
      channels = count("channels"),
      levels = count("powerlevels");
    if (
      key === "band" &&
      bands !== undefined &&
      channels !== undefined &&
      channels > 0 &&
      unsigned(args[0], bands) &&
      Number(args[0]) >= 1 &&
      args.length === 4 + channels &&
      token(args[1], 8) &&
      token(args[2], 1) &&
      ["FACTORY", "CUSTOM"].includes(args[3]?.toUpperCase()) &&
      args.slice(4).every((value) => unsigned(value, 65535))
    ) {
      set(
        `band ${Number(args[0])}`,
        [
          args[1].toUpperCase(),
          args[2].toUpperCase(),
          args[3].toUpperCase(),
          ...args.slice(4).map(Number),
        ].join(" "),
      );
      continue;
    }
    if (
      (key === "powervalues" || key === "powerlabels") &&
      levels !== undefined &&
      args.length === levels &&
      args.every((value) =>
        key === "powervalues" ? unsigned(value, 65535) : token(value, 3),
      )
    ) {
      set(
        key,
        args
          .map((value) =>
            key === "powervalues" ? String(Number(value)) : value.toUpperCase(),
          )
          .join(" "),
      );
      continue;
    }
    values.clear();
  }
  return values;
}

export function compareVtxTables(a: ConfigDocument, b: ConfigDocument) {
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
  const left = table(a),
    right = table(b);
  const order = [
    ...dimensions,
    ...Array.from({ length: 8 }, (_, i) => `band ${i + 1}`),
    "powervalues",
    "powerlabels",
  ];
  return order
    .filter((key) => left.has(key) || right.has(key))
    .map((key) => {
      const x = left.get(key),
        y = right.get(key);
      return {
        key,
        a: x,
        b: y,
        status:
          !x || !y
            ? "Unknown"
            : !compatible
              ? "Not comparable"
              : x.value === y.value
                ? "Equal"
                : "Changed",
      };
    });
}
