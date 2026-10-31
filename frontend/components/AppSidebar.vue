<script setup lang="ts">
/**
 * 侧栏导航与连通检测面板容器。
 *
 * @author lhy
 */

import { ArrowRightLeft, Route, Settings } from "lucide-vue-next";
import type { Config } from "../api";
import { t } from "../i18n";
import ConnectivityPanel from "./ConnectivityPanel.vue";
defineProps<{
  /** 当前选中的页面。 */
  page: "config" | "routes" | "settings";
  /** 当前分流配置。 */
  config: Config;
  /** 是否正在执行互斥操作。 */
  busy: boolean;
}>();
const emit = defineEmits<{
  /** 请求切换侧栏页面。 */
  navigate: [page: "config" | "routes" | "settings"];
}>();
</script>
<template>
  <aside class="app-sidebar">
    <div class="sidebar-brand">
      <ArrowRightLeft :size="23" :stroke-width="2.4" /><span>SplitRoute</span>
    </div>
    <nav class="sidebar-nav" :aria-label="t('navigation')">
      <button
        v-for="item in ['config', 'routes', 'settings'] as const"
        :key="item"
        :class="{ active: page === item }"
        :aria-current="page === item ? 'page' : undefined"
        :disabled="busy && item === 'routes'"
        @click="emit('navigate', item)"
      >
        <ArrowRightLeft v-if="item === 'config'" :size="20" /><Route
          v-else-if="item === 'routes'"
          :size="20"
        /><Settings v-else :size="20" />
        <span>{{
          t(
            item === "config"
              ? "routingNav"
              : item === "routes"
                ? "routesNav"
                : "settings",
          )
        }}</span>
      </button>
    </nav>
    <ConnectivityPanel :config="config" />
  </aside>
</template>
