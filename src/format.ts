// Locale-aware formatting of sizes, percentages, dates and durations.

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Binary units, like Explorer: 1 KB = 1024 bytes. */
export function formatBytes(bytes: number, locale: string): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 ? 0 : value < 10 ? 2 : value < 100 ? 1 : 0;
  const number = new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
  return `${number} ${UNITS[unit]}`;
}

export function formatCount(count: number, locale: string): string {
  return new Intl.NumberFormat(locale).format(count);
}

export function formatPercent(part: number, whole: number, locale: string): string {
  const ratio = whole > 0 ? part / whole : 0;
  return new Intl.NumberFormat(locale, {
    style: "percent",
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(ratio);
}

export function formatDate(ms: number | null, locale: string): string {
  if (ms === null) return "—";
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(ms),
  );
}

/** `4.2 s` below a minute, `12:05` (min:s) or `1:02:05` above. */
export function formatDuration(ms: number, locale: string): string {
  if (ms < 60_000) {
    const seconds = new Intl.NumberFormat(locale, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(ms / 1000);
    return `${seconds} s`;
  }
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** `12.3 MB/s`; `—` when not measured yet. */
export function formatRate(bytesPerSec: number | null, locale: string): string {
  return bytesPerSec === null ? "—" : `${formatBytes(bytesPerSec, locale)}/s`;
}

/** `12.3%` of the whole CPU; `—` when not measured yet. */
export function formatCpu(percent: number | null, locale: string): string {
  if (percent === null) return "—";
  return new Intl.NumberFormat(locale, {
    style: "percent",
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(percent / 100);
}

/** A calendar date such as `2026-01-15`, without a time. */
export function formatDay(isoDate: string | null, locale: string): string {
  if (isoDate === null) return "—";
  const [year, month, day] = isoDate.split("-").map(Number);
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(new Date(year, month - 1, day));
}
