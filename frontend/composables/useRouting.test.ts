/**
 * 分流页面状态与操作流程，读取实际网络并保留网络变化提醒。对应的隔离测试。
 *
 * @author lhy
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, type Snapshot } from "../api";
import { useRouting } from "./useRouting";
vi.mock("../api", () => ({
  api: { snapshot: vi.fn(), enable: vi.fn(), restore: vi.fn() },
}));
/**
 * 构造包含双接口的初始路由快照。
 *
 * @author lhy
 */
function snapshot(pending = false): Snapshot {
  return {
    adapters: [
      {
        id: "1",
        name: "内网",
        addresses: ["10.0.0.2/24"],
        gateway: "10.0.0.1",
      },
      {
        id: "2",
        name: "外网",
        addresses: ["192.168.1.2/24"],
        gateway: "192.168.1.1",
      },
    ],
    routes: [],
    pending,
    routingStatus: "inactive",
    config: null,
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.snapshot).mockResolvedValue(snapshot());
});
/**
 * 分流状态、路由读取与操作边界的测试组。
 *
 * @author lhy
 */
describe("routing state", () => {
  /**
   * 验证实际路由状态与撤销记录分别判断。
   *
   * @author lhy
   */
  it("uses actual status separately from the undo record", async () => {
    vi.mocked(api.snapshot).mockResolvedValue({
      ...snapshot(true),
      routingStatus: "changed",
    });
    const r = useRouting();
    await r.initialize();
    expect(r.status.value).toBe("配置已变化");
    expect(r.data.value.pending).toBe(true);
    expect(r.canEnable.value).toBe(false);
    vi.mocked(api.snapshot).mockResolvedValue({
      ...snapshot(false),
      routingStatus: "inactive",
    });
    await r.act("refresh");
    expect(r.status.value).toBe("尚未启用");
    r.select("internal", "1");
    r.select("external", "2");
    expect(r.canEnable.value).toBe(true);
    vi.mocked(api.snapshot).mockRejectedValue("读取失败");
    await r.act("refresh");
    expect(r.status.value).toBe("无法判断");
    expect(r.canEnable.value).toBe(false);
  });
  /**
   * 验证路由筛选、全部路由展示及成功读取时间。
   *
   * @author lhy
   */
  it("filters related routes, supports all routes and timestamps successful reads", async () => {
    const base = {
      interface: "1",
      gateway: "10.0.0.1",
      metric: 10,
      protocol: 3,
    };
    vi.mocked(api.snapshot).mockResolvedValue({
      ...snapshot(),
      routes: [
        { ...base, network: "0.0.0.0", prefix: 0 },
        { ...base, network: "10.0.0.0", prefix: 8 },
        { ...base, network: "192.168.1.0", prefix: 24 },
      ],
    });
    const r = useRouting();
    expect(r.updatedAt.value).toBeNull();
    await r.initialize();
    r.setSubnetMode("manual");
    r.config.cidr = "10.0.0.0/8";
    expect(r.routes.value).toHaveLength(2);
    expect(r.updatedAt.value).toBeInstanceOf(Date);
    r.allRoutes.value = true;
    expect(r.routes.value).toHaveLength(3);
    const lastRead = r.updatedAt.value;
    vi.mocked(api.snapshot).mockRejectedValue("读取失败");
    await r.act("inspect");
    expect(r.updatedAt.value).toBe(lastRead);
    expect(r.routes.value).toHaveLength(3);
  });
  /**
   * 验证内外网需选择不同接口，并填入对应网关。
   *
   * @author lhy
   */
  it("requires explicit different adapters and fills their gateways", async () => {
    const r = useRouting();
    await r.initialize();
    expect(r.canEnable.value).toBe(false);
    r.select("internal", "1");
    r.select("external", "1");
    expect(r.canEnable.value).toBe(false);
    r.select("external", "2");
    expect(r.canEnable.value).toBe(true);
    expect(r.config.externalGateway).toBe("192.168.1.1");
  });
  /**
   * 验证自动网段推导，以及刷新和切换接口时保留手动输入。
   *
   * @author lhy
   */
  it("derives subnets in auto mode and preserves manual edits through refresh and adapter changes", async () => {
    const r = useRouting();
    await r.initialize();
    r.select("internal", "1");
    expect(r.config.cidr).toBe("10.0.0.0/8");
    r.setSubnetMode("manual");
    r.config.cidr = "10.0.0.0/8";
    await r.act("refresh");
    expect(r.config.cidr).toBe("10.0.0.0/8");
    r.select("internal", "2");
    expect(r.config.cidr).toBe("10.0.0.0/8");
    r.setSubnetMode("auto");
    expect(r.config.cidr).toBe("192.168.0.0/16");
  });
  /**
   * 验证恢复保存的手动网段，不擅自缩小范围。
   *
   * @author lhy
   */
  it("restores saved manual ranges instead of narrowing them automatically", async () => {
    vi.mocked(api.snapshot).mockResolvedValue({
      ...snapshot(),
      config: {
        internalId: "1",
        externalId: "2",
        internalGateway: "10.0.0.1",
        externalGateway: "192.168.1.1",
        cidr: "10.0.0.0/16",
      },
    });
    const r = useRouting();
    await r.initialize();
    expect(r.subnetMode.value).toBe("manual");
    expect(r.config.cidr).toBe("10.0.0.0/16");
  });
  /**
   * 验证操作失败后重新读取撤销状态。
   *
   * @author lhy
   */
  it("refreshes recovery state after a failed operation", async () => {
    const r = useRouting();
    await r.initialize();
    r.select("internal", "1");
    r.select("external", "2");
    vi.mocked(api.enable).mockRejectedValue("恢复尚未完成");
    vi.mocked(api.snapshot).mockResolvedValue(snapshot(true));
    await r.act("enable");
    expect(r.error.value).toBe("恢复尚未完成");
    expect(r.data.value.pending).toBe(true);
    expect(r.busy.value).toBe(false);
    expect(r.canEnable.value).toBe(false);
  });
  /**
   * 验证未启用分流时刷新会清除已断开的接口选择。
   *
   * @author lhy
   */
  it("clears disconnected selections on refresh", async () => {
    const r = useRouting();
    await r.initialize();
    r.select("internal", "1");
    vi.mocked(api.snapshot).mockResolvedValue({ ...snapshot(), adapters: [] });
    await r.act("refresh");
    expect(r.config.internalId).toBe("");
  });
});

