<script setup lang="ts">
/**
 * 内外网延迟曲线与探测状态展示组件。
 *
 * @author lhy
 */

import { computed, ref, watch, onUnmounted } from "vue";
import { Activity, RefreshCw, Cable, Globe2 } from "lucide-vue-next";
import { type Config } from "../api";
import { useConnectivity } from "../composables/useConnectivity";
import {
  latencyPaths,
  latencyCeiling,
  latencyY,
  HISTORY_LENGTH,
} from "../connectivity";
import { t } from "../i18n";
import { preferences } from "../composables/usePreferences";
const props = defineProps<{
  /** 当前分流配置。 */
  config: Config;
}>();
const { report, history, probing, failure, sample } = useConnectivity(() => ({
  internalId: props.config.internalId,
  internalGateway:
    preferences.value.internalProbeTarget || props.config.internalGateway,
  externalTarget: preferences.value.externalProbeTarget,
  externalId: props.config.externalId,
}));
const ceiling = computed(() =>
  latencyCeiling(history.value.flatMap((s) => [s.internal, s.external])),
);
const latest = computed(
  () => history.value.at(-1) ?? { internal: null, external: null },
);
const plot = ref<SVGGElement | null>(null);
let animation: Animation | undefined;
watch(
  history,
  (next, previous) => {
    animation?.cancel();
    if (
      plot.value &&
      next.length &&
      previous.length &&
      !window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      animation = plot.value.animate(
        [
          { transform: `translateX(${156 / (HISTORY_LENGTH - 1)}px)` },
          { transform: "translateX(0px)" },
        ],
        { duration: 1000, easing: "linear" },
      );
    }
  },
  { flush: "post" },
);
onUnmounted(() => animation?.cancel());
const manualRefreshing = ref(false);
/**
 * 执行手动连通检测，并独立控制刷新图标动画。
 *
 * @author lhy
 */
async function refreshManually() {
  manualRefreshing.value = true;
  try {
    await sample();
  } finally {
    manualRefreshing.value = false;
  }
}
const paths = computed(() => ({
  internal: latencyPaths(
    history.value.map((s) => s.internal),
    ceiling.value,
  ),
  external: latencyPaths(
    history.value.map((s) => s.external),
    ceiling.value,
  ),
}));
/**
 * 将探测结果转为界面文本，零毫秒显示为小于一毫秒。
 *
 * @param kind 出口角色或通知类型
 * @returns 延迟或状态文本
 * @author lhy
 */
function label(kind: "internal" | "external") {
  const result = report.value[kind];
  return result.status === "reachable"
    ? result.latencyMs === 0
      ? "<1 ms"
      : `${result.latencyMs} ms`
    : t(result.status);
}
const hasSample = computed(() =>
  history.value.some((s) => s.internal !== null || s.external !== null),
);
</script>
<template>
  <section
    class="connectivity-panel"
    :aria-label="t('connectivity')"
    :title="failure || t('probeDetails')"
  >
    <div class="connectivity-heading">
      <span><Activity :size="14" />{{ t("connectivity") }}</span
      ><button
        class="probe-refresh"
        :aria-label="t('probeNow')"
        :title="t('probeNow')"
        :disabled="probing || (!config.internalId && !config.externalId)"
        @click="refreshManually"
      >
        <RefreshCw :size="13" :class="{ spinning: manualRefreshing }" />
      </button>
    </div>
    <div class="latency-chart" :title="t('latencyHint')">
      <svg viewBox="0 0 156 64" role="img" :aria-label="t('latencyChart')">
        <path d="M0,6 H156 M0,58 H156" class="chart-grid" />
        <g ref="plot">
          <path
            :d="paths.internal.join(' ')"
            class="chart-line internal-line"
          />
          <path
            :d="paths.external.join(' ')"
            class="chart-line external-line"
          />
          <circle
            v-if="latest.internal !== null"
            cx="156"
            :cy="latencyY(latest.internal, ceiling)"
            r="1.8"
            class="chart-point internal-point"
          />
          <circle
            v-if="latest.external !== null"
            cx="156"
            :cy="latencyY(latest.external, ceiling)"
            r="1.8"
            class="chart-point external-point"
          />
        </g>
      </svg>
      <span v-if="!hasSample" class="chart-empty">{{
        t(history.length ? "noReply" : "waitingProbe")
      }}</span>
      <span class="chart-scale">{{ ceiling }} ms</span>
    </div>
    <div class="connectivity-rows">
      <div
        v-for="kind in ['internal', 'external'] as const"
        :key="kind"
        class="connectivity-row"
        :class="kind"
        :title="`${report[kind].source || '—'} → ${report[kind].target || '—'}`"
      >
        <Cable v-if="kind === 'internal'" :size="15" /><Globe2
          v-else
          :size="15"
        /><span>{{ t(kind === "internal" ? "lanShort" : "wanShort") }}</span
        ><strong :class="{ unhealthy: report[kind].status !== 'reachable' }">{{
          label(kind)
        }}</strong>
      </div>
    </div>
  </section>
</template>
