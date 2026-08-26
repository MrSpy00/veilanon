/**
 * veilanon — identity fingerprint helpers (Signal-style visual verification)
 */

/** Split a hex fingerprint into readable 4-char chunks. */
export function fingerprintChunks(fingerprint: string): string[] {
  const hex = fingerprint.replace(/[^0-9A-Fa-f]/g, '');
  const chunks: string[] = [];
  for (let i = 0; i < hex.length; i += 4) {
    chunks.push(hex.slice(i, i + 4).toUpperCase());
  }
  return chunks;
}

/**
 * Group a fingerprint into blocks of 12 hex chars (3 chunks each) — Signal-style rows.
 * Rows can be colored independently by the UI.
 */
export function fingerprintGroups(fingerprint: string, chunksPerGroup = 3): string[][] {
  const chunks = fingerprintChunks(fingerprint);
  const groups: string[][] = [];
  for (let i = 0; i < chunks.length; i += chunksPerGroup) {
    groups.push(chunks.slice(i, i + chunksPerGroup));
  }
  return groups;
}

/** Short display form: first 8 hex chars, e.g. "3A91 F2CE". */
export function shortFingerprint(fingerprint: string): string {
  return fingerprintChunks(fingerprint).slice(0, 2).join(' ');
}

/** Full display form with 4-char chunks joined by spaces. */
export function formatFingerprint(fingerprint: string): string {
  return fingerprintChunks(fingerprint).join(' ');
}

/** Stable color per group index (for visual comparison UI). */
export function groupColor(index: number): string {
  const palette = [
    'hsl(142, 71%, 45%)',
    'hsl(262, 72%, 60%)',
    'hsl(38, 92%, 50%)',
    'hsl(200, 90%, 55%)',
    'hsl(0, 72%, 62%)',
  ];
  return palette[index % palette.length];
}

/** Safe display label for a device: type icon + name. */
export function deviceLabel(device: { name?: string; os?: string }): string {
  const parts = [device.name, device.os].filter(Boolean);
  return parts.join(' · ') || 'Bilinmeyen cihaz';
}
