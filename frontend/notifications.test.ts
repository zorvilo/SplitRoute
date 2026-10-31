/**
 * 浮动通知队列，合并重复内容并限制同时展示数量。对应的隔离测试。
 *
 * @author lhy
 */
import { beforeEach, expect, it } from "vitest";
import { notices, notify, dismissNotice } from "./notifications";
beforeEach(() => {
  notices.value = [];
});
/**
 * 验证相同通知合并、数量限制及手动关闭。
 *
 * @author lhy
 */
it("merges repeated messages and bounds the visible queue", () => {
  notify("saved", "success");
  const old = notices.value[0].id;
  notify("saved", "success");
  expect(notices.value).toHaveLength(1);
  expect(notices.value[0].id).not.toBe(old);
  notify("failed", "error");
  notify("third");
  notify("fourth");
  expect(notices.value.map((n) => n.text)).toEqual([
    "failed",
    "third",
    "fourth",
  ]);
  dismissNotice(notices.value[0].id);
  expect(notices.value).toHaveLength(2);
});
/**
 * 验证通知操作不会自动执行，只有主动调用才触发。
 *
 * @author lhy
 */
it("keeps update actions available without executing them", () => {
  let called = false;
  notify("update", "info", {
    label: "Download",
    /**
     * 在测试中模拟由用户主动触发的通知操作。
     *
     * @author lhy
     */
    run: () => {
      called = true;
    },
  });
  expect(called).toBe(false);
  notices.value[0].action?.run();
  expect(called).toBe(true);
});
