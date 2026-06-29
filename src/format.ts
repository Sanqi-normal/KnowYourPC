export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "\u2014";
  if (bytes === 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let value = bytes;
  let unit = 0;

  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }

  const digits = value >= 100 || unit === 0 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export function formatNumber(value: number): string {
  if (!Number.isFinite(value)) return "\u2014";
  return new Intl.NumberFormat("zh-CN").format(value);
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms)) return "\u2014";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(2)} s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = ((ms % 60_000) / 1000).toFixed(1);
  return `${minutes} min ${seconds} s`;
}

export function formatPercent(value: number): string {
  if (!Number.isFinite(value)) return "\u2014";
  return `${value.toFixed(value >= 10 ? 1 : 2)}%`;
}
