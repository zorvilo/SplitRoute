<script setup lang="ts">
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
const props = defineProps<{ config: Config }>();
const open = ref(false),
  pending = ref(false),
  error = ref("");
function show() {
  open.value = true;
  error.value = "";
}
onMounted(() => adminRequired.addEventListener("required", show));
onUnmounted(() => adminRequired.removeEventListener("required", show));
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
