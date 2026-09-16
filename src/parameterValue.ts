import type { Parameter } from "./bindings/core";
export const value = (p: Parameter) => String(p.value.value);
