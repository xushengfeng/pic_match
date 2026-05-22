export interface MatchResult {
  dx: number;
  dy: number;
  confidence: number;
}

export interface ImageData {
  data: Uint8ClampedArray;
  width: number;
  height: number;
}

let wasmModule: any = null;

async function loadWasm() {
  if (wasmModule) return wasmModule;

  const mod = await import('../pkg/pic_match.js');

  if (typeof process !== 'undefined' && process.versions?.node) {
    const { readFileSync } = await import('fs');
    const { resolve, dirname } = await import('path');
    const { fileURLToPath } = await import('url');
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const wasmPath = resolve(__dirname, '../pkg/pic_match_bg.wasm');
    const wasmBytes = readFileSync(wasmPath);
    mod.initSync({ module: wasmBytes });
  } else {
    await mod.default();
  }

  wasmModule = mod;
  return mod;
}

export async function findOffset(
  imageA: ImageData,
  imageB: ImageData
): Promise<MatchResult> {
  const wasm = await loadWasm();

  const result = wasm.find_offset(
    new Uint8Array(imageA.data.buffer),
    imageA.width,
    imageA.height,
    new Uint8Array(imageB.data.buffer),
    imageB.width,
    imageB.height
  );

  return result as MatchResult;
}

export function resetCache() {
  wasmModule = null;
}
