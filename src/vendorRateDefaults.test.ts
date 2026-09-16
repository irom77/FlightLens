import { execFileSync } from "node:child_process";
import { expect, it } from "vitest";
import type { DocumentView } from "./bindings/core";
import { compareParameters } from "./compareParameters";
import { compareThreeParameters } from "./compareThreeParameters";

it("compares source-verified vendor defaults with explicit values and retains provenance", () => {
  const fixture = (args: string[]): DocumentView =>
    JSON.parse(
      execFileSync(
        process.env.CARGO ?? "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "flightlens-core",
          "--bin",
          "preview_fixture",
          "--",
          ...args,
        ],
        { encoding: "utf8" },
      ),
    ).artifact.document;
  for (const modern of [false, true]) {
    const flags = modern ? ["--vendor-year-defaults"] : [];
    const declared = fixture(["--vendor-rates", "--zero-expo", ...flags]);
    const recovered = fixture(["--vendor-missing-expo", ...flags]);
    const changed = fixture(["--vendor-rates", ...flags]);
    const profile = { pid: 0, rate: 0 };
    const rows = compareParameters(declared, recovered, profile, profile);
    for (const axis of ["roll", "pitch", "yaw"]) {
      const row = rows.find((r) => r.key === `${axis}_expo`)!;
      expect(row.status).toBe("Equal");
      expect(row.a?.declared?.line).toBeGreaterThan(0);
      expect(row.b?.declared).toBeUndefined();
      expect(row.b?.derived?.sourceVersion).toBe(
        modern ? "2025.12.3-alpha.KAACK_V19" : "4.5.3.KAACK_V19",
      );
    }
    const three = compareThreeParameters(
      [declared, changed, recovered],
      [profile, profile, profile],
    );
    expect(three.find((r) => r.key === "roll_expo")?.status).toBe("Left only");
  }
}, 120_000);
