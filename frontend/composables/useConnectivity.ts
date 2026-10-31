/**
 * 连通检测状态：管理采样、切换目标、异步过期结果和资源清理。
 *
 * @author lhy
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  type Connectivity,
  type ProbeRequest,
  type ProbeResult,
} from "../api";
import { HISTORY_LENGTH } from "../connectivity";
/**
 * 创建尚未选择接口的探测结果，保留待检测目标。
 *
 * @param target 固定链接标识或检测目标地址
 * @returns 未配置状态的探测结果
 * @author lhy
 */
const empty = (target: string): ProbeResult => ({
  status: "unconfigured",
  latencyMs: null,
  target,
  source: "",
});
/**
 * 管理内外网采样状态、历史数据及组件生命周期。
 *
 * @param request 读取当前探测请求的函数或请求数据
 * @returns 探测状态、历史曲线与手动采样入口
 * @author lhy
 */
export function useConnectivity(request: () => ProbeRequest) {
  const report = ref<Connectivity>({
    internal: empty(""),
    external: empty(request().externalTarget || "223.5.5.5"),
  });
  const history = ref<
    {
      /** 内网样本或探测结果。 */
      internal: number | null; /** 外网样本或探测结果。 */
      external: number | null;
    }[]
  >([]);
  const probing = ref(false),
    lastRead = ref<Date | null>(null),
    failure = ref("");
  const key = computed(() => JSON.stringify(request()));
  let alive = true;
  let interval: ReturnType<typeof setInterval> | undefined;
  let debounce: ReturnType<typeof setTimeout> | undefined;
  /**
   * 读取当前目标并执行一次检测，跳过隐藏窗口及重叠请求，丢弃过期响应。
   *
   * @author lhy
   */
  async function sample() {
    if (
      !alive ||
      probing.value ||
      (typeof document !== "undefined" && document.hidden)
    )
      return;
    const input = { externalTarget: "223.5.5.5", ...request() },
      started = key.value;
    if (!input.internalId && !input.externalId) return;
    probing.value = true;
    try {
      const result = await api.probeConnectivity(input);
      // 接口或目标切换后，旧请求即使完成也不能覆盖新状态。
      if (!alive || started !== key.value) return;
      report.value = result;
      failure.value = "";
      lastRead.value = new Date();
      history.value = [
        ...history.value,
        {
          internal:
            result.internal.status === "reachable"
              ? result.internal.latencyMs
              : null,
          external:
            result.external.status === "reachable"
              ? result.external.latencyMs
              : null,
        },
      ].slice(-HISTORY_LENGTH);
    } catch (error) {
      // 接口或目标切换后，旧请求即使完成也不能覆盖新状态。
      if (!alive || started !== key.value) return;
      failure.value = String(error);
      report.value = {
        internal: {
          ...empty(input.internalGateway),
          status: input.internalId ? "error" : "unconfigured",
        },
        external: {
          ...empty(request().externalTarget || "223.5.5.5"),
          status: input.externalId ? "error" : "unconfigured",
        },
      };
      history.value = [
        ...history.value,
        { internal: null, external: null },
      ].slice(-HISTORY_LENGTH);
    } finally {
      probing.value = false;
    }
  }
  watch(key, () => {
    history.value = [];
    lastRead.value = null;
    failure.value = "";
    report.value = {
      internal: empty(request().internalGateway),
      external: empty(request().externalTarget || "223.5.5.5"),
    };
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      void sample();
    }, 400);
  });
  /**
   * 窗口恢复可见时立即尝试采样。
   *
   * @author lhy
   */
  function visible() {
    if (!document.hidden) void sample();
  }
  onMounted(() => {
    void sample();
    interval = setInterval(() => {
      void sample();
    }, 1000);
    document.addEventListener("visibilitychange", visible);
  });
  onUnmounted(() => {
    alive = false;
    if (interval) clearInterval(interval);
    if (debounce) clearTimeout(debounce);
    document.removeEventListener("visibilitychange", visible);
  });
  return { report, history, probing, lastRead, failure, sample };
}
