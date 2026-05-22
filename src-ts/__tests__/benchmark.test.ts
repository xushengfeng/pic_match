import { describe, it, expect, beforeAll } from 'vitest';
import { findOffset, resetCache, ImageData } from '../index';
import * as PImage from 'pureimage';

function createTestImage(
  width: number,
  height: number,
  draw: (ctx: any) => void
): ImageData {
  const img = PImage.make(width, height);
  const ctx = img.getContext('2d');
  draw(ctx);
  const buf = img.data;
  const data = new Uint8ClampedArray(buf.buffer);
  return { data, width, height };
}

function formatMs(ms: number): string {
  return ms < 1000 ? `${ms.toFixed(1)}ms` : `${(ms / 1000).toFixed(2)}s`;
}

describe('performance benchmarks', () => {
  beforeAll(() => {
    resetCache();
  });

  it('64x64 small image', async () => {
    const size = 64;
    const imgA = createTestImage(size, size, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10, 20, 20);
      ctx.fillRect(30, 30, 15, 15);
    });
    const imgB = createTestImage(size, size, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(15, 12, 20, 20);
      ctx.fillRect(35, 32, 15, 15);
    });

    const start = performance.now();
    await findOffset(imgA, imgB);
    const elapsed = performance.now() - start;

    console.log(`    64x64: ${formatMs(elapsed)}`);
    expect(elapsed).toBeLessThan(5000);
  });

  it('512x512 medium image', async () => {
    const size = 512;
    const imgA = createTestImage(size, size, (ctx) => {
      ctx.fillStyle = 'rgb(100, 150, 200)';
      ctx.fillRect(50, 50, 100, 100);
      ctx.fillStyle = 'rgb(200, 100, 50)';
      ctx.fillRect(200, 200, 80, 80);
      ctx.fillStyle = 'rgb(50, 200, 100)';
      ctx.fillRect(350, 100, 60, 120);
    });
    const imgB = createTestImage(size, size, (ctx) => {
      ctx.fillStyle = 'rgb(100, 150, 200)';
      ctx.fillRect(70, 60, 100, 100);
      ctx.fillStyle = 'rgb(200, 100, 50)';
      ctx.fillRect(220, 210, 80, 80);
      ctx.fillStyle = 'rgb(50, 200, 100)';
      ctx.fillRect(370, 110, 60, 120);
    });

    const start = performance.now();
    await findOffset(imgA, imgB);
    const elapsed = performance.now() - start;

    console.log(`    512x512: ${formatMs(elapsed)}`);
    expect(elapsed).toBeLessThan(10000);
  });

  it('1920x1080 Full HD', async () => {
    const w = 1920;
    const h = 1080;
    const imgA = createTestImage(w, h, (ctx) => {
      ctx.fillStyle = 'rgb(30, 60, 90)';
      ctx.fillRect(100, 100, 400, 300);
      ctx.fillStyle = 'rgb(200, 100, 50)';
      ctx.fillRect(800, 400, 300, 200);
      ctx.fillStyle = 'rgb(100, 200, 150)';
      ctx.fillRect(1400, 700, 250, 180);
    });
    const imgB = createTestImage(w, h, (ctx) => {
      ctx.fillStyle = 'rgb(30, 60, 90)';
      ctx.fillRect(130, 120, 400, 300);
      ctx.fillStyle = 'rgb(200, 100, 50)';
      ctx.fillRect(830, 420, 300, 200);
      ctx.fillStyle = 'rgb(100, 200, 150)';
      ctx.fillRect(1430, 720, 250, 180);
    });

    const start = performance.now();
    await findOffset(imgA, imgB);
    const elapsed = performance.now() - start;

    console.log(`    1920x1080: ${formatMs(elapsed)}`);
    expect(elapsed).toBeLessThan(30000);
  });

  it('3840x2160 4K worst case (large offset, minimal overlap)', async () => {
    const w = 3840;
    const h = 2160;

    const imgA = createTestImage(w, h, (ctx) => {
      ctx.fillStyle = 'rgb(80, 120, 160)';
      ctx.fillRect(200, 200, 600, 400);
      ctx.fillStyle = 'rgb(200, 80, 80)';
      ctx.fillRect(1500, 800, 500, 300);
      ctx.fillStyle = 'rgb(80, 200, 120)';
      ctx.fillRect(2800, 1400, 400, 350);
    });

    const shiftX = 500;
    const shiftY = 400;
    const imgB = createTestImage(w, h, (ctx) => {
      ctx.fillStyle = 'rgb(80, 120, 160)';
      ctx.fillRect(200 + shiftX, 200 + shiftY, 600, 400);
      ctx.fillStyle = 'rgb(200, 80, 80)';
      ctx.fillRect(1500 + shiftX, 800 + shiftY, 500, 300);
      ctx.fillStyle = 'rgb(80, 200, 120)';
      ctx.fillRect(2800 + shiftX, 1400 + shiftY, 400, 350);
    });

    const start = performance.now();
    const result = await findOffset(imgA, imgB);
    const elapsed = performance.now() - start;

    console.log(`    3840x2160 (worst case): ${formatMs(elapsed)}`);
    console.log(`    Result: dx=${result.dx.toFixed(1)}, dy=${result.dy.toFixed(1)}, confidence=${result.confidence.toFixed(2)}`);
    expect(elapsed).toBeLessThan(120000);
  }, 180000);

  it('3840x2160 4K best case (zero offset)', async () => {
    const w = 3840;
    const h = 2160;

    const img = createTestImage(w, h, (ctx) => {
      ctx.fillStyle = 'rgb(80, 120, 160)';
      ctx.fillRect(200, 200, 600, 400);
      ctx.fillStyle = 'rgb(200, 80, 80)';
      ctx.fillRect(1500, 800, 500, 300);
    });

    const start = performance.now();
    await findOffset(img, img);
    const elapsed = performance.now() - start;

    console.log(`    3840x2160 (best case): ${formatMs(elapsed)}`);
    expect(elapsed).toBeLessThan(120000);
  }, 180000);
});
