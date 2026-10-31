<script setup lang="ts">
import {
  DialogRoot,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogClose,
} from "reka-ui";
import type { Adapter, Config } from "../api";
import { t } from "../i18n";
defineProps<{
  open: boolean;
  config: Config;
  adapters: Adapter[];
  canEnable: boolean;
}>();
const emit = defineEmits<{ "update:open": [value: boolean]; confirm: [] }>();
</script>
<template>
  <DialogRoot :open="open" @update:open="emit('update:open', $event)">
    <DialogPortal>
      <DialogOverlay class="summary-overlay" />
      <DialogContent class="enable-summary">
        <DialogTitle class="summary-title">{{
          t("enableSummary")
        }}</DialogTitle>
        <DialogDescription class="summary-description">{{
          t("summaryHelp")
        }}</DialogDescription>
        <dl class="summary-routes">
          <div>
            <dt>{{ config.cidr }}</dt>
            <dd>
              {{
                adapters.find((a) => a.id === config.internalId)?.name ||
                config.internalId
              }}
            </dd>
            <dd>{{ t("gateway") }} {{ config.internalGateway }}</dd>
          </div>
          <div>
            <dt>{{ t("otherIpv4") }}</dt>
            <dd>
              {{
                adapters.find((a) => a.id === config.externalId)?.name ||
                config.externalId
              }}
            </dd>
            <dd>{{ t("gateway") }} {{ config.externalGateway }}</dd>
          </div>
        </dl>
        <p class="summary-description">{{ t("summaryLifetime") }}</p>
        <div class="summary-actions">
          <DialogClose class="button secondary">{{
            t("cancelEnable")
          }}</DialogClose>
          <button
            class="button primary"
            :disabled="!canEnable"
            @click="emit('confirm')"
          >
            {{ t("confirmEnable") }}
          </button>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
