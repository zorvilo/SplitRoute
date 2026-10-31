<script setup lang="ts">
import { ref } from "vue";
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
  Download,
  CircleCheck,
  CircleAlert,
} from "lucide-vue-next";
import {
  preferences,
  settingsBusy,
  settingsError,
  settingsSaved,
  updatePreferences,
} from "../composables/usePreferences";
import { api, type UpdateInfo } from "../api";
import { t, type Language } from "../i18n";
const tab = ref<"general" | "about">("general");
const showStar = ref(true);
const checking = ref(false),
  update = ref<UpdateInfo | null>(null),
  updateError = ref("");
const languages: [Language, string][] = [
  ["zh-CN", "简体中文"],
  ["zh-TW", "繁體中文"],
  ["en", "English"],
  ["ja", "日本語"],
];
async function link(
  target: "github" | "website" | "changelog" | "download" | "issues",
) {
  try {
    if (!isTauri()) throw new Error(t("preview"));
    await api.openLink(target);
  } catch (e) {
    updateError.value = String(e);
  }
}
async function check() {
  checking.value = true;
  updateError.value = "";
  update.value = null;
  try {
    if (!isTauri()) throw new Error(t("preview"));
    update.value = await api.checkUpdate();
  } catch (e) {
    updateError.value = String(e);
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
      <div class="setting-row">
        <div class="setting-icon"><PanelBottom :size="19" /></div>
        <div>
          <h3>{{ t("closeToTray") }}</h3>
          <p>{{ t("closeToTrayHelp") }}</p>
        </div>
        <button
          class="toggle"
          role="switch"
          :aria-label="t('closeToTray')"
          :aria-checked="preferences.closeToTray"
          :disabled="settingsBusy || !isTauri()"
          @click="updatePreferences({ closeToTray: !preferences.closeToTray })"
        >
          <span />
        </button>
      </div>
    </div>
    <p v-if="!isTauri()" class="settings-help">{{ t("previewSettings") }}</p>
    <p
      class="settings-result"
      :class="{ error: settingsError }"
      role="status"
      aria-live="polite"
    >
      {{
        settingsError ||
        (settingsBusy ? t("saving") : settingsSaved ? t("saved") : "")
      }}
    </p>
  </section>
  <section v-else class="settings-body">
    <div class="about-card">
      <div class="about-header">
        <div class="about-brand">
          <div class="about-logo"><ArrowRightLeft :size="30" /></div>
          <div>
            <h2>SplitRoute</h2>
            <span class="version-badge">{{ t("version") }} v0.1.0</span>
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
  <Teleport to="body">
    <div
      v-if="update || updateError"
      class="update-toast"
      role="status"
      aria-live="polite"
    >
      <CircleAlert v-if="updateError" :size="16" class="toast-error" />
      <CircleCheck v-else :size="16" class="toast-success" />
      <span>{{
        updateError ||
        (update?.available
          ? t("newVersion", { version: update.latest })
          : t(update?.published ? "upToDate" : "noRelease"))
      }}</span>
      <button
        v-if="update?.available"
        class="button primary"
        @click="link('download')"
      >
        <Download :size="14" />{{ t("download") }}
      </button>
      <button
        class="toast-close"
        :aria-label="t('dismissNotice')"
        @click="
          update = null;
          updateError = '';
        "
      >
        <X :size="12" />
      </button>
    </div>
  </Teleport>
</template>
