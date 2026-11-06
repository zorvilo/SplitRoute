<script setup lang="ts">
/**
 * 通用偏好、检测目标及关于信息页面。
 *
 * @author lhy
 */

import { notify } from "../notifications";
import { ref, watch } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import {
  Sun,
  Moon,
  Monitor,
  Power,
  PanelBottom,
  Github,
  Globe,
  MessageSquare,
  Star,
  X,
  ArrowRightLeft,
  ExternalLink,
  RefreshCw,
} from "lucide-vue-next";
import {
  preferences,
  settingsBusy,
  settingsError,
  updatePreferences,
} from "../composables/usePreferences";
import { api, type UpdateInfo } from "../api";
import { t, type Language } from "../i18n";
const tab = ref<"general" | "about">("general");
const showStar = ref(true);
const checking = ref(false),
  update = ref<UpdateInfo | null>(null),
  updateError = ref("");
const customInternal = ref(!!preferences.value.internalProbeTarget);
const internalTarget = ref(preferences.value.internalProbeTarget);
const externalTarget = ref(preferences.value.externalProbeTarget);
watch(preferences, (p) => {
  customInternal.value = !!p.internalProbeTarget;
  internalTarget.value = p.internalProbeTarget;
  externalTarget.value = p.externalProbeTarget;
});
/**
 * 校验编辑后的 IPv4 目标并提交保存，内网空值表示使用网关。
 *
 * @param field 需要保存的检测目标字段
 * @param text 待校验地址或通知内容
 * @author lhy
 */
function saveTarget(
  field: "internalProbeTarget" | "externalProbeTarget",
  text: string,
) {
  const value = text.trim();
  const parts = value.split(".");
  const valid =
    parts.length === 4 &&
    parts.every((p) => /^(0|[1-9]\d{0,2})$/.test(p) && Number(p) <= 255) &&
    Number(parts[0]) > 0 &&
    Number(parts[0]) < 224 &&
    Number(parts[0]) !== 127;
  if (!(field === "internalProbeTarget" && !value) && !valid) {
    settingsError.value = t("invalidProbeIp");
    notify(settingsError.value, "error");
    return;
  }
  void updatePreferences({ [field]: value });
}
const languages: [Language, string][] = [
  ["zh-CN", "简体中文"],
  ["zh-TW", "繁體中文"],
  ["en", "English"],
  ["ja", "日本語"],
];
/**
 * 打开固定项目链接，浏览器预览或打开失败时显示错误通知。
 *
 * @param target 固定链接标识或检测目标地址
 * @author lhy
 */
async function link(
  target: "github" | "website" | "changelog" | "download" | "issues",
) {
  try {
    if (!isTauri()) throw new Error(t("preview"));
    await api.openLink(target);
  } catch (e) {
    updateError.value = String(e);
    notify(updateError.value, "error");
  }
}
/**
 * 执行手动更新检查并提供必要的下载入口。
 *
 * @author lhy
 */
