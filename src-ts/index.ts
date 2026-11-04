export interface MatchResult {
	/** X 方向偏移量（正数表示 imageB 相对于 imageA 右移） 精度为小数，提供亚像素偏移 */
	dx: number;
	/** Y 方向偏移量（正数表示 imageB 相对于 imageA 下移） 精度为小数，提供亚像素偏移 */
	dy: number;
	/**
	 * 匹配程度：两张图重叠区域的归一化互相关（NCC），取值 -1 ~ 1。
	 * 1 表示重叠区完全一致；一般 ≥0.8 可信，<0.4 基本不可信（例如尺度不同、内容无关）。
	 */
	confidence: number;
	/** 重叠面积占较小一张图的比例，取值 0 ~ 1。 */
	overlap: number;
	/** 互功率谱的峰值高度，调试用。 */
	peak: number;
	/** 峰值旁瓣比（PSR），调试用。 */
	psr: number;
}

export interface ImageData {
	data: Uint8ClampedArray;
	width: number;
	height: number;
}

let wasmModule: typeof import("../pkg/pic_match.js") | null = null;

async function loadWasm() {
	if (wasmModule) return wasmModule;

	const mod = await import("../pkg/pic_match.js");
	await mod.default();

	wasmModule = mod;
	return mod;
}

export async function findOffset(
	imageA: ImageData,
	imageB: ImageData,
): Promise<MatchResult> {
	const wasm = await loadWasm();

	const result = wasm.find_offset(
		new Uint8Array(imageA.data.buffer),
		imageA.width,
		imageA.height,
		new Uint8Array(imageB.data.buffer),
		imageB.width,
		imageB.height,
	);

	return result as MatchResult;
}

export function resetCache() {
	wasmModule = null;
}
