/**
 * 延迟图表的采样长度、纵轴范围与路径计算。对应的隔离测试。
 *
 * @author lhy
 */
import { expect, it } from "vitest";
import { latencyPaths, latencyCeiling, latencyY } from "./connectivity";
/**
 * 验证延迟分档与纵坐标边界保持稳定。
 *
 * @author lhy
 */
it("keeps the scale stable within latency bands", () => {
  expect(latencyCeiling([null])).toBe(100);
  expect(latencyCeiling([43, 49, 65])).toBe(100);
  expect(latencyCeiling([120])).toBe(200);
  expect(latencyCeiling([210])).toBe(400);
  expect(latencyY(0, 100)).toBe(58);
  expect(latencyY(200, 100)).toBe(6);
});
/**
 * 验证无响应样本断开曲线，不生成虚假的零延迟。
 *
 * @author lhy
 */
it("leaves gaps for missing replies instead of inventing zero latency", () => {
  const paths = latencyPaths([1, 2, null, 3, 4], 20);
  expect(paths).toHaveLength(2);
  expect(paths[0]).toContain(" L");
  expect(paths[1]).toMatch(/^M/);
  expect(latencyPaths([null, null], 20)).toEqual([]);
  expect(latencyPaths([1, null, 2], 20)).toEqual([]);
});
/**
 * 验证历史长度限制，并保留真实零毫秒响应。
 *
 * @author lhy
 */
it("keeps the last 30 samples and handles zero-ms replies", () => {
  const path = latencyPaths(Array(40).fill(0), 20)[0];
  expect(path).toMatch(/^M0\.0,58\.0/);
  expect(path).toContain("L156.0,58.0");
  expect(path.match(/L/g)).toHaveLength(29);
});
