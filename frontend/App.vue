<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import {
  RefreshCw,
  Play,
  RotateCcw,
  ShieldCheck,
  Check,
  CircleAlert,
} from "lucide-vue-next";
import AppNotifications from "./components/AppNotifications.vue";
import AdminPrompt from "./components/AdminPrompt.vue";
import EnableSummary from "./components/EnableSummary.vue";
import AppSidebar from "./components/AppSidebar.vue";
import RoutesView from "./components/RoutesView.vue";
import NetworkCard from "./components/NetworkCard.vue";
import SettingsView from "./components/SettingsView.vue";
import { useRouting } from "./composables/useRouting";
import { loadPreferences } from "./composables/usePreferences";
import { t } from "./i18n";
const {
  data,
  networkNotice,
  networkDetail,
  networkHeading,
  adapterNames,
  checkNetwork,
  config,
  busy,
  initialized,
  status,
  canEnable,
  routes,
  allRoutes,
  updatedAt,
  select,
  subnetMode,
  setSubnetMode,
  act,
  initialize,
} = useRouting();
const manualChecking = ref(false);
async function recheck() {
  if (busy.value || manualChecking.value) return;
  manualChecking.value = true;
  try {
    await act("inspect");
  } finally {
    manualChecking.value = false;
  }
}
const summaryOpen = ref(false);
function confirmEnable() {
  if (!canEnable.value) return;
  summaryOpen.value = false;
  void act("enable");
}
const page = ref<"config" | "routes" | "settings">("config");
async function showRoutes() {
  if (busy.value) return;
  page.value = "routes";
  await act("inspect");
}
function navigate(next: "config" | "routes" | "settings") {
  if (next === "routes") void showRoutes();
  else page.value = next;
}
let networkTimer: ReturnType<typeof setInterval> | undefined;
function onFocus() {
  if (initialized.value && !busy.value && page.value !== "settings")
    void act("refresh", true);
}
onMounted(() => {
  void initialize();
  void loadPreferences();
  networkTimer = setInterval(() => {
    void checkNetwork();
  }, 3000);
  window.addEventListener("focus", onFocus);
});
onUnmounted(() => {
  window.removeEventListener("focus", onFocus);
  if (networkTimer) clearInterval(networkTimer);
});
</script>
<template>
  <AppNotifications />
  <AdminPrompt :config="config" />
  <EnableSummary
    v-model:open="summaryOpen"
    :config="config"
    :adapters="data.adapters"
    :can-enable="canEnable"
    @confirm="confirmEnable"
  />
  <div class="app-layout">
    <AppSidebar
      :page="page"
      :config="config"
      :busy="busy"
      @navigate="navigate"
    />
    <main
      class="workspace"
      :class="{ 'config-workspace': page === 'config' }"
      :aria-busy="busy"
    >
      <header class="content-header">
        <div>
          <div class="config-title-row">
            <h1>
              {{
                t(
                  page === "config"
                    ? "config"
                    : page === "routes"
                      ? "routes"
                      : "settings",
                )
              }}
            </h1>
          </div>
          <p v-if="page !== 'settings'">
            {{ t(page === "config" ? "intro" : "routeIntro") }}
          </p>
        </div>
        <span
          v-if="page === 'config'"
          class="status-pill"
          :class="{
            warning:
              data.routingStatus === 'changed' ||
              data.routingStatus === 'unknown',
            recorded: data.routingStatus === 'active',
          }"
          ><span class="status-dot" aria-hidden="true"></span
          >{{
            networkNotice
              ? t("networkChangedTitle")
              : data.routingStatus === "active"
                ? t("routingActive")
                : status
          }}</span
        >
        <button
          v-if="page !== 'settings' && !networkNotice"
          class="button quiet"
          :disabled="busy"
          @click="act(page === 'routes' ? 'inspect' : 'refresh')"
        >
          <RefreshCw :size="16" :class="{ spinning: busy }" />{{
            t(page === "routes" ? "refreshRoutes" : "refreshAdapters")
          }}
        </button>
      </header>
      <div
        v-if="networkNotice"
        class="network-alert"
        :title="t('networkSafetyHint')"
        role="status"
        aria-live="polite"
      >
        <CircleAlert :size="20" />
        <div>
          <strong>{{ networkHeading }}</strong>
          <p>{{ networkDetail }}</p>
        </div>
        <button class="button recheck-button" :disabled="busy" @click="recheck">
          <RefreshCw :size="15" :class="{ spinning: manualChecking }" />{{
            t(manualChecking ? "checking" : "recheckNetwork")
          }}
        </button>
      </div>

      <SettingsView v-if="page === 'settings'" />
      <div v-show="page === 'config'" class="routing-config">
        <div
          v-if="
            initialized &&
            !data.pending &&
            !networkNotice &&
            data.adapters.length < 2
          "
          class="notice"
        >
          <CircleAlert :size="17" />{{
            t("tooFew", { count: data.adapters.length })
          }}
        </div>
        <div class="network-list">
          <NetworkCard
            :offline-name="adapterNames[config.internalId]"
            kind="internal"
            :selected="config.internalId"
            :gateway="config.internalGateway"
            :adapters="data.adapters"
            :disabled="busy || data.pending"
            @select="select('internal', $event)"
            @gateway="config.internalGateway = $event"
          >
            <div class="gateway-field subnet-field">
              <label for="internal-subnet">{{ t("subnet") }}</label>
              <div class="subnet-control">
                <input
                  id="internal-subnet"
                  v-model="config.cidr"
                  :aria-label="t('internalSubnet')"
                  :disabled="busy || data.pending"
                  :readonly="subnetMode === 'auto'"
                  spellcheck="false"
                  :placeholder="
                    t(
                      subnetMode === 'auto'
                        ? config.internalId
                          ? 'notPrivate'
                          : 'chooseFirst'
                        : 'subnetExample',
                    )
                  "
                  :title="t(subnetMode === 'auto' ? 'autoHint' : 'manualHint')"
                />
                <div
                  class="subnet-mode"
                  role="group"
                  :aria-label="t('subnetMode')"
                >
                  <button
                    type="button"
                    :aria-pressed="subnetMode === 'auto'"
                    :disabled="busy || data.pending"
                    @click="setSubnetMode('auto')"
                  >
                    {{ t("auto") }}</button
                  ><button
                    type="button"
                    :aria-pressed="subnetMode === 'manual'"
                    :disabled="busy || data.pending"
                    @click="setSubnetMode('manual')"
                  >
                    {{ t("manual") }}
                  </button>
                </div>
              </div>
            </div>
          </NetworkCard>
          <NetworkCard
            :offline-name="adapterNames[config.externalId]"
            kind="external"
            :selected="config.externalId"
            :gateway="config.externalGateway"
            :adapters="data.adapters"
            :disabled="busy || data.pending"
            @select="select('external', $event)"
            @gateway="config.externalGateway = $event"
          />
        </div>
        <div class="actions">
          <span class="security-note"
            ><ShieldCheck :size="16" />{{ t("recoveryNote") }}</span
          >
          <div class="action-buttons">
            <button
              class="button secondary"
              :disabled="busy || !data.pending"
              @click="act('restore')"
            >
              <RotateCcw :size="16" />{{ t("undo") }}
            </button>
            <button
              class="button primary"
              :class="{
                'enabled-button': data.routingStatus === 'active',
                'needs-check-button':
                  data.pending && data.routingStatus !== 'active',
              }"
              :disabled="!canEnable"
              @click="summaryOpen = true"
            >
              <CircleAlert
                v-if="data.pending && data.routingStatus !== 'active'"
                :size="16"
              /><Check
                v-else-if="data.routingStatus === 'active'"
                :size="16"
              /><Play v-else :size="16" />{{
                t(
                  data.pending && data.routingStatus !== "active"
                    ? "needsCheckButton"
                    : data.routingStatus === "active"
                      ? "enabledButton"
                      : "enable",
                )
              }}
            </button>
          </div>
        </div>
      </div>
      <RoutesView
        v-if="page === 'routes'"
        :routes="routes"
        :adapters="data.adapters"
        :all-routes="allRoutes"
        :updated-at="updatedAt"
        @update:all-routes="allRoutes = $event"
      />
    </main>
  </div>
</template>
