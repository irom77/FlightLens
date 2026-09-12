import equivalence from "./parameterEquivalence.json";
import type {
  ConfigDocument,
  Derived,
  Parameter,
  Scope,
} from "./bindings/core";

export type ProfileSelection = { pid: number; rate: number };
export type ComparisonValue = { declared?: Parameter; derived?: Derived };
export type ParameterDifference = {
  key: string;
  scope: Scope["kind"];
  a?: ComparisonValue;
  b?: ComparisonValue;
  status: "Changed" | "Equal" | "Unknown" | "Not comparable";
};

function selected(scope: Scope, profiles: ProfileSelection) {
  return !("index" in scope) || scope.index === profiles[scope.kind];
}

function values(document: ConfigDocument, profiles: ProfileSelection) {
  const result = new Map<string, ComparisonValue>();
  for (const derived of Object.values(document.derived)) {
    if (derived && selected(derived.scope, profiles))
      result.set(`${derived.scope.kind}:${derived.key}`, { derived });
  }
  for (const declared of Object.values(document.parameters)) {
    if (declared && selected(declared.scope, profiles))
      result.set(`${declared.scope.kind}:${declared.semanticKey}`, {
        declared,
      });
  }
  return result;
}

export function compareParameters(
  a: ConfigDocument,
  b: ConfigDocument,
  pa: ProfileSelection,
  pb: ProfileSelection,
): ParameterDifference[] {
  const av = values(a, pa);
  const bv = values(b, pb);
  // A shared key or schema alone does not prove cross-version equivalence.
  const compatible = Boolean(
    a.firmware.packId &&
      a.firmware.packId === b.firmware.packId &&
      a.firmware.family === b.firmware.family &&
      a.firmware.version &&
      a.firmware.version === b.firmware.version,
  );
  const certifiedRelease = (document: ConfigDocument) => {
    const { family, version, packId } = document.firmware;
    return (
      family === equivalence.family &&
      version != null &&
      Object.hasOwn(equivalence.releases, version) &&
      (equivalence.releases as Record<string, string>)[version] === packId
    );
  };
  const crossVersion = certifiedRelease(a) && certifiedRelease(b);
  const known = (v?: ComparisonValue) =>
    v?.declared
      ? v.declared.valid && v.declared.supported
        ? v.declared.value
        : undefined
      : v?.derived?.value;
  return [...new Set([...av.keys(), ...bv.keys()])].sort().map((id) => {
    const left = av.get(id);
    const right = bv.get(id);
    const x = known(left);
    const y = known(right);
    const scope = (left?.declared ??
      left?.derived ??
      right?.declared ??
      right?.derived)!.scope.kind;
    const key = id.slice(id.indexOf(":") + 1);
    const mapping = equivalence.mappings.find(
      (entry) => entry.key === key && entry.scope === scope,
    );
    const mapped =
      crossVersion &&
      mapping &&
      x?.kind === "integer" &&
      y?.kind === "integer" &&
      x.value >= mapping.min &&
      x.value <= mapping.max &&
      y.value >= mapping.min &&
      y.value <= mapping.max;
    return {
      key,
      scope,
      a: left,
      b: right,
      status:
        !x || !y || scope === "unknown"
          ? "Unknown"
          : !(compatible || mapped)
            ? "Not comparable"
            : x.kind === y.kind && x.value === y.value
              ? "Equal"
              : "Changed",
    };
  });
}
