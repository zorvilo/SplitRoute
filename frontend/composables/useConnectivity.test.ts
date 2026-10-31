/**
 * 连通检测状态：管理采样、切换目标、异步过期结果和资源清理。对应的隔离测试。
 *
 * @author lhy
 */
import { createSSRApp, ref } from "vue";
import { renderToString } from "@vue/server-renderer";
import { beforeEach, expect, it, vi } from "vitest";
import { api, type Connectivity, type ProbeRequest } from "../api";
import { useConnectivity } from "./useConnectivity";
vi.mock("../api", () => ({ api: { probeConnectivity: vi.fn() } }));
beforeEach(() => vi.resetAllMocks());
/**
 * 创建可替换输入的连通检测测试状态。
 *
 * @returns 测试输入及检测状态
 * @author lhy
 */
async function setup() {
  const input = ref<ProbeRequest>({
    internalId: "1",
    internalGateway: "10.0.0.1",
    externalId: "2",
  });
  let state!: ReturnType<typeof useConnectivity>;
  await renderToString(
    createSSRApp({
      setup() {
        state = useConnectivity(() => input.value);
        return () => null;
      },
    }),
  );
  return { input, state };
}
/**
 * 构造内网零毫秒成功、外网超时的固定测试结果。
 *
 * @returns 固定的内外网检测结果
 * @author lhy
 */
const reply = (): Connectivity => ({
  internal: {
    status: "reachable",
    latencyMs: 0,
    target: "10.0.0.1",
    source: "10.0.0.2",
  },
  external: {
    status: "timeout",
    latencyMs: null,
    target: "223.5.5.5",
    source: "192.168.1.2",
  },
});
/**
 * 验证保存真实探测结果并将超时记录为空样本。
 *
 * @author lhy
 */
it("stores real replies and represents timeout samples as gaps", async () => {
  const { state } = await setup();
  vi.mocked(api.probeConnectivity).mockResolvedValue(reply());
  await state.sample();
  expect(state.history.value).toEqual([{ internal: 0, external: null }]);
  expect(state.lastRead.value).toBeInstanceOf(Date);
  for (let i = 0; i < 31; i++) await state.sample();
  expect(state.history.value).toHaveLength(30);
});
/**
 * 验证探测不重叠，并丢弃旧接口选择对应的响应。
 *
 * @author lhy
 */
it("does not overlap probes and discards replies from a previous adapter selection", async () => {
  const { input, state } = await setup();
  let resolve!: (value: Connectivity) => void;
  vi.mocked(api.probeConnectivity).mockReturnValue(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const running = state.sample();
  await state.sample();
  expect(api.probeConnectivity).toHaveBeenCalledTimes(1);
  input.value.externalId = "3";
  resolve(reply());
  await running;
  expect(state.history.value).toEqual([]);
  expect(state.probing.value).toBe(false);
});
/**
 * 验证下一次探测进行时保留旧曲线与状态。
 *
 * @author lhy
 */
it("keeps the previous curve and status visible while the next probe runs", async () => {
  const { state } = await setup();
  vi.mocked(api.probeConnectivity).mockResolvedValueOnce(reply());
  await state.sample();
  const previousTime = state.lastRead.value;
  let resolve!: (value: Connectivity) => void;
  vi.mocked(api.probeConnectivity).mockReturnValueOnce(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const pending = state.sample();
  expect(state.probing.value).toBe(true);
  expect(state.history.value).toEqual([{ internal: 0, external: null }]);
  expect(state.report.value.internal.status).toBe("reachable");
  expect(state.lastRead.value).toBe(previousTime);
  resolve(reply());
  await pending;
  expect(state.history.value).toHaveLength(2);
});
/**
 * 验证未配置接口不探测，检测错误不伪装为成功。
 *
 * @author lhy
 */
it("skips unconfigured adapters and reports probe errors honestly", async () => {
  const { input, state } = await setup();
  input.value = { internalId: "", internalGateway: "", externalId: "" };
  await state.sample();
  expect(api.probeConnectivity).not.toHaveBeenCalled();
  input.value.internalId = "1";
  vi.mocked(api.probeConnectivity).mockRejectedValue("read failed");
  await state.sample();
  expect(state.report.value.internal.status).toBe("error");
  expect(state.report.value.external.status).toBe("unconfigured");
  expect(state.lastRead.value).toBeNull();
});

/**
 * 验证发送自定义目标，并丢弃目标切换前的响应。
 *
 * @author lhy
 */
it("sends custom targets and discards a reply after the target changes", async () => {
  const { input, state } = await setup();
  input.value.externalTarget = "1.1.1.1";
  let resolve!: (value: Connectivity) => void;
  vi.mocked(api.probeConnectivity).mockReturnValueOnce(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const pending = state.sample();
  expect(api.probeConnectivity).toHaveBeenCalledWith(
    expect.objectContaining({ externalTarget: "1.1.1.1" }),
  );
  input.value.externalTarget = "8.8.8.8";
  resolve(reply());
  await pending;
  expect(state.history.value).toEqual([]);
});
