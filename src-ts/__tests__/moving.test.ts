import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache } from '../index';
import { makeScene, paste, affine } from './helpers';

/**
 * 画面里同时存在"不动的部分"和"发生偏移的部分"时，应返回偏移部分的位移，
 * 而不是把不动部分当成唯一答案；有多个偏移时，取涉及面积更大的那个。
 */
describe('不动画面与偏移画面共存', () => {
  beforeAll(() => {
    resetCache();
  });

  /** A 中物体在 (x, y)，B 中同一物体在 (x + dx, y + dy)，其余背景保持不动 */
  function movingObject(
    size: number,
    objectSize: number,
    at: [number, number],
    delta: [number, number],
    seed = 61,
  ) {
    const background = makeScene(size, size, seed);
    const object = makeScene(objectSize, objectSize, seed + 1);
    const a = paste(background, object, at[0], at[1]);
    const b = paste(background, object, at[0] + delta[0], at[1] + delta[1]);
    return { a, b };
  }

  it('背景不动、物体偏移时返回物体的偏移而不是 0', async () => {
    const { a, b } = movingObject(256, 64, [80, 80], [18, 12]);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx - 18)).toBeLessThan(1.5);
    expect(Math.abs(r.dy - 12)).toBeLessThan(1.5);
  });

  it('偏移区域比不动区域小时也选偏移', async () => {
    const { a, b } = movingObject(320, 48, [120, 100], [-24, 16], 71);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 24)).toBeLessThan(1.5);
    expect(Math.abs(r.dy - 16)).toBeLessThan(1.5);
  });

  it('多个偏移时选涉及面积更大的那个', async () => {
    const background = makeScene(320, 320, 81);
    // 小物体对比度更高（相位相关峰更强），大物体对比度更低，验证按面积而不是峰值选择
    const small = affine(makeScene(56, 56, 82), 1.6, -70);
    const large = affine(makeScene(72, 72, 83), 0.5, 80);

    // 小物体向右下移动，大物体向左上移动
    let a = paste(background, small, 40, 40);
    a = paste(a, large, 180, 180);
    let b = paste(background, small, 40 + 20, 40 + 14);
    b = paste(b, large, 180 - 16, 180 - 22);

    const r = await findOffset(a, b);
    expect(Math.abs(r.dx + 16)).toBeLessThan(1.5);
    expect(Math.abs(r.dy + 22)).toBeLessThan(1.5);
  });

  it('只有不动画面时仍然返回 0', async () => {
    const { a, b } = movingObject(256, 64, [80, 80], [0, 0]);
    const r = await findOffset(a, b);
    expect(Math.abs(r.dx)).toBeLessThan(0.5);
    expect(Math.abs(r.dy)).toBeLessThan(0.5);
  });
});
