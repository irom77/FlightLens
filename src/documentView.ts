import type { ConfigDocument, DocumentView } from "./bindings/core";
export type { DocumentView } from "./bindings/core";

// Core regression tests compare full documents with runtime compact views.
export type ComparisonDocument = ConfigDocument | DocumentView;

export function comparisonSyntax(document: ComparisonDocument) {
  return "sourceEvidence" in document
    ? document.sourceEvidence.comparisonSyntax
    : document.syntax;
}

export function sourceLine(document: ComparisonDocument, line: number) {
  return comparisonSyntax(document).find((source) => source.line === line);
}
