<script setup lang="ts">
import { t } from "../i18n";
import { Cable, Globe2 } from "lucide-vue-next";
import AdapterSelect from "./AdapterSelect.vue";
import type { Adapter } from "../api";
defineProps<{
  kind: "internal" | "external";
  selected: string;
  gateway: string;
  adapters: Adapter[];
  disabled: boolean;
}>();
const emit = defineEmits<{
  select: [value: string];
  gateway: [value: string];
}>();
</script>
<template>
  <section
    class="network-card"
    :class="kind"
    :aria-labelledby="`${kind}-title`"
  >
    <div class="network-icon">
      <Cable v-if="kind === 'internal'" :size="23" /><Globe2
        v-else
        :size="23"
      />
    </div>
    <div class="network-description">
      <h2 :id="`${kind}-title`">
        {{ t(kind === "internal" ? "internal" : "external")
        }}<span class="tag">{{
          t(kind === "internal" ? "specified" : "defaultExit")
        }}</span>
      </h2>
      <p>
        {{
          t(kind === "internal" ? "internalDescription" : "externalDescription")
        }}
      </p>
    </div>
    <div class="network-fields">
      <AdapterSelect
        :label="t(kind === 'internal' ? 'internalAdapter' : 'externalAdapter')"
        :model-value="selected"
        :adapters="adapters"
        :disabled="disabled"
        @update:model-value="emit('select', $event)"
      />
      <label class="gateway-field"
        ><span>{{ t("gateway") }}</span
        ><input
          :aria-label="
            t(kind === 'internal' ? 'internalGateway' : 'externalGateway')
          "
          :value="gateway"
          :disabled="disabled"
          :placeholder="t('gatewayExample')"
          spellcheck="false"
          @input="emit('gateway', ($event.target as HTMLInputElement).value)"
      /></label>
      <slot />
    </div>
  </section>
</template>
