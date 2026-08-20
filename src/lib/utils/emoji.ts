/**
 * veilanon — emoji helpers (reaction picker, validation)
 */

export const COMMON_EMOJI = ['👍', '👎', '😀', '😂', '😍', '😢', '😮', '🔥', '🎉', '❤️', '💯', '🤔'];

/** Guess whether a short string is a single emoji (lenient: 1-3 code points). */
export function isEmoji(input: string): boolean {
  const trimmed = input.trim();
  if (!trimmed || [...trimmed].length > 3) return false;
  return /[\p{Extended_Pictographic}\u200d\ufe0f]/u.test(trimmed);
}

/** Pick a deterministic pseudo-random emoji from the common set. */
export function randomEmoji(seed?: number): string {
  const idx = seed !== undefined ? Math.abs(seed) % COMMON_EMOJI.length : Math.floor(Math.random() * COMMON_EMOJI.length);
  return COMMON_EMOJI[idx];
}
