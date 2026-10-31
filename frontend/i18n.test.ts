/**
 * 界面文案与参数插值，按当前语言选择对应文本。对应的隔离测试。
 *
 * @author lhy
 */
import { afterEach, expect, it } from "vitest";
import { language, t } from "./i18n";
afterEach(() => {
  language.value = "zh-CN";
});
/**
 * 验证四种语言切换与文案参数插值。
 *
 * @author lhy
 */
it("switches interface text and interpolates values for all four languages", () => {
  const labels = [];
  for (const locale of ["zh-CN", "zh-TW", "en", "ja"] as const) {
    language.value = locale;
    labels.push(t("config"));
    expect(t("tooFew", { count: 2 })).toContain("2");
    expect(t("newVersion", { version: "1.0.0" })).toContain("1.0.0");
  }
  expect(new Set(labels).size).toBe(4);
});
