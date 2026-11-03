import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache, MatchResult } from '../index';
import { makeScene, crop, affine, occlude, resize } from './helpers';

type Strength = 'strong' | 'partial' | 'none';

/**
 * 把各种场景的匹配结果打印成一张表：误差、置信度（重叠区 NCC）、重叠比例。
 * 用来直观看到"匹配到什么程度"。
 */
describe('匹配程度报告', () => {
  beforeAll(() => {
    resetCache();
  });

  it('打印各场景的误差与置信度', async () => {
    const scene = makeScene(1200, 1000, 51);
    const rows: string[] = [];
    let failures = 0;

    const run = async (
      name: string,
      a: { data: Uint8ClampedArray; width: number; height: number },
      b: { data: Uint8ClampedArray; width: number; height: number },
      expectDx: number,
      expectDy: number,
      strength: Strength,
    ) => {
      const r: MatchResult = await findOffset(a, b);
      const err = Math.hypot(r.dx - expectDx, r.dy - expectDy);
      let ok: boolean;
      if (strength === 'strong') {
        ok = err <= 2.5 && r.confidence >= 0.8;
      } else if (strength === 'partial') {
        ok = err <= 2.5 && r.confidence >= 0.2 && r.confidence < 0.8;
      } else {
        ok = r.confidence < 0.5;
      }
      if (!ok) failures++;
      rows.push(
        `${ok ? '✓' : '✗'} ${name.padEnd(30)}` +
          `期望=(${expectDx.toFixed(0)},${expectDy.toFixed(0)})`.padEnd(20) +
          `得到=(${r.dx.toFixed(1)},${r.dy.toFixed(1)})`.padEnd(22) +
          `误差=${err.toFixed(2)}px`.padEnd(14) +
          `NCC=${r.confidence.toFixed(3)}`.padEnd(12) +
          `重叠=${(r.overlap * 100).toFixed(0)}%`,
      );
    };

    const a400 = crop(scene, 0, 0, 400, 400);
    await run('完全相同', a400, a400, 0, 0, 'strong');
    await run('平移 1px', a400, crop(scene, 1, 0, 400, 400), -1, 0, 'strong');
    await run('平移 10px', a400, crop(scene, 10, 0, 400, 400), -10, 0, 'strong');
    await run('对角重叠 75%', a400, crop(scene, 100, 100, 400, 400), -100, -100, 'strong');
    await run('对角重叠 50%', a400, crop(scene, 200, 200, 400, 400), -200, -200, 'strong');
    await run('对角重叠 25%', a400, crop(scene, 300, 300, 400, 400), -300, -300, 'strong');
    await run('对角重叠 15%', a400, crop(scene, 340, 340, 400, 400), -340, -340, 'strong');
    await run('大图找小图 200px', a400, crop(scene, 150, 120, 200, 200), -150, -120, 'strong');
    await run('大图找小图 64px', a400, crop(scene, 300, 300, 64, 64), -300, -300, 'strong');
    await run(
      '亮度变化 (×0.7+30)',
      a400,
      affine(crop(scene, 120, 90, 200, 200), 0.7, 30),
      -120,
      -90,
      'strong',
    );
    await run(
      '遮挡 30%（位置仍正确）',
      a400,
      occlude(crop(scene, 120, 90, 200, 200), 0, 0, 60, 200, 255),
      -120,
      -90,
      'partial',
    );

    const other = makeScene(1200, 1000, 52);
    await run('内容无关', a400, crop(other, 0, 0, 400, 400), 0, 0, 'none');
    await run('内容放大 2 倍', a400, resize(a400, 800, 800), 0, 0, 'none');
    await run('内容缩小到 75%', a400, resize(a400, 300, 300), 0, 0, 'none');

    console.log('\n' + rows.join('\n'));
    expect(failures).toBe(0);
  });
});
