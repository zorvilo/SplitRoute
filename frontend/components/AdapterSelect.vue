<script setup lang="ts">
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
  modelValue: string;
  adapters: Adapter[];
  label: string;
  disabled?: boolean;
  offlineName?: string;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
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
