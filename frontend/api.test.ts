import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { api, adminRequired } from "./api";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: () => true,
}));
beforeEach(() => vi.resetAllMocks());
it("requests an elevation dialog only for explicit permission errors", async () => {
  const listener = vi.fn();
  adminRequired.addEventListener("required", listener);
  try {
    vi.mocked(invoke).mockRejectedValueOnce("ADMIN_REQUIRED");
    await expect(api.restore()).rejects.toThrow();
    expect(listener).toHaveBeenCalledTimes(1);
    vi.mocked(invoke).mockRejectedValueOnce("network failure");
    await expect(api.restore()).rejects.toBe("network failure");
    expect(listener).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalledWith(
      "restart_as_admin",
      expect.anything(),
    );
  } finally {
    adminRequired.removeEventListener("required", listener);
  }
});
it("passes only the configuration to the fixed restart command", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(undefined);
  await api.restartAsAdmin(null);
  expect(invoke).toHaveBeenCalledWith("restart_as_admin", { config: null });
});