/**
 * 验证网络变化延迟确认、保留所选接口及恢复后清除提醒。
 *
 * @author lhy
 */
it("debounces changed routing, retains selected adapters and clears after recovery", async () => {
  const r = useRouting();
  vi.mocked(api.snapshot).mockResolvedValue({
    ...snapshot(true),
    routingStatus: "active",
  });
  await r.initialize();
  r.select("internal", "1");
  const now = vi.spyOn(Date, "now").mockReturnValue(1000);
  vi.mocked(api.snapshot).mockResolvedValue({
    ...snapshot(true),
    adapters: [],
    routingStatus: "changed",
  });
  await r.checkNetwork();
  expect(r.networkNotice.value).toBe("");
  expect(r.config.internalId).toBe("1");
  now.mockReturnValue(3100);
  await r.checkNetwork();
  expect(r.networkNotice.value).toBe("networkChanged");
  await r.checkNetwork();
  expect(api.enable).not.toHaveBeenCalled();
  expect(api.restore).not.toHaveBeenCalled();
  vi.mocked(api.snapshot).mockResolvedValue({
    ...snapshot(true),
    routingStatus: "active",
  });
  await r.checkNetwork();
  expect(r.networkNotice.value).toBe("");
  now.mockRestore();
});
/**
 * 验证读取失败时显示未知状态，不继续声称分流正常。
 *
 * @author lhy
 */
it("reports monitoring failure without claiming routing remains active", async () => {
  const r = useRouting();
  vi.mocked(api.snapshot).mockResolvedValue({
    ...snapshot(true),
    routingStatus: "active",
  });
  await r.initialize();
  vi.mocked(api.snapshot).mockRejectedValue("unavailable");
  await r.checkNetwork();
  expect(r.networkNotice.value).toBe("networkCheckFailed");
  expect(r.data.value.routingStatus).toBe("unknown");
});

/**
 * 验证读取路由成功不通知，失败仍提示且保留原成功时间。
 *
 * @author lhy
 */
it("reads routes silently while updating timestamps and still reports failures", async () => {
  const { notices } = await import("../notifications");
  notices.value = [];
  const r = useRouting();
  await r.initialize();
  await r.act("inspect");
  const lastRead = r.updatedAt.value;
  expect(lastRead).toBeInstanceOf(Date);
  expect(notices.value).toEqual([]);
  vi.mocked(api.snapshot).mockRejectedValue("route read failed");
  await r.act("inspect");
  expect(r.updatedAt.value).toBe(lastRead);
  expect(
    notices.value.some(
      (n) => n.kind === "error" && n.text === "route read failed",
    ),
  ).toBe(true);
});
/**
 * 验证接口断开后重开仍保留原接口名称、网关与具体提醒。
 *
 * @author lhy
 */
it("keeps saved adapter names when reopening after a disconnection", async () => {
  const base = snapshot(true);
  vi.mocked(api.snapshot).mockResolvedValue({
    ...base,
    savedAdapters: base.adapters,
    adapters: [base.adapters[0]],
    routingStatus: "changed",
    config: {
      internalId: "1",
      externalId: "2",
      internalGateway: "10.0.0.1",
      externalGateway: "192.168.1.1",
      cidr: "10.0.0.0/8",
    },
  });
  const r = useRouting();
  await r.initialize();
  expect(r.config.externalId).toBe("2");
  expect(r.adapterNames["2"]).toBe("外网");
  expect(r.networkHeading.value).toBe("外网接口不可用");
  expect(r.networkDetail.value).toContain("外网");
  expect(r.networkDetail.value).toContain("当前不可用");
  expect(r.config.externalGateway).toBe("192.168.1.1");
});
