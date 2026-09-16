import type { ComparisonDocument } from "./documentView";
export function defaultProfile(profiles: number[]): number {
  return profiles.includes(0) ? 0 : (profiles[0] ?? 0);
}

export function populatedProfiles(
  d: ComparisonDocument,
  kind: "pid" | "rate",
): number[] {
  const settings = [
    ...Object.values(d.parameters),
    ...Object.values(d.derived),
  ];
  return (kind === "pid" ? d.pidProfiles : d.rateProfiles).filter((index) =>
    settings.some(
      (setting) =>
        setting?.scope.kind === kind && setting.scope.index === index,
    ),
  );
}
