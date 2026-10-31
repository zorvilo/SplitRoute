<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import {
  RefreshCw,
  Play,
  RotateCcw,
  ListTree,
  ShieldCheck,
  CircleAlert,
  CheckCircle2,
} from "lucide-vue-next";
import AppSidebar from "./components/AppSidebar.vue";
import RoutesView from "./components/RoutesView.vue";
import NetworkCard from "./components/NetworkCard.vue";
import SettingsView from "./components/SettingsView.vue";
import { useRouting } from "./composables/useRouting";
import { loadPreferences } from "./composables/usePreferences";
import { t } from "./i18n";
const {
  data,
  config,
  busy,
  error,
  message,
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
function onFocus() {
  if (initialized.value && !busy.value && page.value !== "settings")
    void act("refresh");
}
onMounted(() => {
  void initialize();
  void loadPreferences();
  window.addEventListener("focus", onFocus);
});
onUnmounted(() => window.removeEventListener("focus", onFocus));
</script>
<template>
  <div class="app-layout">
    <AppSidebar
      :page="page"
      :config="config"
      :busy="busy"
      @navigate="navigate"
    />
    <main class="workspace" :aria-busy="busy">
      <header class="content-header">
        <div>
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
          ><span class="status-dot"></span
          >{{ busy ? t("processing") : status }}</span
        >
        <button
          v-if="page !== 'settings'"
          class="button quiet"
          :disabled="busy"
          @click="act(page === 'routes' ? 'inspect' : 'refresh')"
        >
          <RefreshCw :size="16" :class="{ spinning: busy }" />{{
            t(page === "routes" ? "refreshRoutes" : "refreshAdapters")
          }}
        </button>
      </header>
      <SettingsView v-if="page === 'settings'" />
      <div v-show="page === 'config'">
        <div v-if="initialized && data.adapters.length < 2" class="notice">
          <CircleAlert :size="17" />{{
            t("tooFew", { count: data.adapters.length })
          }}
        </div>
        <div class="network-list">
          <NetworkCard
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
              :disabled="busy || !initialized"
              @click="showRoutes"
            >
              <ListTree :size="16" />{{ t("viewRoutes") }}
            </button>
            <button
              class="button secondary"
              :disabled="busy || !data.pending"
              @click="act('restore')"
            >
              <RotateCcw :size="16" />{{ t("undo") }}
            </button>
            <button
              class="button primary"
              :disabled="!canEnable"
              @click="act('enable')"
            >
              <Play :size="16" />{{ t("enable") }}
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
      <div
        v-if="page !== 'settings'"
        class="feedback"
        :class="{ error: error }"
        role="status"
        aria-live="polite"
      >
        <CircleAlert v-if="error" :size="17" /><CheckCircle2
          v-else
          :size="17"
        /><span>{{ error || message }}</span>
      </div>
      <footer v-if="page !== 'settings'">
        <span>SplitRoute <span class="version">0.1.0</span></span>
      </footer>
    </main>
  </div>
</template>
