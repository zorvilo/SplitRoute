<script setup lang="ts">
/**
 * 内网或外网出口的接口、网关和附加配置卡片。
 *
 * @author lhy
 */

import { t } from "../i18n";
import { Cable, Globe2 } from "lucide-vue-next";
import AdapterSelect from "./AdapterSelect.vue";
import type { Adapter } from "../api";
defineProps<{
  /** 出口角色或通知类型。 */
  kind: "internal" | "external";
  /** 所选网络接口的标识。 */
  selected: string;
  /** 下一跳网关地址。 */
  gateway: string;
  /** 当前可用接口列表。 */
  adapters: Adapter[];
  /** 是否禁止当前控件操作。 */
  disabled: boolean;
  /** 接口不可用时保留的原名称。 */
  offlineName?: string;
}>();
const emit = defineEmits<{
  /** 请求更新所选接口。 */
  select: [value: string];
  /** 下一跳网关地址。 */
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
        :offline-name="offlineName"
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
