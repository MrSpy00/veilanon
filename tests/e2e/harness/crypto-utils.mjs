/**
 * VeilAnon E2E Test Harness — Crypto Utilities
 * Real cryptographic primitives and deterministic generators.
 */
import { createHash, randomBytes } from 'node:crypto';

/**
 * Compute SHA-1 hex digest (uppercase) for k-Anonymity checks
 * @param {string} input 
 * @returns {string} 40-character uppercase hexadecimal SHA-1 string
 */
export function sha1HexUpper(input) {
  return createHash('sha1').update(input, 'utf8').digest('hex').toUpperCase();
}

/**
 * Compute SHA-256 hex digest (lowercase)
 * @param {string} input 
 * @returns {string} 64-character lowercase hexadecimal SHA-256 string
 */
export function sha256Hex(input) {
  return createHash('sha256').update(input, 'utf8').digest('hex').toLowerCase();
}

/**
 * Mulberry32 32-bit PRNG from integer seed
 * @param {number} seed 
 * @returns {() => number} Returns float in [0, 1)
 */
export function createMulberry32(seed) {
  let s = seed | 0;
  return function () {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Derive integer seed from arbitrary string
 * @param {string} str 
 * @returns {number} 32-bit signed integer
 */
export function stringToSeed(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  return hash;
}

/**
 * Escape XML/SVG special characters for XSS prevention
 * @param {string} unsafe 
 * @returns {string}
 */
export function escapeXml(unsafe) {
  return String(unsafe)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

/**
 * Deterministic Privacy Avatar (Identicon) Generator
 * Generates an SVG XML string based on seed string.
 * Symmetric 5x5 grid with deterministic colors and zero tracking.
 * 
 * @param {string} seed 
 * @param {number} [size=128] 
 * @returns {string} Valid SVG XML string
 */
export function generateDeterministicSvgAvatar(seed, size = 128) {
  const safeSeed = seed ?? '';
  const hash = sha256Hex(safeSeed);
  
  // Extract RGB components from hash
  const r = parseInt(hash.slice(0, 2), 16);
  const g = parseInt(hash.slice(2, 4), 16);
  const b = parseInt(hash.slice(4, 6), 16);
  
  // Background color (dark tint)
  const bgR = Math.floor(r * 0.2);
  const bgG = Math.floor(g * 0.2);
  const bgB = Math.floor(b * 0.2);
  
  // Primary shape color
  const fgR = Math.min(255, r + 40);
  const fgG = Math.min(255, g + 40);
  const fgB = Math.min(255, b + 40);
  
  // Secondary accent color
  const accR = parseInt(hash.slice(6, 8), 16);
  const accG = parseInt(hash.slice(8, 10), 16);
  const accB = parseInt(hash.slice(10, 12), 16);

  const cellSize = size / 5;
  const rects = [];

  // 5x5 symmetric grid (3 columns mirrored to 5)
  for (let x = 0; x < 3; x++) {
    for (let y = 0; y < 5; y++) {
      const idx = x * 5 + y;
      const byteVal = parseInt(hash.slice((idx % 30) + 12, (idx % 30) + 14) || '00', 16);
      if (byteVal % 2 === 1) {
        const fill = byteVal % 3 === 0 
          ? `rgb(${accR},${accG},${accB})` 
          : `rgb(${fgR},${fgG},${fgB})`;
        
        // Left/center column
        const xPos1 = x * cellSize;
        const yPos = y * cellSize;
        rects.push(`<rect x="${xPos1.toFixed(1)}" y="${yPos.toFixed(1)}" width="${cellSize.toFixed(1)}" height="${cellSize.toFixed(1)}" fill="${fill}" rx="2"/>`);
        
        // Mirror to right column (if not center column x=2)
        if (x !== 2) {
          const xPos2 = (4 - x) * cellSize;
          rects.push(`<rect x="${xPos2.toFixed(1)}" y="${yPos.toFixed(1)}" width="${cellSize.toFixed(1)}" height="${cellSize.toFixed(1)}" fill="${fill}" rx="2"/>`);
        }
      }
    }
  }

  // Sanitize seed for metadata attribute
  const escapedSeed = escapeXml(safeSeed.slice(0, 64));

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}" data-seed="${escapedSeed}"><rect width="100%" height="100%" fill="rgb(${bgR},${bgG},${bgB})" rx="${(size / 8).toFixed(1)}"/>${rects.join('')}</svg>`;
}

/**
 * Validate that a string is a well-formed SVG XML
 * @param {string} svgContent 
 * @returns {{ valid: boolean, error?: string }}
 */
export function validateSvgXml(svgContent) {
  if (typeof svgContent !== 'string') {
    return { valid: false, error: 'SVG content must be a string' };
  }
  const trimmed = svgContent.trim();
  if (!trimmed.startsWith('<svg') || !trimmed.endsWith('</svg>')) {
    return { valid: false, error: 'SVG must start with <svg and end with </svg>' };
  }
  if (!trimmed.includes('xmlns="http://www.w3.org/2000/svg"')) {
    return { valid: false, error: 'Missing xmlns attribute' };
  }
  if (!trimmed.includes('viewBox=') && !trimmed.includes('width=')) {
    return { valid: false, error: 'Missing viewBox or dimensions' };
  }
  return { valid: true };
}
