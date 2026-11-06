/**
 * 前后端命令类型与调用边界，集中处理管理员权限提示。
 *
 * @author lhy
 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import { t, type Language } from "./i18n";
/**
 * 网络接口标识、名称、IPv4 地址与网关。
 *
 * @author lhy
 */
export interface Adapter {
  /** Windows 网络接口的 LUID 字符串。 */
  id: string;
  /** 接口显示名称。 */
  name: string;
  /** 带前缀长度的 IPv4 地址列表。 */
  addresses: string[];
  /** 下一跳网关地址。 */
  gateway: string;
}
/**
 * 从系统读取的单条 IPv4 路由。
 *
 * @author lhy
 */
export interface Route {
  /** 路由所属接口的标识。 */
  interface: string;
  /** 目标 IPv4 网络地址。 */
  network: string;
  /** 目标网络前缀长度，单位为位。 */
  prefix: number;
  /** 下一跳网关；零地址表示直连路由。 */
  gateway: string;
  /** 路由优先级，不包含接口优先级。 */
  metric: number;
  /** 系统路由协议标识。 */
  protocol: number;
}
/**
 * 内外网出口及目标网段配置。
 *
 * @author lhy
 */
export interface Config {
  /** 内网接口标识。 */
  internalId: string;
  /** 外网接口标识。 */
  externalId: string;
  /** 内网路由的下一跳 IPv4 网关。 */
  internalGateway: string;
  /** 外网默认出口的网关。 */
  externalGateway: string;
  /** 走内网出口的目标 CIDR 网段。 */
  cidr: string;
}
/**
 * 后端读取的实际网络、保存配置与分流状态。
 *
 * @author lhy
 */
export interface Snapshot {
  /** 本次记录保存的原接口，供断开后保留显示名称。 */
  savedAdapters?: Adapter[];
  /** 当前可用接口列表。 */
  adapters: Adapter[];
  /** 系统读取的 IPv4 路由列表。 */
  routes: Route[];
  /** 是否存在本次开机的待撤销变更。 */
  pending: boolean;
  /** 后端根据实际网络判定的状态。 */
  routingStatus: "inactive" | "active" | "changed" | "unknown";
  /** 分流配置；快照中可为空。 */
  config: Config | null;
  /** 配置读取等过程中的异常提示。 */
  warning?: string | null;
}
/**
 * 界面、窗口行为和连通检测偏好。
 *
 * @author lhy
 */
export interface Preferences {
  /** 界面语言代码。 */
  language: Language;
  /** 浅色、深色或跟随系统的主题。 */
  theme: "light" | "dark" | "system";
  /** 关闭窗口时是否隐藏到托盘，空值表示每次询问。 */
  closeToTray: boolean | null;
  /** 是否在用户登录 Windows 后启动软件。 */
  autoStart: boolean;
  /** 自定义内网检测目标，空字符串表示使用网关。 */
  internalProbeTarget: string;
  /** 外网检测目标的 IPv4 地址。 */
  externalProbeTarget: string;
}
/**
 * 正式发布版本的检查结果。
 *
 * @author lhy
 */
export interface UpdateInfo {
  /** 是否存在高于当前版本的正式发布。 */
  available: boolean;
  /** 发布版本号。 */
  latest: string;
  /** 仓库是否存在正式发布。 */
  published: boolean;
}
/**
 * 所选接口与内外网 IPv4 检测目标。
 *
 * @author lhy
 */
export interface ProbeRequest {
  /** 外网实际检测目标；未指定时使用默认地址。 */
  externalTarget?: string;
  /** 内网接口标识。 */
  internalId: string;
  /** 实际内网检测目标，自动模式下为网关。 */
  internalGateway: string;
  /** 外网接口标识。 */
  externalId: string;
}
/**
 * 探测成功、超时或出口异常等结果状态。
 *
 * @author lhy
 */
