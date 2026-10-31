/**
 * 分流页面状态与操作流程，读取实际网络并保留网络变化提醒。
 *
 * @author lhy
 */
import { notify } from "../notifications";
import { t, type MessageKey } from "../i18n";
import { computed, reactive, ref } from "vue";
import { api, type Snapshot, type Config } from "../api";
import { privateNetworkFromAddress } from "../subnet";

/**
 * 管理分流配置、实际状态、网络提醒及受控操作。
 *
 * @returns 分流页面共享状态及操作入口
 * @author lhy
 */
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
  const adapterNames = reactive<Record<string, string>>({});
  const networkHeading = computed(() => {
    if (networkNotice.value === "networkCheckFailed")
      return t("networkReadUnavailable");
    const internalMissing = !data.value.adapters.some(
      (a) => a.id === config.internalId,
    );
    const externalMissing = !data.value.adapters.some(
      (a) => a.id === config.externalId,
    );
    return t(
      internalMissing && externalMissing
        ? "bothAdaptersUnavailable"
        : internalMissing
          ? "internalUnavailable"
          : externalMissing
            ? "externalUnavailable"
            : "networkChangedTitle",
    );
  });
  const networkDetail = computed(() => {
    if (networkNotice.value === "networkCheckFailed")
      return t("networkCheckFailed");
    const missing = (["internal", "external"] as const).filter((kind) => {
      const id = kind === "internal" ? config.internalId : config.externalId;
      return id && !data.value.adapters.some((a) => a.id === id);
    });
    return missing.length
      ? t("selectedAdapterUnavailable", {
          names: missing
            .map((kind) => {
              const id =
                kind === "internal" ? config.internalId : config.externalId;
              return `${t(kind)} ${adapterNames[id] || id}`;
            })
            .join("、"),
        })
      : t("networkChanged");
  });
  const networkNotice = ref<"networkChanged" | "networkCheckFailed" | "">("");
  let changedSince: number | null = null;
  /**
   * 持续异常达到确认时间后保留提醒，实际恢复匹配时清除提醒。
   *
   * @author lhy
   */
  function updateNotice() {
    if (!data.value.pending || data.value.routingStatus === "active") {
      networkNotice.value = "";
      changedSince = null;
    } else {
      changedSince ??= Date.now();
      if (Date.now() - changedSince >= 2000)
        networkNotice.value = "networkChanged";
    }
  }
  /**
   * 存在本次记录时只读检查网络，避免与其他路由操作重叠。
   *
   * @author lhy
   */
  async function checkNetwork() {
    if (
      !initialized.value ||
      busy.value ||
      (!data.value.pending && !networkNotice.value)
    )
      return;
    busy.value = true;
    try {
      await refresh();
    } catch {
      data.value.routingStatus = "unknown";
      networkNotice.value = "networkCheckFailed";
    } finally {
      busy.value = false;
    }
  }
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
  /**
   * 切换网段填写方式，自动模式使用当前接口推导的私有范围。
   *
   * @param mode 自动或手动填写方式
   * @author lhy
   */
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
  /**
   * 更新指定出口的接口与网关，内网自动模式同时更新目标网段。
   *
   * @param kind 出口角色或通知类型
   * @param id 接口或通知的标识
   * @author lhy
   */
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
  /**
   * 读取实际网络并更新成功时间，首次读取恢复保存配置。
   *
   * @param initial 是否在首次读取时恢复保存配置
   * @author lhy
   */
  async function refresh(initial = false) {
    const next = await api.snapshot();
    for (const a of [...(next.savedAdapters || []), ...next.adapters])
      adapterNames[a.id] = a.name;
    data.value = next;
    updatedAt.value = new Date();
    if (initial && next.config) {
      Object.assign(config, next.config);
      subnetMode.value =
        config.cidr === suggestedSubnet.value ? "auto" : "manual";
    }
    if (next.warning && next.warning !== warningMessage.value)
      notify(next.warning, "error");
    warningMessage.value = next.warning || "";
    for (const kind of ["internal", "external"] as const) {
      const id = kind === "internal" ? config.internalId : config.externalId;
      // 有待撤销记录时保留接口标识，断线后才能继续显示原配置并核对恢复。
      if (!next.pending && id && !next.adapters.some((a) => a.id === id))
        select(kind, "");
    }
    initialized.value = true;
    updateNotice();
    if (subnetMode.value === "auto" && !next.pending)
      config.cidr = suggestedSubnet.value;
  }
  /**
   * 互斥执行启用、撤销或读取流程，失败后尝试重新核对状态。
   *
   * @param action 操作名称或可选通知按钮
   * @param quiet 是否抑制操作结果通知
   * @author lhy
   */
  async function act(
    action: "refresh" | "enable" | "restore" | "inspect",
    quiet = false,
  ) {
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
      if (!quiet && action !== "inspect") notify(message.value, "success");
    } catch (e) {
      error.value = String(e);
      if (!quiet && !String(e).includes(t("adminRequired")))
        notify(error.value, "error");
      try {
        await refresh();
      } catch {
        data.value.routingStatus = "unknown";
        // 保留原操作错误，避免后续刷新失败掩盖真正原因。
      }
    } finally {
      busy.value = false;
    }
  }
  /**
   * 首次读取网络并恢复有效配置，失败时保留未知状态并提示原因。
   *
   * @author lhy
   */
  async function initialize() {
    busy.value = true;
    try {
      await refresh(true);
    } catch (e) {
      error.value = String(e);
      if (!String(e).includes(t("adminRequired"))) notify(error.value, "error");
    } finally {
      busy.value = false;
    }
  }

  return {
    data,
    networkNotice,
    networkDetail,
    networkHeading,
    adapterNames,
    checkNetwork,
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
