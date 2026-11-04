import type { ImageData } from "../index";

/**
 * 生成确定性的多倍频值噪声图像，用作"场景"。
 * 既有低频结构也有细节，接近真实照片的频谱分布，且同一 seed 完全可复现。
 */
export function makeScene(width: number, height: number, seed = 1): ImageData {
  let state = seed >>> 0;
  const rand = () => {
    state = (state * 1664525 + 1013904223) >>> 0;
    return state / 4294967296;
  };

  const acc = new Float64Array(width * height);
  for (let cell = 48; cell >= 2; cell = Math.floor(cell / 2)) {
    const gw = Math.ceil(width / cell) + 2;
    const gh = Math.ceil(height / cell) + 2;
    const grid = new Float64Array(gw * gh);
    for (let i = 0; i < grid.length; i++) grid[i] = rand();

    for (let y = 0; y < height; y++) {
      const fy = y / cell;
      const y0 = Math.floor(fy);
      const ty = fy - y0;
      const sy = ty * ty * (3 - 2 * ty);
      for (let x = 0; x < width; x++) {
        const fx = x / cell;
        const x0 = Math.floor(fx);
        const tx = fx - x0;
        const sx = tx * tx * (3 - 2 * tx);
        const v =
          grid[y0 * gw + x0] * (1 - sx) * (1 - sy) +
          grid[y0 * gw + x0 + 1] * sx * (1 - sy) +
          grid[(y0 + 1) * gw + x0] * (1 - sx) * sy +
          grid[(y0 + 1) * gw + x0 + 1] * sx * sy;
        acc[y * width + x] += v * cell;
      }
    }
  }

  let min = Infinity;
  let max = -Infinity;
  for (let i = 0; i < acc.length; i++) {
    if (acc[i] < min) min = acc[i];
    if (acc[i] > max) max = acc[i];
  }
  const span = max - min || 1;

  const data = new Uint8ClampedArray(width * height * 4);
  for (let i = 0; i < acc.length; i++) {
    const v = Math.round(20 + ((acc[i] - min) / span) * 215);
    data[i * 4] = v;
    data[i * 4 + 1] = v;
    data[i * 4 + 2] = v;
    data[i * 4 + 3] = 255;
  }
  return { data, width, height };
}

/** 从图像中裁一块 */
export function crop(img: ImageData, x: number, y: number, w: number, h: number): ImageData {
  const data = new Uint8ClampedArray(w * h * 4);
  for (let row = 0; row < h; row++) {
    const start = ((y + row) * img.width + x) * 4;
    data.set(img.data.subarray(start, start + w * 4), row * w * 4);
  }
  return { data, width: w, height: h };
}

/** 双线性缩放，模拟"同一张图的不同分辨率" */
export function resize(img: ImageData, w: number, h: number): ImageData {
  const { data: src, width: sw, height: sh } = img;
  const data = new Uint8ClampedArray(w * h * 4);
  for (let y = 0; y < h; y++) {
    const fy = (y * (sh - 1)) / Math.max(1, h - 1);
    const y0 = Math.floor(fy);
    const y1 = Math.min(y0 + 1, sh - 1);
    const ty = fy - y0;
    for (let x = 0; x < w; x++) {
      const fx = (x * (sw - 1)) / Math.max(1, w - 1);
      const x0 = Math.floor(fx);
      const x1 = Math.min(x0 + 1, sw - 1);
      const tx = fx - x0;
      for (let c = 0; c < 4; c++) {
        data[(y * w + x) * 4 + c] =
          src[(y0 * sw + x0) * 4 + c] * (1 - tx) * (1 - ty) +
          src[(y0 * sw + x1) * 4 + c] * tx * (1 - ty) +
          src[(y1 * sw + x0) * 4 + c] * (1 - tx) * ty +
          src[(y1 * sw + x1) * 4 + c] * tx * ty;
      }
    }
  }
  return { data, width: w, height: h };
}

/** 转灰度后做仿射变换，模拟亮度/对比度变化 */
export function affine(img: ImageData, mul: number, add: number): ImageData {
  const data = new Uint8ClampedArray(img.data.length);
  for (let i = 0; i < data.length; i += 4) {
    const g = 0.299 * img.data[i] + 0.587 * img.data[i + 1] + 0.114 * img.data[i + 2];
    const v = mul * g + add;
    data[i] = v;
    data[i + 1] = v;
    data[i + 2] = v;
    data[i + 3] = 255;
  }
  return { data, width: img.width, height: img.height };
}

/** 把一块矩形涂成固定灰度，模拟遮挡 */
export function occlude(
  img: ImageData,
  x: number,
  y: number,
  w: number,
  h: number,
  value = 255,
): ImageData {
  const data = new Uint8ClampedArray(img.data);
  for (let row = y; row < y + h; row++) {
    for (let col = x; col < x + w; col++) {
      const i = (row * img.width + col) * 4;
      data[i] = value;
      data[i + 1] = value;
      data[i + 2] = value;
    }
  }
  return { data, width: img.width, height: img.height };
}

/** 把 src 覆盖到 dst 的 (x, y) 处，返回新的图像 */
export function paste(dst: ImageData, src: ImageData, x: number, y: number): ImageData {
  const data = new Uint8ClampedArray(dst.data);
  for (let row = 0; row < src.height; row++) {
    const py = y + row;
    if (py < 0 || py >= dst.height) continue;
    const cols = Math.max(0, Math.min(src.width, dst.width - x));
    if (cols <= 0) continue;
    const from = (row * src.width) * 4;
    const to = (py * dst.width + x) * 4;
    data.set(src.data.subarray(from, from + cols * 4), to);
  }
  return { data, width: dst.width, height: dst.height };
}

/** 一片纯色图 */
export function flat(width: number, height: number, value = 128): ImageData {
  const data = new Uint8ClampedArray(width * height * 4).fill(value);
  for (let i = 3; i < data.length; i += 4) data[i] = 255;
  return { data, width, height };
}
