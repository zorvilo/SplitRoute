import { expect, it } from "vitest";
import { privateNetworkFromAddress } from "./subnet";
it("maps private IPv4 addresses to their full private ranges", () => {
  for (const [address, network] of [
    ["172.16.0.1/24", "172.16.0.0/12"],
    ["172.31.255.254/24", "172.16.0.0/12"],
    ["10.147.109.39/24", "10.0.0.0/8"],
    ["10.147.109.39/8", "10.0.0.0/8"],
    ["172.20.10.11/28", "172.16.0.0/12"],
    ["192.168.3.7/23", "192.168.0.0/16"],
    ["192.168.3.7/32", "192.168.0.0/16"],
    ["192.168.3.7/1", "192.168.0.0/16"],
  ])
    expect(privateNetworkFromAddress(address)).toBe(network);
  for (const bad of [
    "172.15.0.1/24",
    "172.32.0.1/24",
    "192.169.0.1/24",
    "8.8.8.8/24",
    "127.0.0.1/8",
    "169.254.1.1/16",
    "",
    "10.0.0.1",
    "10.0.0.1/0",
    "10.0.0.1/33",
    "999.0.0.1/24",
    "10.0.0.1/24x",
  ])
    expect(privateNetworkFromAddress(bad)).toBe("");
});
