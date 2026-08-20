/**
 * Built-in effects — 26 pre-built effects using Canvas 2D + MediaPipe landmarks
 *
 * Each effect implements the Effect interface:
 *   process(ctx, width, height, tracking, params, time)
 *
 * Privacy: No pixel data leaves the browser. Only landmark coordinates are used.
 * Performance: Effects run at 30+ FPS on modern hardware.
 */

import type { Effect, EffectParams, TrackingResult, Landmark, EffectCategory } from './types';
import { GestureDetector, type GestureType } from './gesture';

function toPixel(lm: Landmark, w: number, h: number): [number, number] {
  return [lm.x * w, lm.y * h];
}

function faceBBox(face: Landmark[], w: number, h: number): { x: number; y: number; w: number; h: number; cx: number; cy: number } {
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

const softBlurFace: Effect = {
  id: 'soft-blur-face',
  name: 'Soft Blur',
  nameTr: 'Yüz Bulanıklığı',
  description: 'Gaussian blur over face region for privacy',
  descriptionTr: 'Gizlilik için yüz bölgesi üzerine gaussian bulanıklığı',
  category: 'face',
  difficulty: 'easy',
  icon: 'eye',
  thumbnail: 'linear-gradient(135deg, #38bdf8, #818cf8, #a855f7)',
  requires: ['face'],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 1, max: 20, step: 1, default: 8 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const intensity = (params.intensity as number) ?? 8;
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
};

// ── Effect 2: Neon Outline ──────────────────────────────────────────────────

const neonOutline: Effect = {
  id: 'neon-outline',
  name: 'Neon Outline',
  nameTr: 'Neon Çizgi',
  description: 'Glowing neon lines along face landmarks',
  descriptionTr: 'Yüz landmark\'ları boyunca parlayan neon çizgiler',
  category: 'face',
  difficulty: 'easy',
  icon: 'sparkle',
  thumbnail: 'linear-gradient(135deg, #00ff88, #00ffcc)',
  requires: ['face'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#00ff88' },
    { name: 'thickness', label: 'Kalınlık', type: 'number', min: 1, max: 6, step: 0.5, default: 2 },
    { name: 'glow', label: 'Parıltı', type: 'number', min: 0, max: 30, step: 1, default: 12 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const color = (params.color as string) ?? '#00ff88';
    const thickness = (params.thickness as number) ?? 2;
    const glow = (params.glow as number) ?? 12;

    // Draw key face contour lines
    const contourIndices = [
      // Jawline
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

    // Draw eye outlines
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

    // Lips outline
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
};

// ── Effect 3: Anime Eyes ────────────────────────────────────────────────────

const animeEyes: Effect = {
  id: 'anime-eyes',
  name: 'Anime Eyes',
  nameTr: 'Anime Gözleri',
  description: 'Oversized anime-style eyes with sparkle',
  descriptionTr: 'Büyütülmüş anime tarzı gözler ve ışıltı',
  category: 'face',
  difficulty: 'medium',
  icon: 'sparkles',
  thumbnail: 'linear-gradient(135deg, #ff6b9d, #c44dff)',
  requires: ['face'],
  params: [
    { name: 'scale', label: 'Boyut', type: 'number', min: 0.5, max: 3, step: 0.1, default: 1.5 },
    { name: 'sparkle', label: 'Işıltı', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const scale = (params.scale as number) ?? 1.5;
    const sparkle = params.sparkle as boolean;

    // Left eye center (landmark 159) and right eye center (landmark 386)
    const leftEyeCenter = face[159];
    const rightEyeCenter = face[386];
    if (!leftEyeCenter || !rightEyeCenter) return;

    const bbox = faceBBox(face, w, h);
    const eyeRadius = bbox.w * 0.12 * scale;

    const drawAnimeEye = (cx: number, cy: number) => {
      ctx.save();
      ctx.translate(cx, cy);

      // Outer eye (white)
      ctx.beginPath();
      ctx.ellipse(0, 0, eyeRadius, eyeRadius * 1.2, 0, 0, Math.PI * 2);
      ctx.fillStyle = '#fff';
      ctx.fill();
      ctx.strokeStyle = '#222';
      ctx.lineWidth = 2;
      ctx.stroke();

      // Iris
      const irisR = eyeRadius * 0.65;
      const gradient = ctx.createRadialGradient(0, 0, 0, 0, 0, irisR);
      gradient.addColorStop(0, '#c44dff');
      gradient.addColorStop(0.5, '#8b5cf6');
      gradient.addColorStop(1, '#4c1d95');
      ctx.beginPath();
      ctx.arc(0, 0, irisR, 0, Math.PI * 2);
      ctx.fillStyle = gradient;
      ctx.fill();

      // Pupil
      ctx.beginPath();
      ctx.arc(0, 0, irisR * 0.35, 0, Math.PI * 2);
      ctx.fillStyle = '#000';
      ctx.fill();

      // Highlight / sparkle
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

      // Upper lash line
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
};

// ── Effect 4: Cat Ears + Whiskers ───────────────────────────────────────────

const catEars: Effect = {
  id: 'cat-ears',
  name: 'Cat Ears',
  nameTr: 'Kedi Kulakları',
  description: 'Cat ears and whiskers that follow head pose',
  descriptionTr: 'Kafa pozunu takip eden kedi kulakları ve bıyıklar',
  category: 'face',
  difficulty: 'medium',
  icon: 'smile',
  thumbnail: 'linear-gradient(135deg, #ff9a56, #ff6b35)',
  requires: ['face'],
  params: [
    { name: 'earColor', label: 'Kulak Rengi', type: 'color', default: '#ff9a56' },
    { name: 'whiskers', label: 'Bıyık', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const earColor = (params.earColor as string) ?? '#ff9a56';
    const showWhiskers = params.whiskers as boolean;

    const bbox = faceBBox(face, w, h);
    const earSize = bbox.w * 0.25;

    // Head tilt from forehead to chin
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
    // Inner ear
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
      const nose = face[4]; // nose tip
      if (!nose) return;
      const [nx, ny] = toPixel(nose, w, h);
      const whiskerLen = bbox.w * 0.4;

      ctx.save();
      ctx.strokeStyle = 'rgba(255,255,255,0.7)';
      ctx.lineWidth = 1.5;
      ctx.lineCap = 'round';

      // Left whiskers
      for (const angle of [-0.15, 0, 0.15]) {
        ctx.beginPath();
        ctx.moveTo(nx - bbox.w * 0.05, ny + bbox.h * 0.02);
        ctx.lineTo(nx - whiskerLen, ny + angle * whiskerLen + bbox.h * 0.02);
        ctx.stroke();
      }
      // Right whiskers
      for (const angle of [-0.15, 0, 0.15]) {
        ctx.beginPath();
        ctx.moveTo(nx + bbox.w * 0.05, ny + bbox.h * 0.02);
        ctx.lineTo(nx + whiskerLen, ny + angle * whiskerLen + bbox.h * 0.02);
        ctx.stroke();
      }
      ctx.restore();
    }
  },
};

// ── Effect 5: Sunglasses ────────────────────────────────────────────────────

const sunglasses: Effect = {
  id: 'sunglasses',
  name: 'Sunglasses',
  nameTr: 'Güneş Gözlüğü',
  description: 'Stylish sunglasses that track eye position',
  descriptionTr: 'Göz pozisyonunu takip eden şık güneş gözlüğü',
  category: 'face',
  difficulty: 'easy',
  icon: 'glasses',
  thumbnail: 'linear-gradient(135deg, #1e1b4b, #3b0764, #4338ca)',
  requires: ['face'],
  params: [
    { name: 'tint', label: 'Renk Tonu', type: 'color', default: '#1a1a2e' },
    { name: 'opacity', label: 'Saydamlık', type: 'number', min: 0.3, max: 1, step: 0.05, default: 0.85 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const tint = (params.tint as string) ?? '#1a1a2e';
    const opacity = (params.opacity as number) ?? 0.85;

    const leftEye = face[33]; // left eye inner corner
    const rightEye = face[263]; // right eye outer corner
    const leftTemple = face[127]; // left temple
    const rightTemple = face[356]; // right temple
    if (!leftEye || !rightEye || !leftTemple || !rightTemple) return;

    const [lex, ley] = toPixel(leftEye, w, h);
    const [rex, rey] = toPixel(rightEye, w, h);
    const [ltx, lty] = toPixel(leftTemple, w, h);
    const [rtx, rty] = toPixel(rightTemple, w, h);

    const eyeWidth = Math.abs(rex - lex) * 0.55;
    const eyeHeight = eyeWidth * 0.45;
    const bridge = (rex - lex) * 0.12;

    ctx.save();
    ctx.globalAlpha = opacity;

    // Bridge
    ctx.beginPath();
    ctx.moveTo(lex + eyeWidth * 0.5, ley);
    ctx.quadraticCurveTo((lex + rex) / 2, ley - eyeHeight * 0.3, rex - eyeWidth * 0.5, rey);
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 3;
    ctx.stroke();

    // Left lens
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

    // Right lens
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

    // Temples (arms)
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

    // Lens reflection
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
};

// ── Effect 6: Face Paint / Mask ─────────────────────────────────────────────

const facePaint: Effect = {
  id: 'face-paint',
  name: 'Face Paint',
  nameTr: 'Yüz Boyası',
  description: 'Colorful face paint that sticks to landmarks',
  descriptionTr: 'Landmark\'lara yapışan renkli yüz boyası',
  category: 'face',
  difficulty: 'medium',
  icon: 'mask',
  thumbnail: 'linear-gradient(135deg, #ff6b6b, #ffd93d, #6bcb77, #4d96ff)',
  requires: ['face'],
  params: [
    { name: 'style', label: 'Stil', type: 'select', options: ['warrior', 'butterfly', 'hearts'], default: 'warrior' },
    { name: 'color1', label: 'Renk 1', type: 'color', default: '#ff6b6b' },
    { name: 'color2', label: 'Renk 2', type: 'color', default: '#4d96ff' },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const style = (params.style as string) ?? 'warrior';
    const c1 = (params.color1 as string) ?? '#ff6b6b';
    const c2 = (params.color2 as string) ?? '#4d96ff';
    const bbox = faceBBox(face, w, h);

    ctx.save();
    ctx.globalAlpha = 0.6;

    if (style === 'warrior') {
      // Horizontal stripes across cheeks
      const cheekL = face[50]; // left cheek
      const cheekR = face[280]; // right cheek
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

          ctx.strokeStyle = i % 2 === 0 ? c1 : c2;
          ctx.beginPath();
          ctx.moveTo(crx - bbox.w * 0.15, cry + offset);
          ctx.lineTo(crx + bbox.w * 0.15, cry + offset);
          ctx.stroke();
        }
      }
    } else if (style === 'butterfly') {
      // Butterfly wings on cheeks
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
      // Heart shapes on cheeks
      const cheekL = face[50];
      const cheekR = face[280];
      const drawHeart = (cx: number, cy: number, size: number) => {
        ctx.beginPath();
        ctx.moveTo(cx, cy + size * 0.3);
        ctx.bezierCurveTo(cx - size * 0.5, cy - size * 0.3, cx - size, cy + size * 0.1, cx, cy + size);
        ctx.bezierCurveTo(cx + size, cy + size * 0.1, cx + size * 0.5, cy - size * 0.3, cx, cy + size * 0.3);
        ctx.fillStyle = c1;
        ctx.fill();
      };
      if (cheekL && cheekR) {
        const [lx, ly] = toPixel(cheekL, w, h);
        const [rx, ry] = toPixel(cheekR, w, h);
        drawHeart(lx, ly - bbox.h * 0.05, bbox.h * 0.08);
        drawHeart(rx, ry - bbox.h * 0.05, bbox.h * 0.08);
      }
    }

    ctx.restore();
  },
};

// ── Effect 7: Particle Hands ────────────────────────────────────────────────

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  size: number;
  color: string;
}

const particleBuffers = new Map<string, Particle[]>();

const particleHands: Effect = {
  id: 'particle-hands',
  name: 'Particle Hands',
  nameTr: 'Parçacık Elleri',
  description: 'Sparkle particles from fingertips',
  descriptionTr: 'Parmak uçlarından çıkan parıltı parçacıkları',
  category: 'hand',
  difficulty: 'medium',
  icon: 'hand',
  thumbnail: 'linear-gradient(135deg, #a855f7, #ec4899)',
  requires: ['hands'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#a855f7' },
    { name: 'density', label: 'Yoğunluk', type: 'number', min: 1, max: 10, step: 1, default: 5 },
  ],
  process(ctx, w, h, tracking, params) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const color = (params.color as string) ?? '#a855f7';
    const density = (params.density as number) ?? 5;

    // Persistent particle buffer across frames
    const particles = particleBuffers.get('particle-hands') ?? [];

    for (let i = particles.length - 1; i >= 0; i--) {
      const p = particles[i];
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.02; // slight gravity
      p.life -= 1;
      if (p.life <= 0) particles.splice(i, 1);
    }

    // Fingertip landmarks: 4 (thumb), 8 (index), 12 (middle), 16 (ring), 20 (pinky)
    const tips = [4, 8, 12, 16, 20];

    for (const hand of hands) {
      for (const tipIdx of tips) {
        if (tipIdx >= hand.length) continue;
        const tip = hand[tipIdx];
        const [px, py] = toPixel(tip, w, h);

        for (let i = 0; i < density; i++) {
          const angle = Math.random() * Math.PI * 2;
          const speed = 1 + Math.random() * 3;
          particles.push({
            x: px,
            y: py,
            vx: Math.cos(angle) * speed,
            vy: Math.sin(angle) * speed,
            life: 40 + Math.random() * 30,
            maxLife: 70,
            size: 2 + Math.random() * 4,
            color,
          });
        }
      }
    }

    for (const p of particles) {
      const alpha = Math.max(0, p.life / p.maxLife);
      ctx.save();
      ctx.globalAlpha = alpha;
      ctx.fillStyle = p.color;
      ctx.shadowColor = p.color;
      ctx.shadowBlur = 8;
      ctx.beginPath();
      ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    particleBuffers.set('particle-hands', particles);
  },
};

// ── Effect 8: Laser Fingers ─────────────────────────────────────────────────

const laserFingers: Effect = {
  id: 'laser-fingers',
  name: 'Laser Fingers',
  nameTr: 'Lazer Parmakları',
  description: 'Laser beams from index finger',
  descriptionTr: 'İşaret parmağından çıkan lazer ışınları',
  category: 'hand',
  difficulty: 'easy',
  icon: 'crosshair',
  thumbnail: 'linear-gradient(135deg, #ff0040, #ff4081)',
  requires: ['hands'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#ff0040' },
    { name: 'width', label: 'Genişlik', type: 'number', min: 1, max: 8, step: 0.5, default: 3 },
  ],
  process(ctx, w, h, tracking, params) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const color = (params.color as string) ?? '#ff0040';
    const beamWidth = (params.width as number) ?? 3;

    for (const hand of hands) {
      const indexTip = hand[8]; // index finger tip
      const indexMcp = hand[5]; // index finger base
      if (!indexTip || !indexMcp) continue;

      const [tx, ty] = toPixel(indexTip, w, h);
      const [bx, by] = toPixel(indexMcp, w, h);

      // Direction from base to tip
      const dx = tx - bx;
      const dy = ty - by;
      const len = Math.sqrt(dx * dx + dy * dy);
      if (len < 1) continue;

      // Extend beam in the same direction
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

      // Bright core
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = beamWidth * 0.3;
      ctx.shadowBlur = 5;
      ctx.beginPath();
      ctx.moveTo(tx, ty);
      ctx.lineTo(ex, ey);
      ctx.stroke();

      // Glow at origin
      ctx.beginPath();
      ctx.arc(tx, ty, beamWidth * 2, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.4;
      ctx.fill();

      ctx.restore();
    }
  },
};

// ── Effect 9: Magic Trail ───────────────────────────────────────────────────

const magicTrail: Effect = {
  id: 'magic-trail',
  name: 'Magic Trail',
  nameTr: 'Sihirli İz',
  description: 'Colorful trail following hand movement',
  descriptionTr: 'El hareketiyle renkli iz bırakma',
  category: 'hand',
  difficulty: 'medium',
  icon: 'wand',
  thumbnail: 'linear-gradient(135deg, #f59e0b, #ef4444, #8b5cf6)',
  requires: ['hands'],
  params: [
    { name: 'trailLength', label: 'İz Uzunluğu', type: 'number', min: 5, max: 30, step: 1, default: 15 },
    { name: 'rainbow', label: 'Gökkuşağı', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const rainbow = params.rainbow as boolean;

    const tipIdx = 8; // index fingertip

    for (const hand of hands) {
      if (tipIdx >= hand.length) continue;
      const tip = hand[tipIdx];
      const [px, py] = toPixel(tip, w, h);

      // Draw a soft glow at fingertip
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

      // Draw concentric rings
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
};

// ── Effect 10: Skeleton Overlay ─────────────────────────────────────────────

const skeletonOverlay: Effect = {
  id: 'skeleton-overlay',
  name: 'Skeleton',
  nameTr: 'İskelet',
  description: 'Pose skeleton overlay',
  descriptionTr: 'Vücut iskeleti göstergesi',
  category: 'body',
  difficulty: 'easy',
  icon: 'activity',
  thumbnail: 'linear-gradient(135deg, #22c55e, #16a34a)',
  requires: ['pose'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#22c55e' },
    { name: 'joints', label: 'Eklem Noktaları', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const pose = tracking.pose?.[0];
    if (!pose) return;
    const color = (params.color as string) ?? '#22c55e';
    const showJoints = params.joints as boolean;

    // MediaPipe Pose connections
    const connections: [number, number][] = [
      [11, 12], [11, 13], [13, 15], [12, 14], [14, 16], // Arms
      [11, 23], [12, 24], [23, 24], // Torso
      [23, 25], [25, 27], [24, 26], [26, 28], // Legs
      [27, 29], [29, 31], [28, 30], [30, 32], // Lower legs
      [15, 17], [15, 19], [15, 21], [16, 18], [16, 20], [16, 22], // Hands
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
        if (i > 32) break; // Only first 33 landmarks
        const [px, py] = toPixel(pose[i], w, h);
        ctx.beginPath();
        ctx.arc(px, py, 5, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    ctx.restore();
  },
};

// ── Effect 11: Energy Aura ──────────────────────────────────────────────────

function hexToRgba(hex: string, alpha: number): string {
  if (!hex.startsWith('#')) {
    const temp = document.createElement('div');
    temp.style.color = hex;
    document.body.appendChild(temp);
    const computed = getComputedStyle(temp).color;
    document.body.removeChild(temp);
    const match = computed.match(/(\d+)/g);
    if (match) return `rgba(${match[0]},${match[1]},${match[2]},${alpha})`;
    return `rgba(128,128,128,${alpha})`;
  }
  let r = 0, g = 0, b = 0;
  if (hex.length === 4) {
    r = parseInt(hex[1] + hex[1], 16);
    g = parseInt(hex[2] + hex[2], 16);
    b = parseInt(hex[3] + hex[3], 16);
  } else if (hex.length >= 7) {
    r = parseInt(hex.slice(1, 3), 16);
    g = parseInt(hex.slice(3, 5), 16);
    b = parseInt(hex.slice(5, 7), 16);
  }
  return `rgba(${r},${g},${b},${alpha})`;
}

const energyAura: Effect = {
  id: 'energy-aura',
  name: 'Energy Aura',
  nameTr: 'Enerji Aurası',
  description: 'Pulsing energy field around the body',
  descriptionTr: 'Vücut etrafında nabız atan enerji alanı',
  category: 'body',
  difficulty: 'medium',
  icon: 'zap',
  thumbnail: 'linear-gradient(135deg, #f59e0b, #ef4444)',
  requires: ['pose'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#f59e0b' },
    { name: 'pulse', label: 'Nabız Hızı', type: 'number', min: 0.5, max: 3, step: 0.1, default: 1.5 },
  ],
  process(ctx, w, h, tracking, params) {
    const pose = tracking.pose?.[0];
    if (!pose) return;
    const color = (params.color as string) ?? '#f59e0b';
    const pulseSpeed = (params.pulse as number) ?? 1.5;

    // Get body center and bounds
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
    gradient.addColorStop(0.5, hexToRgba(color, 0.13));
    gradient.addColorStop(0.8, hexToRgba(color, 0.25));
    gradient.addColorStop(1, 'transparent');
    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  },
};

// ── Effect 12: Gesture Trigger ──────────────────────────────────────────────

const gestureDetector = new GestureDetector();

const GESTURE_OPTIONS: GestureType[] = [
  'peace', 'thumbsUp', 'thumbsDown', 'fist', 'openPalm', 'wave', 'pinch', 'point',
];

const gestureTrigger: Effect = {
  id: 'gesture-trigger',
  name: 'Gesture Spark',
  nameTr: 'Jest Tetikleme',
  description: 'Sparks when specific hand gesture is detected',
  descriptionTr: 'Belirli el jesti algılandığında kıvılcım çıkarır',
  category: 'gesture',
  difficulty: 'hard',
  icon: 'sparkles',
  thumbnail: 'linear-gradient(135deg, #06b6d4, #3b82f6)',
  requires: ['hands'],
  params: [
    { name: 'gesture', label: 'Jest', type: 'select', options: GESTURE_OPTIONS, default: 'peace' },
    { name: 'sparkColor', label: 'Kıvılcım Rengi', type: 'color', default: '#06b6d4' },
    { name: 'confidence', label: 'Güven Eşiği', type: 'number', min: 0.3, max: 1, step: 0.05, default: 0.5 },
  ],
  process(ctx, w, h, tracking, params) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const gesture = (params.gesture as string) ?? 'peace';
    const sparkColor = (params.sparkColor as string) ?? '#06b6d4';
    const confidenceThreshold = (params.confidence as number) ?? 0.5;

    gestureDetector.configure({ confidenceThreshold });

    const result = gestureDetector.detect(hands[0]);
    if (!result || result.gesture !== gesture) return;

    const center = hands[0][9];
    const [cx, cy] = toPixel(center, w, h);

    const intensity = Math.round(8 + result.confidence * 12);
    ctx.save();
    const time = tracking.timestamp * 0.005;
    for (let i = 0; i < intensity; i++) {
      const angle = (i / intensity) * Math.PI * 2 + time;
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
  },
};

// ── Effect 13: Mirror Face ──────────────────────────────────────────────────

const mirrorFace: Effect = {
  id: 'mirror-face',
  name: 'Mirror',
  nameTr: 'Ayna',
  description: 'Distorts face like a funhouse mirror',
  descriptionTr: 'Yüzü eğlence aynası gibi bozar',
  category: 'face',
  difficulty: 'medium',
  icon: 'layers',
  thumbnail: 'linear-gradient(135deg, #818cf8, #c084fc)',
  requires: ['face'],
  params: [
    { name: 'strength', label: 'Güç', type: 'number', min: 0.1, max: 2, step: 0.1, default: 0.8 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const strength = (params.strength as number) ?? 0.8;

    const bbox = faceBBox(face, w, h);
    const nose = face[4];
    if (!nose) return;
    const [nx, ny] = toPixel(nose, w, h);

    const radius = bbox.w * 0.6;

    ctx.save();
    ctx.beginPath();
    ctx.arc(nx, ny, radius, 0, Math.PI * 2);
    ctx.clip();

    // Spherize distortion using pixel manipulation
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
    ctx.globalAlpha = 0.8;
    ctx.drawImage(
      tempCanvas,
      0, 0, tempCanvas.width, tempCanvas.height,
      nx - radius * scale, ny - radius * scale,
      radius * 2 * scale, radius * 2 * scale
    );
    ctx.globalAlpha = 1;
    ctx.restore();
  },
};

// ── Effect 14: Glitch Face ──────────────────────────────────────────────────

const glitchFace: Effect = {
  id: 'glitch-face',
  name: 'Glitch',
  nameTr: 'Glitch',
  description: 'Digital glitch distortion on face landmarks',
  descriptionTr: 'Yüz landmark\'larında dijital bozulma',
  category: 'face',
  difficulty: 'medium',
  icon: 'tv',
  thumbnail: 'linear-gradient(135deg, #ef4444, #3b82f6, #22c55e)',
  requires: ['face'],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 1, max: 20, step: 1, default: 8 },
    { name: 'scanlines', label: 'Tarama Çizgileri', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const intensity = (params.intensity as number) ?? 8;
    const showScanlines = params.scanlines as boolean;

    const bbox = faceBBox(face, w, h);

    ctx.save();

    // Random horizontal displacement slices
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

    // Color channel shift
    const shift = Math.floor(intensity * 0.5);
    ctx.globalCompositeOperation = 'screen';
    ctx.globalAlpha = 0.15;
    ctx.drawImage(ctx.canvas, shift, 0);
    ctx.globalCompositeOperation = 'source-over';
    ctx.globalAlpha = 1;

    // Scanlines
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
};

// ── Effect 16: Color Grade ─────────────────────────────────────────────────

const colorGrade: Effect = {
  id: 'color-grade',
  name: 'Color Grade',
  nameTr: 'Renk Derecelendirme',
  description: 'Cinematic color grading with adjustable warmth, contrast and saturation',
  descriptionTr: 'Ayarlanabilir sıcaklık, kontrast ve doygunlukla sinematik renk derecelendirme',
  category: 'face',
  difficulty: 'easy',
  icon: 'palette',
  thumbnail: 'linear-gradient(135deg, #f97316, #eab308, #ef4444)',
  requires: ['face'],
  params: [
    { name: 'warmth', label: 'Sıcaklık', type: 'number', min: -50, max: 50, step: 1, default: 15 },
    { name: 'contrast', label: 'Kontrast', type: 'number', min: 0, max: 100, step: 1, default: 20 },
    { name: 'saturation', label: 'Doygunluk', type: 'number', min: -50, max: 100, step: 1, default: 15 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const warmth = (params.warmth as number) ?? 15;
    const contrast = (params.contrast as number) ?? 20;
    const saturation = (params.saturation as number) ?? 15;
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

    // Warmth: shift red channel up, blue down (or vice versa)
    const warmShift = warmth * 0.5;
    ctx.globalCompositeOperation = 'color';
    ctx.fillStyle = `rgba(${128 + warmShift}, 100, ${128 - warmShift}, 0.15)`;
    ctx.fillRect(x, y, bw, bh);

    // Contrast via overlay blend
    ctx.globalCompositeOperation = 'overlay';
    const contrastAlpha = Math.abs(contrast) / 200;
    ctx.fillStyle = contrast > 0
      ? `rgba(128,128,128,${contrastAlpha})`
      : `rgba(128,128,128,${contrastAlpha})`;
    ctx.fillRect(x, y, bw, bh);

    // Saturation via desaturate or oversaturate
    ctx.globalCompositeOperation = 'source-over';
    if (saturation < 0) {
      ctx.globalAlpha = Math.abs(saturation) / 100;
      ctx.fillStyle = 'rgba(128,128,128,1)';
      ctx.filter = 'grayscale(1)';
      ctx.fillRect(x, y, bw, bh);
      ctx.filter = 'none';
    } else if (saturation > 0) {
      ctx.globalAlpha = saturation / 300;
      ctx.fillStyle = 'rgba(128,128,128,1)';
      ctx.globalCompositeOperation = 'soft-light';
      ctx.fillRect(x, y, bw, bh);
      ctx.filter = `saturate(${1 + saturation / 50})`;
      ctx.fillStyle = 'white';
      ctx.globalCompositeOperation = 'source-over';
      ctx.globalAlpha = saturation / 500;
      ctx.fillRect(x, y, bw, bh);
      ctx.filter = 'none';
    }

    ctx.restore();
  },
};

// ── Effect 17: Crown / Halo ────────────────────────────────────────────────

const crownHalo: Effect = {
  id: 'crown-halo',
  name: 'Crown / Halo',
  nameTr: 'Taç / Hale',
  description: 'Floating crown or halo above head using face landmarks',
  descriptionTr: 'Yüz landmark\'larını kullanarak başın üzerinde yüzen taç veya hale',
  category: 'face',
  difficulty: 'medium',
  icon: 'crown',
  thumbnail: 'linear-gradient(135deg, #fbbf24, #f59e0b, #d97706)',
  requires: ['face'],
  params: [
    { name: 'style', label: 'Stil', type: 'select', options: ['crown', 'halo'], default: 'crown' },
    { name: 'color', label: 'Renk', type: 'color', default: '#fbbf24' },
    { name: 'float', label: 'Yükselme', type: 'number', min: 0.1, max: 0.5, step: 0.05, default: 0.3 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const style = (params.style as string) ?? 'crown';
    const color = (params.color as string) ?? '#fbbf24';
    const floatHeight = (params.float as number) ?? 0.3;

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

      // Crown body
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

      // Jewels
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
      // Halo
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

      // Inner glow
      ctx.globalAlpha = 0.3;
      ctx.lineWidth = 6;
      ctx.beginPath();
      ctx.ellipse(fx, cy, haloW / 2, haloH, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    ctx.restore();
  },
};

// ── Effect 18: Pixelate Face ───────────────────────────────────────────────

const pixelateFace: Effect = {
  id: 'pixelate-face',
  name: 'Pixelate Face',
  nameTr: 'Piksel Yüzü',
  description: 'Privacy pixelation mosaic over face region',
  descriptionTr: 'Yüz bölgesi üzerine gizlilik piksel mozaik efekti',
  category: 'face',
  difficulty: 'easy',
  icon: 'box',
  thumbnail: 'linear-gradient(135deg, #6b7280, #9ca3af, #d1d5db)',
  requires: ['face'],
  params: [
    { name: 'blockSize', label: 'Blok Boyutu', type: 'number', min: 4, max: 32, step: 2, default: 12 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const blockSize = (params.blockSize as number) ?? 12;
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

    // Sample and draw pixel blocks using single image data buffer (100x faster)
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
};

// ── Effect 19: Rainbow Trail ───────────────────────────────────────────────

const rainbowTrail: Effect = {
  id: 'rainbow-trail',
  name: 'Rainbow Trail',
  nameTr: 'Gökkuşağı İzi',
  description: 'Rainbow-colored arc trail from hand movements',
  descriptionTr: 'El hareketlerinden çıkan gökkuşağı renkli yay izi',
  category: 'hand',
  difficulty: 'medium',
  icon: 'aperture',
  thumbnail: 'linear-gradient(135deg, #ef4444, #f59e0b, #22c55e, #3b82f6, #8b5cf6)',
  requires: ['hands'],
  params: [
    { name: 'thickness', label: 'Kalınlık', type: 'number', min: 2, max: 12, step: 1, default: 6 },
    { name: 'arcLength', label: 'Yay Uzunluğu', type: 'number', min: 0.3, max: 2, step: 0.1, default: 1 },
  ],
  process(ctx, w, h, tracking, params) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const thickness = (params.thickness as number) ?? 6;
    const arcLen = (params.arcLength as number) ?? 1;

    // Fingertip landmarks
    const tips = [4, 8, 12, 16, 20];

    for (const hand of hands) {
      // Draw rainbow arc from wrist (0) through fingertips
      const wrist = hand[0];
      if (!wrist) continue;
      const [wx, wy] = toPixel(wrist, w, h);

      const rainbowColors = ['#ef4444', '#f59e0b', '#22c55e', '#3b82f6', '#8b5cf6'];

      for (let i = 0; i < tips.length; i++) {
        const tipIdx = tips[i];
        if (tipIdx >= hand.length) continue;
        const tip = hand[tipIdx];
        const [tx, ty] = toPixel(tip, w, h);

        // Control point for arc (perpendicular to wrist-tip line)
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
};

// ── Effect 20: Force Field ─────────────────────────────────────────────────

const forceField: Effect = {
  id: 'force-field',
  name: 'Force Field',
  nameTr: 'Kuvvet Alanı',
  description: 'Transparent energy bubble around body',
  descriptionTr: 'Vücut etrafında saydam enerji baloncuğu',
  category: 'body',
  difficulty: 'medium',
  icon: 'shield',
  thumbnail: 'linear-gradient(135deg, rgba(99,102,241,0.4), rgba(168,85,247,0.4))',
  requires: ['pose'],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#6366f1' },
    { name: 'pulse', label: 'Nabız', type: 'number', min: 0.5, max: 3, step: 0.1, default: 1.5 },
    { name: 'opacity', label: 'Saydamlık', type: 'number', min: 0.1, max: 0.6, step: 0.05, default: 0.3 },
  ],
  process(ctx, w, h, tracking, params) {
    const pose = tracking.pose?.[0];
    if (!pose) return;
    const color = (params.color as string) ?? '#6366f1';
    const pulseSpeed = (params.pulse as number) ?? 1.5;
    const opacity = (params.opacity as number) ?? 0.3;

    // Get body bounds from shoulders, hips, wrists, ankles
    const keyPoints = [11, 12, 23, 24, 15, 16, 27, 28]; // shoulders, hips, wrists, ankles
    const points: [number, number][] = [];
    for (const idx of keyPoints) {
      if (idx < pose.length && pose[idx]) {
        points.push(toPixel(pose[idx], w, h));
      }
    }
    if (points.length < 4) return;

    // Compute bounding circle
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

    // Outer glow
    const outerGrad = ctx.createRadialGradient(cx, cy, radius * 0.7, cx, cy, radius * pulse);
    outerGrad.addColorStop(0, 'transparent');
    outerGrad.addColorStop(0.8, color + '10');
    outerGrad.addColorStop(0.95, color + '40');
    outerGrad.addColorStop(1, 'transparent');
    ctx.fillStyle = outerGrad;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
    ctx.fill();

    // Ring
    ctx.globalAlpha = opacity;
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.shadowColor = color;
    ctx.shadowBlur = 15;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * pulse, 0, Math.PI * 2);
    ctx.stroke();

    // Inner ring
    ctx.globalAlpha = opacity * 0.5;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * pulse * 0.85, 0, Math.PI * 2);
    ctx.stroke();

    // Rotating dashes
    ctx.globalAlpha = opacity * 0.4;
    ctx.setLineDash([8, 12]);
    ctx.lineDashOffset = -time * 30;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * pulse * 0.92, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.restore();
  },
};

// ── Effect 21: Disco Mode ──────────────────────────────────────────────────

const discoMode: Effect = {
  id: 'disco-mode',
  name: 'Disco Mode',
  nameTr: 'Disko Modu',
  description: 'Pulsing color overlay with hue rotation over face',
  descriptionTr: 'Yüz üzerine nabız atan renk örtüsü ve ton döndürme',
  category: 'face',
  difficulty: 'easy',
  icon: 'disc',
  thumbnail: 'linear-gradient(135deg, #ec4899, #8b5cf6, #06b6d4, #22c55e)',
  requires: ['face'],
  params: [
    { name: 'speed', label: 'Hız', type: 'number', min: 0.5, max: 5, step: 0.1, default: 2 },
    { name: 'opacity', label: 'Yoğunluk', type: 'number', min: 0.05, max: 0.4, step: 0.05, default: 0.2 },
    { name: 'spots', label: 'Noktalar', type: 'boolean', default: true },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const speed = (params.speed as number) ?? 2;
    const opacity = (params.opacity as number) ?? 0.2;
    const showSpots = params.spots as boolean;
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
};

// ── Effect 22: Color Overlay (no tracking) ─────────────────────────────────

const colorOverlay: Effect = {
  id: 'color-overlay',
  name: 'Color Overlay',
  nameTr: 'Renk Örtüsü',
  description: 'Full-screen color tint overlay',
  descriptionTr: 'Tam ekran renk tonu örtüsü',
  category: 'face',
  difficulty: 'easy',
  icon: 'palette',
  thumbnail: 'linear-gradient(135deg, rgba(255,100,100,0.5), rgba(100,100,255,0.5))',
  requires: [],
  params: [
    { name: 'color', label: 'Renk', type: 'color', default: '#7c3aed' },
    { name: 'opacity', label: 'Saydamlık', type: 'number', min: 0.05, max: 0.5, step: 0.05, default: 0.2 },
  ],
  process(ctx, w, h, _tracking, params) {
    const color = (params.color as string) ?? '#7c3aed';
    const opacity = (params.opacity as number) ?? 0.2;
    ctx.save();
    ctx.globalAlpha = opacity;
    ctx.globalCompositeOperation = 'color';
    ctx.fillStyle = color;
    ctx.fillRect(0, 0, w, h);
    ctx.restore();
  },
};

// ── Effect 23: Vignette (no tracking) ──────────────────────────────────────

const vignette: Effect = {
  id: 'vignette',
  name: 'Vignette',
  nameTr: 'Vinyet',
  description: 'Dark vignette around the edges',
  descriptionTr: 'Kenarlıklarda karanlık vinyet efekti',
  category: 'face',
  difficulty: 'easy',
  icon: 'moon',
  thumbnail: 'radial-gradient(circle, transparent 30%, rgba(0,0,0,0.7) 100%)',
  requires: [],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0.1, max: 1, step: 0.05, default: 0.6 },
  ],
  process(ctx, w, h, _tracking, params) {
    const intensity = (params.intensity as number) ?? 0.6;
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
};

// ── Effect 24: Warm Tone (no tracking) ─────────────────────────────────────

const warmTone: Effect = {
  id: 'warm-tone',
  name: 'Warm Tone',
  nameTr: 'Sıcak Ton',
  description: 'Warm color temperature shift',
  descriptionTr: 'Sıcak renk sıcaklığı kayması',
  category: 'face',
  difficulty: 'easy',
  icon: 'sun',
  thumbnail: 'linear-gradient(135deg, rgba(255,150,50,0.3), rgba(255,100,50,0.2))',
  requires: [],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0.05, max: 0.4, step: 0.05, default: 0.15 },
  ],
  process(ctx, w, h, _tracking, params) {
    const intensity = (params.intensity as number) ?? 0.15;
    ctx.save();
    ctx.globalCompositeOperation = 'overlay';
    ctx.fillStyle = `rgba(255,140,50,${intensity})`;
    ctx.fillRect(0, 0, w, h);
    ctx.restore();
  },
};

// ── Effect 25: Cool Tone (no tracking) ─────────────────────────────────────

const coolTone: Effect = {
  id: 'cool-tone',
  name: 'Cool Tone',
  nameTr: 'Soğuk Ton',
  description: 'Cool blue color temperature shift',
  descriptionTr: 'Soğuk mavi renk sıcaklığı kayması',
  category: 'face',
  difficulty: 'easy',
  icon: 'snowflake',
  thumbnail: 'linear-gradient(135deg, rgba(50,100,255,0.3), rgba(100,150,255,0.2))',
  requires: [],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0.05, max: 0.4, step: 0.05, default: 0.15 },
  ],
  process(ctx, w, h, _tracking, params) {
    const intensity = (params.intensity as number) ?? 0.15;
    ctx.save();
    ctx.globalCompositeOperation = 'overlay';
    ctx.fillStyle = `rgba(50,120,255,${intensity})`;
    ctx.fillRect(0, 0, w, h);
    ctx.restore();
  },
};

// ── Effect 26: Film Grain (no tracking) ────────────────────────────────────

const filmGrain: Effect = {
  id: 'film-grain',
  name: 'Film Grain',
  nameTr: 'Film Taneciği',
  description: 'Cinematic film grain noise overlay',
  descriptionTr: 'Sinematik film taneciği gürültü örtüsü',
  category: 'face',
  difficulty: 'easy',
  icon: 'film',
  thumbnail: 'linear-gradient(135deg, #444, #888, #444)',
  requires: [],
  params: [
    { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0.02, max: 0.15, step: 0.01, default: 0.06 },
  ],
  process(ctx, w, h, _tracking, params) {
    const intensity = (params.intensity as number) ?? 0.06;
    const imageData = ctx.getImageData(0, 0, w, h);
    const data = imageData.data;
    const len = data.length;

    for (let i = 0; i < len; i += 4) {
      const noise = (Math.random() - 0.5) * 255 * intensity;
      data[i] = Math.min(255, Math.max(0, data[i] + noise));
      data[i + 1] = Math.min(255, Math.max(0, data[i + 1] + noise));
      data[i + 2] = Math.min(255, Math.max(0, data[i + 2] + noise));
    }

    ctx.putImageData(imageData, 0, 0);
  },
};

// ── Effect 27: Ghost Face (Hayalet Yüz) ─────────────────────────────────────

const ghostFace: Effect = {
  id: 'ghost-face',
  name: 'Spectral Ghost',
  nameTr: 'Hayalet Yüz',
  description: 'Ethereal glowing ghost aura with floating spirit wisps',
  descriptionTr: 'Süzülen ruh parçacıkları ve parlayan hayalet aurası',
  category: 'face',
  difficulty: 'medium',
  icon: 'ghost',
  thumbnail: 'linear-gradient(135deg, #06b6d4, #8b5cf6, #3b82f6)',
  requires: ['face'],
  params: [
    { name: 'glowIntensity', label: 'Aura Yoğunluğu', type: 'number', min: 5, max: 35, step: 1, default: 20 },
    { name: 'color', label: 'Hayalet Rengi', type: 'color', default: '#38bdf8' },
    { name: 'wispCount', label: 'Ruh Sayısı', type: 'number', min: 4, max: 20, step: 1, default: 12 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const color = (params.color as string) ?? '#38bdf8';
    const glow = (params.glowIntensity as number) ?? 20;
    const wispCount = (params.wispCount as number) ?? 12;
    const bbox = faceBBox(face, w, h);

    ctx.save();
    // Ethereal pulsing aura around face
    const pulse = Math.sin(time * 0.003) * 0.15 + 0.85;
    const grad = ctx.createRadialGradient(bbox.cx, bbox.cy, bbox.w * 0.2, bbox.cx, bbox.cy, bbox.w * 0.9 * pulse);
    grad.addColorStop(0, 'rgba(56, 189, 248, 0.05)');
    grad.addColorStop(0.5, 'rgba(139, 92, 246, 0.25)');
    grad.addColorStop(1, 'rgba(6, 182, 212, 0)');
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(bbox.cx, bbox.cy, bbox.w * 0.9 * pulse, 0, Math.PI * 2);
    ctx.fill();

    // Floating spirit wisps
    ctx.fillStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = glow;
    for (let i = 0; i < wispCount; i++) {
      const angle = (time * 0.0015 + (i * Math.PI * 2) / wispCount) % (Math.PI * 2);
      const distOffset = (bbox.w * 0.6) + Math.sin(time * 0.004 + i) * 20;
      const wx = bbox.cx + Math.cos(angle) * distOffset;
      const wy = bbox.cy + Math.sin(angle) * distOffset * 0.8 - ((time * 0.03 + i * 15) % 40);
      const rad = 3 + Math.sin(time * 0.005 + i) * 2;
      ctx.beginPath();
      ctx.arc(wx, wy, Math.max(1, rad), 0, Math.PI * 2);
      ctx.fill();
    }

    // Glowing eyes highlight
    const leftEye = toPixel(face[159], w, h);
    const rightEye = toPixel(face[386], w, h);
    for (const [ex, ey] of [leftEye, rightEye]) {
      ctx.fillStyle = '#ffffff';
      ctx.shadowColor = color;
      ctx.shadowBlur = 15;
      ctx.beginPath();
      ctx.arc(ex, ey, 5, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  },
};

// ── Effect 28: Matrix Digital Rain (Matrix Kodu) ────────────────────────────

const matrixRain: Effect = {
  id: 'matrix-code',
  name: 'Matrix Rain',
  nameTr: 'Matrix Kodu',
  description: 'Digital cyber matrix rain falling across face contours',
  descriptionTr: 'Yüz hatları boyunca dikey akan dijital siber kod yağmuru',
  category: 'face',
  difficulty: 'medium',
  icon: 'terminal',
  thumbnail: 'linear-gradient(135deg, #001100, #00ff66, #003300)',
  requires: ['face'],
  params: [
    { name: 'speed', label: 'Akış Hızı', type: 'number', min: 1, max: 10, step: 1, default: 5 },
    { name: 'charSize', label: 'Karakter Boyutu', type: 'number', min: 8, max: 20, step: 2, default: 12 },
    { name: 'color', label: 'Kod Rengi', type: 'color', default: '#00ff66' },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const speed = (params.speed as number) ?? 5;
    const charSize = (params.charSize as number) ?? 12;
    const color = (params.color as string) ?? '#00ff66';
    const bbox = faceBBox(face, w, h);

    const chars = '0123456789ABCDEFｦｱｳｴｵｶｷｹｺｻｼｽｾｿﾀﾂﾃﾅﾆﾇﾈﾊﾋﾎﾏﾐﾑﾒﾓﾔﾕﾗﾘﾜ';
    const cols = Math.floor(bbox.w / charSize);

    ctx.save();
    ctx.font = `bold ${charSize}px monospace`;
    ctx.shadowColor = color;
    ctx.shadowBlur = 8;

    for (let c = 0; c < cols; c++) {
      const cx = bbox.x + c * charSize;
      const dropY = ((time * 0.05 * speed + c * 27) % (bbox.h + 60)) + bbox.y - 30;

      for (let r = 0; r < 7; r++) {
        const py = dropY - r * charSize;
        if (py < bbox.y - 20 || py > bbox.y + bbox.h + 20) continue;
        const charIdx = Math.floor((c * 13 + r * 7 + time * 0.01) % chars.length);
        const ch = chars[charIdx];

        if (r === 0) {
          ctx.fillStyle = '#ffffff'; // Bright head
        } else {
          const alpha = Math.max(0.15, 1 - r * 0.15);
          ctx.fillStyle = color.startsWith('#')
            ? color + Math.floor(alpha * 255).toString(16).padStart(2, '0')
            : color;
        }
        ctx.fillText(ch, cx, py);
      }
    }
    ctx.restore();
  },
};

// ── Effect 29: Cyberpunk Visor & HUD (Siber Vizör) ──────────────────────────

const cyberpunkVisor: Effect = {
  id: 'cyberpunk-visor',
  name: 'Cyber Visor',
  nameTr: 'Siber Vizör & HUD',
  description: 'Futuristic sci-fi cybernetic HUD visor with telemetry overlays',
  descriptionTr: 'Gözler üzerinde fütüristik siber vizör ve HUD telemetri grafikleri',
  category: 'face',
  difficulty: 'medium',
  icon: 'crosshair',
  thumbnail: 'linear-gradient(135deg, #ec4899, #8b5cf6, #06b6d4)',
  requires: ['face'],
  params: [
    { name: 'visorColor', label: 'Vizör Rengi', type: 'color', default: '#06b6d4' },
    { name: 'hudText', label: 'HUD Metni', type: 'select', options: ['TARGET_LOCKED', 'SYSTEM_SYNC', 'ANON_SHIELD', 'NEURAL_LINK'], default: 'TARGET_LOCKED' },
    { name: 'opacity', label: 'Saydamlık', type: 'number', min: 0.3, max: 1, step: 0.1, default: 0.85 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const visorColor = (params.visorColor as string) ?? '#06b6d4';
    const hudText = (params.hudText as string) ?? 'TARGET_LOCKED';
    const opacity = (params.opacity as number) ?? 0.85;

    const leftOuter = toPixel(face[33], w, h);
    const rightOuter = toPixel(face[263], w, h);
    const noseBridge = toPixel(face[6], w, h);

    const visorW = Math.hypot(rightOuter[0] - leftOuter[0], rightOuter[1] - leftOuter[1]) * 1.55;
    const visorH = visorW * 0.32;
    const midX = (leftOuter[0] + rightOuter[0]) / 2;
    const midY = noseBridge[1] - 4;
    const angle = Math.atan2(rightOuter[1] - leftOuter[1], rightOuter[0] - leftOuter[0]);

    ctx.save();
    ctx.translate(midX, midY);
    ctx.rotate(angle);
    ctx.globalAlpha = opacity;

    // Glass Visor Polygon
    ctx.beginPath();
    ctx.moveTo(-visorW / 2, -visorH / 2);
    ctx.lineTo(visorW / 2, -visorH / 2);
    ctx.lineTo(visorW / 2 - 12, visorH / 2);
    ctx.lineTo(-visorW / 2 + 12, visorH / 2);
    ctx.closePath();

    ctx.fillStyle = 'rgba(6, 182, 212, 0.22)';
    ctx.fill();
    ctx.strokeStyle = visorColor;
    ctx.lineWidth = 2.5;
    ctx.shadowColor = visorColor;
    ctx.shadowBlur = 14;
    ctx.stroke();

    // Animated Scanlines inside visor
    const scanY = ((time * 0.08) % visorH) - visorH / 2;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.6)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(-visorW / 2 + 15, scanY);
    ctx.lineTo(visorW / 2 - 15, scanY);
    ctx.stroke();

    // Target reticles
    ctx.strokeStyle = visorColor;
    ctx.lineWidth = 1.5;
    for (const side of [-visorW * 0.25, visorW * 0.25]) {
      ctx.beginPath();
      ctx.arc(side, 0, 14, 0, Math.PI * 2);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(side - 18, 0); ctx.lineTo(side + 18, 0);
      ctx.moveTo(side, -18); ctx.lineTo(side, 18);
      ctx.stroke();
    }

    // Telemetry text
    ctx.font = '700 9px monospace';
    ctx.fillStyle = '#fff';
    ctx.shadowColor = visorColor;
    ctx.shadowBlur = 6;
    ctx.fillText(`▶ ${hudText} [${Math.floor(95 + Math.sin(time * 0.005) * 4)}%]`, -visorW / 2 + 18, visorH / 2 - 6);

    ctx.restore();
  },
};

// ── Effect 30: Fireball Palm (Ateş Çemberi) ──────────────────────────────────

const fireballPalm: Effect = {
  id: 'fireball-palm',
  name: 'Fire Vortex',
  nameTr: 'Ateş Çemberi',
  description: 'Swirling flame vortex and blazing embers radiating from palms',
  descriptionTr: 'Avuç içlerinden yayılan alev vorteksi ve kıvılcım parçacıkları',
  category: 'hand',
  difficulty: 'medium',
  icon: 'flame',
  thumbnail: 'linear-gradient(135deg, #ff4500, #ff8c00, #ffd700)',
  requires: ['hands'],
  params: [
    { name: 'flameSize', label: 'Alev Boyutu', type: 'number', min: 30, max: 120, step: 5, default: 65 },
    { name: 'coreColor', label: 'Ateş Rengi', type: 'color', default: '#ff6600' },
  ],
  process(ctx, w, h, tracking, params, time) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const flameSize = (params.flameSize as number) ?? 65;
    const coreColor = (params.coreColor as string) ?? '#ff6600';

    ctx.save();
    for (const hand of hands) {
      if (!hand.length) continue;
      // Palm center: average of wrist(0) and MCP joints(5, 9, 13, 17)
      let px = 0, py = 0;
      const pts = [0, 5, 9, 13, 17];
      for (const idx of pts) {
        const [x, y] = toPixel(hand[idx], w, h);
        px += x; py += y;
      }
      px /= pts.length;
      py /= pts.length;

      // Swirling flame gradient
      const grad = ctx.createRadialGradient(px, py, 4, px, py, flameSize);
      grad.addColorStop(0, '#ffffff');
      grad.addColorStop(0.25, '#ffdd00');
      grad.addColorStop(0.65, coreColor);
      grad.addColorStop(1, 'rgba(255, 69, 0, 0)');

      ctx.fillStyle = grad;
      ctx.shadowColor = '#ff4500';
      ctx.shadowBlur = 24;
      ctx.beginPath();
      ctx.arc(px, py, flameSize, 0, Math.PI * 2);
      ctx.fill();

      // Rotating flame tongues
      for (let i = 0; i < 8; i++) {
        const angle = time * 0.004 + (i * Math.PI) / 4;
        const flen = flameSize * (0.8 + Math.sin(time * 0.01 + i) * 0.3);
        const fx = px + Math.cos(angle) * flen;
        const fy = py + Math.sin(angle) * flen - 10;
        ctx.fillStyle = i % 2 === 0 ? '#ffaa00' : '#ff3300';
        ctx.beginPath();
        ctx.arc(fx, fy, 6 + Math.sin(time * 0.008 + i) * 3, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    ctx.restore();
  },
};

// ── Effect 31: Ice Crystals (Buz Krallığı) ───────────────────────────────────

const iceCrystals: Effect = {
  id: 'ice-crystals',
  name: 'Ice Crystals',
  nameTr: 'Buz Kristalleri',
  description: 'Sharp crystalline frost and sparkling snowflakes from fingertips',
  descriptionTr: 'Parmak uçlarından filizlenen keskin buz kristalleri ve kar taneleri',
  category: 'hand',
  difficulty: 'medium',
  icon: 'snowflake',
  thumbnail: 'linear-gradient(135deg, #a5f3fc, #38bdf8, #0284c7)',
  requires: ['hands'],
  params: [
    { name: 'crystalLength', label: 'Kristal Boyu', type: 'number', min: 10, max: 50, step: 2, default: 28 },
    { name: 'frostColor', label: 'Buz Rengi', type: 'color', default: '#38bdf8' },
  ],
  process(ctx, w, h, tracking, params, time) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const crystalLength = (params.crystalLength as number) ?? 28;
    const frostColor = (params.frostColor as string) ?? '#38bdf8';

    const tips = [4, 8, 12, 16, 20]; // Fingertips

    ctx.save();
    ctx.strokeStyle = frostColor;
    ctx.fillStyle = '#ffffff';
    ctx.shadowColor = frostColor;
    ctx.shadowBlur = 12;
    ctx.lineWidth = 2;

    for (const hand of hands) {
      for (const tipIdx of tips) {
        if (!hand[tipIdx]) continue;
        const [tx, ty] = toPixel(hand[tipIdx], w, h);

        // Branching crystal spines
        for (let b = 0; b < 4; b++) {
          const bAngle = -Math.PI / 2 + (b - 1.5) * 0.45 + Math.sin(time * 0.002 + tipIdx) * 0.1;
          const ex = tx + Math.cos(bAngle) * crystalLength;
          const ey = ty + Math.sin(bAngle) * crystalLength;

          ctx.beginPath();
          ctx.moveTo(tx, ty);
          ctx.lineTo(ex, ey);
          // Secondary mini spine
          ctx.lineTo(ex + Math.cos(bAngle + 0.5) * (crystalLength * 0.4), ey + Math.sin(bAngle + 0.5) * (crystalLength * 0.4));
          ctx.stroke();

          // Diamond glint at tip
          ctx.beginPath();
          ctx.arc(ex, ey, 2.5, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }
    ctx.restore();
  },
};

// ── Effect 32: 3D Hologram Mesh (Hologram Izgara) ───────────────────────────

const hologramMesh: Effect = {
  id: 'hologram-grid',
  name: 'Holo Grid',
  nameTr: '3D Hologram Izgara',
  description: 'Sci-fi holographic wireframe triangulation over face and body',
  descriptionTr: 'Yüz hatları üzerinde parlayan 3D siber tel örgü (wireframe) ızgarası',
  category: 'face',
  difficulty: 'medium',
  icon: 'grid',
  thumbnail: 'linear-gradient(135deg, #00f2fe, #4facfe)',
  requires: ['face'],
  params: [
    { name: 'lineColor', label: 'Izgara Rengi', type: 'color', default: '#00f2fe' },
    { name: 'glow', label: 'Parıltı', type: 'number', min: 2, max: 20, step: 1, default: 10 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const lineColor = (params.lineColor as string) ?? '#00f2fe';
    const glow = (params.glow as number) ?? 10;

    // Face triangulation triangles (subset of MediaPipe landmarks)
    const triangles = [
      [10, 338, 297], [10, 109, 67], [10, 151, 9], [9, 8, 168], [168, 6, 197],
      [197, 195, 5], [5, 4, 1], [1, 2, 164], [164, 0, 11], [11, 12, 13],
      [33, 160, 158], [158, 133, 153], [153, 144, 33], // Left Eye
      [263, 385, 387], [387, 362, 380], [380, 373, 263], // Right Eye
      [61, 185, 40], [40, 39, 37], [37, 0, 267], [267, 269, 270], [270, 409, 291], // Mouth
      [152, 148, 176], [176, 149, 150], [150, 136, 172], [152, 377, 400], [400, 378, 379], // Chin
    ];

    ctx.save();
    ctx.strokeStyle = lineColor;
    ctx.lineWidth = 1.2;
    ctx.shadowColor = lineColor;
    ctx.shadowBlur = glow;
    ctx.globalAlpha = 0.85 + Math.sin(time * 0.005) * 0.15;

    for (const [p1, p2, p3] of triangles) {
      if (!face[p1] || !face[p2] || !face[p3]) continue;
      const [x1, y1] = toPixel(face[p1], w, h);
      const [x2, y2] = toPixel(face[p2], w, h);
      const [x3, y3] = toPixel(face[p3], w, h);

      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.lineTo(x3, y3);
      ctx.closePath();
      ctx.stroke();

      // Vertex dots
      ctx.fillStyle = '#ffffff';
      ctx.beginPath();
      ctx.arc(x1, y1, 1.5, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  },
};

// ── Effect 33: Cinematic Bokeh (Sinematik Bokeh) ────────────────────────────

const cinematicBokeh: Effect = {
  id: 'bokeh-particles',
  name: 'Cinematic Bokeh',
  nameTr: 'Sinematik Bokeh',
  description: 'Warm floating out-of-focus chromatic light discs and lens flares',
  descriptionTr: 'Süzülen sıcak kromatik ışık daireleri ve sinematik lens parıltıları',
  category: 'face',
  difficulty: 'easy',
  icon: 'sun',
  thumbnail: 'linear-gradient(135deg, #f59e0b, #ec4899, #8b5cf6)',
  requires: [],
  params: [
    { name: 'count', label: 'Daire Sayısı', type: 'number', min: 10, max: 40, step: 2, default: 22 },
    { name: 'tint', label: 'Işık Tonu', type: 'select', options: ['Altın Sarısı', 'Mor Neon', 'Cyan Mavi', 'Pastel'], default: 'Altın Sarısı' },
  ],
  process(ctx, w, h, _tracking, params, time) {
    const count = (params.count as number) ?? 22;
    const tint = (params.tint as string) ?? 'Altın Sarısı';

    const colors = tint === 'Mor Neon'
      ? ['rgba(168, 85, 247, 0.35)', 'rgba(236, 72, 153, 0.3)', 'rgba(124, 58, 237, 0.25)']
      : tint === 'Cyan Mavi'
      ? ['rgba(6, 182, 212, 0.35)', 'rgba(56, 189, 248, 0.3)', 'rgba(99, 102, 241, 0.25)']
      : tint === 'Pastel'
      ? ['rgba(244, 114, 182, 0.3)', 'rgba(129, 140, 248, 0.3)', 'rgba(52, 211, 153, 0.25)']
      : ['rgba(245, 158, 11, 0.35)', 'rgba(251, 191, 36, 0.3)', 'rgba(239, 68, 68, 0.2)'];

    ctx.save();
    for (let i = 0; i < count; i++) {
      const seed = i * 137.5;
      const speed = 0.0004 + (i % 5) * 0.0001;
      const bx = ((seed * 11 + time * speed * w) % (w + 120)) - 60;
      const by = ((seed * 23 + Math.sin(time * 0.001 + i) * 60) % (h + 120)) - 60;
      const radius = 18 + (i % 7) * 8 + Math.sin(time * 0.002 + i) * 6;

      ctx.fillStyle = colors[i % colors.length];
      ctx.beginPath();
      ctx.arc(bx, by, radius, 0, Math.PI * 2);
      ctx.fill();

      // Soft rim highlight
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.lineWidth = 1;
      ctx.stroke();
    }
    ctx.restore();
  },
};

// ── Effect 34: Retro VHS 1984 (Retro VHS) ───────────────────────────────────

const vhsRetro: Effect = {
  id: 'vhs-retro',
  name: 'VHS 1984',
  nameTr: 'Retro VHS 1984',
  description: 'Vintage 80s tape tracking glitch with scanlines and RGB color split',
  descriptionTr: '80ler video kaset paraziti, CRT tarama çizgileri ve RGB renk ayrışması',
  category: 'face',
  difficulty: 'easy',
  icon: 'tv',
  thumbnail: 'linear-gradient(135deg, #1e1b4b, #4338ca, #e11d48)',
  requires: [],
  params: [
    { name: 'scanlineDensity', label: 'Tarama Çizgisi', type: 'number', min: 2, max: 8, step: 1, default: 4 },
    { name: 'noise', label: 'Bant Paraziti', type: 'number', min: 0.1, max: 1, step: 0.1, default: 0.4 },
  ],
  process(ctx, w, h, _tracking, params, time) {
    const scanlineDensity = (params.scanlineDensity as number) ?? 4;
    const noise = (params.noise as number) ?? 0.4;

    ctx.save();
    // 1. Horizontal CRT Scanlines
    ctx.fillStyle = 'rgba(0, 0, 0, 0.28)';
    for (let y = 0; y < h; y += scanlineDensity) {
      ctx.fillRect(0, y, w, 1);
    }

    // 2. Intermittent VHS tracking distortion bar
    const barY = (time * 0.12) % (h + 100) - 50;
    if (barY > 0 && barY < h) {
      ctx.fillStyle = `rgba(255, 255, 255, ${0.08 * noise})`;
      ctx.fillRect(0, barY, w, 16);
      ctx.fillStyle = `rgba(0, 255, 255, ${0.12 * noise})`;
      ctx.fillRect(0, barY + 4, w, 4);
    }

    // 3. Vintage VHS Timestamp OSB
    ctx.font = 'bold 13px monospace';
    ctx.fillStyle = 'rgba(255, 255, 255, 0.9)';
    ctx.shadowColor = '#000';
    ctx.shadowBlur = 4;
    ctx.fillText('PLAY ▶ 00:24:19', 24, 36);

    const blink = Math.floor(time * 0.002) % 2 === 0;
    if (blink) {
      ctx.fillStyle = '#ef4444';
      ctx.beginPath();
      ctx.arc(w - 75, 30, 5, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = 'rgba(255, 255, 255, 0.85)';
    ctx.fillText('REC', w - 62, 34);
    ctx.fillText('SP 1984-08-20', 24, h - 24);

    ctx.restore();
  },
};

// ── Effect 35: Neon Cyber Frame (Neon Çerçeve) ──────────────────────────────

const neonCyberFrame: Effect = {
  id: 'neon-cyber-frame',
  name: 'Neon Frame',
  nameTr: 'Neon Kenar Çerçevesi',
  description: 'Pulsing cybernetic glowing border with corner targeting brackets',
  descriptionTr: 'Nefes alan parlayan neon çerçeve ve köşe hedefleme braketleri',
  category: 'face',
  difficulty: 'easy',
  icon: 'square',
  thumbnail: 'linear-gradient(135deg, #7c3aed, #06b6d4, #f43f5e)',
  requires: [],
  params: [
    { name: 'frameColor', label: 'Çerçeve Rengi', type: 'color', default: '#8b5cf6' },
    { name: 'thickness', label: 'Kalınlık', type: 'number', min: 2, max: 10, step: 1, default: 4 },
  ],
  process(ctx, w, h, _tracking, params, time) {
    const frameColor = (params.frameColor as string) ?? '#8b5cf6';
    const thickness = (params.thickness as number) ?? 4;
    const pad = 12;

    ctx.save();
    ctx.strokeStyle = frameColor;
    ctx.lineWidth = thickness;
    ctx.shadowColor = frameColor;
    ctx.shadowBlur = 18 + Math.sin(time * 0.005) * 6;

    // Outer bounding frame
    ctx.strokeRect(pad, pad, w - pad * 2, h - pad * 2);

    // Corner brackets
    const arm = 30;
    ctx.lineWidth = thickness * 1.8;
    ctx.beginPath();
    // Top-Left
    ctx.moveTo(pad - 2, pad + arm); ctx.lineTo(pad - 2, pad - 2); ctx.lineTo(pad + arm, pad - 2);
    // Top-Right
    ctx.moveTo(w - pad + 2 - arm, pad - 2); ctx.lineTo(w - pad + 2, pad - 2); ctx.lineTo(w - pad + 2, pad + arm);
    // Bottom-Left
    ctx.moveTo(pad - 2, h - pad + 2 - arm); ctx.lineTo(pad - 2, h - pad + 2); ctx.lineTo(pad + arm, h - pad + 2);
    // Bottom-Right
    ctx.moveTo(w - pad + 2 - arm, h - pad + 2); ctx.lineTo(w - pad + 2, h - pad + 2); ctx.lineTo(w - pad + 2, h - pad + 2 - arm);
    ctx.stroke();

    ctx.restore();
  },
};

// ── Effect 36: Cosmic Star Dust (Yıldız Yağmuru) ────────────────────────────

const starDust: Effect = {
  id: 'star-dust',
  name: 'Star Dust',
  nameTr: 'Kozmik Yıldız Yağmuru',
  description: 'Glittering celestial stars and cosmic dust radiating around head',
  descriptionTr: 'Baş ve yüz çevresinde parıldayan yıldızlar ve kozmik tozlar',
  category: 'gesture',
  difficulty: 'medium',
  icon: 'sparkles',
  thumbnail: 'linear-gradient(135deg, #fef08a, #e879f9, #38bdf8)',
  requires: ['face'],
  params: [
    { name: 'starCount', label: 'Yıldız Sayısı', type: 'number', min: 8, max: 30, step: 2, default: 18 },
    { name: 'twinkleSpeed', label: 'Pırıltı Hızı', type: 'number', min: 1, max: 10, step: 1, default: 5 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face) return;
    const starCount = (params.starCount as number) ?? 18;
    const speed = (params.twinkleSpeed as number) ?? 5;
    const bbox = faceBBox(face, w, h);

    ctx.save();
    for (let i = 0; i < starCount; i++) {
      const angle = (i * Math.PI * 2) / starCount + time * 0.001;
      const radDist = bbox.w * 0.75 + Math.sin(time * 0.003 * speed + i) * 25;
      const sx = bbox.cx + Math.cos(angle) * radDist;
      const sy = bbox.cy + Math.sin(angle) * radDist * 0.85;

      const starSize = 4 + Math.sin(time * 0.006 * speed + i * 2) * 3;
      if (starSize <= 0) continue;

      ctx.fillStyle = i % 2 === 0 ? '#ffffff' : '#fef08a';
      ctx.shadowColor = '#e879f9';
      ctx.shadowBlur = 10;

      // 4-point star
      ctx.beginPath();
      ctx.moveTo(sx, sy - starSize * 2);
      ctx.lineTo(sx + starSize * 0.5, sy - starSize * 0.5);
      ctx.lineTo(sx + starSize * 2, sy);
      ctx.lineTo(sx + starSize * 0.5, sy + starSize * 0.5);
      ctx.lineTo(sx, sy + starSize * 2);
      ctx.lineTo(sx - starSize * 0.5, sy + starSize * 0.5);
      ctx.lineTo(sx - starSize * 2, sy);
      ctx.lineTo(sx - starSize * 0.5, sy - starSize * 0.5);
      ctx.closePath();
      ctx.fill();
    }
    ctx.restore();
  },
};

// ── Effect 37: Bandit Stealth Mask (Karanlık Maske) ──────────────────────────

const banditMask: Effect = {
  id: 'bandit-mask',
  name: 'Bandit Mask',
  nameTr: 'Karanlık Maske',
  description: 'Sleek dark vigilante eye mask with reflective slit highlights',
  descriptionTr: 'Göz bölgesini örten şık siyah maske ve göz parlaması',
  category: 'face',
  difficulty: 'easy',
  icon: 'shield',
  thumbnail: 'linear-gradient(135deg, #09090b, #27272a, #52525b)',
  requires: ['face'],
  params: [
    { name: 'maskColor', label: 'Maske Rengi', type: 'color', default: '#18181b' },
    { name: 'edgeGlow', label: 'Kenar Parıltısı', type: 'color', default: '#8b5cf6' },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const maskColor = (params.maskColor as string) ?? '#18181b';
    const edgeGlow = (params.edgeGlow as string) ?? '#8b5cf6';

    const leftTemple = toPixel(face[127], w, h);
    const rightTemple = toPixel(face[356], w, h);
    const browMid = toPixel(face[9], w, h);
    const noseBase = toPixel(face[2], w, h);

    ctx.save();
    ctx.fillStyle = maskColor;
    ctx.strokeStyle = edgeGlow;
    ctx.lineWidth = 1.8;
    ctx.shadowColor = edgeGlow;
    ctx.shadowBlur = 8;

    // Mask polygon
    ctx.beginPath();
    ctx.moveTo(leftTemple[0] - 10, leftTemple[1] - 8);
    ctx.quadraticCurveTo(browMid[0], browMid[1] - 18, rightTemple[0] + 10, rightTemple[1] - 8);
    ctx.quadraticCurveTo(rightTemple[0] + 14, rightTemple[1] + 16, rightTemple[0], rightTemple[1] + 24);
    ctx.quadraticCurveTo(noseBase[0], noseBase[1] + 10, leftTemple[0], leftTemple[1] + 24);
    ctx.quadraticCurveTo(leftTemple[0] - 14, leftTemple[1] + 16, leftTemple[0] - 10, leftTemple[1] - 8);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // Eye cutouts
    for (const eyeIdx of [159, 386]) {
      const [ex, ey] = toPixel(face[eyeIdx], w, h);
      ctx.fillStyle = '#ffffff';
      ctx.shadowColor = '#ffffff';
      ctx.shadowBlur = 12;
      ctx.beginPath();
      ctx.ellipse(ex, ey, 9, 4.5, 0, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  },
};

// ── Effect 38: 8-Bit Pixel Mosaic (Piksel Mozaik) ────────────────────────────

const pixelMosaic: Effect = {
  id: 'pixel-mosaic',
  name: '8-Bit Mosaic',
  nameTr: '8-Bit Piksel Mozaik',
  description: 'Retro blocky pixelated mosaic filter over face region',
  descriptionTr: 'Yüz bölgesi üzerinde ayarlanabilir 8-bit piksel mozaik filtresi',
  category: 'face',
  difficulty: 'easy',
  icon: 'box',
  thumbnail: 'linear-gradient(135deg, #10b981, #3b82f6, #6366f1)',
  requires: ['face'],
  params: [
    { name: 'pixelSize', label: 'Piksel Boyutu', type: 'number', min: 8, max: 36, step: 2, default: 16 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const pixelSize = (params.pixelSize as number) ?? 16;
    const bbox = faceBBox(face, w, h);

    const pad = 15;
    const startX = Math.max(0, Math.floor(bbox.x - pad));
    const startY = Math.max(0, Math.floor(bbox.y - pad));
    const blockW = Math.min(w - startX, Math.floor(bbox.w + pad * 2));
    const blockH = Math.min(h - startY, Math.floor(bbox.h + pad * 2));

    ctx.save();
    const imgData = ctx.getImageData(startX, startY, blockW, blockH);
    const data = imgData.data;

    for (let py = 0; py < blockH; py += pixelSize) {
      for (let px = 0; px < blockW; px += pixelSize) {
        const centerIndex = (py * blockW + px) * 4;
        const r = data[centerIndex];
        const g = data[centerIndex + 1];
        const b = data[centerIndex + 2];

        for (let subY = 0; subY < pixelSize && py + subY < blockH; subY++) {
          for (let subX = 0; subX < pixelSize && px + subX < blockW; subX++) {
            const idx = ((py + subY) * blockW + (px + subX)) * 4;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
          }
        }
      }
    }
    ctx.putImageData(imgData, startX, startY);
    ctx.restore();
  },
};

// ── Effect 39: Prismatic Rainbow Aura (Gökkuşağı Dalgası) ───────────────────

const rainbowAura: Effect = {
  id: 'rainbow-aura',
  name: 'Rainbow Aura',
  nameTr: 'Gökkuşağı Dalgası',
  description: 'Shimmering iridescent prismatic rainbow wave along face contours',
  descriptionTr: 'Yüz hatlarından dalga dalga yayılan prizmatik gökkuşağı parıltısı',
  category: 'face',
  difficulty: 'easy',
  icon: 'zap',
  thumbnail: 'linear-gradient(135deg, #ef4444, #f59e0b, #10b981, #3b82f6, #8b5cf6)',
  requires: ['face'],
  params: [
    { name: 'speed', label: 'Dalga Hızı', type: 'number', min: 1, max: 10, step: 1, default: 5 },
    { name: 'thickness', label: 'Çizgi Kalınlığı', type: 'number', min: 2, max: 8, step: 1, default: 3.5 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const speed = (params.speed as number) ?? 5;
    const thickness = (params.thickness as number) ?? 3.5;
    const bbox = faceBBox(face, w, h);

    const hueOffset = (time * 0.05 * speed) % 360;
    const grad = ctx.createLinearGradient(bbox.x, bbox.y, bbox.x + bbox.w, bbox.y + bbox.h);
    grad.addColorStop(0, `hsl(${hueOffset}, 100%, 60%)`);
    grad.addColorStop(0.33, `hsl(${(hueOffset + 120) % 360}, 100%, 60%)`);
    grad.addColorStop(0.66, `hsl(${(hueOffset + 240) % 360}, 100%, 60%)`);
    grad.addColorStop(1, `hsl(${hueOffset}, 100%, 60%)`);

    ctx.save();
    ctx.strokeStyle = grad;
    ctx.lineWidth = thickness;
    ctx.shadowColor = `hsl(${hueOffset}, 100%, 60%)`;
    ctx.shadowBlur = 14;

    const jaw = [10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136, 172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109];
    ctx.beginPath();
    for (let i = 0; i < jaw.length; i++) {
      const [px, py] = toPixel(face[jaw[i]], w, h);
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.closePath();
    ctx.stroke();
    ctx.restore();
  },
};

// ── Effect 40: Detective Noir (Film Noir & Büyüteç) ──────────────────────────

const detectiveNoir: Effect = {
  id: 'detective-noir',
  name: 'Detective Noir',
  nameTr: 'Dedektif Noir',
  description: 'Monochrome vintage film noir with interactive magnifying glass on eye',
  descriptionTr: 'Siyah-beyaz film noir tonlaması ve göz üzerinde büyüteç efekti',
  category: 'gesture',
  difficulty: 'hard',
  icon: 'search',
  thumbnail: 'linear-gradient(135deg, #1c1917, #78716c, #e7e5e4)',
  requires: ['face'],
  params: [
    { name: 'zoom', label: 'Büyütme Oranı', type: 'number', min: 1.5, max: 3.5, step: 0.2, default: 2.2 },
  ],
  process(ctx, w, h, tracking, params) {
    const face = tracking.face?.[0];
    if (!face) return;
    const zoom = (params.zoom as number) ?? 2.2;
    const eye = toPixel(face[159], w, h); // Left eye

    ctx.save();
    // 1. High contrast sepia noir overlay
    ctx.fillStyle = 'rgba(28, 25, 23, 0.45)';
    ctx.fillRect(0, 0, w, h);

    // 2. Magnifying glass circle over eye
    const mgRadius = 46;
    ctx.save();
    ctx.beginPath();
    ctx.arc(eye[0], eye[1], mgRadius, 0, Math.PI * 2);
    ctx.clip();

    // Zoom magnified image
    ctx.drawImage(
      ctx.canvas,
      eye[0] - mgRadius / zoom,
      eye[1] - mgRadius / zoom,
      (mgRadius * 2) / zoom,
      (mgRadius * 2) / zoom,
      eye[0] - mgRadius,
      eye[1] - mgRadius,
      mgRadius * 2,
      mgRadius * 2,
    );
    ctx.restore();

    // Brass rim & handle
    ctx.strokeStyle = '#d97706';
    ctx.lineWidth = 4;
    ctx.shadowColor = 'rgba(0,0,0,0.8)';
    ctx.shadowBlur = 12;
    ctx.beginPath();
    ctx.arc(eye[0], eye[1], mgRadius, 0, Math.PI * 2);
    ctx.stroke();

    // Handle
    ctx.beginPath();
    ctx.moveTo(eye[0] + mgRadius * 0.7, eye[1] + mgRadius * 0.7);
    ctx.lineTo(eye[0] + mgRadius * 1.5, eye[1] + mgRadius * 1.5);
    ctx.lineWidth = 6;
    ctx.strokeStyle = '#78350f';
    ctx.stroke();

    ctx.restore();
  },
};

// ── Effect 41: Lightning Strike (Elektrik Şimşek) ────────────────────────────

const lightningStrike: Effect = {
  id: 'lightning-strike',
  name: 'Lightning Storm',
  nameTr: 'Elektrik & Şimşek',
  description: 'Crackling electric lightning arcs jumping across hands and fingers',
  descriptionTr: 'Parmaklar ve eller arasında sıçrayan çatırdayan elektrik şimşekleri',
  category: 'hand',
  difficulty: 'medium',
  icon: 'zap',
  thumbnail: 'linear-gradient(135deg, #38bdf8, #818cf8, #c084fc)',
  requires: ['hands'],
  params: [
    { name: 'boltColor', label: 'Şimşek Rengi', type: 'color', default: '#38bdf8' },
    { name: 'branches', label: 'Dal Sayısı', type: 'number', min: 2, max: 8, step: 1, default: 4 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const hands = tracking.hands;
    if (!hands?.length) return;
    const boltColor = (params.boltColor as string) ?? '#38bdf8';
    const branches = (params.branches as number) ?? 4;

    ctx.save();
    ctx.strokeStyle = boltColor;
    ctx.shadowColor = boltColor;
    ctx.shadowBlur = 16;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    for (const hand of hands) {
      if (hand.length < 21) continue;
      const [wx, wy] = toPixel(hand[0], w, h); // Wrist

      for (let b = 0; b < branches; b++) {
        const tipIdx = [4, 8, 12, 16, 20][b % 5];
        const [tx, ty] = toPixel(hand[tipIdx], w, h);

        // Generate jagged lightning path
        ctx.lineWidth = b === 0 ? 3 : 1.8;
        ctx.beginPath();
        ctx.moveTo(wx, wy);

        const steps = 7;
        for (let s = 1; s < steps; s++) {
          const t = s / steps;
          const nx = wx + (tx - wx) * t + (Math.random() - 0.5) * 26;
          const ny = wy + (ty - wy) * t + (Math.random() - 0.5) * 26;
          ctx.lineTo(nx, ny);
        }
        ctx.lineTo(tx, ty);
        ctx.stroke();

        // Tip spark
        ctx.fillStyle = '#ffffff';
        ctx.beginPath();
        ctx.arc(tx, ty, 3.5, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    ctx.restore();
  },
};

// ── Effect 42: Imperial Golden Crown (Altın Kral Tacı) ──────────────────────

const goldenCrown: Effect = {
  id: 'golden-crown',
  name: 'Imperial Crown',
  nameTr: 'Görkemli Altın Taç',
  description: 'Majestic 3D golden crown with sparkling ruby gems and radiant glint',
  descriptionTr: 'Kafa üzerinde 3D altın kral tacı, parlayan yakutlar ve ışık pırıltıları',
  category: 'face',
  difficulty: 'medium',
  icon: 'crown',
  thumbnail: 'linear-gradient(135deg, #f59e0b, #fbbf24, #d97706)',
  requires: ['face'],
  params: [
    { name: 'crownSize', label: 'Taç Boyutu', type: 'number', min: 0.8, max: 1.8, step: 0.1, default: 1.25 },
    { name: 'gemColor', label: 'Mücevher Rengi', type: 'color', default: '#ef4444' },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face || face.length < 468) return;
    const size = (params.crownSize as number) ?? 1.25;
    const gemColor = (params.gemColor as string) ?? '#ef4444';

    const topHead = toPixel(face[10], w, h);
    const leftTemple = toPixel(face[127], w, h);
    const rightTemple = toPixel(face[356], w, h);

    const baseW = Math.hypot(rightTemple[0] - leftTemple[0], rightTemple[1] - leftTemple[1]) * size;
    const crownH = baseW * 0.58;
    const midX = topHead[0];
    const midY = topHead[1] - crownH * 0.45;
    const angle = Math.atan2(rightTemple[1] - leftTemple[1], rightTemple[0] - leftTemple[0]);

    ctx.save();
    ctx.translate(midX, midY);
    ctx.rotate(angle);

    // 1. Golden Crown Base
    const goldGrad = ctx.createLinearGradient(-baseW / 2, -crownH, baseW / 2, 0);
    goldGrad.addColorStop(0, '#f59e0b');
    goldGrad.addColorStop(0.5, '#fef08a');
    goldGrad.addColorStop(1, '#d97706');

    ctx.fillStyle = goldGrad;
    ctx.strokeStyle = '#78350f';
    ctx.lineWidth = 2;
    ctx.shadowColor = '#f59e0b';
    ctx.shadowBlur = 14;

    ctx.beginPath();
    ctx.moveTo(-baseW / 2, 0);
    ctx.lineTo(-baseW / 2 + 5, -crownH * 0.7); // Left peak
    ctx.lineTo(-baseW * 0.25, -crownH * 0.35);
    ctx.lineTo(0, -crownH); // Center high peak
    ctx.lineTo(baseW * 0.25, -crownH * 0.35);
    ctx.lineTo(baseW / 2 - 5, -crownH * 0.7); // Right peak
    ctx.lineTo(baseW / 2, 0);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // 2. Embedded ruby jewels on peaks
    const peaks = [
      [-baseW / 2 + 5, -crownH * 0.7],
      [0, -crownH],
      [baseW / 2 - 5, -crownH * 0.7],
    ];

    for (const [px, py] of peaks) {
      ctx.fillStyle = gemColor;
      ctx.shadowColor = gemColor;
      ctx.shadowBlur = 10;
      ctx.beginPath();
      ctx.arc(px, py + 4, 5, 0, Math.PI * 2);
      ctx.fill();

      // Glint
      ctx.fillStyle = '#fff';
      ctx.beginPath();
      ctx.arc(px - 1.5, py + 2.5, 1.5, 0, Math.PI * 2);
      ctx.fill();
    }

    ctx.restore();
  },
};

// ── Effect 43: Manga Speed Lines (Anime Hız Çizgileri) ──────────────────────

const speedLines: Effect = {
  id: 'speed-lines',
  name: 'Manga Speed',
  nameTr: 'Anime Hız Çizgileri',
  description: 'High-intensity radial manga speed action lines focusing on face',
  descriptionTr: 'Yüz merkezine odaklanan yüksek aksiyonlu anime/manga hız çizgileri',
  category: 'face',
  difficulty: 'easy',
  icon: 'activity',
  thumbnail: 'linear-gradient(135deg, #000000, #333333, #ffffff)',
  requires: ['face'],
  params: [
    { name: 'lineCount', label: 'Çizgi Yoğunluğu', type: 'number', min: 20, max: 80, step: 5, default: 45 },
  ],
  process(ctx, w, h, tracking, params, time) {
    const face = tracking.face?.[0];
    if (!face) return;
    const lineCount = (params.lineCount as number) ?? 45;
    const bbox = faceBBox(face, w, h);

    const maxR = Math.hypot(w, h);
    const minR = bbox.w * 0.75;

    ctx.save();
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.85)';
    ctx.fillStyle = 'rgba(0, 0, 0, 0.9)';

    for (let i = 0; i < lineCount; i++) {
      const angle = (i * Math.PI * 2) / lineCount + (Math.sin(time * 0.01 + i) * 0.05);
      const innerDist = minR + (Math.sin(time * 0.02 + i * 3) * 20);

      const x1 = bbox.cx + Math.cos(angle) * maxR;
      const y1 = bbox.cy + Math.sin(angle) * maxR;
      const x2 = bbox.cx + Math.cos(angle) * innerDist;
      const y2 = bbox.cy + Math.sin(angle) * innerDist;

      ctx.lineWidth = 1.5 + (i % 3) * 1.5;
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    }
    ctx.restore();
  },
};

// ── Custom (Plugin placeholder) ─────────────────────────────────────────────

const customEffect: Effect = {
  id: 'custom',
  name: 'Custom',
  nameTr: 'Özel',
  description: 'User-defined custom effect from plugin',
  descriptionTr: 'Kullanıcı tanımlı özel efekt (plugin)',
  category: 'custom',
  difficulty: 'hard',
  icon: 'puzzle',
  thumbnail: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
  requires: ['face'],
  params: [],
  process() {
    // Custom effects are rendered by the plugin engine
  },
};

// ── Export all effects ───────────────────────────────────────────────────────

export const BUILTIN_EFFECTS: Effect[] = [
  softBlurFace,
  neonOutline,
  animeEyes,
  catEars,
  sunglasses,
  facePaint,
  particleHands,
  laserFingers,
  magicTrail,
  skeletonOverlay,
  energyAura,
  gestureTrigger,
  mirrorFace,
  glitchFace,
  colorGrade,
  crownHalo,
  pixelateFace,
  rainbowTrail,
  forceField,
  discoMode,
  colorOverlay,
  vignette,
  warmTone,
  coolTone,
  filmGrain,
  ghostFace,
  matrixRain,
  cyberpunkVisor,
  fireballPalm,
  iceCrystals,
  hologramMesh,
  cinematicBokeh,
  vhsRetro,
  neonCyberFrame,
  starDust,
  banditMask,
  pixelMosaic,
  rainbowAura,
  detectiveNoir,
  lightningStrike,
  goldenCrown,
  speedLines,
  customEffect,
];

/** Get effect by ID */
export function getEffect(id: string): Effect | undefined {
  return BUILTIN_EFFECTS.find(e => e.id === id);
}

/** Get effects by category */
export function getEffectsByCategory(category: EffectCategory): Effect[] {
  return BUILTIN_EFFECTS.filter(e => e.category === category);
}

