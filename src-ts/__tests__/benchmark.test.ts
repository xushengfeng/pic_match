import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache } from '../index';
import { makeScene, crop } from './helpers';

function formatMs(ms: number): string {
  return ms < 1000 ? `${ms.toFixed(1)}ms` : `${(ms / 1000).toFixed(2)}s`;
}

/** 跑多次取最小值，减少 CI 抖动 */
async function measure(fn: () => Promise<unknown>, runs = 3): Promise<number> {
  let best = Infinity;
  for (let i = 0; i < runs; i++) {
    const start = performance.now();
    await fn();
    best = Math.min(best, performance.now() - start);
  }
  return best;
}

describe('性能基准', () => {
  beforeAll(() => {
    resetCache();
  });

  const sizes: Array<[number, number, number, number, number]> = [
    // [w, h, dx, dy, 预算 ms]
    [64, 64, 4, 4, 300],
    [128, 128, 10, 8, 400],
    [256, 256, 20, 16, 500],
    [512, 512, 40, 32, 900],
    [1024, 1024, 80, 64, 1200],
    [1920, 1080, 100, 80, 1200],
    [3840, 2160, 120, 90, 2500],
  ];

  for (const [w, h, dx, dy, budget] of sizes) {
    it(`${w}x${h}`, async () => {
      const scene = makeScene(w + dx, h + dy, 41);
      const a = crop(scene, 0, 0, w, h);
      const b = crop(scene, dx, dy, w, h);
      const ms = await measure(() => findOffset(a, b));
      console.log(`    ${w}x${h}: ${formatMs(ms)}（预算 ${formatMs(budget)}）`);
      expect(ms).toBeLessThan(budget);
    });
  }

  it('1920x1080 里找 120px 图标（大图找小图）', async () => {
    const screen = makeScene(1920, 1080, 42);
    const tpl = crop(screen, 700, 400, 120, 120);
    const ms = await measure(() => findOffset(screen, tpl));
    console.log(`    1920x1080 找小图: ${formatMs(ms)}（预算 1.20s）`);
    expect(ms).toBeLessThan(1200);
  });

  it('3840x2160 里找 120px 图标（大图找小图）', async () => {
    const screen = makeScene(3840, 2160, 43);
    const tpl = crop(screen, 1400, 800, 120, 120);
    const ms = await measure(() => findOffset(screen, tpl));
    console.log(`    3840x2160 找小图: ${formatMs(ms)}（预算 2.50s）`);
    expect(ms).toBeLessThan(2500);
  });
});
