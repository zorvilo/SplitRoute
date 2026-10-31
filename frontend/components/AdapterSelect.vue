<script setup lang="ts">
/**
 * 网络接口选择组件，保留不可用接口的原名称与状态。
 *
 * @author lhy
 */

import {
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectIcon,
  SelectPortal,
  SelectContent,
  SelectViewport,
  SelectItem,
  SelectItemText,
  SelectItemIndicator,
} from "reka-ui";
import { t } from "../i18n";
import { Check, ChevronDown } from "lucide-vue-next";
import { computed } from "vue";
import type { Adapter } from "../api";
const props = defineProps<{
  /** 当前选中的接口标识。 */
  modelValue: string;
  /** 当前可用接口列表。 */
  adapters: Adapter[];
  /** 控件或操作按钮的显示文本。 */
  label: string;
  /** 是否禁止当前控件操作。 */
  disabled?: boolean;
  /** 接口不可用时保留的原名称。 */
  offlineName?: string;
}>();
const emit = defineEmits<{
  /** 请求更新当前选中的接口。 */
  "update:modelValue": [value: string];
}>();
const selected = computed(() =>
  props.adapters.find((adapter) => adapter.id === props.modelValue),
);
</script>
<template>
  <SelectRoot
    :model-value="modelValue"
    :disabled="disabled"
    @update:model-value="(value) => emit('update:modelValue', String(value))"
  >
    <SelectTrigger class="select-trigger" :aria-label="label">
      <SelectValue
        :placeholder="
          modelValue
            ? `${offlineName || modelValue} · ${t('adapterOffline')}`
            : t('chooseAdapter')
        "
        class="selected-value"
      >
        <span v-if="selected" class="adapter-label"
          ><span class="adapter-name">{{ selected.name }}</span
          ><span class="option-address">{{
            selected.addresses[0]?.split("/")[0]
          }}</span></span
        ><span v-else-if="modelValue" class="adapter-offline"
          >{{ offlineName || modelValue }} · {{ t("adapterOffline") }}</span
        > </SelectValue
      ><SelectIcon><ChevronDown :size="16" /></SelectIcon>
    </SelectTrigger>
    <SelectPortal
      ><SelectContent class="select-content" position="popper" :side-offset="6"
        ><SelectViewport>
          <SelectItem
            v-for="adapter in adapters"
            :key="adapter.id"
            :value="adapter.id"
            class="select-item"
          >
            <SelectItemText class="adapter-label"
              ><span class="adapter-name">{{ adapter.name }}</span
              ><span class="option-address">{{
                adapter.addresses[0]?.split("/")[0]
              }}</span></SelectItemText
            >
            <SelectItemIndicator class="select-check"
              ><Check :size="16"
            /></SelectItemIndicator>
          </SelectItem> </SelectViewport></SelectContent
    ></SelectPortal>
  </SelectRoot>
</template>
