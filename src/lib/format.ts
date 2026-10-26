export function classNames(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(' ');
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function relativeTime(iso: string, locale = 'fa-IR'): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const diff = date.getTime() - Date.now();
  const abs = Math.abs(diff);
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  if (abs < 60_000) return formatter.format(Math.round(diff / 1000), 'second');
  if (abs < 3_600_000) return formatter.format(Math.round(diff / 60_000), 'minute');
  if (abs < 86_400_000) return formatter.format(Math.round(diff / 3_600_000), 'hour');
  return formatter.format(Math.round(diff / 86_400_000), 'day');
}

export function shortPath(path: string | null | undefined, max = 48): string {
  if (!path) return '';
  const normalized = path.replace(/\\/g, '/');
  if (normalized.length <= max) return normalized;
  return `…${normalized.slice(normalized.length - max + 1)}`;
}

export function titleFromPath(path: string | null | undefined): string {
  if (!path) return '';
  const parts = path.replace(/\\/g, '/').split('/').filter(Boolean);
  return parts[parts.length - 1] || path;
}
