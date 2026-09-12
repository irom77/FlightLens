import type { ConfigDocument, SyntaxLine } from "./bindings/core";

function collections(document: ConfigDocument) {
  const groups = new Map<string, SyntaxLine[]>();
  for (const line of document.syntax) {
    if (line.command.kind !== "collection") continue;
    const name = line.command.name;
    const group = groups.get(name) ?? [];
    group.push(line);
    groups.set(name, group);
  }
  return groups;
}

export function compareCollections(a: ConfigDocument, b: ConfigDocument) {
  const left = collections(a),
    right = collections(b);
  return [...new Set([...left.keys(), ...right.keys()])].sort().map((name) => {
    const x = left.get(name),
      y = right.get(name);
    return {
      name,
      a: x,
      b: y,
      status:
        !x || !y
          ? "Unknown"
          : x.length === y.length && x.every((line, i) => line.raw === y[i].raw)
            ? "Matching text"
            : "Different text",
    };
  });
}
