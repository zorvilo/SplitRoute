import { expect, it } from "vitest";
import { latencyPaths, latencyCeiling, latencyY } from "./connectivity";
it("keeps the scale stable within latency bands", () => {
  expect(latencyCeiling([null])).toBe(100);
  expect(latencyCeiling([43, 49, 65])).toBe(100);
  expect(latencyCeiling([120])).toBe(200);
  expect(latencyCeiling([210])).toBe(400);
  expect(latencyY(0, 100)).toBe(58);
  expect(latencyY(200, 100)).toBe(6);
});
it("leaves gaps for missing replies instead of inventing zero latency", () => {
  const paths = latencyPaths([1, 2, null, 3, 4], 20);
  expect(paths).toHaveLength(2);
  expect(paths[0]).toContain(" L");
  expect(paths[1]).toMatch(/^M/);
  expect(latencyPaths([null, null], 20)).toEqual([]);
  expect(latencyPaths([1, null, 2], 20)).toEqual([]);
});
it("keeps the last 30 samples and handles zero-ms replies", () => {
  const path = latencyPaths(Array(40).fill(0), 20)[0];
  expect(path).toMatch(/^M0\.0,58\.0/);
  expect(path).toContain("L156.0,58.0");
  expect(path.match(/L/g)).toHaveLength(29);
});
