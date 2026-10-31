<script setup lang="ts">
/**
 * 启用分流前的配置摘要与确认弹窗，不直接修改路由。
 *
 * @author lhy
 */

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
  /** 确认弹窗是否打开。 */
  open: boolean;
  /** 当前分流配置。 */
  config: Config;
  /** 当前可用接口列表。 */
  adapters: Adapter[];
  /** 当前是否允许提交启用操作。 */
  canEnable: boolean;
}>();
const emit = defineEmits<{
  /** 请求更新弹窗开关状态。 */
  "update:open": [value: boolean]; /** 用户明确确认后发出的操作事件。 */
  confirm: [];
}>();
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
