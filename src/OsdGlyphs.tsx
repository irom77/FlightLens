import font from "./assets/osd-default.mcm?raw";
// MAX7456 character memory: 12 × 18 pixels, two bits per pixel, 64 bytes per glyph.
// Font rendering belongs in the presentation layer; firmware position decoding stays in Rust.
const bytes = font
  .trim()
  .split(/\r?\n/)
  .slice(1)
  .map((line) => parseInt(line, 2));
const paths = Array.from({ length: 256 }, (_, glyph) => {
  const layers = ["", ""];
  for (let pixel = 0; pixel < 216; pixel++) {
    const value =
      (bytes[glyph * 64 + Math.floor(pixel / 4)] >> (6 - (pixel % 4) * 2)) & 3;
    if (value === 0 || value === 2)
      layers[value === 2 ? 1 : 0] +=
        `M${pixel % 12} ${Math.floor(pixel / 12)}h1v1h-1z`;
  }
  return layers;
});
export function OsdGlyphs({ text }: { text: string }) {
  const chars = Array.from(text.toUpperCase());
  return (
    <svg
      viewBox={`0 0 ${chars.length * 12} 18`}
      aria-hidden="true"
      preserveAspectRatio="none"
    >
      {chars.map((char, i) => {
        const glyph = paths[char.charCodeAt(0)] ?? paths[63];
        return (
          <g key={i} transform={`translate(${i * 12} 0)`}>
            <path d={glyph[0]} fill="black" />
            <path d={glyph[1]} fill="white" />
          </g>
        );
      })}
    </svg>
  );
}
