/**
 * 共享偏好状态及自动保存，界面缓存仅保存语言和主题。对应的隔离测试。
 *
 * @author lhy
 */
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import { nextTick } from "vue";
const calls = vi.hoisted(() => ({
  getPreferences: vi.fn(),
  savePreferences: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  /**
   * 在测试中模拟桌面运行环境。
   *
   * @author lhy
   */
  isTauri: () => true,
}));
vi.mock("../api", () => ({ api: calls }));
beforeEach(() => {
  vi.resetModules();
  vi.resetAllMocks();
  vi.stubGlobal("localStorage", {
    /**
     * 在测试中模拟没有已缓存的外观设置。
     *
     * @author lhy
     */
    getItem: () => null,
    setItem: vi.fn(),
  });
  vi.stubGlobal("document", { documentElement: { lang: "", dataset: {} } });
});
afterEach(() => vi.unstubAllGlobals());
/**
 * 验证加载偏好后同步文档语言与主题。
 *
 * @author lhy
 */
it("loads saved language and theme and applies them to the document", async () => {
  calls.getPreferences.mockResolvedValue({
    language: "ja",
    theme: "dark",
    closeToTray: true,
    autoStart: false,
  });
  const p = await import("./usePreferences");
  await p.loadPreferences();
  await nextTick();
  expect(document.documentElement.lang).toBe("ja");
  expect(document.documentElement.dataset.theme).toBe("dark");
  expect(p.preferences.value.closeToTray).toBe(true);
});
/**
 * 验证后端拒绝保存时保留原偏好并报告错误。
 *
 * @author lhy
 */
it("keeps existing preferences when the backend rejects a change", async () => {
  const p = await import("./usePreferences");
  calls.savePreferences.mockRejectedValue("无法保存");
  await p.updatePreferences({ language: "en", autoStart: true });
  expect(p.preferences.value.language).toBe("zh-CN");
  expect(p.preferences.value.autoStart).toBe(false);
  expect(p.settingsError.value).toBe("无法保存");
  expect(p.settingsBusy.value).toBe(false);
});
