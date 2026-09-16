// Run with node --expose-gc. Only the Rust probe's built-in synthetic inputs
// are used; stdout contains aggregate measurements, never document contents.
import { execFileSync, spawnSync } from "node:child_process";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

if (!global.gc)
  throw new Error("Run node --expose-gc tools/measure_document_cost.mjs");
const binary = new URL(
  "../target/release/document_cost" +
    (process.platform === "win32" ? ".exe" : ""),
  import.meta.url,
);
const rows = [];
for (const scenario of ["fixture", "64k", "1m", "16m", "settings"]) {
  for (const format of ["full", "compact"]) {
    // Capture both streams without printing the synthetic payload.
    const result = spawnSync(fileURLToPath(binary), [scenario, format], {
      encoding: "utf8",
      maxBuffer: 128 * 1024 * 1024,
    });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(result.stderr);
    const metrics = result.stderr.trim();
    const payload = result.stdout;
    global.gc();
    const before = process.memoryUsage().heapUsed;
    const start = performance.now();
    let document = JSON.parse(payload);
    const jsonParseMs = performance.now() - start;
    global.gc();
    const parsedHeapBytes = process.memoryUsage().heapUsed - before;
    if (document.kind !== "config")
      throw new Error("Expected synthetic config");
    rows.push({ ...JSON.parse(metrics), jsonParseMs, parsedHeapBytes });
    document = null;
  }
}
console.log(
  JSON.stringify(
    {
      node: process.version,
      rust: execFileSync(
        process.env.FLIGHTLENS_RUSTC ?? "rustc",
        ["--version"],
        { encoding: "utf8" },
      ).trim(),
      rows,
    },
    null,
    2,
  ),
);
