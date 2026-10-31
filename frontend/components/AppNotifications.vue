<script setup lang="ts">
/**
 * 全局浮动通知组件，管理自动关闭、悬停暂停和手动关闭。
 *
 * @author lhy
 */

import { onUnmounted, watch } from "vue";
import { CircleCheck, CircleAlert, Info, X } from "lucide-vue-next";
import { notices, dismissNotice } from "../notifications";
import { t } from "../i18n";
const timers = new Map<number, ReturnType<typeof setTimeout>>();
/**
 * 清理指定通知的自动关闭计时器。
 *
 * @param id 接口或通知的标识
 * @author lhy
 */
function pause(id: number) {
  clearTimeout(timers.get(id));
  timers.delete(id);
}
/**
 * 为无操作按钮的非错误通知重新设置三秒关闭计时。
 *
 * @param id 接口或通知的标识
 * @author lhy
 */
function resume(id: number) {
  pause(id);
  const n = notices.value.find((n) => n.id === id);
  if (n && n.kind !== "error" && !n.action)
    timers.set(
      id,
      setTimeout(() => dismissNotice(id), 3000),
    );
}
watch(
  notices,
  (items) => {
    for (const id of timers.keys())
      if (!items.some((n) => n.id === id)) pause(id);
    for (const item of items) if (!timers.has(item.id)) resume(item.id);
  },
  { immediate: true },
);
onUnmounted(() => {
  for (const id of timers.keys()) pause(id);
});
</script>
<template>
  <Teleport to="body"
    ><div class="app-notifications" :aria-label="t('notifications')">
      <div
        v-for="n in notices"
        :key="n.id"
        class="app-notice"
        :class="n.kind"
        :role="n.kind === 'error' ? 'alert' : 'status'"
        @mouseenter="pause(n.id)"
        @mouseleave="resume(n.id)"
        @focusin="pause(n.id)"
        @focusout="resume(n.id)"
      >
        <CircleCheck v-if="n.kind === 'success'" :size="18" /><CircleAlert
          v-else-if="n.kind === 'error'"
          :size="18"
        /><Info v-else :size="18" />
        <span>{{ n.text }}</span>
        <button
          v-if="n.action"
          class="button secondary"
          @click="n.action.run()"
        >
          {{ n.action.label }}
        </button>
        <button
          class="notice-close"
          :aria-label="t('dismissNotice')"
          @click="dismissNotice(n.id)"
        >
          <X :size="15" />
        </button>
      </div></div
  ></Teleport>
</template>
