import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, type Snapshot } from "../api";
import { useRouting } from "./useRouting";
vi.mock("../api", () => ({
  api: { snapshot: vi.fn(), enable: vi.fn(), restore: vi.fn() },
}));
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
describe("routing state", () => {
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
  it("clears disconnected selections on refresh", async () => {
    const r = useRouting();
    await r.initialize();
    r.select("internal", "1");
    vi.mocked(api.snapshot).mockResolvedValue({ ...snapshot(), adapters: [] });
    await r.act("refresh");
    expect(r.config.internalId).toBe("");
  });
});
