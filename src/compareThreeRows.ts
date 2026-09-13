import { classifyThree } from "./compareThreeParameters";

export type PairRow<T> = { a?: T; b?: T; status: string };

export function compareThreeRows<R extends PairRow<unknown>>(
  pairs: [R[], R[], R[]],
  rowKey: (row: R) => string,
) {
  const maps = pairs.map(
    (rows) => new Map(rows.map((row) => [rowKey(row), row])),
  );
  const normalized = (status?: string) => {
    switch (status) {
      case "Equal":
      case "Matching text":
        return "Equal";
      case "Changed":
      case "Different text":
        return "Changed";
      case "Not comparable":
        return "Not comparable";
      default:
        return "Unknown";
    }
  };
  return [...new Set(maps.flatMap((map) => [...map.keys()]))]
    .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
    .map((key) => {
      const [left, right, peers] = maps.map((map) => map.get(key));
      return {
        key,
        row: (left ?? right ?? peers)!,
        values: [
          left?.a ?? right?.a,
          left?.b ?? peers?.a,
          right?.b ?? peers?.b,
        ] as [R["a"], R["a"], R["a"]],
        left: left?.status ?? "Unknown",
        right: right?.status ?? "Unknown",
        status: classifyThree(
          normalized(left?.status),
          normalized(right?.status),
          normalized(peers?.status),
        ),
      };
    });
}
