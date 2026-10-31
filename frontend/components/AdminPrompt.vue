<script setup lang="ts">
/**
 * 管理员授权引导组件，取消授权时保留原窗口。
 *
 * @author lhy
 */

import { notify } from "../notifications";
import { ref, onMounted, onUnmounted } from "vue";
import {
  DialogRoot,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogClose,
} from "reka-ui";
import { api, adminRequired, type Config } from "../api";
import { t } from "../i18n";
const props = defineProps<{
  /** 当前分流配置。 */
  config: Config;
}>();
const open = ref(false),
  pending = ref(false),
  error = ref("");
/**
 * 打开权限引导弹窗并清理上一次授权错误。
 *
 * @author lhy
 */
function show() {
  open.value = true;
  error.value = "";
}
onMounted(() => adminRequired.addEventListener("required", show));
onUnmounted(() => adminRequired.removeEventListener("required", show));
/**
 * 提交完整有效配置并请求管理员重启，取消或失败时展示通知。
 *
 * @author lhy
 */
async function restart() {
  if (pending.value) return;
  pending.value = true;
  error.value = "";
  try {
    await api.restartAsAdmin(
      props.config.internalId &&
        props.config.externalId &&
        props.config.cidr &&
        props.config.internalGateway &&
        props.config.externalGateway
        ? { ...props.config }
        : null,
    );
  } catch (e) {
    error.value =
      String(e) === "ELEVATION_CANCELLED" ? t("adminCancelled") : String(e);
    notify(error.value, "error");
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
    <DialogPortal
      ><DialogOverlay class="summary-overlay" /><DialogContent
        class="enable-summary"
      >
        <DialogTitle class="summary-title">{{
          t("adminRequired")
        }}</DialogTitle>
        <DialogDescription class="summary-description">{{
          t("adminHelp")
        }}</DialogDescription>
        <div class="summary-actions">
          <DialogClose class="button secondary" :disabled="pending">{{
            t("cancelEnable")
          }}</DialogClose
          ><button class="button primary" :disabled="pending" @click="restart">
            {{ t(pending ? "adminStarting" : "adminRestart") }}
          </button>
        </div>
      </DialogContent></DialogPortal
    >
  </DialogRoot>
</template>
