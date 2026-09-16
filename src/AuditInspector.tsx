import type { Inspection } from "./bindings/core";
export function AuditInspector({
  inspection,
  source,
}: {
  inspection: Inspection;
  source: (n: number) => void;
}) {
  return (
    <>
      <p className="notice">
        {
          inspection.audits.filter((a) =>
            ["pass", "finding"].includes(a.status),
          ).length
        }{" "}
        / {inspection.audits.length} rules evaluated. No findings does not mean
        safe to fly.
      </p>
      {inspection.audits.map((a) => (
        <article className="audit-card" key={a.id}>
          <div>
            <span className={`audit-status ${a.status}`}>
              {a.status.replaceAll("_", " ")}
            </span>
            <small>{a.severity}</small>
          </div>
          <h3>{a.id.replaceAll("-", " ")}</h3>
          <p>{a.explanation}</p>
          <div className="actions">
            {a.lines.map((n) => (
              <button
                className="source-value"
                key={n}
                onClick={() => source(n)}
              >
                Line {n}
              </button>
            ))}
            {a.reference && (
              <small className="muted" title={a.reference}>
                Schema reference: Betaflight{" "}
                {a.reference.split("/blob/")[1]?.split("/")[0]} ·{" "}
                {a.reference.split("/src/main/")[1]}
              </small>
            )}
          </div>
        </article>
      ))}
    </>
  );
}
