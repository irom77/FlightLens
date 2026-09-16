import type { ComparisonDocument } from "./documentView";
import {
  compareParameters,
  type ParameterDifference,
  type ProfileSelection,
} from "./compareParameters";

type Status = ParameterDifference["status"];

export function classifyThree(left: Status, right: Status, peers: Status) {
  if ([left, right, peers].includes("Unknown")) return "Unknown";
  if ([left, right, peers].includes("Not comparable")) return "Not comparable";
  if (left === "Equal" && right === "Equal") return "Equal";
  if (left === "Changed" && right === "Equal") return "Left only";
  if (left === "Equal" && right === "Changed") return "Right only";
  return peers === "Equal" ? "Same change" : "Conflict";
}

export function compareThreeParameters(
  documents: [ComparisonDocument, ComparisonDocument, ComparisonDocument],
  profiles: [ProfileSelection, ProfileSelection, ProfileSelection],
) {
  const pairs = [
    [0, 1],
    [0, 2],
    [1, 2],
  ].map(
    ([a, b]) =>
      new Map(
        compareParameters(
          documents[a],
          documents[b],
          profiles[a],
          profiles[b],
        ).map((row) => [`${row.scope}:${row.key}`, row]),
      ),
  );
  return [...new Set(pairs.flatMap((pair) => [...pair.keys()]))]
    .sort()
    .map((id) => {
      const [left, right, peers] = pairs.map((pair) => pair.get(id));
      const row = (left ?? right ?? peers)!;
      return {
        key: row.key,
        scope: row.scope,
        values: [
          left?.a ?? right?.a,
          left?.b ?? peers?.a,
          right?.b ?? peers?.b,
        ],
        left: left?.status ?? "Unknown",
        right: right?.status ?? "Unknown",
        status: classifyThree(
          left?.status ?? "Unknown",
          right?.status ?? "Unknown",
          peers?.status ?? "Unknown",
        ),
      };
    });
}
