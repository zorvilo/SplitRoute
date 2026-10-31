import { createSSRApp, ref } from "vue";
import { renderToString } from "@vue/server-renderer";
import { beforeEach, expect, it, vi } from "vitest";
import { api, type Connectivity, type ProbeRequest } from "../api";
import { useConnectivity } from "./useConnectivity";
vi.mock("../api", () => ({ api: { probeConnectivity: vi.fn() } }));
beforeEach(() => vi.resetAllMocks());
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
it("stores real replies and represents timeout samples as gaps", async () => {
  const { state } = await setup();
  vi.mocked(api.probeConnectivity).mockResolvedValue(reply());
  await state.sample();
  expect(state.history.value).toEqual([{ internal: 0, external: null }]);
  expect(state.lastRead.value).toBeInstanceOf(Date);
  for (let i = 0; i < 31; i++) await state.sample();
  expect(state.history.value).toHaveLength(30);
});
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
