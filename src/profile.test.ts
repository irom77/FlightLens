import { describe, expect, it } from "vitest";
import { bestProfile } from "./App";
import type { ConfigDocument } from "./bindings/core";

const document = (parameters: ConfigDocument["parameters"]): ConfigDocument =>
  ({
    pidProfiles: [0, 1],
    rateProfiles: [0, 1],
    selectedPid: 0,
    selectedRate: 0,
    parameters,
  }) as ConfigDocument;

const known = (key: string) =>
  ({ key, valid: true }) as ConfigDocument["parameters"][string];

describe("bestProfile", () => {
  it("chooses a populated profile when the dump restores an empty one", () => {
    const d = document({
      "pid:1:p_roll": known("p_roll"),
      "pid:1:i_roll": known("i_roll"),
      "rate:1:rates_type": known("rates_type"),
      "rate:1:roll_rc_rate": known("roll_rc_rate"),
      "rate:1:pitch_rc_rate": known("pitch_rc_rate"),
      "rate:1:yaw_rc_rate": known("yaw_rc_rate"),
    });
    expect(bestProfile(d, "pid")).toBe(1);
    expect(bestProfile(d, "rate")).toBe(1);
  });
});
