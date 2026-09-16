import { create } from "zustand";
import type { ComparisonDocument } from "../documentView";

export type Profiles = { rate: number; pid: number };
export type ComparisonSelection = {
  documents: string[];
  baseline: string | null;
};

export function defaultProfiles(d: ComparisonDocument): Profiles {
  const first = (kind: "pid" | "rate", indices: number[]) => {
    const populated = indices.filter((index) =>
      [...Object.values(d.parameters), ...Object.values(d.derived)].some(
        (p) => p?.scope.kind === kind && p.scope.index === index,
      ),
    );
    return populated.includes(0) ? 0 : (populated[0] ?? indices[0] ?? 0);
  };
  return {
    rate: first("rate", d.rateProfiles),
    pid: first("pid", d.pidProfiles),
  };
}

export function restoredProfiles(d: ComparisonDocument, saved: Profiles) {
  const profiles = { ...saved };
  const warnings: string[] = [];
  for (const kind of ["rate", "pid"] as const) {
    const options = kind === "rate" ? d.rateProfiles : d.pidProfiles;
    if (!(options.length ? options : [0]).includes(saved[kind])) {
      profiles[kind] = defaultProfiles(d)[kind];
      warnings.push(
        `${d.title}: saved ${kind.toUpperCase()} profile ${saved[kind] + 1} unavailable; showing profile ${profiles[kind] + 1}.`,
      );
    }
  }
  return { profiles, warnings };
}

interface Selections {
  profiles: Record<string, Profiles>;
  comparison: ComparisonSelection | null;
  setProfile: (
    d: ComparisonDocument,
    kind: keyof Profiles,
    value: number,
  ) => void;
  setComparison: (comparison: ComparisonSelection | null) => void;
}
export const useSelections = create<Selections>((set) => ({
  profiles: {},
  comparison: null,
  setProfile: (d, kind, value) =>
    set((s) => ({
      profiles: {
        ...s.profiles,
        [d.id]: { ...(s.profiles[d.id] ?? defaultProfiles(d)), [kind]: value },
      },
    })),
  setComparison: (comparison) => set({ comparison }),
}));

export function useProfiles(d?: ComparisonDocument) {
  const state = useSelections();
  const profiles = d
    ? (state.profiles[d.id] ?? defaultProfiles(d))
    : { rate: 0, pid: 0 };
  return {
    ...profiles,
    setRate: (value: number) => {
      if (d) state.setProfile(d, "rate", value);
    },
    setPid: (value: number) => {
      if (d) state.setProfile(d, "pid", value);
    },
  };
}
