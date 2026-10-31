export const HISTORY_LENGTH = 30;
export function latencyCeiling(values: (number | null)[]): number {
  const maximum = Math.max(
    100,
    ...values.filter((v): v is number => v !== null && Number.isFinite(v)),
  );
  return 100 * 2 ** Math.ceil(Math.log2(maximum / 100));
}
export function latencyY(value: number, ceiling: number): number {
  return 58 - Math.min(Math.max(value, 0) / Math.max(ceiling, 1), 1) * 52;
}
export function latencyPaths(
  samples: (number | null)[],
  ceiling: number,
): string[] {
  const paths: string[] = [];
  let segment: string[] = [];
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
