/**
 * 共享偏好状态及自动保存，界面缓存仅保存语言和主题。
 *
 * @author lhy
 */
import { notify } from "../notifications";
import { ref, watchEffect } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { api, type Preferences } from "../api";
import { language, t } from "../i18n";
const defaults: Preferences = {
  language: "zh-CN",
  theme: "system",
  closeToTray: null,
  autoStart: false,
  internalProbeTarget: "",
  externalProbeTarget: "223.5.5.5",
};
export const preferences = ref<Preferences>({ ...defaults });
export const settingsBusy = ref(false),
  settingsError = ref(""),
  settingsSaved = ref(false);
const media =
  typeof window !== "undefined"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;
const systemDark = ref(media?.matches ?? false);
media?.addEventListener("change", (e) => {
  systemDark.value = e.matches;
});
if (typeof localStorage !== "undefined") {
  try {
    const p = JSON.parse(localStorage.getItem("splitroute-appearance") ?? "{}");
    if (["zh-CN", "zh-TW", "en", "ja"].includes(p.language))
      preferences.value.language = p.language;
    if (["light", "dark", "system"].includes(p.theme))
      preferences.value.theme = p.theme;
  } catch {
    // 外观缓存损坏时保留默认值，避免缓存阻止应用启动。
  }
}
watchEffect(() => {
  language.value = preferences.value.language;
  if (typeof document !== "undefined") {
    document.documentElement.lang = language.value;
    document.documentElement.dataset.theme =
      preferences.value.theme === "system"
        ? systemDark.value
          ? "dark"
          : "light"
        : preferences.value.theme;
  }
});
/**
 * 缓存界面语言与主题，减少启动时的外观跳变。
 *
 * @author lhy
 */
function cache() {
  localStorage.setItem(
    "splitroute-appearance",
    JSON.stringify({
      language: preferences.value.language,
      theme: preferences.value.theme,
    }),
  );
}
/**
 * 加载后端偏好并同步外观缓存，读取失败时保留当前值。
 *
 * @author lhy
 */
export async function loadPreferences() {
  if (!isTauri()) return;
  settingsBusy.value = true;
  try {
    preferences.value = await api.getPreferences();
    cache();
  } catch (e) {
    settingsError.value = String(e);
    if (!String(e).includes(t("adminRequired")))
      notify(settingsError.value, "error");
  } finally {
    settingsBusy.value = false;
  }
}
/**
 * 合并偏好修改，保存成功后更新共享状态与界面缓存。
 *
 * @param patch 本次修改的偏好字段
 * @author lhy
 */
export async function updatePreferences(patch: Partial<Preferences>) {
  if (settingsBusy.value) return;
  settingsBusy.value = true;
  settingsError.value = "";
  settingsSaved.value = false;
  try {
    const next = { ...preferences.value, ...patch };
    if (isTauri()) await api.savePreferences(next);
    else if (patch.autoStart !== undefined || patch.closeToTray !== undefined)
      throw new Error(t("preview"));
    preferences.value = next;
    cache();
    settingsSaved.value = true;
    notify(t("saved"), "success");
  } catch (e) {
    settingsError.value = String(e);
    if (!String(e).includes(t("adminRequired")))
      notify(settingsError.value, "error");
  } finally {
    settingsBusy.value = false;
  }
}
