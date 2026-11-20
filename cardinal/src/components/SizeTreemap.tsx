import React, { useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import type { SizeItem } from '../hooks/useFolderSizes';
import { formatBytes, squarify } from '../utils/folderSize';
import type { Rect } from '../utils/folderSize';

const PALETTE = [
  'hsl(209 55% 62%)',
  'hsl(166 45% 52%)',
  'hsl(270 38% 66%)',
  'hsl(120 35% 55%)',
  'hsl(331 45% 70%)',
  'hsl(43 70% 62%)',
  'hsl(187 30% 55%)',
  'hsl(238 32% 66%)',
];
const FILE_COLOR = 'hsl(25 75% 62%)';
const SMALL_COLOR = 'hsl(220 8% 62%)';
const HEADER = 18;
const MAX_TOP_ITEMS = 300;
/** A folder taking this share of the view gets its contents colored individually. */
const DOMINANT_SHARE = 0.75;

type Tile = {
  item: SizeItem;
  rect: Rect;
  color: string;
  label: string;
  inner: Array<{ item: SizeItem; rect: Rect; color: string | null; label: string }>;
};

type SizeTreemapProps = {
  items: SizeItem[];
  selectedPath: string | null;
  smallLabel: (item: SizeItem) => string;
  onSelect: (item: SizeItem) => void;
  onOpen: (item: SizeItem) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, item: SizeItem) => void;
};

function colorFor(item: SizeItem, index: number): string {
  if (item.kind === 'file') return FILE_COLOR;
  if (item.kind === 'small') return SMALL_COLOR;
  return PALETTE[index % PALETTE.length];
}

export function SizeTreemap({
  items,
  selectedPath,
  smallLabel,
  onSelect,
  onOpen,
  onContextMenu,
}: SizeTreemapProps): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useLayoutEffect(() => {
    const node = containerRef.current;
    if (!node) return;
    const update = () => setSize({ width: node.clientWidth, height: node.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  const tiles = useMemo<Tile[]>(() => {
    const shown = items.filter((i) => i.size > 0).slice(0, MAX_TOP_ITEMS);
    const total = shown.reduce((sum, i) => sum + i.size, 0);
    const rects = squarify(
      shown.map((i) => i.size),
      { x: 0, y: 0, width: size.width, height: size.height },
    );
    return shown.map((item, index) => {
      const rect = rects[index];
      const color = colorFor(item, index);
      const name = item.kind === 'small' ? smallLabel(item) : item.name;
      const tile: Tile = {
        item,
        rect,
        color,
        label: `${name}  ${formatBytes(item.size)}`,
        inner: [],
      };
      const children = item.children?.filter((c) => c.size > 0) ?? [];
      if (item.kind === 'dir' && children.length > 0 && rect.width > 80 && rect.height > 60) {
        const innerBounds = {
          x: 3,
          y: HEADER,
          width: rect.width - 6,
          height: rect.height - HEADER - 3,
        };
        const dominant = total > 0 && item.size >= total * DOMINANT_SHARE;
        const innerRects = squarify(
          children.map((c) => c.size),
          innerBounds,
        );
        tile.inner = children.map((child, k) => ({
          item: child,
          rect: innerRects[k],
          color: dominant ? colorFor(child, k) : null,
          label:
            child.kind === 'small'
              ? smallLabel(child)
              : dominant
                ? `${child.name}  ${formatBytes(child.size)}`
                : child.name,
        }));
      }
      return tile;
    });
  }, [items, size.width, size.height, smallLabel]);

  return (
    <div ref={containerRef} className="size-treemap" role="presentation">
      {tiles.map((tile) => {
        const { rect, item } = tile;
        if (rect.width < 1 || rect.height < 1) return null;
        const selected = selectedPath === item.path && item.kind !== 'small';
        return (
          <div
            key={`${item.kind}:${item.path}:${item.name}`}
            className={`size-tile${selected ? ' is-selected' : ''}${item.kind === 'small' ? ' is-small' : ''}`}
            style={{
              left: rect.x,
              top: rect.y,
              width: rect.width,
              height: rect.height,
              background: tile.color,
            }}
            title={tile.label}
            onClick={() => onSelect(item)}
            onDoubleClick={() => onOpen(item)}
            onContextMenu={(event) => onContextMenu(event, item)}
          >
            {rect.width > 46 && rect.height > 16 ? (
              <span className="size-tile__label">{tile.label}</span>
            ) : null}
            {tile.inner.map((inner) =>
              inner.rect.width < 1 || inner.rect.height < 1 ? null : (
                <div
                  key={`${inner.item.kind}:${inner.item.path}:${inner.item.name}`}
                  className={`size-tile__inner${inner.color ? ' is-colored' : ''}`}
                  style={{
                    left: inner.rect.x,
                    top: inner.rect.y,
                    width: inner.rect.width,
                    height: inner.rect.height,
                    background: inner.color ?? undefined,
                  }}
                  title={`${inner.label}  ${formatBytes(inner.item.size)}`}
                >
                  {inner.rect.width > 70 && inner.rect.height > 16 ? (
                    <span className="size-tile__inner-label">{inner.label}</span>
                  ) : null}
                </div>
              ),
            )}
          </div>
        );
      })}
    </div>
  );
}
