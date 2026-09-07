import { useState } from "react";
import type { Curve } from "./bindings/core";
export function Plot({
  curves,
  frequency = false,
}: {
  curves: Curve[];
  frequency?: boolean;
}) {
  const [hover, setHover] = useState<number | null>(null);
  const valid = curves.filter((c) => c.points.length);
  const max = frequency
    ? 0
    : Math.max(
        200,
        ...valid.flatMap((c) => c.points.map((p) => Math.abs(p.y))),
      );
  const min = frequency ? -60 : -max;
  const xmax = frequency
    ? Math.max(1, ...valid.flatMap((c) => c.points.map((p) => p.x)))
    : 1;
  const xmin = frequency ? 0 : -1;
  const x = (v: number) => 62 + ((v - xmin) / (xmax - xmin)) * 710;
  const y = (v: number) => 26 + ((max - Math.max(min, v)) / (max - min)) * 288;
  return (
    <div className="plot-wrap">
      <svg
        viewBox="0 0 800 365"
        role="img"
        aria-label={
          frequency
            ? "Static filter frequency response"
            : "Rate curves: stick input versus angular velocity"
        }
        onMouseLeave={() => setHover(null)}
        onMouseMove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          setHover(
            Math.max(
              xmin,
              Math.min(
                xmax,
                xmin +
                  ((((e.clientX - rect.left) / rect.width) * 800 - 62) / 710) *
                    (xmax - xmin),
              ),
            ),
          );
        }}
      >
        {[0, 0.25, 0.5, 0.75, 1].map((t) => (
          <g key={t}>
            <line
              x1="62"
              x2="772"
              y1={26 + t * 288}
              y2={26 + t * 288}
              className="gridline"
            />
            <text x="51" y={30 + t * 288} textAnchor="end">
              {Math.round(max - t * (max - min))}
            </text>
            <line
              x1={62 + t * 710}
              x2={62 + t * 710}
              y1="26"
              y2="314"
              className="gridline"
            />
            <text x={62 + t * 710} y="339" textAnchor="middle">
              {frequency
                ? Math.round(t * xmax)
                : `${Math.round((t * 2 - 1) * 100)}%`}
            </text>
          </g>
        ))}
        {valid.map((c, i) => (
          <path
            key={c.name}
            d={c.points
              .map((p, j) => `${j ? "L" : "M"}${x(p.x)},${y(p.y)}`)
              .join(" ")}
            fill="none"
            className={`series-${i % 3}`}
            strokeWidth="2.5"
            strokeDasharray={i === 1 ? "7 3" : i === 2 ? "2 3" : undefined}
          />
        ))}
        {hover !== null && (
          <line
            x1={x(hover)}
            x2={x(hover)}
            y1="26"
            y2="314"
            className="plot-cursor"
            strokeDasharray="3 4"
          />
        )}
        <text x="417" y="362" textAnchor="middle">
          {frequency ? "Frequency · Hz" : "Stick input"}
        </text>
      </svg>
      <div className="legend">
        {valid.map((c, i) => {
          const point =
            hover === null
              ? null
              : c.points.reduce((a, b) =>
                  Math.abs(a.x - hover) < Math.abs(b.x - hover) ? a : b,
                );
          return (
            <span key={c.name} className={`series-${i % 3}`}>
              {c.name}{" "}
              {point ? `${point.y.toFixed(1)} ${frequency ? "dB" : "°/s"}` : ""}
            </span>
          );
        })}
      </div>
      {curves
        .filter((c) => c.reason)
        .map((c) => (
          <p className="muted" key={c.name}>
            {c.name}: {c.reason}
          </p>
        ))}
    </div>
  );
}
