/**
 * 浮动通知队列，合并重复内容并限制同时展示数量。
 *
 * @author lhy
 */
import { ref } from "vue";
/**
 * 带类型、标识和可选操作的浮动通知。
 *
 * @author lhy
 */
export interface Notice {
  /** 当前通知队列中的唯一编号。 */
  id: number;
  /** 通知内容。 */
  text: string;
  /** 通知类型：成功、错误或普通信息。 */
  kind: "success" | "error" | "info";
  /** 需要用户主动点击的可选通知操作。 */
  action?: {
    /** 控件或操作按钮的显示文本。 */
    label: string; /** 点击通知操作按钮时执行的回调。 */
    run: () => void;
  };
}
export const notices = ref<Notice[]>([]);
let sequence = 0;
/**
 * 按通知标识移除指定提示。
 *
 * @param id 接口或通知的标识
 * @author lhy
 */
export function dismissNotice(id: number) {
  notices.value = notices.value.filter((n) => n.id !== id);
}
/**
 * 合并相同内容后追加通知，最多保留三条可见提示。
 *
 * @param text 待校验地址或通知内容
 * @param kind 出口角色或通知类型
 * @param action 操作名称或可选通知按钮
 * @author lhy
 */
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
