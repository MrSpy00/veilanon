/**
 * veilanon — formatting helpers (dates, file sizes, numbers)
 */

/** Format a unix-seconds timestamp as local time (HH:MM). */
export function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('tr-TR', {
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** Format a unix-seconds timestamp as a date (DD MMM YYYY). */
export function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('tr-TR', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

/** Relative label: Bugün / Dün / date. */
export function formatDayLabel(ts: number): string {
  const d = new Date(ts * 1000);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return 'Bugün';
  if (d.toDateString() === yesterday.toDateString()) return 'Dün';
  return d.toLocaleDateString('tr-TR', { year: 'numeric', month: 'long', day: 'numeric' });
}

/** Relative time, e.g. "az önce", "5 dk önce", "2 sa önce", "3 gün önce". */
export function formatRelativeTime(ts: number): string {
  if (!Number.isFinite(ts) || ts <= 0) return '—';
  const diff = Math.max(0, Date.now() / 1000 - ts);
  if (diff < 60) return 'az önce';
  const minutes = Math.floor(diff / 60);
  if (minutes < 60) return `${minutes} dk önce`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} sa önce`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days} gün önce`;
  return formatDate(ts);
}

/** Human file size: 0 B, 1.2 KB, 3.4 MB, 1.1 GB. */
export function formatFileSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value >= 100 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

/** Compact numbers: 1.2B, 3.4K. */
export function formatCompact(n: number): string {
  return new Intl.NumberFormat('tr-TR', {
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(n);
}

/** Duration in seconds → "3:24" or "1:02:03". */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = h > 0 ? String(m).padStart(2, '0') : String(m);
  const ss = String(sec).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}
