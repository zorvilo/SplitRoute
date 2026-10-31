import { ref } from "vue";
export interface Notice {
  id: number;
  text: string;
  kind: "success" | "error" | "info";
  action?: { label: string; run: () => void };
}
export const notices = ref<Notice[]>([]);
let sequence = 0;
export function dismissNotice(id: number) {
  notices.value = notices.value.filter((n) => n.id !== id);
}
export function notify(
  text: string,
  kind: Notice["kind"] = "info",
  action?: Notice["action"],
) {
  if (!text) return;
  notices.value = [
    ...notices.value.filter((n) => n.text !== text),
    { id: ++sequence, text, kind, action },
  ].slice(-3);
}
