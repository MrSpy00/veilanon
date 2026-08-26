/**
 * VeilAnon — Effects System Unit Tests
 * Tests all 21 built-in effects' process() with mock Canvas context.
 *
 * Pattern: Node.js assert module (matching tests/e2e/ harness style)
 * Each effect is called with a mock CanvasRenderingContext2D and must not throw.
 */

import assert from 'node:assert/strict';

// ── Helpers ──────────────────────────────────────────────────────────────────

function toPixel(lm, w, h) {
  return [lm.x * w, lm.y * h];
}

function faceBBox(face, w, h) {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const lm of face) {
    const px = lm.x * w;
    const py = lm.y * h;
    if (px < minX) minX = px;
    if (py < minY) minY = py;
    if (px > maxX) maxX = px;
    if (py > maxY) maxY = py;
  }
  const bw = maxX - minX;
  const bh = maxY - minY;
  return { x: minX, y: minY, w: bw, h: bh, cx: minX + bw / 2, cy: minY + bh / 2 };
}

// ── Mock Canvas Context ──────────────────────────────────────────────────────

function createMockCtx() {
  return {
    canvas: { width: 1280, height: 720 },
    save: () => {},
    restore: () => {},
    beginPath: () => {},
    closePath: () => {},
    moveTo: () => {},
    lineTo: () => {},
    stroke: () => {},
    fill: () => {},
    arc: () => {},
    ellipse: () => {},
    rect: () => {},
    roundRect: () => {},
    clip: () => {},
    clearRect: () => {},
    drawImage: () => {},
    createRadialGradient: () => ({ addColorStop: () => {} }),
    createLinearGradient: () => ({ addColorStop: () => {} }),
    getImageData: () => ({ data: new Uint8ClampedArray(1280 * 720 * 4), width: 1280, height: 720 }),
    putImageData: () => {},
    measureText: () => ({ width: 100 }),
    translate: () => {},
    rotate: () => {},
    scale: () => {},
    fillRect: () => {},
    strokeRect: () => {},
    fillText: () => {},
    strokeText: () => {},
    shadowColor: '',
    shadowBlur: 0,
    lineWidth: 1,
    lineCap: 'butt',
    lineJoin: 'miter',
    strokeStyle: '',
    fillStyle: '',
    font: '',
    textAlign: 'start',
    textBaseline: 'alphabetic',
    filter: 'none',
    globalAlpha: 1,
    globalCompositeOperation: 'source-over',
    quadraticCurveTo: () => {},
    bezierCurveTo: () => {},
    setLineDash: () => {},
    lineDashOffset: 0,
  };
}

// Mock document.createElement for mirrorFace effect (Node.js doesn't have DOM)
const _origDocument = globalThis.document;
globalThis.document = {
  createElement: (tag) => {
    if (tag === 'canvas') {
      return {
        width: 0,
        height: 0,
        getContext: () => ({
          putImageData: () => {},
          drawImage: () => {},
        }),
      };
    }
    return {};
  },
};

// ── Mock Landmarks ───────────────────────────────────────────────────────────

function generateFaceLandmarks(count = 478) {
  const landmarks = [];
  for (let i = 0; i < count; i++) {
    landmarks.push({
      x: 0.3 + (Math.sin(i * 0.1) * 0.2),
      y: 0.3 + (Math.cos(i * 0.1) * 0.2),
      z: Math.sin(i * 0.05) * 0.01,
      visibility: 0.9,
    });
  }
  return landmarks;
}

function generateHandLandmarks(count = 21) {
  const landmarks = [];
  for (let i = 0; i < count; i++) {
    landmarks.push({
      x: 0.5 + (Math.sin(i * 0.3) * 0.1),
      y: 0.5 + (Math.cos(i * 0.3) * 0.1),
      z: 0,
      visibility: 0.9,
    });
  }
  return landmarks;
}

function generatePoseLandmarks(count = 33) {
  const landmarks = [];
  for (let i = 0; i < count; i++) {
    landmarks.push({
      x: 0.4 + (Math.sin(i * 0.2) * 0.15),
      y: 0.2 + (i / count) * 0.6,
      z: 0,
      visibility: 0.9,
    });
  }
  return landmarks;
}

const W = 1280;
const H = 720;
const TIMESTAMP = 1700000000000;

const emptyTracking = { timestamp: TIMESTAMP };

const faceTracking = {
  face: [generateFaceLandmarks(478)],
  timestamp: TIMESTAMP,
};

const handsTracking = {
  hands: [generateHandLandmarks(21)],
  timestamp: TIMESTAMP,
};

const poseTracking = {
  pose: [generatePoseLandmarks(33)],
  timestamp: TIMESTAMP,
};

const fullTracking = {
  face: [generateFaceLandmarks(478)],
  hands: [generateHandLandmarks(21)],
  pose: [generatePoseLandmarks(33)],
  timestamp: TIMESTAMP,
};

// ── Effect Definitions (mirrored from src/lib/effects/effects.ts) ─────────────

