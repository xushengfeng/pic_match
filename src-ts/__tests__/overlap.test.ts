import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache } from '../index';
import { makeScene, crop } from './helpers';

describe('部分重叠（两图互不包含）', () => {
  const size = 400;
  const scene = makeScene(760, 760, 21);

  beforeAll(() => {
    resetCache();
  });

  // 重叠比例 = ((size - d) / size)^2
  const cases = [10, 40, 100, 140, 170, 200, 220, 250, 280, 300, 320, 340];

  for (const d of cases) {
    const overlap = ((size - d) / size) ** 2;
    it(`对角错开 ${d}px（重叠 ${(overlap * 100).toFixed(0)}%）`, async () => {
      const a = crop(scene, 0, 0, size, size);
      const b = crop(scene, d, d, size, size);
      const r = await findOffset(a, b);
      expect(Math.abs(r.dx + d)).toBeLessThan(1.5);
      expect(Math.abs(r.dy + d)).toBeLessThan(1.5);
      expect(r.confidence).toBeGreaterThan(0.9);
      expect(r.overlap).toBeCloseTo(overlap, 2);
    });
  }

  it('只沿一条轴错开 340px（重叠 15%）', async () => {
    const a = crop(scene, 0, 0, size, size);
    const b = crop(scene, 340, 0, size, size);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 340)).toBeLessThan(1.5);
    expect(Math.abs(r.dy)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
    expect(r.overlap).toBeCloseTo(0.15, 2);
  });

  it('尺寸不同、互不包含 (500x400 vs 600x350)', async () => {
    const a = crop(scene, 0, 0, 500, 400);
    const b = crop(scene, 0, 50, 600, 350);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 50)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
    expect(r.overlap).toBeCloseTo(0.875, 2);
  });

  it('尺寸不同、互不包含 (400x400 vs 300x500)', async () => {
    const a = crop(scene, 0, 0, 400, 400);
    const b = crop(scene, 250, 0, 300, 500);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 250)).toBeLessThan(1.5);
    expect(Math.abs(r.dy)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });
});

describe('全景拼接场景', () => {
  const scene = makeScene(1400, 900, 22);

  beforeAll(() => {
    resetCache();
  });

  for (const [dx, dy] of [
    [300, 0],
    [400, 0],
    [480, 0],
    [520, 0],
    [600, 0],
    [400, 20],
  ]) {
    const overlap = ((800 - dx) * (600 - dy)) / (800 * 600);
    it(`800x600 横向平移 ${dx}（重叠 ${(overlap * 100).toFixed(0)}%）`, async () => {
      const a = crop(scene, 0, 0, 800, 600);
      const b = crop(scene, dx, dy, 800, 600);
      const r = await findOffset(a, b);
      expect(Math.abs(r.dx + dx)).toBeLessThan(1.5);
      expect(Math.abs(r.dy + dy)).toBeLessThan(1.5);
      expect(r.confidence).toBeGreaterThan(0.9);
      expect(r.overlap).toBeCloseTo(overlap, 2);
    });
  }
});
