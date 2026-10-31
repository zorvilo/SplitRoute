<script setup lang="ts">
import type { Adapter, Route } from "../api";
import { t, language } from "../i18n";
defineProps<{
  routes: Route[];
  adapters: Adapter[];
  allRoutes: boolean;
  updatedAt: Date | null;
}>();
const emit = defineEmits<{ "update:allRoutes": [value: boolean] }>();
</script>
<template>
  <section :aria-label="t('routes')">
    <div class="route-heading">
      <label class="route-filter"
        ><input
          type="checkbox"
          :checked="allRoutes"
          @change="
            emit(
              'update:allRoutes',
              ($event.target as HTMLInputElement).checked,
            )
          "
        />{{ t("allRoutes") }}</label
      >
    </div>
    <div class="route-panel">
      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{{ t("destination") }}</th>
              <th scope="col">{{ t("nextHop") }}</th>
              <th scope="col">{{ t("interface") }}</th>
              <th scope="col" :title="t('metricHint')">
                {{ t("routeMetric") }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(route, index) in routes" :key="index">
              <td>{{ route.network }}/{{ route.prefix }}</td>
              <td>
                {{ route.gateway === "0.0.0.0" ? t("onLink") : route.gateway }}
              </td>
              <td>
                {{
                  adapters.find((a) => a.id === route.interface)?.name ??
                  route.interface
                }}
              </td>
              <td>{{ route.metric }}</td>
            </tr>
            <tr v-if="!routes.length">
              <td colspan="4" class="empty-routes">{{ t("noRoutes") }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    <div class="route-summary">
      <span>{{ t("routeCount", { count: routes.length }) }}</span
      ><span
        >{{ t("lastRead") }}:
        {{
          updatedAt
            ? updatedAt.toLocaleTimeString(language, { hour12: false })
            : t("notRead")
        }}</span
      >
    </div>
    <p class="route-note">{{ t("routeNote") }}</p>
  </section>
</template>
