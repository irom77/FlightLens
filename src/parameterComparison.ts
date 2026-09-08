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
    return {
      key: id.slice(id.indexOf(":") + 1),
      scope,
      a: left,
      b: right,
      status:
        !x || !y || scope === "unknown"
          ? "Unknown"
          : !compatible
            ? "Not comparable"
            : x.kind === y.kind && x.value === y.value
              ? "Equal"
              : "Changed",
    };
  });
}
