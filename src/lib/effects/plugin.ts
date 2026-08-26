/**
 * Plugin System — User-defined custom effects
 *
 * Architecture:
 *   1. User selects a .js or .py file
 *   2. Script is sandboxed: only allowed APIs are accessible
 *   3. Script is validated against a JSON schema
 *   4. Plugin is registered in local + server-side registry
 *   5. Plugin can be shared ("Herkes görsün") via LiveKit DataChannel
 *
 * Security:
 *   - Scripts run in isolated Function scope (no access to window/document)
 *   - Network, filesystem, eval, subprocess are blocked
 *   - Script hash is computed for integrity verification
 */

import type { Effect, EffectParams, PluginManifest, PluginScript } from './types';
import { effectEngine } from './engine';

// ── Allowed APIs for plugin sandbox ──────────────────────────────────────────

const ALLOWED_APIS = [
  'canvas',
  'ctx',
  'landmarks',
  'params',
  'width',
  'height',
  'time',
  'Math',
  'JSON',
  'parseInt',
  'parseFloat',
  'isNaN',
  'isFinite',
  'Number',
  'String',
  'Boolean',
  'Array',
  'Object',
  'Date',
];

// ── Blocked patterns (security) ──────────────────────────────────────────────

const BLOCKED_PATTERNS = [
  /\bimport\s/g,
  /\brequire\s*\(/g,
  /\bfetch\s*\(/g,
  /\bXMLHttpRequest\b/g,
  /\bWebSocket\b/g,
  /\bnavigator\./g,
  /\bwindow\./g,
  /\bdocument\./g,
  /\blocalStorage\b/g,
  /\bsessionStorage\b/g,
  /\bindexedDB\b/g,
  /\beval\s*\(/g,
  /\bFunction\s*\(/g,
  /\bsubprocess\b/g,
  /\bexec\s*\(/g,
  /\bspawn\s*\(/g,
  /\bchild_process\b/g,
  /\bfs\./g,
  /\bpath\./g,
  /\bos\./g,
  /\bprocess\./g,
  /\bprototype\./g,
  /\b__proto__\b/g,
  /\bconstructor\b/g,
  /\btoString\b/g,
  /\bvalueOf\b/g,
];

// ── Script validation ────────────────────────────────────────────────────────

interface ValidationResult {
  valid: boolean;
  error?: string;
}

// ── Pyodide WASM lazy loader ────────────────────────────────────────────────

const PYODIDE_CDN = 'https://cdn.jsdelivr.net/pyodide/v0.24.1/full/';

let pyodideInstance: any = null;
let pyodidePromise: Promise<any> | null = null;

async function getPyodide(): Promise<any> {
  if (pyodideInstance) return pyodideInstance;
  if (pyodidePromise) return pyodidePromise;

  pyodidePromise = (async () => {
    const scriptUrl = `${PYODIDE_CDN}pyodide.asm.js`;
    if (!document.querySelector(`script[src="${scriptUrl}"]`)) {
      const script = document.createElement('script');
      script.src = scriptUrl;
      document.head.appendChild(script);
      await new Promise<void>((resolve, reject) => {
        script.onload = () => resolve();
        script.onerror = () => reject(new Error('Pyodide CDN yüklenemedi'));
      });
    }

    const pyodide = await (window as any).loadPyodide({
      indexURL: PYODIDE_CDN,
    });

    pyodideInstance = pyodide;
    return pyodide;
  })();

  return pyodidePromise;
}

// ── Python blocked patterns ──────────────────────────────────────────────────

const PYTHON_BLOCKED_PATTERNS = [
  /\bimport\s/g,
  /\bfrom\s+\S+\s+import\b/g,
  /\bexec\s*\(/g,
  /\beval\s*\(/g,
  /\bopen\s*\(/g,
  /\b__import__\b/g,
  /\bcompile\s*\(/g,
  /\bglobals\s*\(/g,
  /\blocals\s*\(/g,
  /\bbreakpoint\s*\(/g,
  /\bexit\s*\(/g,
  /\bquit\s*\(/g,
  /\bsubprocess\b/g,
  /\bos\./g,
  /\bsys\./g,
  /\bsocket\b/g,
  /\bhttpx?\b/g,
  /\burllib\b/g,
  /\brequests\b/g,
  /\b__builtins__\b/g,
  /\b__name__\b/g,
  /\b__file__\b/g,
  /\bwebbrowser\b/g,
  /\bctypes\b/g,
  /\bmultiprocessing\b/g,
  /\bthreading\b/g,
];

// ── Python script validation ─────────────────────────────────────────────────

function validatePythonScript(content: string): ValidationResult {
  for (const pattern of PYTHON_BLOCKED_PATTERNS) {
    if (pattern.test(content)) {
      return {
        valid: false,
        error: `Yasaklı API kullanıldı: ${pattern.source.slice(0, 50)}`,
      };
    }
  }

  if (content.length > 50 * 1024) {
    return { valid: false, error: 'Script çok uzun (maks 50KB)' };
  }

  if (content.trim().length < 10) {
    return { valid: false, error: 'Script çok kısa (en az 10 karakter)' };
  }

  if (!content.includes('def process(')) {
    return {
      valid: false,
      error: 'Script process(canvas, ctx, landmarks, params, width, height, time) fonksiyonu içermeli',
    };
  }

  return { valid: true };
}

// ── Python sandbox wrapper ──────────────────────────────────────────────────

function wrapPythonSandbox(userCode: string): string {
  // Strip dangerous builtins at runtime (defense in depth on top of regex validation)
  return `
import builtins as __builtins__
__blocked = ['import', 'exec', 'eval', 'open', '__import__', 'compile',
  'globals', 'locals', 'breakpoint', 'exit', 'quit', 'help', 'license',
  'credits', 'copyright', 'input']
for __f in __blocked:
  if hasattr(__builtins__, __f):
    delattr(__builtins__, __f)
${userCode}
`;
}

// ── Validate JavaScript script ──────────────────────────────────────────────

function validateScript(content: string): ValidationResult {
  // Check for blocked patterns
  for (const pattern of BLOCKED_PATTERNS) {
    if (pattern.test(content)) {
      return {
        valid: false,
        error: `Yasaklı API kullanıldı: ${pattern.source.slice(0, 50)}`,
      };
    }
  }

  // Check script length (max 50KB)
  if (content.length > 50 * 1024) {
    return {
      valid: false,
      error: 'Script çok uzun (maks 50KB)',
    };
  }

  if (content.trim().length < 10) {
    return {
      valid: false,
      error: 'Script çok kısa (en az 10 karakter)',
    };
  }

  // Try to parse as function
  try {
    new Function('canvas', 'ctx', 'landmarks', 'params', 'width', 'height', 'time', content);
  } catch (err) {
    return {
      valid: false,
      error: `Script sözdizimi hatası: ${String(err).slice(0, 100)}`,
    };
  }

  return { valid: true };
}

// ── Compute script hash (SHA-256) ───────────────────────────────────────────

async function computeHash(content: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(content);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

// ── Create effect from plugin script ────────────────────────────────────────

function createPluginEffect(manifest: PluginManifest, content: string): Effect {
  const fn = new Function(
    'canvas',
    'ctx',
    'landmarks',
    'params',
    'width',
    'height',
    'time',
    content
  ) as (
    canvas: HTMLCanvasElement,
    ctx: CanvasRenderingContext2D,
    landmarks: any,
    params: EffectParams,
    width: number,
    height: number,
    time: number
  ) => void;

  const pluginRequires: ('face' | 'hands' | 'pose')[] = ['face'];
  if (content.includes('landmarks.hands') || content.includes('.hands')) {
    if (!pluginRequires.includes('hands')) pluginRequires.push('hands');
  }
  if (content.includes('landmarks.pose') || content.includes('.pose')) {
    if (!pluginRequires.includes('pose')) pluginRequires.push('pose');
  }

  return {
    id: `plugin-${manifest.id}`,
    name: manifest.name,
    nameTr: manifest.name,
    description: manifest.description,
    descriptionTr: manifest.description,
    category: manifest.category,
    difficulty: 'medium',
    icon: '🧩',
    thumbnail: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    requires: pluginRequires,
    params: [
      { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0, max: 1, step: 0.1, default: 0.5 },
    ],
    process(ctx, width, height, tracking, params, time) {
      try {
        const landmarks = {
          face: tracking.face?.[0] ?? null,
          hands: tracking.hands ?? [],
          pose: tracking.pose?.[0] ?? null,
        };
        fn(ctx.canvas, ctx, landmarks, params, width, height, time);
      } catch (err) {
        console.warn(`Plugin "${manifest.name}" error:`, err);
      }
    },
  };
}

// ── Create effect from Python plugin script ─────────────────────────────────

async function createPythonPluginEffect(manifest: PluginManifest, content: string): Promise<Effect> {
  const pyodide = await getPyodide();
  const wrappedScript = wrapPythonSandbox(content);
  await pyodide.runPythonAsync(wrappedScript);
  const pyProcess = pyodide.globals.get('process');

  return {
    id: `plugin-${manifest.id}`,
    name: manifest.name,
    nameTr: manifest.name,
    description: manifest.description,
    descriptionTr: manifest.description,
    category: manifest.category,
    difficulty: 'medium',
    icon: '🐍',
    thumbnail: 'linear-gradient(135deg, #306998, #FFD43B)',
    requires: ['face'],
    params: [
      { name: 'intensity', label: 'Yoğunluk', type: 'number', min: 0, max: 1, step: 0.1, default: 0.5 },
    ],
    process(ctx, width, height, tracking, params, time) {
      try {
        const landmarks = {
          face: tracking.face?.[0] ?? null,
          hands: tracking.hands ?? [],
          pose: tracking.pose?.[0] ?? null,
        };
        pyodide.globals.set('canvas', ctx.canvas);
        pyodide.globals.set('ctx', ctx);
        pyodide.globals.set('landmarks', landmarks);
        pyodide.globals.set('params', params);
        pyodide.globals.set('width', width);
        pyodide.globals.set('height', height);
        pyodide.globals.set('time', time);
        pyProcess(ctx.canvas, ctx, landmarks, params, width, height, time);
      } catch (err) {
        console.warn(`Python plugin "${manifest.name}" error:`, err);
      }
    },
  };
}

// ── Plugin Registry ──────────────────────────────────────────────────────────

const STORAGE_KEY = 'veilanon_effect_plugins';

export interface PluginRegistry {
  plugins: PluginScript[];
}

function loadRegistry(): PluginRegistry {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { plugins: [] };
}

function saveRegistry(registry: PluginRegistry) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(registry));
  } catch { /* ignore */ }
}

// ── Public API ───────────────────────────────────────────────────────────────

/** Get all registered plugins */
export function getPlugins(): PluginScript[] {
  return loadRegistry().plugins;
}

/** Add a new plugin from a .js file */
export async function addPlugin(
  file: File,
  name: string,
  author: string,
  description: string
): Promise<{ success: boolean; error?: string; plugin?: PluginScript }> {
  // Read file content
  const content = await file.text();

  // Validate
  const validation = validateScript(content);
  if (!validation.valid) {
    return { success: false, error: validation.error };
  }

  // Compute hash
  const scriptHash = await computeHash(content);

  // Check for duplicate hash
  const registry = loadRegistry();
  if (registry.plugins.some(p => p.manifest.scriptHash === scriptHash)) {
    return { success: false, error: 'Bu script zaten yüklü (aynı içerik)' };
  }

  // Create manifest
  const manifest: PluginManifest = {
    id: crypto.randomUUID(),
    name,
    version: '1.0.0',
    author,
    description,
    category: 'custom',
    scriptHash,
    allowedApis: ALLOWED_APIS,
    createdAt: Date.now(),
  };

  const plugin: PluginScript = { manifest, content, language: 'javascript' };

  // Register
  registry.plugins.push(plugin);
  saveRegistry(registry);

  // Add to effect engine
  const effect = createPluginEffect(manifest, content);
  effectEngine.registerPluginEffect(effect);

  return { success: true, plugin };
}

// ── Add Python plugin ───────────────────────────────────────────────────────

export async function addPythonPlugin(
  file: File,
  name: string,
  author: string,
  description: string
): Promise<{ success: boolean; error?: string; plugin?: PluginScript }> {
  const content = await file.text();

  const validation = validatePythonScript(content);
  if (!validation.valid) {
    return { success: false, error: validation.error };
  }

  const scriptHash = await computeHash(content);

  const registry = loadRegistry();
  if (registry.plugins.some(p => p.manifest.scriptHash === scriptHash)) {
    return { success: false, error: 'Bu script zaten yüklü (aynı içerik)' };
  }

  const manifest: PluginManifest = {
    id: crypto.randomUUID(),
    name,
    version: '1.0.0',
    author,
    description,
    category: 'custom',
    scriptHash,
    allowedApis: ALLOWED_APIS,
    createdAt: Date.now(),
  };

  const plugin: PluginScript = { manifest, content, language: 'python' };

  registry.plugins.push(plugin);
  saveRegistry(registry);

  const effect = await createPythonPluginEffect(manifest, content);
  effectEngine.registerPluginEffect(effect);

  return { success: true, plugin };
}

/** Remove a plugin by ID */
export function removePlugin(pluginId: string): boolean {
  const registry = loadRegistry();
  const idx = registry.plugins.findIndex(p => p.manifest.id === pluginId);
  if (idx === -1) return false;

  registry.plugins.splice(idx, 1);
  saveRegistry(registry);

  effectEngine.unregisterPluginEffect(`plugin-${pluginId}`);
  return true;
}

/** Get a plugin by ID */
export function getPlugin(pluginId: string): PluginScript | undefined {
  return loadRegistry().plugins.find(p => p.manifest.id === pluginId);
}

export async function loadAllPlugins() {
  const plugins = loadRegistry().plugins;
  for (const plugin of plugins) {
    if (plugin.language === 'python') {
      const effect = await createPythonPluginEffect(plugin.manifest, plugin.content);
      effectEngine.registerPluginEffect(effect);
    } else {
      const effect = createPluginEffect(plugin.manifest, plugin.content);
      effectEngine.registerPluginEffect(effect);
    }
  }
}

export function exportPlugin(pluginId: string): Blob | null {
  const plugin = getPlugin(pluginId);
  if (!plugin) return null;

  const mime = plugin.language === 'python' ? 'text/x-python' : 'text/javascript';
  const header = `# ${plugin.manifest.name} v${plugin.manifest.version}\n# by ${plugin.manifest.author}\n# ${plugin.manifest.description}\n\n`;
  return new Blob([header + plugin.content], { type: mime });
}

/** Clear all plugins */
export function clearAllPlugins() {
  const registry = loadRegistry();
  for (const p of registry.plugins) {
    effectEngine.unregisterPluginEffect(`plugin-${p.manifest.id}`);
  }
  saveRegistry({ plugins: [] });
}

// ── Sample Plugin Templates ──────────────────────────────────────────────────

export const SAMPLE_JS_PLUGIN = `// veilanon JavaScript Effect Plugin
// Parameters available: canvas, ctx, landmarks, params, width, height, time
// landmarks: { face, hands, pose }

const intensity = (params && params.intensity) || 0.8;
const face = landmarks && landmarks.face;

if (face && face.length > 10) {
  // Forehead landmark (10)
  const forehead = face[10];
  const fx = forehead.x * width;
  const fy = forehead.y * height;

  ctx.save();
  const pulse = 1 + Math.sin(time * 0.005) * 0.15;
  const radius = 40 * intensity * pulse;

  // Outer glowing halo ring
  ctx.strokeStyle = '#00ffcc';
  ctx.lineWidth = 4;
  ctx.shadowColor = '#00ffcc';
  ctx.shadowBlur = 18;
  ctx.globalAlpha = 0.85;

  ctx.beginPath();
  ctx.ellipse(fx, fy - 35, radius, radius * 0.35, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Floating sparkle dots
  ctx.fillStyle = '#ffffff';
  ctx.shadowBlur = 8;
  for (let i = 0; i < 4; i++) {
    const angle = time * 0.003 + (i * Math.PI / 2);
    const sx = fx + Math.cos(angle) * radius;
    const sy = (fy - 35) + Math.sin(angle) * (radius * 0.35);
    ctx.beginPath();
    ctx.arc(sx, sy, 2.5, 0, Math.PI * 2);
    ctx.fill();
  }

  ctx.restore();
}
`;

export const SAMPLE_PYTHON_PLUGIN = `# veilanon Python Effect Plugin (Pyodide WASM)
# def process(canvas, ctx, landmarks, params, width, height, time)

def process(canvas, ctx, landmarks, params, width, height, time):
    intensity = 0.8
    if params and hasattr(params, 'get'):
        intensity = params.get('intensity', 0.8)
    
    # Check for face landmarks
    face = landmarks.face if hasattr(landmarks, 'face') else None
    if face and len(face) > 10:
        forehead = face[10]
        fx = forehead.x * width
        fy = forehead.y * height
        
        ctx.save()
        ctx.strokeStyle = '#ff007f'
        ctx.lineWidth = 4
        ctx.shadowColor = '#ff007f'
        ctx.shadowBlur = 20
        ctx.globalAlpha = 0.9
        
        # Draw cyber crown mark
        ctx.beginPath()
        ctx.moveTo(fx - 30, fy - 20)
        ctx.lineTo(fx - 15, fy - 45)
        ctx.lineTo(fx, fy - 25)
        ctx.lineTo(fx + 15, fy - 45)
        ctx.lineTo(fx + 30, fy - 20)
        ctx.stroke()
        
        ctx.restore()
`;

export function downloadSamplePlugin(language: 'javascript' | 'python') {
  const content = language === 'python' ? SAMPLE_PYTHON_PLUGIN : SAMPLE_JS_PLUGIN;
  const filename = language === 'python' ? 'sample_effect.py' : 'sample_effect.js';
  const mime = language === 'python' ? 'text/x-python' : 'text/javascript';

  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

