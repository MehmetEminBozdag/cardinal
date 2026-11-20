import { describe, expect, it } from 'vitest';
import { breadcrumbs, formatBytes, parentPath, squarify, tildePath } from '../folderSize';

describe('formatBytes', () => {
  it('uses decimal units like Finder', () => {
    expect(formatBytes(0, 'en-US')).toBe('0 B');
    expect(formatBytes(999, 'en-US')).toBe('999 B');
    expect(formatBytes(1_500_000, 'en-US')).toBe('1.5 MB');
    expect(formatBytes(12_345_678_901, 'en-US')).toBe('12.3 GB');
    expect(formatBytes(245_110_000_000, 'en-US')).toBe('245 GB');
  });
});

describe('squarify', () => {
  const bounds = { x: 0, y: 0, width: 400, height: 300 };
  const values = [50, 30, 10, 5, 3, 2];
  const rects = squarify(values, bounds);

  it('fills the bounds exactly', () => {
    const area = rects.reduce((sum, r) => sum + r.width * r.height, 0);
    expect(area).toBeCloseTo(120_000, 3);
  });

  it('keeps each area proportional to its value', () => {
    rects.forEach((r, i) => {
      expect((r.width * r.height) / 120_000).toBeCloseTo(values[i] / 100, 4);
    });
  });

  it('does not overlap', () => {
    const shrink = (r: (typeof rects)[number]) => ({
      x1: r.x + 0.01,
      y1: r.y + 0.01,
      x2: r.x + r.width - 0.01,
      y2: r.y + r.height - 0.01,
    });
    for (let i = 0; i < rects.length; i += 1) {
      for (let j = i + 1; j < rects.length; j += 1) {
        const a = shrink(rects[i]);
        const b = shrink(rects[j]);
        const overlap = a.x1 < b.x2 && b.x1 < a.x2 && a.y1 < b.y2 && b.y1 < a.y2;
        expect(overlap).toBe(false);
      }
    }
  });

  it('returns empty rectangles for empty input', () => {
    expect(squarify([], bounds)).toEqual([]);
    expect(squarify([0, 0], bounds).every((r) => r.width === 0)).toBe(true);
  });
});

describe('path helpers', () => {
  it('builds breadcrumbs below the root', () => {
    expect(breadcrumbs('/Users/me', '/Users/me/Library/Caches').map((c) => c.label)).toEqual([
      '/Users/me',
      'Library',
      'Caches',
    ]);
    expect(breadcrumbs('/', '/Users').map((c) => c.path)).toEqual(['/', '/Users']);
  });

  it('computes parents and tilde paths', () => {
    expect(parentPath('/Users/me/a')).toBe('/Users/me');
    expect(parentPath('/Users')).toBe('/');
    expect(tildePath('/Users/me/Downloads', '/Users/me')).toBe('~/Downloads');
    expect(tildePath('/Applications', '/Users/me')).toBe('/Applications');
  });
});