async function check() {
  checking.value = true;
  updateError.value = "";
  update.value = null;
  try {
    if (!isTauri()) throw new Error(t("preview"));
    update.value = await api.checkUpdate();
    notify(
      update.value.available
        ? t("newVersion", { version: update.value.latest })
        : t(update.value.published ? "upToDate" : "noRelease"),
      update.value.available ? "info" : "success",
      update.value.available
        ? {
            label: t("download"),
            /**
             * 执行用户点击下载通知后打开发布页的操作。
             *
             * @author lhy
             */
            run: () => {
              void link("download");
            },
          }
        : undefined,
    );
  } catch (e) {
    updateError.value = String(e);
    notify(updateError.value, "error");
  } finally {
    checking.value = false;
  }
}
</script>
<template>
  <div class="settings-tabs" role="group" :aria-label="t('settings')">
    <button
      v-for="item in ['general', 'about'] as const"
      :key="item"
      :aria-pressed="tab === item"
      @click="tab = item"
    >
      {{ t(item) }}
    </button>
  </div>
  <section v-if="tab === 'general'" class="settings-body general-settings">
    <h2 class="settings-section-title">
      <Monitor :size="16" />{{ t("appearanceGroup") }}
    </h2>
    <div class="settings-group">
      <div class="setting-section">
        <div class="setting-copy">
          <h3>{{ t("language") }}</h3>
          <p>{{ t("languageHelp") }}</p>
        </div>
        <div class="segmented" role="group" :aria-label="t('language')">
          <button
            v-for="[value, label] in languages"
            :key="value"
            :aria-pressed="preferences.language === value"
            :disabled="settingsBusy"
            @click="updatePreferences({ language: value })"
          >
            {{ label }}
          </button>
        </div>
      </div>
      <div class="setting-section">
        <div class="setting-copy">
          <h3>{{ t("theme") }}</h3>
          <p>{{ t("themeHelp") }}</p>
        </div>
        <div class="segmented" role="group" :aria-label="t('theme')">
          <button
            v-for="value in ['light', 'dark', 'system'] as const"
            :key="value"
            :aria-pressed="preferences.theme === value"
            :disabled="settingsBusy"
            @click="updatePreferences({ theme: value })"
          >
            <Sun v-if="value === 'light'" :size="15" /><Moon
              v-else-if="value === 'dark'"
              :size="15"
            /><Monitor v-else :size="15" />{{ t(value) }}
          </button>
        </div>
      </div>
    </div>
    <h2 class="settings-section-title">
      <PanelBottom :size="16" />{{ t("startupWindowGroup") }}
    </h2>
    <div class="settings-group">
      <div class="setting-row">
        <div class="setting-icon"><Power :size="19" /></div>
        <div>
          <h3>{{ t("autoStart") }}</h3>
          <p>{{ t("autoStartHelp") }}</p>
        </div>
        <button
          class="toggle"
          role="switch"
          :aria-label="t('autoStart')"
          :aria-checked="preferences.autoStart"
          :disabled="settingsBusy || !isTauri()"
          @click="updatePreferences({ autoStart: !preferences.autoStart })"
        >
          <span />
        </button>
      </div>
      <div class="setting-section">
        <div class="setting-copy">
          <h3>{{ t("closeBehavior") }}</h3>
          <p>{{ t("trayHint") }}</p>
        </div>
        <div class="segmented" role="group" :aria-label="t('closeBehavior')">
          <button
            v-for="[value, label] in [
              [null, 'askOnClose'],
              [true, 'hideToTray'],
              [false, 'exitProgram'],
            ] as const"
            :key="label"
            :aria-pressed="preferences.closeToTray === value"
            :disabled="settingsBusy || !isTauri()"
            @click="updatePreferences({ closeToTray: value })"
          >
            {{ t(label) }}
          </button>
        </div>
      </div>
    </div>
    <h2 class="settings-section-title">
      <Globe :size="16" />{{ t("probeSettings") }}
    </h2>
    <div class="settings-group probe-settings">
      <div class="setting-section">
        <div class="setting-copy">
          <h3>{{ t("internalProbe") }}</h3>
          <p>{{ t("probeHelp") }}</p>
        </div>
        <div class="probe-fields">
          <select
            :aria-label="t('internalProbeMode')"
            :disabled="settingsBusy"
            :value="customInternal ? 'custom' : 'auto'"
            @change="
              ($event.target as HTMLSelectElement).value === 'auto'
                ? ((customInternal = false),
                  updatePreferences({ internalProbeTarget: '' }))
                : (customInternal = true)
            "
          >
            <option value="auto">{{ t("gatewayProbe") }}</option>
            <option value="custom">{{ t("customProbe") }}</option>
          </select>
          <input
            v-if="customInternal"
            v-model="internalTarget"
            :aria-label="t('internalProbe')"
            :disabled="settingsBusy"
            placeholder="10.0.0.1"
            @change="saveTarget('internalProbeTarget', internalTarget)"
          />
        </div>
      </div>
      <div class="setting-section">
        <div class="setting-copy">
          <h3>{{ t("externalProbe") }}</h3>
          <p>{{ t("probeHelp") }}</p>
        </div>
        <div class="probe-fields">
          <input
            v-model="externalTarget"
            :aria-label="t('externalProbe')"
            :disabled="settingsBusy"
            @change="saveTarget('externalProbeTarget', externalTarget)"
          /><button
            class="button secondary"
            :disabled="settingsBusy"
            @click="updatePreferences({ externalProbeTarget: '223.5.5.5' })"
          >
            {{ t("resetProbe") }}
          </button>
        </div>
      </div>
    </div>
    <p v-if="!isTauri()" class="settings-help">{{ t("previewSettings") }}</p>
  </section>
  <section v-else class="settings-body">
    <div class="about-card">
      <div class="about-header">
        <div class="about-brand">
          <div class="about-logo"><ArrowRightLeft :size="30" /></div>
          <div>
            <h2>SplitRoute</h2>
            <span class="version-badge">{{ t("version") }} v1.0.1</span>
          </div>
        </div>
        <button class="button secondary" :disabled="checking" @click="check">
          <RefreshCw :size="14" :class="{ spinning: checking }" />{{
            t(checking ? "checking" : "checkUpdate")
          }}
        </button>
      </div>
      <p class="about-description">{{ t("productDescription") }}</p>
      <div class="about-actions">
        <button class="button secondary" @click="link('website')">
          <Globe :size="14" />{{ t("homepage") }}
        </button>
        <button class="button secondary" @click="link('changelog')">
          <ExternalLink :size="14" />{{ t("changelog") }}
        </button>
        <button class="button secondary" @click="link('issues')">
          <MessageSquare :size="14" />{{ t("feedback") }}
        </button>
      </div>
      <div v-if="showStar" class="about-support">
        <Star :size="20" class="support-star" />
        <p>{{ t("star") }}</p>
        <button class="button secondary" @click="link('github')">
          <Github :size="14" />{{ t("giveStar") }}
        </button>
        <button
          class="about-dismiss"
          :aria-label="t('dismissStar')"
          @click="showStar = false"
        >
          <X :size="14" />
        </button>
      </div>
    </div>
    <dl class="about-project-info">
      <div>
        <dt>{{ t("author") }}</dt>
        <dd>zorvilo</dd>
      </div>
      <div>
        <dt>{{ t("sourceRepository") }}</dt>
        <dd>
          <button @click="link('github')">
            zorvilo/SplitRoute <ExternalLink :size="12" />
          </button>
        </dd>
      </div>
    </dl>
  </section>
</template>
