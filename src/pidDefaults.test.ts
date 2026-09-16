import { execFileSync } from "node:child_process";
import { expect, it } from "vitest";
import type { DocumentView } from "./bindings/core";
import { compareParameters } from "./compareParameters";
import { compareThreeParameters } from "./compareThreeParameters";

it("compares real Rust recovered PID values across independent profiles", () => {
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
  const declared = fixture([]);
  const recovered = fixture(["--pid-defaults"]);
  const p0 = { pid: 0, rate: 0 };
  const p1 = { pid: 1, rate: 0 };
  const equal = compareParameters(declared, recovered, p0, p0);
  const roll = equal.find((r) => r.key === "p_roll")!;
  expect(roll.status).toBe("Equal");
  expect(roll.a?.declared?.line).toBeGreaterThan(0);
  expect(roll.b?.derived?.sourceVersion).toBe("4.5.0");
  expect(roll.b?.declared).toBeUndefined();
  expect(equal.find((r) => r.key === "d_roll")?.status).toBe("Unknown");
  expect(equal.find((r) => r.key === "d_yaw")?.status).toBe("Equal");
  const changed = compareParameters(declared, recovered, p0, p1);
  expect(changed.find((r) => r.key === "p_roll")?.status).toBe("Changed");
  expect(changed.find((r) => r.key === "i_roll")?.status).toBe("Unknown");
  const three = compareThreeParameters(
    [declared, recovered, recovered],
    [p0, p1, p0],
  );
  expect(three.find((r) => r.key === "p_roll")?.status).toBe("Left only");
  expect(three.find((r) => r.key === "i_roll")?.status).toBe("Unknown");
}, 120_000);
