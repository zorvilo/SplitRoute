<script setup lang="ts">
/**
 * 选择隐藏到托盘或退出，可保存为默认关闭行为。
 *
 * @author lhy
 */
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Check, X } from "lucide-vue-next";
import {
  CheckboxIndicator,
  CheckboxRoot,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { onMounted, onUnmounted, ref } from "vue";
import { api } from "../api";
import { preferences, settingsBusy } from "../composables/usePreferences";
import { t } from "../i18n";
import { notify } from "../notifications";

const open = ref(false);
const remember = ref(false);
const pending = ref(false);
let subscription: Promise<UnlistenFn> | undefined;
onMounted(() => {
  if (!isTauri()) return;
  subscription = listen("window-close-choice", () => {
    if (open.value) return;
    remember.value = false;
    open.value = true;
  });
  void subscription.catch((error) => notify(String(error), "error"));
});
onUnmounted(() => {
  void subscription?.then(
    (unlisten) => unlisten(),
    () => {},
  );
});

/**
 * 弹出时聚焦弹窗本身，按 Tab 后再显示控件的焦点提示。
 *
 * @author lhy
 */
function focusDialog(event: Event) {
  event.preventDefault();
  if (event.target instanceof HTMLElement) event.target.focus();
}

/**
 * 执行本次关闭选择，成功后同步已保存的设置。
 *
 * @author lhy
 */
async function choose(toTray: boolean) {
  if (pending.value || settingsBusy.value) return;
  pending.value = true;
  try {
    await api.finishWindowClose(toTray, remember.value);
    if (remember.value) preferences.value.closeToTray = toTray;
    open.value = false;
  } catch (error) {
    notify(String(error), "error");
  } finally {
    pending.value = false;
  }
}
</script>

<template>
  <DialogRoot
    :open="open"
    @update:open="
      (value) => {
        if (!pending) open = value;
      }
    "
  >
    <DialogPortal>
      <DialogOverlay class="summary-overlay" />
      <DialogContent
        class="enable-summary close-prompt"
        tabindex="-1"
        @open-auto-focus="focusDialog"
      >
        <div class="close-prompt-header">
          <div>
            <DialogTitle class="summary-title">{{
              t("closeBehavior")
            }}</DialogTitle>
            <DialogDescription class="summary-description">{{
              t("trayHint")
            }}</DialogDescription>
          </div>
          <DialogClose
            class="about-dismiss"
            :aria-label="t('cancelEnable')"
            :disabled="pending"
          >
            <X :size="14" />
          </DialogClose>
        </div>
        <div class="close-prompt-footer">
          <div class="close-prompt-remember">
            <CheckboxRoot
              id="remember-close-choice"
              v-model="remember"
              aria-labelledby="remember-close-label"
              :disabled="pending"
            >
              <CheckboxIndicator><Check :size="12" /></CheckboxIndicator>
            </CheckboxRoot>
            <label id="remember-close-label" for="remember-close-choice">
              {{ t("rememberCloseChoice") }}
            </label>
          </div>
          <div class="summary-actions">
            <button
              class="button secondary"
              :disabled="pending || settingsBusy"
              @click="choose(false)"
            >
              {{ t("exitProgram") }}
            </button>
            <button
              class="button primary"
              :disabled="pending || settingsBusy"
              @click="choose(true)"
            >
              {{ t("hideToTray") }}
            </button>
          </div>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