const effects = [
  {
    id: 'soft-blur-face',
    name: 'Soft Blur',
    category: 'face',
    requires: ['face'],
    params: [{ name: 'intensity', type: 'number', default: 8 }],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face) return;
      const intensity = params.intensity ?? 8;
      const bbox = faceBBox(face, w, h);
      const pad = bbox.w * 0.15;
      const x = Math.max(0, bbox.x - pad);
      const y = Math.max(0, bbox.y - pad);
      const bw = Math.min(w - x, bbox.w + pad * 2);
      const bh = Math.min(h - y, bbox.h + pad * 2);
      ctx.save();
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, 20);
      ctx.clip();
      ctx.filter = `blur(${intensity}px)`;
      ctx.drawImage(ctx.canvas, 0, 0);
      ctx.filter = 'none';
      ctx.restore();
    },
  },
  {
    id: 'neon-outline',
    name: 'Neon Outline',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'color', type: 'color', default: '#00ff88' },
      { name: 'thickness', type: 'number', default: 2 },
      { name: 'glow', type: 'number', default: 12 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const color = params.color ?? '#00ff88';
      const thickness = params.thickness ?? 2;
      const glow = params.glow ?? 12;
      const contourIndices = [
        10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288,
        397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136,
        172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109,
      ];
      ctx.save();
      ctx.strokeStyle = color;
      ctx.lineWidth = thickness;
      ctx.shadowColor = color;
      ctx.shadowBlur = glow;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.beginPath();
      for (let i = 0; i < contourIndices.length; i++) {
        const idx = contourIndices[i];
        if (idx >= face.length) continue;
        const [px, py] = toPixel(face[idx], w, h);
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.stroke();
      const leftEye = [33, 7, 163, 144, 145, 153, 154, 155, 133, 173, 157, 158, 159, 160, 161, 246];
      const rightEye = [362, 382, 381, 380, 374, 373, 390, 249, 263, 466, 388, 387, 386, 385, 384, 398];
      for (const eye of [leftEye, rightEye]) {
        ctx.beginPath();
        for (let i = 0; i < eye.length; i++) {
          const idx = eye[i];
          if (idx >= face.length) continue;
          const [px, py] = toPixel(face[idx], w, h);
          if (i === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        }
        ctx.closePath();
        ctx.stroke();
      }
      const lips = [61, 146, 91, 181, 84, 17, 314, 405, 321, 375, 291, 409, 270, 269, 267, 0, 37, 39, 40, 185];
      ctx.beginPath();
      for (let i = 0; i < lips.length; i++) {
        const idx = lips[i];
        if (idx >= face.length) continue;
        const [px, py] = toPixel(face[idx], w, h);
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();
      ctx.stroke();
      ctx.restore();
    },
  },
  {
    id: 'anime-eyes',
    name: 'Anime Eyes',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'scale', type: 'number', default: 1.5 },
      { name: 'sparkle', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const scale = params.scale ?? 1.5;
      const sparkle = params.sparkle;
      const leftEyeCenter = face[159];
      const rightEyeCenter = face[386];
      if (!leftEyeCenter || !rightEyeCenter) return;
      const bbox = faceBBox(face, w, h);
      const eyeRadius = bbox.w * 0.12 * scale;
      const drawAnimeEye = (cx, cy) => {
        ctx.save();
        ctx.translate(cx, cy);
        ctx.beginPath();
        ctx.ellipse(0, 0, eyeRadius, eyeRadius * 1.2, 0, 0, Math.PI * 2);
        ctx.fillStyle = '#fff';
        ctx.fill();
        ctx.strokeStyle = '#222';
        ctx.lineWidth = 2;
        ctx.stroke();
        const irisR = eyeRadius * 0.65;
        const gradient = ctx.createRadialGradient(0, 0, 0, 0, 0, irisR);
        gradient.addColorStop(0, '#c44dff');
        gradient.addColorStop(0.5, '#8b5cf6');
        gradient.addColorStop(1, '#4c1d95');
        ctx.beginPath();
        ctx.arc(0, 0, irisR, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();
        ctx.beginPath();
        ctx.arc(0, 0, irisR * 0.35, 0, Math.PI * 2);
        ctx.fillStyle = '#000';
        ctx.fill();
        if (sparkle) {
          ctx.beginPath();
          ctx.arc(-irisR * 0.3, -irisR * 0.3, irisR * 0.2, 0, Math.PI * 2);
          ctx.fillStyle = 'rgba(255,255,255,0.9)';
          ctx.fill();
          ctx.beginPath();
          ctx.arc(irisR * 0.2, irisR * 0.15, irisR * 0.1, 0, Math.PI * 2);
          ctx.fillStyle = 'rgba(255,255,255,0.6)';
          ctx.fill();
        }
        ctx.beginPath();
        ctx.ellipse(0, -eyeRadius * 0.1, eyeRadius * 1.1, eyeRadius * 0.3, 0, Math.PI, 0);
        ctx.strokeStyle = '#1a1a2e';
        ctx.lineWidth = 3;
        ctx.stroke();
        ctx.restore();
      };
      const [lx, ly] = toPixel(leftEyeCenter, w, h);
      const [rx, ry] = toPixel(rightEyeCenter, w, h);
      drawAnimeEye(lx, ly);
      drawAnimeEye(rx, ry);
    },
  },
  {
    id: 'cat-ears',
    name: 'Cat Ears',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'earColor', type: 'color', default: '#ff9a56' },
      { name: 'whiskers', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const earColor = params.earColor ?? '#ff9a56';
      const showWhiskers = params.whiskers;
      const bbox = faceBBox(face, w, h);
      const earSize = bbox.w * 0.25;
      const forehead = face[10];
      const chin = face[152];
      if (!forehead || !chin) return;
      const tilt = Math.atan2(
        toPixel(chin, w, h)[0] - toPixel(forehead, w, h)[0],
        toPixel(chin, w, h)[1] - toPixel(forehead, w, h)[1]
      );
      const [fcx, fcy] = toPixel(forehead, w, h);
      // Left ear
      ctx.save();
      ctx.translate(fcx - bbox.w * 0.32, fcy - earSize * 0.6);
      ctx.rotate(tilt - 0.3);
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(-earSize * 0.3, -earSize);
      ctx.lineTo(earSize * 0.3, -earSize * 0.2);
      ctx.closePath();
      ctx.fillStyle = earColor;
      ctx.fill();
      ctx.strokeStyle = 'rgba(0,0,0,0.3)';
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(0, -earSize * 0.15);
      ctx.lineTo(-earSize * 0.15, -earSize * 0.7);
      ctx.lineTo(earSize * 0.1, -earSize * 0.25);
      ctx.closePath();
      ctx.fillStyle = 'rgba(255,180,180,0.5)';
      ctx.fill();
      ctx.restore();
      // Right ear
      ctx.save();
      ctx.translate(fcx + bbox.w * 0.32, fcy - earSize * 0.6);
      ctx.rotate(tilt + 0.3);
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(earSize * 0.3, -earSize);
      ctx.lineTo(-earSize * 0.3, -earSize * 0.2);
      ctx.closePath();
      ctx.fillStyle = earColor;
      ctx.fill();
      ctx.strokeStyle = 'rgba(0,0,0,0.3)';
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(0, -earSize * 0.15);
      ctx.lineTo(earSize * 0.15, -earSize * 0.7);
      ctx.lineTo(-earSize * 0.1, -earSize * 0.25);
      ctx.closePath();
      ctx.fillStyle = 'rgba(255,180,180,0.5)';
      ctx.fill();
      ctx.restore();
      // Whiskers
      if (showWhiskers) {
        const nose = face[4];
        if (!nose) return;
        const [nx, ny] = toPixel(nose, w, h);
        const whiskerLen = bbox.w * 0.4;
        ctx.save();
        ctx.strokeStyle = 'rgba(255,255,255,0.7)';
        ctx.lineWidth = 1.5;
        ctx.lineCap = 'round';
        for (const angle of [-0.15, 0, 0.15]) {
          ctx.beginPath();
          ctx.moveTo(nx - bbox.w * 0.05, ny + bbox.h * 0.02);
          ctx.lineTo(nx - whiskerLen, ny + angle * whiskerLen + bbox.h * 0.02);
          ctx.stroke();
        }
        for (const angle of [-0.15, 0, 0.15]) {
          ctx.beginPath();
          ctx.moveTo(nx + bbox.w * 0.05, ny + bbox.h * 0.02);
          ctx.lineTo(nx + whiskerLen, ny + angle * whiskerLen + bbox.h * 0.02);
          ctx.stroke();
        }
        ctx.restore();
      }
    },
  },
  {
    id: 'sunglasses',
    name: 'Sunglasses',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'tint', type: 'color', default: '#1a1a2e' },
      { name: 'opacity', type: 'number', default: 0.85 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const tint = params.tint ?? '#1a1a2e';
      const opacity = params.opacity ?? 0.85;
      const leftEye = face[33];
      const rightEye = face[263];
      const leftTemple = face[127];
      const rightTemple = face[356];
      if (!leftEye || !rightEye || !leftTemple || !rightTemple) return;
      const [lex, ley] = toPixel(leftEye, w, h);
      const [rex, rey] = toPixel(rightEye, w, h);
      const [ltx, lty] = toPixel(leftTemple, w, h);
      const [rtx, rty] = toPixel(rightTemple, w, h);
      const eyeWidth = Math.abs(rex - lex) * 0.55;
      const eyeHeight = eyeWidth * 0.45;
      ctx.save();
      ctx.globalAlpha = opacity;
      ctx.beginPath();
      ctx.moveTo(lex + eyeWidth * 0.5, ley);
      ctx.quadraticCurveTo((lex + rex) / 2, ley - eyeHeight * 0.3, rex - eyeWidth * 0.5, rey);
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 3;
      ctx.stroke();
      const lcx = lex + (rex - lex) * 0.25;
      const lcy = ley;
      ctx.beginPath();
      ctx.ellipse(lcx, lcy, eyeWidth, eyeHeight, 0, 0, Math.PI * 2);
      const lGrad = ctx.createRadialGradient(lcx, lcy, 0, lcx, lcy, eyeWidth);
      lGrad.addColorStop(0, tint);
      lGrad.addColorStop(1, 'rgba(0,0,0,0.9)');
      ctx.fillStyle = lGrad;
      ctx.fill();
      ctx.strokeStyle = '#222';
      ctx.lineWidth = 2.5;
      ctx.stroke();
      const rcx = rex - (rex - lex) * 0.25;
      const rcy = rey;
      ctx.beginPath();
      ctx.ellipse(rcx, rcy, eyeWidth, eyeHeight, 0, 0, Math.PI * 2);
      const rGrad = ctx.createRadialGradient(rcx, rcy, 0, rcx, rcy, eyeWidth);
      rGrad.addColorStop(0, tint);
      rGrad.addColorStop(1, 'rgba(0,0,0,0.9)');
      ctx.fillStyle = rGrad;
      ctx.fill();
      ctx.strokeStyle = '#222';
      ctx.lineWidth = 2.5;
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(lcx - eyeWidth, lcy);
      ctx.lineTo(ltx, lty);
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 3;
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(rcx + eyeWidth, rcy);
      ctx.lineTo(rtx, rty);
      ctx.stroke();
      ctx.globalAlpha = 0.15;
      ctx.beginPath();
      ctx.ellipse(lcx - eyeWidth * 0.2, lcy - eyeHeight * 0.2, eyeWidth * 0.25, eyeHeight * 0.2, -0.3, 0, Math.PI * 2);
      ctx.fillStyle = '#fff';
      ctx.fill();
      ctx.beginPath();
      ctx.ellipse(rcx - eyeWidth * 0.2, rcy - eyeHeight * 0.2, eyeWidth * 0.25, eyeHeight * 0.2, -0.3, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    },
  },
  {
    id: 'face-paint',
    name: 'Face Paint',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'style', type: 'select', default: 'warrior' },
      { name: 'color1', type: 'color', default: '#ff6b6b' },
      { name: 'color2', type: 'color', default: '#4d96ff' },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const style = params.style ?? 'warrior';
      const c1 = params.color1 ?? '#ff6b6b';
      const c2 = params.color2 ?? '#4d96ff';
      const bbox = faceBBox(face, w, h);
      ctx.save();
      ctx.globalAlpha = 0.6;
      if (style === 'warrior') {
        const cheekL = face[50];
        const cheekR = face[280];
        if (cheekL && cheekR) {
          const [clx, cly] = toPixel(cheekL, w, h);
          const [crx, cry] = toPixel(cheekR, w, h);
          const stripeH = bbox.h * 0.06;
          for (let i = 0; i < 3; i++) {
            const offset = (i - 1) * stripeH * 2.5;
            ctx.beginPath();
            ctx.moveTo(clx - bbox.w * 0.15, cly + offset);
            ctx.lineTo(clx + bbox.w * 0.15, cly + offset);
            ctx.strokeStyle = i % 2 === 0 ? c1 : c2;
            ctx.lineWidth = stripeH;
            ctx.lineCap = 'round';
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(crx - bbox.w * 0.15, cry + offset);
            ctx.lineTo(crx + bbox.w * 0.15, cry + offset);
            ctx.stroke();
          }
        }
      } else if (style === 'butterfly') {
        const nose = face[4];
        if (nose) {
          const [nx, ny] = toPixel(nose, w, h);
          const wingSize = bbox.w * 0.18;
          for (const side of [-1, 1]) {
            ctx.beginPath();
            ctx.ellipse(nx + side * wingSize * 1.2, ny - wingSize * 0.3, wingSize, wingSize * 0.6, side * 0.3, 0, Math.PI * 2);
            ctx.fillStyle = c1;
            ctx.fill();
            ctx.beginPath();
            ctx.ellipse(nx + side * wingSize * 0.8, ny + wingSize * 0.3, wingSize * 0.7, wingSize * 0.4, side * -0.2, 0, Math.PI * 2);
            ctx.fillStyle = c2;
            ctx.fill();
          }
        }
      } else if (style === 'hearts') {
        const cheekL = face[50];
        const cheekR = face[280];
        if (cheekL && cheekR) {
          const [lx, ly] = toPixel(cheekL, w, h);
          const [rx, ry] = toPixel(cheekR, w, h);
          const drawHeart = (cx, cy, size) => {
            ctx.beginPath();
            ctx.moveTo(cx, cy + size * 0.3);
            ctx.bezierCurveTo(cx - size * 0.5, cy - size * 0.3, cx - size, cy + size * 0.1, cx, cy + size);
            ctx.bezierCurveTo(cx + size, cy + size * 0.1, cx + size * 0.5, cy - size * 0.3, cx, cy + size * 0.3);
            ctx.fillStyle = c1;
            ctx.fill();
          };
          drawHeart(lx, ly - bbox.h * 0.05, bbox.h * 0.08);
          drawHeart(rx, ry - bbox.h * 0.05, bbox.h * 0.08);
        }
      }
      ctx.restore();
    },
  },
  {
    id: 'particle-hands',
    name: 'Particle Hands',
    category: 'hand',
    requires: ['hands'],
    params: [
      { name: 'color', type: 'color', default: '#a855f7' },
      { name: 'density', type: 'number', default: 5 },
    ],
    process(ctx, w, h, tracking, params) {
      const hands = tracking.hands;
      if (!hands?.length) return;
      const color = params.color ?? '#a855f7';
      const density = params.density ?? 5;
      const tips = [4, 8, 12, 16, 20];
      for (const hand of hands) {
        for (const tipIdx of tips) {
          if (tipIdx >= hand.length) continue;
          const tip = hand[tipIdx];
          const [px, py] = toPixel(tip, w, h);
          for (let i = 0; i < density; i++) {
            const angle = Math.random() * Math.PI * 2;
            const speed = 1 + Math.random() * 3;
            const size = 2 + Math.random() * 4;
            const alpha = 0.5 + Math.random() * 0.5;
            ctx.save();
            ctx.globalAlpha = alpha;
            ctx.fillStyle = color;
            ctx.shadowColor = color;
            ctx.shadowBlur = 8;
            ctx.beginPath();
            ctx.arc(
              px + Math.cos(angle + tracking.timestamp * 0.003) * speed * 5,
              py + Math.sin(angle + tracking.timestamp * 0.003) * speed * 5,
              size,
              0,
              Math.PI * 2
            );
            ctx.fill();
            ctx.restore();
          }
        }
      }
    },
  },
  {
    id: 'laser-fingers',
    name: 'Laser Fingers',
    category: 'hand',
    requires: ['hands'],
    params: [
      { name: 'color', type: 'color', default: '#ff0040' },
      { name: 'width', type: 'number', default: 3 },
    ],
    process(ctx, w, h, tracking, params) {
      const hands = tracking.hands;
      if (!hands?.length) return;
      const color = params.color ?? '#ff0040';
      const beamWidth = params.width ?? 3;
      for (const hand of hands) {
        const indexTip = hand[8];
        const indexMcp = hand[5];
        if (!indexTip || !indexMcp) continue;
        const [tx, ty] = toPixel(indexTip, w, h);
        const [bx, by] = toPixel(indexMcp, w, h);
        const dx = tx - bx;
        const dy = ty - by;
        const len = Math.sqrt(dx * dx + dy * dy);
        if (len < 1) continue;
        const extend = Math.max(w, h) * 1.5;
        const ex = tx + (dx / len) * extend;
        const ey = ty + (dy / len) * extend;
        ctx.save();
        ctx.strokeStyle = color;
        ctx.lineWidth = beamWidth;
        ctx.shadowColor = color;
        ctx.shadowBlur = 20;
        ctx.lineCap = 'round';
        ctx.beginPath();
        ctx.moveTo(tx, ty);
        ctx.lineTo(ex, ey);
        ctx.stroke();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = beamWidth * 0.3;
        ctx.shadowBlur = 5;
        ctx.beginPath();
        ctx.moveTo(tx, ty);
        ctx.lineTo(ex, ey);
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(tx, ty, beamWidth * 2, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.globalAlpha = 0.4;
        ctx.fill();
        ctx.restore();
      }
    },
  },
  {
    id: 'magic-trail',
    name: 'Magic Trail',
    category: 'hand',
    requires: ['hands'],
    params: [
      { name: 'trailLength', type: 'number', default: 15 },
      { name: 'rainbow', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const hands = tracking.hands;
      if (!hands?.length) return;
      const trailLen = params.trailLength ?? 15;
      const rainbow = params.rainbow;
      const tipIdx = 8;
      for (const hand of hands) {
        if (tipIdx >= hand.length) continue;
        const tip = hand[tipIdx];
        const [px, py] = toPixel(tip, w, h);
        const gradient = ctx.createRadialGradient(px, py, 0, px, py, 20);
        const hue = rainbow ? (tracking.timestamp * 0.1) % 360 : 280;
        const color = `hsl(${hue}, 80%, 60%)`;
        gradient.addColorStop(0, color);
        gradient.addColorStop(1, 'transparent');
        ctx.save();
        ctx.fillStyle = gradient;
        ctx.globalAlpha = 0.6;
        ctx.beginPath();
        ctx.arc(px, py, 20, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
        for (let i = 0; i < 4; i++) {
          const radius = 8 + i * 6;
          const alpha = 0.4 - i * 0.08;
          ctx.save();
          ctx.strokeStyle = color;
          ctx.lineWidth = 2;
          ctx.globalAlpha = Math.max(0, alpha);
          ctx.beginPath();
          ctx.arc(px, py, radius, 0, Math.PI * 2);
          ctx.stroke();
          ctx.restore();
        }
      }
    },
  },
  {
    id: 'skeleton-overlay',
    name: 'Skeleton',
    category: 'body',
    requires: ['pose'],
    params: [
      { name: 'color', type: 'color', default: '#22c55e' },
      { name: 'joints', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const pose = tracking.pose?.[0];
      if (!pose) return;
      const color = params.color ?? '#22c55e';
      const showJoints = params.joints;
      const connections = [
        [11, 12], [11, 13], [13, 15], [12, 14], [14, 16],
        [11, 23], [12, 24], [23, 24],
        [23, 25], [25, 27], [24, 26], [26, 28],
        [27, 29], [29, 31], [28, 30], [30, 32],
        [15, 17], [15, 19], [15, 21], [16, 18], [16, 20], [16, 22],
      ];
      ctx.save();
      ctx.strokeStyle = color;
      ctx.lineWidth = 3;
      ctx.shadowColor = color;
      ctx.shadowBlur = 6;
      ctx.lineCap = 'round';
      for (const [a, b] of connections) {
        if (a >= pose.length || b >= pose.length) continue;
        const [ax, ay] = toPixel(pose[a], w, h);
        const [bx, by] = toPixel(pose[b], w, h);
        ctx.beginPath();
        ctx.moveTo(ax, ay);
        ctx.lineTo(bx, by);
        ctx.stroke();
      }
      if (showJoints) {
        ctx.fillStyle = color;
        for (let i = 0; i < pose.length; i++) {
          if (i > 32) break;
          const [px, py] = toPixel(pose[i], w, h);
          ctx.beginPath();
          ctx.arc(px, py, 5, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.restore();
    },
  },
  {
    id: 'energy-aura',
    name: 'Energy Aura',
    category: 'body',
    requires: ['pose'],
    params: [
      { name: 'color', type: 'color', default: '#f59e0b' },
      { name: 'pulse', type: 'number', default: 1.5 },
    ],
    process(ctx, w, h, tracking, params) {
      const pose = tracking.pose?.[0];
      if (!pose) return;
      const color = params.color ?? '#f59e0b';
      const pulseSpeed = params.pulse ?? 1.5;
      const shoulders = [pose[11], pose[12]].filter(Boolean);
      const hips = [pose[23], pose[24]].filter(Boolean);
      if (!shoulders.length || !hips.length) return;
      const allPoints = [...shoulders, ...hips];
      let cx = 0, cy = 0;
      for (const p of allPoints) {
        const [px, py] = toPixel(p, w, h);
        cx += px;
        cy += py;
      }
      cx /= allPoints.length;
      cy /= allPoints.length;
      const radius = Math.max(w, h) * 0.35;
      const pulse = 1 + Math.sin(tracking.timestamp * 0.001 * pulseSpeed) * 0.15;
      ctx.save();
      const gradient = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius * pulse);
      gradient.addColorStop(0, 'transparent');
      gradient.addColorStop(0.5, color + '20');
      gradient.addColorStop(0.8, color + '40');
      gradient.addColorStop(1, 'transparent');
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    },
  },
  {
    id: 'gesture-trigger',
    name: 'Gesture Spark',
    category: 'gesture',
    requires: ['hands'],
    params: [
      { name: 'gesture', type: 'select', default: 'peace' },
      { name: 'sparkColor', type: 'color', default: '#06b6d4' },
    ],
    process(ctx, w, h, tracking, params) {
      const hands = tracking.hands;
      if (!hands?.length) return;
      const gesture = params.gesture ?? 'peace';
      const sparkColor = params.sparkColor ?? '#06b6d4';
      for (const hand of hands) {
        let detected = false;
        if (gesture === 'peace') {
          const indexExtended = hand[8].y < hand[6].y;
          const middleExtended = hand[12].y < hand[10].y;
          const ringFolded = hand[16].y > hand[14].y;
          const pinkyFolded = hand[20].y > hand[18].y;
          detected = indexExtended && middleExtended && ringFolded && pinkyFolded;
        } else if (gesture === 'thumbsUp') {
          const thumbUp = hand[4].y < hand[3].y && hand[4].y < hand[2].y;
          const fingersFolded = hand[8].y > hand[6].y;
          detected = thumbUp && fingersFolded;
        } else if (gesture === 'fist') {
          detected = hand[8].y > hand[6].y && hand[12].y > hand[10].y && hand[16].y > hand[14].y;
        }
        if (detected) {
          const center = hand[9];
          const [cx, cy] = toPixel(center, w, h);
          ctx.save();
          const time = tracking.timestamp * 0.005;
          for (let i = 0; i < 12; i++) {
            const angle = (i / 12) * Math.PI * 2 + time;
            const dist = 20 + Math.sin(time + i) * 10;
            const sx = cx + Math.cos(angle) * dist;
            const sy = cy + Math.sin(angle) * dist;
            ctx.beginPath();
            ctx.arc(sx, sy, 3, 0, Math.PI * 2);
            ctx.fillStyle = sparkColor;
            ctx.shadowColor = sparkColor;
            ctx.shadowBlur = 10;
            ctx.fill();
          }
          ctx.restore();
        }
      }
    },
  },
  {
    id: 'mirror-face',
    name: 'Mirror',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'strength', type: 'number', default: 0.8 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const strength = params.strength ?? 0.8;
      const bbox = faceBBox(face, w, h);
      const nose = face[4];
      if (!nose) return;
      const [nx, ny] = toPixel(nose, w, h);
      const radius = bbox.w * 0.6;
      ctx.save();
      ctx.beginPath();
      ctx.arc(nx, ny, radius, 0, Math.PI * 2);
      ctx.clip();
      const imgData = ctx.getImageData(
        Math.max(0, Math.floor(nx - radius)),
        Math.max(0, Math.floor(ny - radius)),
        Math.min(w, Math.ceil(radius * 2)),
        Math.min(h, Math.ceil(radius * 2))
      );
      const tempCanvas = document.createElement('canvas');
      tempCanvas.width = imgData.width;
      tempCanvas.height = imgData.height;
      const tempCtx = tempCanvas.getContext('2d');
      if (!tempCtx) { ctx.restore(); return; }
      tempCtx.putImageData(imgData, 0, 0);
      const scale = 1 + strength * 0.3;
      ctx.globalAlpha = 0.5;
      ctx.drawImage(tempCanvas, nx - radius, ny - radius, radius * 2, radius * 2);
      ctx.globalAlpha = 1;
      ctx.restore();
    },
  },
  {
    id: 'glitch-face',
    name: 'Glitch',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'intensity', type: 'number', default: 8 },
      { name: 'scanlines', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const intensity = params.intensity ?? 8;
      const showScanlines = params.scanlines;
      const bbox = faceBBox(face, w, h);
      ctx.save();
      const sliceCount = 5 + Math.floor(Math.random() * 5);
      for (let i = 0; i < sliceCount; i++) {
        const sliceY = bbox.y + Math.random() * bbox.h;
        const sliceH = 2 + Math.random() * 8;
        const offset = (Math.random() - 0.5) * intensity * 3;
        ctx.drawImage(
          ctx.canvas,
          bbox.x, sliceY, bbox.w, sliceH,
          bbox.x + offset, sliceY, bbox.w, sliceH
        );
      }
      const shift = Math.floor(intensity * 0.5);
      ctx.globalCompositeOperation = 'screen';
      ctx.globalAlpha = 0.15;
      ctx.drawImage(ctx.canvas, shift, 0);
      ctx.globalCompositeOperation = 'source-over';
      ctx.globalAlpha = 1;
      if (showScanlines) {
        ctx.globalAlpha = 0.08;
        for (let y = 0; y < h; y += 3) {
          ctx.fillStyle = y % 6 === 0 ? '#fff' : '#000';
          ctx.fillRect(0, y, w, 1);
        }
        ctx.globalAlpha = 1;
      }
      ctx.restore();
    },
  },
  {
    id: 'color-grade',
    name: 'Color Grade',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'warmth', type: 'number', default: 15 },
      { name: 'contrast', type: 'number', default: 20 },
      { name: 'saturation', type: 'number', default: 15 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face) return;
      const warmth = params.warmth ?? 15;
      const contrast = params.contrast ?? 20;
      const saturation = params.saturation ?? 15;
      const bbox = faceBBox(face, w, h);
      const pad = bbox.w * 0.2;
      const x = Math.max(0, bbox.x - pad);
      const y = Math.max(0, bbox.y - pad);
      const bw = Math.min(w - x, bbox.w + pad * 2);
      const bh = Math.min(h - y, bbox.h + pad * 2);
      ctx.save();
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, 16);
      ctx.clip();
      const warmShift = warmth * 0.5;
      ctx.globalCompositeOperation = 'color';
      ctx.fillStyle = `rgba(${128 + warmShift}, 100, ${128 - warmShift}, 0.15)`;
      ctx.fillRect(x, y, bw, bh);
      ctx.globalCompositeOperation = 'overlay';
      const contrastAlpha = Math.abs(contrast) / 200;
      ctx.fillStyle = `rgba(128,128,128,${contrastAlpha})`;
      ctx.fillRect(x, y, bw, bh);
      ctx.globalCompositeOperation = 'source-over';
      if (saturation < 0) {
        ctx.globalAlpha = Math.abs(saturation) / 100;
        ctx.fillStyle = 'rgba(128,128,128,1)';
        ctx.filter = 'grayscale(1)';
        ctx.fillRect(x, y, bw, bh);
        ctx.filter = 'none';
      } else if (saturation > 0) {
        ctx.globalAlpha = saturation / 200;
        ctx.fillStyle = `hsl(${(tracking.timestamp * 0.02) % 360}, 60%, 50%)`;
        ctx.globalCompositeOperation = 'soft-light';
        ctx.fillRect(x, y, bw, bh);
      }
      ctx.restore();
    },
  },
  {
    id: 'crown-halo',
    name: 'Crown / Halo',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'style', type: 'select', default: 'crown' },
      { name: 'color', type: 'color', default: '#fbbf24' },
      { name: 'float', type: 'number', default: 0.3 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face || face.length < 468) return;
      const style = params.style ?? 'crown';
      const color = params.color ?? '#fbbf24';
      const floatHeight = params.float ?? 0.3;
      const bbox = faceBBox(face, w, h);
      const forehead = face[10];
      if (!forehead) return;
      const [fx, fy] = toPixel(forehead, w, h);
      const crownY = fy - bbox.h * floatHeight;
      const crownW = bbox.w * 0.7;
      const bobble = Math.sin(tracking.timestamp * 0.003) * 4;
      ctx.save();
      if (style === 'crown') {
        const cy = crownY + bobble;
        ctx.fillStyle = color;
        ctx.shadowColor = color;
        ctx.shadowBlur = 12;
        ctx.beginPath();
        ctx.moveTo(fx - crownW / 2, cy + bbox.h * 0.06);
        ctx.lineTo(fx - crownW / 2, cy - bbox.h * 0.02);
        ctx.lineTo(fx - crownW * 0.25, cy - bbox.h * 0.06);
        ctx.lineTo(fx, cy - bbox.h * 0.1);
        ctx.lineTo(fx + crownW * 0.25, cy - bbox.h * 0.06);
        ctx.lineTo(fx + crownW / 2, cy - bbox.h * 0.02);
        ctx.lineTo(fx + crownW / 2, cy + bbox.h * 0.06);
        ctx.closePath();
        ctx.fill();
        ctx.fillStyle = '#ef4444';
        ctx.beginPath();
        ctx.arc(fx, cy - bbox.h * 0.06, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#3b82f6';
        ctx.beginPath();
        ctx.arc(fx - crownW * 0.25, cy - bbox.h * 0.02, 2.5, 0, Math.PI * 2);
        ctx.fill();
        ctx.beginPath();
        ctx.arc(fx + crownW * 0.25, cy - bbox.h * 0.02, 2.5, 0, Math.PI * 2);
        ctx.fill();
      } else {
        const cy = crownY + bobble;
        const haloW = crownW * 0.8;
        const haloH = bbox.h * 0.04;
        ctx.strokeStyle = color;
        ctx.lineWidth = 3;
        ctx.shadowColor = color;
        ctx.shadowBlur = 20;
        ctx.globalAlpha = 0.85;
        ctx.beginPath();
        ctx.ellipse(fx, cy, haloW / 2, haloH, 0, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalAlpha = 0.3;
        ctx.lineWidth = 6;
        ctx.beginPath();
        ctx.ellipse(fx, cy, haloW / 2, haloH, 0, 0, Math.PI * 2);
        ctx.stroke();
      }
      ctx.restore();
    },
  },
  {
    id: 'pixelate-face',
    name: 'Pixelate Face',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'blockSize', type: 'number', default: 12 },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face) return;
      const blockSize = params.blockSize ?? 12;
      const bbox = faceBBox(face, w, h);
      const pad = bbox.w * 0.1;
      const x = Math.max(0, Math.floor(bbox.x - pad));
      const y = Math.max(0, Math.floor(bbox.y - pad));
      const bw = Math.min(w - x, Math.ceil(bbox.w + pad * 2));
      const bh = Math.min(h - y, Math.ceil(bbox.h + pad * 2));
      if (bw <= 0 || bh <= 0) return;
      ctx.save();
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, 12);
      ctx.clip();
      const imgData = ctx.getImageData(x, y, bw, bh);
      const data = imgData?.data;
      if (!data) { ctx.restore(); return; }
      for (let py = 0; py < bh; py += blockSize) {
        for (let px = 0; px < bw; px += blockSize) {
          const sw = Math.min(blockSize, bw - px);
          const sh = Math.min(blockSize, bh - py);
          if (sw <= 0 || sh <= 0) continue;
          const sampleX = Math.min(px + Math.floor(sw / 2), bw - 1);
          const sampleY = Math.min(py + Math.floor(sh / 2), bh - 1);
          const idx = (sampleY * bw + sampleX) * 4;
          const r = data[idx] ?? 128;
          const g = data[idx + 1] ?? 128;
          const b = data[idx + 2] ?? 128;
          ctx.fillStyle = `rgb(${r},${g},${b})`;
          ctx.fillRect(x + px, y + py, sw, sh);
        }
      }
      ctx.restore();
    },
  },
  {
    id: 'rainbow-trail',
    name: 'Rainbow Trail',
    category: 'hand',
    requires: ['hands'],
    params: [
      { name: 'thickness', type: 'number', default: 6 },
      { name: 'arcLength', type: 'number', default: 1 },
    ],
    process(ctx, w, h, tracking, params) {
      const hands = tracking.hands;
      if (!hands?.length) return;
      const thickness = params.thickness ?? 6;
      const arcLen = params.arcLength ?? 1;
      const tips = [4, 8, 12, 16, 20];
      for (const hand of hands) {
        const wrist = hand[0];
        if (!wrist) continue;
        const [wx, wy] = toPixel(wrist, w, h);
        for (let i = 0; i < tips.length; i++) {
          const tipIdx = tips[i];
          if (tipIdx >= hand.length) continue;
          const tip = hand[tipIdx];
          const [tx, ty] = toPixel(tip, w, h);
          const midX = (wx + tx) / 2;
          const midY = (wy + ty) / 2;
          const dx = tx - wx;
          const dy = ty - wy;
          const cpX = midX + (-dy * 0.3 * arcLen);
          const cpY = midY + (dx * 0.3 * arcLen);
          const time = tracking.timestamp * 0.002;
          const hue = (i * 72 + time * 50) % 360;
          ctx.save();
          ctx.strokeStyle = `hsl(${hue}, 85%, 55%)`;
          ctx.lineWidth = thickness - i * 0.5;
          ctx.lineCap = 'round';
          ctx.shadowColor = `hsl(${hue}, 85%, 55%)`;
          ctx.shadowBlur = 8;
          ctx.globalAlpha = 0.8;
          ctx.beginPath();
          ctx.moveTo(wx, wy);
          ctx.quadraticCurveTo(cpX, cpY, tx, ty);
          ctx.stroke();
          ctx.restore();
        }
      }
    },
  },
  {
    id: 'force-field',
    name: 'Force Field',
    category: 'body',
    requires: ['pose'],
    params: [
      { name: 'color', type: 'color', default: '#6366f1' },
      { name: 'pulse', type: 'number', default: 1.5 },
      { name: 'opacity', type: 'number', default: 0.3 },
    ],
    process(ctx, w, h, tracking, params) {
      const pose = tracking.pose?.[0];
      if (!pose) return;
      const color = params.color ?? '#6366f1';
      const pulseSpeed = params.pulse ?? 1.5;
      const opacity = params.opacity ?? 0.3;
      const keyPoints = [11, 12, 23, 24, 15, 16, 27, 28];
      const points = [];
      for (const idx of keyPoints) {
        if (idx < pose.length && pose[idx]) {
          points.push(toPixel(pose[idx], w, h));
        }
      }
      if (points.length < 4) return;
      let cx = 0, cy = 0;
      for (const [px, py] of points) { cx += px; cy += py; }
      cx /= points.length;
      cy /= points.length;
      let maxDist = 0;
      for (const [px, py] of points) {
        const d = Math.sqrt((px - cx) ** 2 + (py - cy) ** 2);
        if (d > maxDist) maxDist = d;
      }
      const radius = maxDist * 1.2;
      const time = tracking.timestamp * 0.001 * pulseSpeed;
      const pulse = 1 + Math.sin(time) * 0.05;
      ctx.save();
      const outerGrad = ctx.createRadialGradient(cx, cy, radius * 0.7, cx, cy, radius * pulse);
      outerGrad.addColorStop(0, 'transparent');
      outerGrad.addColorStop(0.8, color + '10');
      outerGrad.addColorStop(0.95, color + '40');
      outerGrad.addColorStop(1, 'transparent');
      ctx.fillStyle = outerGrad;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = opacity;
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.shadowColor = color;
      ctx.shadowBlur = 15;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = opacity * 0.5;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * pulse * 0.85, 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = opacity * 0.4;
      ctx.setLineDash([8, 12]);
      ctx.lineDashOffset = -time * 30;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * pulse * 0.92, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.restore();
    },
  },
  {
    id: 'disco-mode',
    name: 'Disco Mode',
    category: 'face',
    requires: ['face'],
    params: [
      { name: 'speed', type: 'number', default: 2 },
      { name: 'opacity', type: 'number', default: 0.2 },
      { name: 'spots', type: 'boolean', default: true },
    ],
    process(ctx, w, h, tracking, params) {
      const face = tracking.face?.[0];
      if (!face) return;
      const speed = params.speed ?? 2;
      const opacity = params.opacity ?? 0.2;
      const showSpots = params.spots;
      const bbox = faceBBox(face, w, h);
      const pad = bbox.w * 0.15;
      const x = Math.max(0, bbox.x - pad);
      const y = Math.max(0, bbox.y - pad);
      const bw = Math.min(w - x, bbox.w + pad * 2);
      const bh = Math.min(h - y, bbox.h + pad * 2);
      const time = tracking.timestamp * 0.001 * speed;
      const hue1 = (time * 60) % 360;
      const hue2 = (hue1 + 120) % 360;
      const pulse = 0.5 + Math.sin(time * 2) * 0.5;
      ctx.save();
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, 16);
      ctx.clip();
      ctx.globalAlpha = opacity * pulse;
      ctx.globalCompositeOperation = 'color';
      const grad = ctx.createLinearGradient(x, y, x + bw, y + bh);
      grad.addColorStop(0, `hsl(${hue1}, 80%, 50%)`);
      grad.addColorStop(1, `hsl(${hue2}, 80%, 50%)`);
      ctx.fillStyle = grad;
      ctx.fillRect(x, y, bw, bh);
      if (showSpots) {
        ctx.globalCompositeOperation = 'screen';
        const spotCount = 6;
        for (let i = 0; i < spotCount; i++) {
          const spotHue = (hue1 + i * 60) % 360;
          const sx = x + (bw * (0.2 + 0.6 * ((Math.sin(time + i * 1.3) + 1) / 2)));
          const sy = y + (bh * (0.2 + 0.6 * ((Math.cos(time * 0.7 + i * 1.7) + 1) / 2)));
          const spotR = 8 + Math.sin(time + i) * 4;
          ctx.globalAlpha = 0.15 + pulse * 0.1;
          const spotGrad = ctx.createRadialGradient(sx, sy, 0, sx, sy, spotR);
          spotGrad.addColorStop(0, `hsl(${spotHue}, 90%, 60%)`);
          spotGrad.addColorStop(1, 'transparent');
          ctx.fillStyle = spotGrad;
          ctx.beginPath();
          ctx.arc(sx, sy, spotR, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.restore();
    },
  },
  {
    id: 'color-overlay',
    name: 'Color Overlay',
    category: 'face',
    requires: [],
    params: [
      { name: 'color', type: 'color', default: '#7c3aed' },
      { name: 'opacity', type: 'number', default: 0.2 },
    ],
    process(ctx, w, h, _tracking, params) {
      const color = params.color ?? '#7c3aed';
      const opacity = params.opacity ?? 0.2;
      ctx.save();
      ctx.globalAlpha = opacity;
      ctx.globalCompositeOperation = 'color';
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, w, h);
      ctx.restore();
    },
  },
  {
    id: 'vignette',
    name: 'Vignette',
    category: 'face',
    requires: [],
    params: [
      { name: 'intensity', type: 'number', default: 0.6 },
    ],
    process(ctx, w, h, _tracking, params) {
      const intensity = params.intensity ?? 0.6;
      const cx = w / 2;
      const cy = h / 2;
      const radius = Math.max(w, h) * 0.7;
      ctx.save();
      const gradient = ctx.createRadialGradient(cx, cy, radius * 0.3, cx, cy, radius);
      gradient.addColorStop(0, 'transparent');
      gradient.addColorStop(1, `rgba(0,0,0,${intensity})`);
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, w, h);
      ctx.restore();
    },
  },
  {
    id: 'warm-tone',
    name: 'Warm Tone',
    category: 'face',
    requires: [],
    params: [
      { name: 'intensity', type: 'number', default: 0.15 },
    ],
    process(ctx, w, h, _tracking, params) {
      const intensity = params.intensity ?? 0.15;
      ctx.save();
      ctx.globalCompositeOperation = 'overlay';
      ctx.fillStyle = `rgba(255,140,50,${intensity})`;
      ctx.fillRect(0, 0, w, h);
      ctx.restore();
    },
  },
  {
    id: 'cool-tone',
    name: 'Cool Tone',
    category: 'face',
    requires: [],
    params: [
      { name: 'intensity', type: 'number', default: 0.15 },
    ],
    process(ctx, w, h, _tracking, params) {
      const intensity = params.intensity ?? 0.15;
      ctx.save();
      ctx.globalCompositeOperation = 'overlay';
      ctx.fillStyle = `rgba(50,120,255,${intensity})`;
      ctx.fillRect(0, 0, w, h);
      ctx.restore();
    },
  },
  {
    id: 'film-grain',
    name: 'Film Grain',
    category: 'face',
    requires: [],
    params: [
      { name: 'intensity', type: 'number', default: 0.06 },
    ],
    process(ctx, w, h, _tracking, params) {
      const intensity = params.intensity ?? 0.06;
      const imageData = ctx.getImageData(0, 0, w, h);
      const data = imageData.data;
      for (let i = 0; i < data.length; i += 4) {
        const noise = (Math.random() - 0.5) * 255 * intensity;
        data[i] = Math.min(255, Math.max(0, data[i] + noise));
        data[i + 1] = Math.min(255, Math.max(0, data[i + 1] + noise));
        data[i + 2] = Math.min(255, Math.max(0, data[i + 2] + noise));
      }
      ctx.putImageData(imageData, 0, 0);
    },
  },
  {
    id: 'custom',
    name: 'Custom',
    category: 'custom',
    requires: ['face'],
    params: [],
    process() {
      // Custom effects are rendered by the plugin engine
    },
  },
];

