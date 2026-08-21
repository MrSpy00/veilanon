import { invoke } from '@tauri-apps/api/core';

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
    // Rust command validates extension (png/jpg/jpeg/webp/gif) and returns a full data URL.
    return await invoke<string>('read_image_as_base64', { path: filePath });
  } catch (err) {
    console.warn('Failed to read image via read_image_as_base64:', err);
    throw err;
  }
}
