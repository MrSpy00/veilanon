import { readFile } from '@tauri-apps/plugin-fs';
import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Loads a local file path as a safe base64 Data URL so that WebView <img> and
 * CanvasRenderingContext2D.drawImage() can render it without broken states, CORS, or asset protocol restrictions.
 */
export async function readLocalImageAsDataUrl(filePath: string): Promise<string> {
  if (!filePath) return '';
  if (filePath.startsWith('data:') || filePath.startsWith('blob:')) {
    return filePath;
  }
  try {
    const bytes = await readFile(filePath);
    const ext = filePath.split('.').pop()?.toLowerCase() || 'png';
    const mime =
      ext === 'jpg' || ext === 'jpeg'
        ? 'image/jpeg'
        : ext === 'webp'
        ? 'image/webp'
        : ext === 'gif'
        ? 'image/gif'
        : 'image/png';

    let binary = '';
    const len = bytes.byteLength;
    // Chunking to prevent stack overflow on large images
    const chunkSize = 0x8000;
    for (let i = 0; i < len; i += chunkSize) {
      binary += String.fromCharCode.apply(
        null,
        bytes.subarray(i, Math.min(i + chunkSize, len)) as unknown as number[]
      );
    }
    return `data:${mime};base64,${btoa(binary)}`;
  } catch (err) {
    console.warn('Failed to read image bytes via plugin-fs, falling back to convertFileSrc:', err);
    return convertFileSrc(filePath);
  }
}