// ── Tracking data map ────────────────────────────────────────────────────────

const trackingMap = {
  face: faceTracking,
  hand: handsTracking,
  body: poseTracking,
  gesture: handsTracking,
  custom: emptyTracking,
};

// ── Test Runner ──────────────────────────────────────────────────────────────

let passed = 0;
let failed = 0;
const failures = [];

function test(name, fn) {
  try {
    fn();
    passed++;
    console.log(`  \x1b[32m✔\x1b[0m ${name}`);
  } catch (err) {
    failed++;
    failures.push({ name, error: err.message });
    console.log(`  \x1b[31m✖\x1b[0m ${name}`);
    console.log(`    \x1b[31m${err.message}\x1b[0m`);
  }
}

export async function runEffectsTests(reporter) {
  console.log('\n\x1b[1m\x1b[36m▶ Running Effects System Tests...\x1b[0m');

  // ── 1. Effect count ──────────────────────────────────────────────────────
  test('BUILTIN_EFFECTS contains 26 effects', () => {
    assert.equal(effects.length, 26, `Expected 26 effects, got ${effects.length}`);
  });

  // ── 2. Each effect has required fields ───────────────────────────────────
  for (const effect of effects) {
    test(`Effect "${effect.id}" has required fields (id, name, category, requires, params, process)`, () => {
      assert.ok(effect.id, `Missing id`);
      assert.ok(effect.name, `Missing name`);
      assert.ok(effect.category, `Missing category`);
      assert.ok(Array.isArray(effect.requires), `Missing requires`);
      assert.ok(Array.isArray(effect.params), `Missing params`);
      assert.equal(typeof effect.process, 'function', `Missing process function`);
    });
  }

  // ── 3. process() does not throw with mock context ────────────────────────
  for (const effect of effects) {
    const tracking = trackingMap[effect.category] ?? emptyTracking;
    const defaultParams = {};
    for (const p of effect.params) {
      defaultParams[p.name] = p.default;
    }

    test(`Effect "${effect.id}" process() does not throw with default params`, () => {
      const ctx = createMockCtx();
      assert.doesNotThrow(() => {
        effect.process(ctx, W, H, tracking, defaultParams, TIMESTAMP);
      }, `process() threw for effect "${effect.id}"`);
    });
  }

  // ── 4. process() does not throw with empty tracking (graceful no-op) ─────
  for (const effect of effects) {
    test(`Effect "${effect.id}" process() handles empty tracking gracefully`, () => {
      const ctx = createMockCtx();
      const defaultParams = {};
      for (const p of effect.params) {
        defaultParams[p.name] = p.default;
      }
      assert.doesNotThrow(() => {
        effect.process(ctx, W, H, emptyTracking, defaultParams, TIMESTAMP);
      }, `process() threw with empty tracking for "${effect.id}"`);
    });
  }

  // ── 5. process() does not throw with empty params ────────────────────────
  for (const effect of effects) {
    const tracking = trackingMap[effect.category] ?? emptyTracking;
    test(`Effect "${effect.id}" process() handles empty params gracefully`, () => {
      const ctx = createMockCtx();
      assert.doesNotThrow(() => {
        effect.process(ctx, W, H, tracking, {}, TIMESTAMP);
      }, `process() threw with empty params for "${effect.id}"`);
    });
  }

  // ── 6. Effect categories are valid ───────────────────────────────────────
  test('All effects have valid categories', () => {
    const validCategories = new Set(['face', 'hand', 'body', 'gesture', 'custom']);
    for (const effect of effects) {
      assert.ok(validCategories.has(effect.category), `Invalid category "${effect.category}" for "${effect.id}"`);
    }
  });

  // ── 7. Effect requires are valid ─────────────────────────────────────────
  test('All effects have valid requires', () => {
    const validRequires = new Set(['face', 'hands', 'pose']);
    for (const effect of effects) {
      for (const req of effect.requires) {
        assert.ok(validRequires.has(req), `Invalid require "${req}" for "${effect.id}"`);
      }
    }
  });

  // ── 8. Custom effect is a no-op ──────────────────────────────────────────
  test('Custom effect process() is a no-op', () => {
    const ctx = createMockCtx();
    const custom = effects.find(e => e.id === 'custom');
    assert.ok(custom, 'Custom effect not found');
    assert.doesNotThrow(() => {
      custom.process(ctx, W, H, emptyTracking, {}, TIMESTAMP);
    });
  });

  // ── 9. Effects can run in rapid succession (FPS simulation) ──────────────
  test('All effects survive 10 rapid iterations (FPS simulation)', () => {
    const ctx = createMockCtx();
    for (const effect of effects) {
      const tracking = trackingMap[effect.category] ?? emptyTracking;
      const defaultParams = {};
      for (const p of effect.params) {
        defaultParams[p.name] = p.default;
      }
      for (let i = 0; i < 10; i++) {
        effect.process(ctx, W, H, tracking, defaultParams, TIMESTAMP + i * 33);
      }
    }
  });

  // ── Summary ──────────────────────────────────────────────────────────────
  const total = passed + failed;
  console.log(`\n  Effects Tests: \x1b[1m${passed}/${total}\x1b[0m passed`);
  if (failures.length > 0) {
    console.log('\n  Failures:');
    for (const f of failures) {
      console.log(`    \x1b[31m- ${f.name}: ${f.error}\x1b[0m`);
    }
  }

  return { passed, failed, failures };
}
