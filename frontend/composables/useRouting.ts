import { t, type MessageKey } from "../i18n";
import { computed, reactive, ref } from "vue";
import { api, type Snapshot, type Config } from "../api";
import { privateNetworkFromAddress } from "../subnet";

export function useRouting() {
  const data = ref<Snapshot>({
    adapters: [],
    routes: [],
    pending: false,
    routingStatus: "unknown",
    config: null,
  });
  const config = reactive<Config>({
    internalId: "",
    externalId: "",
    internalGateway: "",
    externalGateway: "",
    cidr: "",
  });
  const busy = ref(false),
    error = ref(""),
    allRoutes = ref(false),
    updatedAt = ref<Date | null>(null),
    initialized = ref(false);
  const messageKey = ref<MessageKey>("initialMessage");
  const messageValues = ref<Record<string, string | number>>({});
  const warningMessage = ref("");
  const message = computed(
    () => warningMessage.value || t(messageKey.value, messageValues.value),
  );
  const subnetMode = ref<"auto" | "manual">("auto");
  const suggestedSubnet = computed(() =>
    privateNetworkFromAddress(
      data.value.adapters.find((a) => a.id === config.internalId)
        ?.addresses[0] ?? "",
    ),
  );
  function setSubnetMode(mode: "auto" | "manual") {
    subnetMode.value = mode;
    if (mode === "auto") config.cidr = suggestedSubnet.value;
  }
  const status = computed(() =>
    t(initialized.value ? data.value.routingStatus : "unknown"),
  );
  const canEnable = computed(
    () =>
      initialized.value &&
      !busy.value &&
      !data.value.pending &&
      data.value.routingStatus !== "active" &&
      data.value.routingStatus !== "unknown" &&
      !!config.internalId &&
      !!config.externalId &&
      !!config.cidr.trim() &&
      config.internalId !== config.externalId,
  );
  const routes = computed(() =>
    allRoutes.value
      ? data.value.routes
      : data.value.routes.filter(
          (r) => r.prefix === 0 || `${r.network}/${r.prefix}` === config.cidr,
        ),
  );
  function select(kind: "internal" | "external", id: string) {
    const a = data.value.adapters.find((a) => a.id === id);
    if (kind === "internal") {
      config.internalId = id;
      config.internalGateway = a?.gateway ?? "";
      if (subnetMode.value === "auto") config.cidr = suggestedSubnet.value;
    } else {
      config.externalId = id;
      config.externalGateway = a?.gateway ?? "";
    }
  }
  async function refresh(initial = false) {
    const next = await api.snapshot();
    data.value = next;
    updatedAt.value = new Date();
    if (initial && next.config) {
      Object.assign(config, next.config);
      subnetMode.value =
        config.cidr === suggestedSubnet.value ? "auto" : "manual";
    }
    if (next.warning) warningMessage.value = next.warning;
    for (const kind of ["internal", "external"] as const) {
      const id = kind === "internal" ? config.internalId : config.externalId;
      if (id && !next.adapters.some((a) => a.id === id)) select(kind, "");
    }
    initialized.value = true;
    if (subnetMode.value === "auto" && !next.pending)
      config.cidr = suggestedSubnet.value;
  }
  async function act(action: "refresh" | "enable" | "restore" | "inspect") {
    if (busy.value) return;
    busy.value = true;
    error.value = "";
    warningMessage.value = "";
    try {
      if (action === "enable") {
        await api.enable({ ...config });
        messageKey.value = "enabledMessage";
      }
      if (action === "restore") {
        await api.restore();
        messageKey.value = "undoneMessage";
      }
      await refresh();
      if (action === "refresh") {
        messageKey.value = "refreshed";
        messageValues.value = { count: data.value.adapters.length };
      }
      if (action === "inspect") {
        messageKey.value = "inspected";
      }
    } catch (e) {
      error.value = String(e);
      try {
        await refresh();
      } catch {
        data.value.routingStatus = "unknown";
        /* Keep the original actionable error visible. */
      }
    } finally {
      busy.value = false;
    }
  }
  async function initialize() {
    busy.value = true;
    try {
      await refresh(true);
    } catch (e) {
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  return {
    data,
    config,
    busy,
    error,
    message,
    allRoutes,
    updatedAt,
    initialized,
    status,
    canEnable,
    routes,
    select,
    subnetMode,
    setSubnetMode,
    act,
    initialize,
  };
}
