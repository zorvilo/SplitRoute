import { beforeEach, expect, it } from "vitest";
import { notices, notify, dismissNotice } from "./notifications";
beforeEach(() => {
  notices.value = [];
});
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
it("keeps update actions available without executing them", () => {
  let called = false;
  notify("update", "info", {
    label: "Download",
    run: () => {
      called = true;
    },
  });
  expect(called).toBe(false);
  notices.value[0].action?.run();
  expect(called).toBe(true);
});
