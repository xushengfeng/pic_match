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

describe('pic_match', () => {
  beforeAll(() => {
    resetCache();
  });

  it('should return zero offset for identical images', async () => {
    const img = createTestImage(64, 64, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10, 20, 20);
    });

    const result = await findOffset(img, img);
    expect(Math.abs(result.dx)).toBeLessThan(2);
    expect(Math.abs(result.dy)).toBeLessThan(2);
  });

  it('should detect horizontal offset', async () => {
    const width = 64;
    const height = 64;

    const imgA = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10, 20, 20);
      ctx.fillRect(30, 30, 10, 10);
    });

    const shiftX = 8;
    const imgB = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10 + shiftX, 10, 20, 20);
      ctx.fillRect(30 + shiftX, 30, 10, 10);
    });

    const result = await findOffset(imgA, imgB);
    expect(Math.abs(result.dx - shiftX)).toBeLessThan(3);
    expect(Math.abs(result.dy)).toBeLessThan(3);
  });

  it('should detect vertical offset', async () => {
    const width = 64;
    const height = 64;

    const imgA = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10, 20, 20);
    });

    const shiftY = 12;
    const imgB = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10 + shiftY, 20, 20);
    });

    const result = await findOffset(imgA, imgB);
    expect(Math.abs(result.dx)).toBeLessThan(3);
    expect(Math.abs(result.dy - shiftY)).toBeLessThan(3);
  });

  it('should detect 2D offset', async () => {
    const width = 64;
    const height = 64;

    const imgA = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10, 10, 15, 15);
      ctx.fillRect(35, 35, 10, 10);
    });

    const shiftX = 5;
    const shiftY = 7;
    const imgB = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(10 + shiftX, 10 + shiftY, 15, 15);
      ctx.fillRect(35 + shiftX, 35 + shiftY, 10, 10);
    });

    const result = await findOffset(imgA, imgB);
    expect(Math.abs(result.dx - shiftX)).toBeLessThan(3);
    expect(Math.abs(result.dy - shiftY)).toBeLessThan(3);
  });

  it('should detect offset with partial overlap', async () => {
    const width = 100;
    const height = 100;

    // Image A has a pattern in the center
    const imgA = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'rgb(200, 50, 50)';
      ctx.fillRect(20, 20, 30, 30);
      ctx.fillStyle = 'rgb(50, 200, 50)';
      ctx.fillRect(50, 50, 20, 20);
    });

    // Image B is shifted so only part of the content overlaps
    const shiftX = 15;
    const shiftY = 10;
    const imgB = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'rgb(200, 50, 50)';
      ctx.fillRect(20 + shiftX, 20 + shiftY, 30, 30);
      ctx.fillStyle = 'rgb(50, 200, 50)';
      ctx.fillRect(50 + shiftX, 50 + shiftY, 20, 20);
    });

    const result = await findOffset(imgA, imgB);
    expect(Math.abs(result.dx - shiftX)).toBeLessThan(4);
    expect(Math.abs(result.dy - shiftY)).toBeLessThan(4);
  });

  it('should detect negative offset', async () => {
    const width = 64;
    const height = 64;

    const imgA = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(20, 20, 20, 20);
    });

    const shiftX = -5;
    const shiftY = -3;
    const imgB = createTestImage(width, height, (ctx) => {
      ctx.fillStyle = 'white';
      ctx.fillRect(20 + shiftX, 20 + shiftY, 20, 20);
    });

    const result = await findOffset(imgA, imgB);
    expect(Math.abs(result.dx - shiftX)).toBeLessThan(3);
    expect(Math.abs(result.dy - shiftY)).toBeLessThan(3);
  });
});
