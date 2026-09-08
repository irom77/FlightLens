import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { api } from "./client";
// Command serialization is checked without adding a browser or second parser.
vi.stubGlobal("window", {});
beforeEach(() => clearMocks());
describe("Rust command boundary", () => {
  it("passes source IDs rather than arbitrary file paths", async () => {
    const seen = vi.fn();
    mockIPC((cmd, args) => {
      seen(cmd, args);
      return [];
    });
    await api.open("source-7");
    expect(seen).toHaveBeenCalledWith("open_source", { sourceId: "source-7" });
  });
  it("asks the backend to restore a session without naming a path", async () => {
    const seen = vi.fn();
    mockIPC((cmd, args) => {
      seen(cmd, args);
      return { sources: [], unavailable: [] };
    });
    await api.restore();
    expect(seen).toHaveBeenCalledWith("restore_session", {});
  });
  it("serializes independent profile and export controls", async () => {
    const seen = vi.fn();
    mockIPC((cmd, args) => {
      seen(cmd, args);
      return {};
    });
    const request = {
      groups: ["rates"],
      pidProfile: 1,
      rateProfile: 2,
      destinationHeader: "# Betaflight / test",
      includeSave: false,
    };
    await api.export("hash", request);
    expect(seen).toHaveBeenCalledWith("export_snippet", {
      configId: "hash",
      request,
    });
  });
  it("forwards backend failures without disguising them as empty success", async () => {
    mockIPC(() => {
      throw new Error("Source is not registered");
    });
    await expect(api.open("bad")).rejects.toThrow("Source is not registered");
  });
});
