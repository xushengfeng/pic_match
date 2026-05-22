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

function base64ToBytes(base64: string): Uint8Array {
  if (typeof Buffer !== 'undefined') {
    return new Uint8Array(Buffer.from(base64, 'base64'));
  }
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

async function loadWasm() {
  if (wasmModule) return wasmModule;

  const { WASM_BINARY } = await import('./wasm-binary');
  const mod = await import('../pkg/pic_match.js');
  const wasmBytes = base64ToBytes(WASM_BINARY);
  mod.initSync({ module: wasmBytes });

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
