import { invoke, isTauri } from "@tauri-apps/api/core";
import { t, type Language } from "./i18n";
export interface Adapter {
  id: string;
  name: string;
  addresses: string[];
  gateway: string;
}
export interface Route {
  interface: string;
  network: string;
  prefix: number;
  gateway: string;
  metric: number;
  protocol: number;
}
export interface Config {
  internalId: string;
  externalId: string;
  internalGateway: string;
  externalGateway: string;
  cidr: string;
}
export interface Snapshot {
  adapters: Adapter[];
  routes: Route[];
  pending: boolean;
  routingStatus: "inactive" | "active" | "changed" | "unknown";
  config: Config | null;
  warning?: string | null;
}
export interface Preferences {
  language: Language;
  theme: "light" | "dark" | "system";
  closeToTray: boolean;
  autoStart: boolean;
}
export interface UpdateInfo {
  available: boolean;
  latest: string;
  published: boolean;
}
export interface ProbeRequest {
  internalId: string;
  internalGateway: string;
  externalId: string;
}
export type ProbeStatus =
  | "reachable"
  | "timeout"
  | "unavailable"
  | "unconfigured"
  | "invalidTarget"
  | "wrongExit"
  | "error";
export interface ProbeResult {
  status: ProbeStatus;
  latencyMs: number | null;
  target: string;
  source: string;
}
export interface Connectivity {
  internal: ProbeResult;
  external: ProbeResult;
}
export const api = {
  probeConnectivity: (request: ProbeRequest) =>
    isTauri()
      ? invoke<Connectivity>("probe_connectivity", { request })
      : Promise.reject(t("preview")),
  snapshot: () =>
    isTauri() ? invoke<Snapshot>("snapshot") : Promise.reject(t("preview")),
  enable: (config: Config) => invoke<void>("enable", { config }),
  restore: () => invoke<void>("restore"),
  getPreferences: () => invoke<Preferences>("get_preferences"),
  savePreferences: (preferences: Preferences) =>
    invoke<void>("save_preferences", { preferences }),
  openLink: (
    target: "github" | "website" | "changelog" | "download" | "issues",
  ) => invoke<void>("open_project_link", { target }),
  checkUpdate: () => invoke<UpdateInfo>("check_update"),
};
