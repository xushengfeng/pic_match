import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache } from '../index';
import { makeScene, crop, affine, occlude } from './helpers';

describe('模板匹配（大图里找小图）', () => {
  const scene = makeScene(900, 900, 31);

  beforeAll(() => {
    resetCache();
  });

  const a = crop(scene, 0, 0, 480, 480);

  for (const [x, y] of [
    [200, 180],
    [0, 0],
    [40, 40],
    [360, 360],
    [360, 0],
    [0, 360],
  ]) {
    it(`模板位于 (${x},${y})`, async () => {
      const b = crop(scene, x, y, 120, 120);
      const r = await findOffset(a, b);
      expect(Math.abs(r.dx + x)).toBeLessThan(1.5);
      expect(Math.abs(r.dy + y)).toBeLessThan(1.5);
      expect(r.confidence).toBeGreaterThan(0.9);
      expect(r.overlap).toBeCloseTo(1, 6);
    });
  }

  it('模板整体变亮仍可匹配', async () => {
    const b = affine(crop(scene, 200, 180, 120, 120), 0.8, 40);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 200)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 180)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('模板被遮挡一半仍能定位（置信度相应下降）', async () => {
    const b = occlude(crop(scene, 200, 180, 120, 120), 0, 0, 120, 60, 255);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 200)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 180)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.2);
    expect(r.confidence).toBeLessThan(0.9);
  });
});

describe('模板匹配：1080p / 4K 屏幕', () => {
  const screen = makeScene(1920, 1080, 32);

  beforeAll(() => {
    resetCache();
  });

  for (const size of [240, 120, 96, 64]) {
    it(`1920x1080 里找 ${size}px 图标`, async () => {
      const tpl = crop(screen, 700, 400, size, size);
      const r = await findOffset(screen, tpl);
      expect(Math.abs(r.dx + 700)).toBeLessThan(2.5);
      expect(Math.abs(r.dy + 400)).toBeLessThan(2.5);
      expect(r.confidence).toBeGreaterThan(0.8);
    });
  }

  it('图标靠近左边缘', async () => {
    const tpl = crop(screen, 40, 400, 120, 120);
    const r = await findOffset(screen, tpl);
    expect(Math.abs(r.dx + 40)).toBeLessThan(2.5);
    expect(Math.abs(r.dy + 400)).toBeLessThan(2.5);
    expect(r.confidence).toBeGreaterThan(0.8);
  });

  it('图标位于右半屏（旧实现会折返）', async () => {
    const tpl = crop(screen, 1750, 400, 120, 120);
    const r = await findOffset(screen, tpl);
    expect(Math.abs(r.dx + 1750)).toBeLessThan(2.5);
    expect(Math.abs(r.dy + 400)).toBeLessThan(2.5);
    expect(r.confidence).toBeGreaterThan(0.8);
  });
});
