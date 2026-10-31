/**
 * 前后端命令类型与调用边界，集中处理管理员权限提示。对应的隔离测试。
 *
 * @author lhy
 */
import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { api, adminRequired } from "./api";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  /**
   * 在测试中模拟桌面运行环境。
   *
   * @author lhy
   */
  isTauri: () => true,
}));
beforeEach(() => vi.resetAllMocks());
/**
 * 验证仅明确的权限错误触发授权引导，其他错误不触发提权。
 *
 * @author lhy
 */
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
/**
 * 验证重启只提交配置，不接受任意执行目标。
 *
 * @author lhy
 */
it("passes only the configuration to the fixed restart command", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(undefined);
  await api.restartAsAdmin(null);
  expect(invoke).toHaveBeenCalledWith("restart_as_admin", { config: null });
});