export type ProbeStatus =
  | "reachable"
  | "timeout"
  | "unavailable"
  | "unconfigured"
  | "invalidTarget"
  | "wrongExit"
  | "error";
/**
 * 单个检测目标的状态、源地址和延迟。
 *
 * @author lhy
 */
export interface ProbeResult {
  /** 探测状态。 */
  status: ProbeStatus;
  /** 延迟毫秒数，失败或无响应时为空。 */
  latencyMs: number | null;
  /** 实际检测目标地址。 */
  target: string;
  /** 本次探测使用的源 IPv4 地址。 */
  source: string;
}
/**
 * 同一轮内外网检测结果。
 *
 * @author lhy
 */
export interface Connectivity {
  /** 内网样本或探测结果。 */
  internal: ProbeResult;
  /** 外网样本或探测结果。 */
  external: ProbeResult;
}
export const adminRequired = new EventTarget();
/**
 * 调用需要权限的后端命令，仅在明确的权限错误时请求授权弹窗。
 *
 * @param command 后端命令名称
 * @param args 传递给命令的参数
 * @returns 后端命令返回的结果
 * @author lhy
 */
async function privilegedInvoke<T>(
  command: string,
  args: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (String(error) === "ADMIN_REQUIRED") {
      adminRequired.dispatchEvent(new Event("required"));
      throw new Error(t("adminRequired"));
    }
    throw error;
  }
}
export const api = {
  /**
   * 执行关闭选择，按需保存为默认行为。
   *
   * @author lhy
   */
  finishWindowClose: (toTray: boolean, remember: boolean) =>
    invoke<void>("finish_window_close", { toTray, remember }),
  /**
   * 请求后端保存有效配置并通过系统授权重启，不自动执行路由操作。
   *
   * @param config 当前分流配置，空值表示不保存配置
   * @author lhy
   */
  restartAsAdmin: (config: Config | null) =>
    invoke<void>("restart_as_admin", { config }),
  /**
   * 请求检测内外网目标，浏览器预览中拒绝真实探测。
   *
   * @param request 读取当前探测请求的函数或请求数据
   * @author lhy
   */
  probeConnectivity: (request: ProbeRequest) =>
    isTauri()
      ? invoke<Connectivity>("probe_connectivity", { request })
      : Promise.reject(t("preview")),
  /**
   * 读取后端网络快照，浏览器预览中不提供真实网络数据。
   *
   * @author lhy
   */
  snapshot: () =>
    isTauri() ? invoke<Snapshot>("snapshot") : Promise.reject(t("preview")),
  /**
   * 提交当前配置并请求启用分流，由后端执行校验与写入。
   *
   * @param config 当前分流配置，空值表示不保存配置
   * @author lhy
   */
  enable: (config: Config) => privilegedInvoke<void>("enable", { config }),
  /**
   * 请求撤销本次记录的修改，权限或环境不匹配时由后端拒绝。
   *
   * @author lhy
   */
  restore: () => privilegedInvoke<void>("restore", {}),
  /**
   * 读取后端保存的偏好及实际登录启动状态。
   *
   * @author lhy
   */
  getPreferences: () => invoke<Preferences>("get_preferences"),
  /**
   * 提交完整偏好，由后端校验并保存。
   *
   * @param preferences 需要保存的完整偏好
   * @author lhy
   */
  savePreferences: (preferences: Preferences) =>
    privilegedInvoke<void>("save_preferences", { preferences }),
  /**
   * 请求系统浏览器打开预定义的项目链接。
   *
   * @param target 固定链接标识或检测目标地址
   * @author lhy
   */
  openLink: (
    target: "github" | "website" | "changelog" | "download" | "issues",
  ) => invoke<void>("open_project_link", { target }),
  /**
   * 请求后端查询最新正式发布，不自动下载或安装。
   *
   * @author lhy
   */
  checkUpdate: () => invoke<UpdateInfo>("check_update"),
};
