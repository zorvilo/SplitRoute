/**
 * 延迟图表的采样长度、纵轴范围与路径计算。
 *
 * @author lhy
 */
export const HISTORY_LENGTH = 30;
/**
 * 将有效延迟上限归入倍增档位，避免纵轴随每次微小变化缩放。
 *
 * @param values 有效延迟样本或文案占位参数
 * @returns 纵轴显示上限，单位为毫秒
 * @author lhy
 */
export function latencyCeiling(values: (number | null)[]): number {
  const maximum = Math.max(
    100,
    ...values.filter((v): v is number => v !== null && Number.isFinite(v)),
  );
  return 100 * 2 ** Math.ceil(Math.log2(maximum / 100));
}
/**
 * 将毫秒延迟映射到图表纵坐标，并限制在绘图区内。
 *
 * @param value 延迟值，单位为毫秒
 * @param ceiling 纵轴显示上限，单位为毫秒
 * @returns 图表中的纵坐标
 * @author lhy
 */
export function latencyY(value: number, ceiling: number): number {
  return 58 - Math.min(Math.max(value, 0) / Math.max(ceiling, 1), 1) * 52;
}
/**
 * 将最近三十个有效样本转为路径，无响应处断开且不伪造零延迟。
 *
 * @param samples 延迟样本，空值表示无响应
 * @param ceiling 纵轴显示上限，单位为毫秒
 * @returns 按无响应样本分隔的连续路径
 * @author lhy
 */
export function latencyPaths(
  samples: (number | null)[],
  ceiling: number,
): string[] {
  const paths: string[] = [];
  let segment: string[] = [];
  /**
   * 提交至少包含两个点的连续片段，并清空片段缓冲。
   *
   * @author lhy
   */
  function finish() {
    if (segment.length > 1) paths.push(segment.join(" "));
    segment = [];
  }
  samples.slice(-HISTORY_LENGTH).forEach((value, index, values) => {
    if (value === null || !Number.isFinite(value)) {
      finish();
      return;
    }
    const x = (
      ((HISTORY_LENGTH - values.length + index) * 156) /
      (HISTORY_LENGTH - 1)
    ).toFixed(1);
    const y = latencyY(value, ceiling).toFixed(1);
    segment.push(`${segment.length ? "L" : "M"}${x},${y}`);
  });
  finish();
  return paths;
}
