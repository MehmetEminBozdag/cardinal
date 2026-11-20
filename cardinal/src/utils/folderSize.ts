// Helpers for the folder size view: byte formatting and squarified treemap layout.

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** Decimal units, like Finder ("12.3 GB"). */
export function formatBytes(bytes: number, locale?: string): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : value >= 10 ? 1 : 2;
  const formatted = value.toLocaleString(locale, {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  });
  return `${formatted} ${UNITS[unit]}`;
}

export type Rect = { x: number; y: number; width: number; height: number };

function worst(sum: number, max: number, min: number, side: number): number {
  const s2 = sum * sum;
  const w2 = side * side;
  return Math.max((w2 * max) / s2, s2 / (w2 * min));
}

/**
 * Squarified treemap (Bruls, Huizing, van Wijk). `values` must be sorted in
 * descending order; the returned rectangles keep the same order.
 */
export function squarify(values: number[], bounds: Rect): Rect[] {
  const result: Rect[] = values.map(() => ({ x: 0, y: 0, width: 0, height: 0 }));
  const total = values.reduce((sum, v) => sum + v, 0);
  if (total <= 0 || bounds.width <= 1 || bounds.height <= 1) return result;
  const scale = (bounds.width * bounds.height) / total;
  const areas = values.map((v) => v * scale);
  let r = { ...bounds };
  let i = 0;
  while (i < areas.length) {
    const side = Math.min(r.width, r.height);
    if (side <= 0) break;
    let j = i + 1;
    let rowSum = areas[i];
    let best = worst(rowSum, areas[i], areas[i], side);
    while (j < areas.length) {
      const sum = rowSum + areas[j];
      const w = worst(sum, areas[i], areas[j], side);
      if (w > best) break;
      best = w;
      rowSum = sum;
      j += 1;
    }
    if (r.width >= r.height) {
      const w = rowSum / r.height;
      let y = r.y;
      for (let k = i; k < j; k += 1) {
        const h = areas[k] / w;
        result[k] = { x: r.x, y, width: w, height: h };
        y += h;
      }
      r = { x: r.x + w, y: r.y, width: Math.max(0, r.width - w), height: r.height };
    } else {
      const h = rowSum / r.width;
      let x = r.x;
      for (let k = i; k < j; k += 1) {
        const w = areas[k] / h;
        result[k] = { x, y: r.y, width: w, height: h };
        x += w;
      }
      r = { x: r.x, y: r.y + h, width: r.width, height: Math.max(0, r.height - h) };
    }
    i = j;
  }
  return result;
}

/** Path segments from `root` down to `path` (inclusive), for breadcrumbs. */
export function breadcrumbs(root: string, path: string): Array<{ label: string; path: string }> {
  const crumbs = [{ label: root, path: root }];
  if (path === root || !path.startsWith(root)) return crumbs;
  const rest = path.slice(root === '/' ? 1 : root.length + 1);
  let current = root;
  for (const part of rest.split('/').filter(Boolean)) {
    current = current === '/' ? `/${part}` : `${current}/${part}`;
    crumbs.push({ label: part, path: current });
  }
  return crumbs;
}

export function parentPath(path: string): string {
  const index = path.lastIndexOf('/');
  if (index <= 0) return '/';
  return path.slice(0, index);
}

/** `~` for paths inside the home folder. */
export function tildePath(path: string, home: string | null): string {
  if (!home) return path;
  if (path === home) return '~';
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
}
