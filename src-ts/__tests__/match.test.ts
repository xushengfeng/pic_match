import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache } from '../index';
import { makeScene, crop, flat } from './helpers';

describe('pic_match 基础匹配', () => {
  beforeAll(() => {
    resetCache();
  });

  it('相同图像返回零偏移，置信度为 1', async () => {
    const img = makeScene(96, 96, 7);
    const r = await findOffset(img, img);
    expect(Math.abs(r.dx)).toBeLessThan(0.5);
    expect(Math.abs(r.dy)).toBeLessThan(0.5);
    expect(r.confidence).toBeGreaterThan(0.99);
    expect(r.overlap).toBeCloseTo(1, 6);
  });

  it('检测水平偏移', async () => {
    const scene = makeScene(160, 160, 11);
    const a = crop(scene, 0, 0, 128, 128);
    const b = crop(scene, 8, 0, 128, 128);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 8)).toBeLessThan(1.5);
    expect(Math.abs(r.dy)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('检测垂直偏移', async () => {
    const scene = makeScene(160, 160, 12);
    const a = crop(scene, 0, 0, 128, 128);
    const b = crop(scene, 0, 12, 128, 128);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 12)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('检测二维偏移', async () => {
    const scene = makeScene(160, 160, 13);
    const a = crop(scene, 0, 0, 128, 128);
    const b = crop(scene, 5, 7, 128, 128);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 5)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 7)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('检测负向偏移', async () => {
    const scene = makeScene(160, 160, 14);
    const a = crop(scene, 20, 20, 128, 128);
    const b = crop(scene, 15, 17, 128, 128);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx - 5)).toBeLessThan(1.5);
    expect(Math.abs(r.dy - 3)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('支持不同画布尺寸（大图 vs 局部裁剪）', async () => {
    const scene = makeScene(520, 420, 15);
    const a = crop(scene, 0, 0, 400, 400);
    const b = crop(scene, 60, 40, 200, 200);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 60)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 40)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('支持不同长宽比', async () => {
    const scene = makeScene(900, 600, 16);
    const a = crop(scene, 0, 0, 800, 400);
    const b = crop(scene, 100, 50, 300, 200);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 100)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 50)).toBeLessThan(1.5);
    expect(r.confidence).toBeGreaterThan(0.9);
  });

  it('大图找小图时，偏移可以超出半张画布', async () => {
    // 旧实现把可表示的偏移限制在 ±next_pow2(wa+wb)/2 ≈ 256，这里真值是 -300
    const scene = makeScene(720, 720, 17);
    const a = crop(scene, 0, 0, 400, 400);
    const b = crop(scene, 300, 300, 100, 100);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 300)).toBeLessThan(2.5);
    expect(Math.abs(r.dy + 300)).toBeLessThan(2.5);
    expect(r.confidence).toBeGreaterThan(0.9);
    expect(r.overlap).toBeCloseTo(1, 6);
  });

  it('无关图像给出低置信度', async () => {
    const a = crop(makeScene(800, 800, 18), 0, 0, 400, 400);
    const b = crop(makeScene(800, 800, 19), 0, 0, 400, 400);
    const r = await findOffset(a, b);
    expect(r.confidence).toBeLessThan(0.3);
  });

  it('纯色图像没有可匹配的内容', async () => {
    const r = await findOffset(flat(200, 200), flat(200, 200));
    expect(r.confidence).toBe(0);
  });
});
