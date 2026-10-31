import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  type Connectivity,
  type ProbeRequest,
  type ProbeResult,
} from "../api";
import { HISTORY_LENGTH } from "../connectivity";
const empty = (target: string): ProbeResult => ({
  status: "unconfigured",
  latencyMs: null,
  target,
  source: "",
});
export function useConnectivity(request: () => ProbeRequest) {
  const report = ref<Connectivity>({
    internal: empty(""),
    external: empty("223.5.5.5"),
  });
  const history = ref<{ internal: number | null; external: number | null }[]>(
    [],
  );
  const probing = ref(false),
    lastRead = ref<Date | null>(null),
    failure = ref("");
  const key = computed(() => JSON.stringify(request()));
  let alive = true;
  let interval: ReturnType<typeof setInterval> | undefined;
  let debounce: ReturnType<typeof setTimeout> | undefined;
  async function sample() {
    if (
      !alive ||
      probing.value ||
      (typeof document !== "undefined" && document.hidden)
    )
      return;
    const input = { ...request() },
      started = key.value;
    if (!input.internalId && !input.externalId) return;
    probing.value = true;
    try {
      const result = await api.probeConnectivity(input);
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
      if (!alive || started !== key.value) return;
      failure.value = String(error);
      report.value = {
        internal: {
          ...empty(input.internalGateway),
          status: input.internalId ? "error" : "unconfigured",
        },
        external: {
          ...empty("223.5.5.5"),
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
      external: empty("223.5.5.5"),
    };
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      void sample();
    }, 400);
  });
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
