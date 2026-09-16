import { useState } from "react";
import type { Inspection, Parameter } from "./bindings/core";
import { OsdGlyphs } from "./OsdGlyphs";
import { Empty } from "./Empty";
export function Osd({
  elements,
  source,
  parameters,
}: {
  elements: Inspection["osd"];
  source: (n: number) => void;
  parameters: Parameter[];
}) {
  const [display, setDisplay] = useState("auto");
  const [profile, setProfile] = useState(0);
  const num = (key: string) => {
    const p = parameters.find((p) => p.key === key && p.valid);
    return p && typeof p.value.value === "number" ? p.value.value : undefined;
  };
  const knownVideo = parameters.find(
    (p) => p.key === "vcd_video_system" && p.valid,
  )?.value.value;
  const mode =
    display === "auto" ? String(knownVideo ?? "PAL").toUpperCase() : display;
  const savedWidth = num("osd_canvas_width");
  const savedHeight = num("osd_canvas_height");
  const cols =
    savedWidth && savedWidth > 0 ? savedWidth : mode === "HD" ? 53 : 30;
  const rows =
    savedHeight && savedHeight > 0
      ? savedHeight
      : mode === "HD"
        ? 20
        : mode === "NTSC"
          ? 13
          : 16;
  return (
    <>
      <div className="actions">
        <label>
          Display assumption
          <select value={display} onChange={(e) => setDisplay(e.target.value)}>
            <option value="auto">From source, otherwise PAL assumption</option>
            <option value="PAL">PAL · 30 × 16</option>
            <option value="NTSC">NTSC · 30 × 13</option>
            <option value="HD">HD · 53 × 20</option>
          </select>
        </label>
        <label>
          Visibility profile
          <select
            value={profile}
            onChange={(e) => setProfile(Number(e.target.value))}
          >
            {[0, 1, 2].map((n) => (
              <option key={n} value={n}>
                {n + 1}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div
        className="osd-canvas"
        style={{
          aspectRatio: `${Math.max(1, cols)} / ${Math.max(1, rows)}`,
          backgroundSize: `${100 / Math.max(1, cols)}% ${100 / Math.max(1, rows)}%`,
        }}
      >
        <span className="canvas-label">
          {cols} × {rows} · POSITION PREVIEW
        </span>
        {elements
          .filter(
            (e) =>
              (e.visibleProfiles & (1 << profile)) !== 0 &&
              e.x < cols &&
              e.y < rows,
          )
          .map((e) => (
            <button
              key={e.key}
              className="osd-marker"
              title={`${e.key} · (${e.x}, ${e.y}) · type ${e.displayType}`}
              style={{
                left: `${(e.x / cols) * 100}%`,
                top: `${(e.y / rows) * 100}%`,
                width: `${(Array.from(e.preview).length / cols) * 100}%`,
                height: `${100 / rows}%`,
              }}
              onClick={() => source(e.line)}
              aria-label={e.key}
            >
              <OsdGlyphs text={e.preview} />
            </button>
          ))}
      </div>
      <p className="notice">
        Bundled Betaflight default font. Numeric values and glyph footprints are
        illustrative samples, not telemetry. Unsupported elements use a +
        marker. Saved canvas dimensions take precedence when present.
      </p>
      <div className="card">
        <table>
          <thead>
            <tr>
              <th>Element</th>
              <th>Position</th>
              <th>Profiles</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {elements.map((e) => (
              <tr key={e.key}>
                <td>
                  <button
                    className="source-value"
                    onClick={() => source(e.line)}
                  >
                    {e.key}
                  </button>
                </td>
                <td>
                  {e.x}, {e.y}
                </td>
                <td>
                  {[0, 1, 2]
                    .filter((n) => e.visibleProfiles & (1 << n))
                    .map((n) => n + 1)
                    .join(", ") || "Hidden"}
                </td>
                <td>
                  {e.x + Array.from(e.preview).length > cols || e.y >= rows
                    ? "Sample footprint out of bounds"
                    : e.preview === "+"
                      ? "Footprint unknown"
                      : "Sample in bounds"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {!elements.length && (
          <Empty>No supported explicit OSD positions.</Empty>
        )}
      </div>
    </>
  );
}
