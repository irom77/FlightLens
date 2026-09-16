import { comparisonSyntax } from "./documentView";
import type { ComparisonDocument } from "./documentView";
import type { SyntaxLine } from "./bindings/core";

function collections(document: ComparisonDocument) {
  const groups = new Map<string, SyntaxLine[]>();
  for (const line of comparisonSyntax(document)) {
    if (line.command.kind !== "collection") continue;
    const name = line.command.name;
    const group = groups.get(name) ?? [];
    group.push(line);
    groups.set(name, group);
  }
  return groups;
}

export function compareCollections(
  a: ComparisonDocument,
  b: ComparisonDocument,
) {
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
