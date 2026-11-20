import React, { useCallback, useMemo, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import type { SizeItem, SizeSuggestion, useFolderSizes } from '../hooks/useFolderSizes';
import { SizeTreemap } from './SizeTreemap';
import { breadcrumbs, formatBytes, parentPath, tildePath } from '../utils/folderSize';
import { openResultPath } from '../utils/openResultPath';

type FolderSizes = ReturnType<typeof useFolderSizes>;

type SizesPanelProps = {
  sizes: FolderSizes;
  filterQuery: string;
  onItemContextMenu: (event: ReactMouseEvent<HTMLElement>, paths: string[]) => void;
  onRequestFullDiskAccess: () => void;
};

export function SizesPanel({
  sizes,
  filterQuery,
  onItemContextMenu,
  onRequestFullDiskAccess,
}: SizesPanelProps): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const { status, home, currentPath, listing, suggestions, volume, mode, setMode, error } = sizes;
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [pathInput, setPathInput] = useState('');
  const locale = i18n.language;
  const bytes = useCallback((value: number) => formatBytes(value, locale), [locale]);
  const count = useCallback((value: number) => value.toLocaleString(locale), [locale]);

  const smallLabel = useCallback(
    (item: SizeItem) =>
      t('sizes.smallFiles', { count: item.fileCount, formatted: count(item.fileCount) }),
    [count, t],
  );

  const items = useMemo(() => {
    const query = filterQuery.trim().toLowerCase();
    const all = listing?.items ?? [];
    return query
      ? all.filter((i) => i.kind !== 'small' && i.name.toLowerCase().includes(query))
      : all;
  }, [filterQuery, listing?.items]);

  const open = useCallback(
    (item: SizeItem) => {
      if (item.kind === 'dir') {
        sizes.navigate(item.path);
        setSelectedPath(null);
      } else if (item.kind === 'file') {
        openResultPath(item.path);
      } else {
        void invoke('open_in_finder', { path: item.path });
      }
    },
    [sizes],
  );

  const showMenu = useCallback(
    (event: ReactMouseEvent<HTMLElement>, item: { kind: SizeItem['kind']; path: string }) => {
      if (item.kind === 'small') {
        event.preventDefault();
        return;
      }
      setSelectedPath(item.path);
      onItemContextMenu(event, [item.path]);
    },
    [onItemContextMenu],
  );

  const showSuggestion = useCallback(
    (suggestion: SizeSuggestion) => {
      sizes.navigate(parentPath(suggestion.path));
      setSelectedPath(suggestion.path);
      setMode('list');
    },
    [setMode, sizes],
  );

  const submitPath = useCallback(() => {
    const value = pathInput.trim().replace(/^~(?=$|\/)/, home ?? '~');
    if (value) void sizes.scan(value);
  }, [home, pathInput, sizes]);

  const scanning = status?.phase === 'scanning';
  const root = status?.rootPath ?? null;
  const crumbs = root && currentPath ? breadcrumbs(root, currentPath) : [];
  const parentSize = Math.max(listing?.size ?? 1, 1);

  const statusText = (() => {
    if (!status) return '';
    if (scanning) {
      return t('sizes.status.scanning', {
        path: tildePath(status.scanningPath ?? '', home),
        files: count(status.scannedFiles),
        size: bytes(status.scannedBytes),
      });
    }
    if (status.phase === 'ready') {
      const base = t('sizes.status.ready', {
        files: count(status.totalFiles),
        size: bytes(status.totalSize),
      });
      const when = status.fromSnapshot
        ? t('sizes.status.fromSnapshot')
        : status.lastScanMs != null
          ? t('sizes.status.scanTime', { seconds: (status.lastScanMs / 1000).toFixed(1) })
          : '';
      const live = status.pendingUpdates > 0 ? t('sizes.status.updating') : t('sizes.status.live');
      return [base, when, live].filter(Boolean).join(' · ');
    }
    return t('sizes.status.idle');
  })();

  return (
    <div className="sizes-panel">
      <div className="sizes-toolbar">
        <div className="sizes-roots" role="group" aria-label={t('sizes.roots.label')}>
          <button
            type="button"
            disabled={scanning || !home}
            onClick={() => home && void sizes.scan(home)}
          >
            {t('sizes.roots.home')}
          </button>
          <button type="button" disabled={scanning} onClick={() => void sizes.scan('/')}>
            {t('sizes.roots.disk')}
          </button>
          <form
            className="sizes-path-form"
            onSubmit={(event) => {
              event.preventDefault();
              submitPath();
            }}
          >
            <input
              type="text"
              value={pathInput}
              placeholder={t('sizes.roots.pathPlaceholder')}
              aria-label={t('sizes.roots.pathPlaceholder')}
              onChange={(event) => setPathInput(event.target.value)}
              spellCheck={false}
            />
            <button type="submit" disabled={scanning || !pathInput.trim()}>
              {t('sizes.roots.scan')}
            </button>
          </form>
        </div>
        <div className="sizes-modes" role="tablist" aria-label={t('sizes.modes.label')}>
          {(['list', 'treemap', 'suggestions'] as const).map((key) => (
            <button
              key={key}
              type="button"
              role="tab"
              aria-selected={mode === key}
              className={mode === key ? 'is-active' : ''}
              onClick={() => setMode(key)}
            >
              {t(`sizes.modes.${key}`)}
            </button>
          ))}
        </div>
      </div>

      {mode !== 'suggestions' ? (
        <nav className="sizes-breadcrumbs" aria-label={t('sizes.breadcrumbs')}>
          <button
            type="button"
            className="sizes-up"
            onClick={sizes.goUp}
            disabled={!currentPath || currentPath === root}
            title={t('sizes.actions.up')}
            aria-label={t('sizes.actions.up')}
          >
            ‹
          </button>
          {crumbs.map((crumb, index) => (
            <React.Fragment key={crumb.path}>
              {index > 0 ? <span className="sizes-crumb-sep">›</span> : null}
              <button
                type="button"
                className={crumb.path === currentPath ? 'is-current' : ''}
                onClick={() => sizes.navigate(crumb.path)}
              >
                {index === 0 ? tildePath(crumb.label, home) : crumb.label}
              </button>
            </React.Fragment>
          ))}
          {listing ? <span className="sizes-crumb-total">{bytes(listing.size)}</span> : null}
          <button
            type="button"
            className="sizes-rescan"
            onClick={sizes.rescanCurrent}
            disabled={scanning || !currentPath}
            title={t('sizes.actions.rescanHint')}
          >
            ↻ {t('sizes.actions.rescan')}
          </button>
        </nav>
      ) : null}

      <div className="sizes-body">
        {!listing && scanning ? (
          <div className="sizes-empty">{statusText}</div>
        ) : !listing ? (
          <div className="sizes-empty">{t('sizes.status.idle')}</div>
        ) : mode === 'list' ? (
          <div className="sizes-table" role="table" aria-label={t('sizes.table.label')}>
            <div className="sizes-row sizes-row--header" role="row">
              <span role="columnheader">{t('sizes.table.name')}</span>
              <span role="columnheader">{t('sizes.table.size')}</span>
              <span role="columnheader">{t('sizes.table.share')}</span>
              <span role="columnheader">{t('sizes.table.files')}</span>
            </div>
            {items.map((item) => {
              const share = item.size / parentSize;
              const selected = selectedPath === item.path && item.kind !== 'small';
              return (
                <div
                  key={`${item.kind}:${item.path}:${item.name}`}
                  role="row"
                  className={`sizes-row${selected ? ' is-selected' : ''} is-${item.kind}`}
                  onClick={() => setSelectedPath(item.path)}
                  onDoubleClick={() => open(item)}
                  onContextMenu={(event) => showMenu(event, item)}
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') open(item);
                  }}
                >
                  <span role="cell" className="sizes-name">
                    <span aria-hidden="true">
                      {item.kind === 'dir' ? '📁' : item.kind === 'file' ? '📄' : '🗂'}
                    </span>
                    <span className="sizes-name__text">
                      {item.kind === 'small' ? smallLabel(item) : item.name}
                    </span>
                    {item.unreadableCount > 0 ? (
                      <span
                        className="sizes-lock"
                        title={t('sizes.unreadableItem', { count: item.unreadableCount })}
                      >
                        🔒
                      </span>
                    ) : null}
                  </span>
                  <span role="cell" className="sizes-num">
                    {bytes(item.size)}
                  </span>
                  <span role="cell" className="sizes-share">
                    <span className="sizes-bar">
                      <span
                        className={`sizes-bar__fill is-${item.kind}`}
                        style={{ width: `${Math.max(1, share * 100)}%` }}
                      />
                    </span>
                    <span className="sizes-num">{(share * 100).toFixed(1)}%</span>
                  </span>
                  <span role="cell" className="sizes-num sizes-muted">
                    {item.kind === 'file' ? '' : count(item.fileCount)}
                  </span>
                </div>
              );
            })}
          </div>
        ) : mode === 'treemap' ? (
          <SizeTreemap
            items={items}
            selectedPath={selectedPath}
            smallLabel={smallLabel}
            onSelect={(item) => setSelectedPath(item.path)}
            onOpen={open}
            onContextMenu={showMenu}
          />
        ) : (
          <SuggestionsList
            suggestions={suggestions}
            home={home}
            bytes={bytes}
            onShow={showSuggestion}
            onContextMenu={showMenu}
          />
        )}
      </div>

      <div className="sizes-status">
        <span>{error ?? statusText}</span>
        {scanning ? (
          <button type="button" onClick={sizes.cancel}>
            {t('sizes.actions.cancel')}
          </button>
        ) : null}
        {status && status.unreadableCount > 0 ? (
          <button type="button" className="sizes-link" onClick={onRequestFullDiskAccess}>
            🔒{' '}
            {t('sizes.unreadable', {
              count: status.unreadableCount,
              formatted: count(status.unreadableCount),
            })}
          </button>
        ) : null}
        {volume && volume.total > 0 ? (
          <VolumeGauge volume={volume} trashSize={status?.trashSize ?? null} bytes={bytes} />
        ) : null}
      </div>
    </div>
  );
}

const LOW_SPACE_SHARE = 0.1;

function VolumeGauge({
  volume,
  trashSize,
  bytes,
}: {
  volume: NonNullable<FolderSizes['volume']>;
  trashSize: number | null;
  bytes: (value: number) => string;
}): React.JSX.Element {
  const { t } = useTranslation();
  const free = Math.min(volume.availableForImportantUsage ?? volume.available, volume.total);
  const used = Math.max(volume.total - free, 0);
  const low = free / volume.total < LOW_SPACE_SHARE;
  const hint = [
    t('sizes.space.hint', { now: bytes(volume.available) }),
    trashSize ? t('sizes.space.trashHint') : null,
  ]
    .filter(Boolean)
    .join('\n');
  return (
    <span className={`sizes-space${low ? ' is-low' : ''}`} title={hint}>
      <span>{t('sizes.space.free', { free: bytes(free), total: bytes(volume.total) })}</span>
      <span className="sizes-space__bar" aria-hidden="true">
        <span style={{ width: `${(used / volume.total) * 100}%` }} />
      </span>
      {trashSize && trashSize > 0 ? (
        <span className="sizes-space__trash">
          {t('sizes.space.trash', { size: bytes(trashSize) })}
        </span>
      ) : null}
    </span>
  );
}

type SuggestionsListProps = {
  suggestions: SizeSuggestion[];
  home: string | null;
  bytes: (value: number) => string;
  onShow: (suggestion: SizeSuggestion) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, item: SizeSuggestion) => void;
};

function SuggestionsList({
  suggestions,
  home,
  bytes,
  onShow,
  onContextMenu,
}: SuggestionsListProps): React.JSX.Element {
  const { t } = useTranslation();
  const total = (level: SizeSuggestion['level']) =>
    suggestions.filter((s) => s.level === level).reduce((sum, s) => sum + s.size, 0);
  if (suggestions.length === 0) {
    return <div className="sizes-empty">{t('sizes.suggestions.empty')}</div>;
  }
  return (
    <div className="sizes-suggestions">
      <div className="sizes-suggestions__summary">
        <span className="is-safe">
          ✓ {t('sizes.suggestions.safeTotal', { size: bytes(total('safe')) })}
        </span>
        <span className="is-review">
          👁 {t('sizes.suggestions.reviewTotal', { size: bytes(total('review')) })}
        </span>
        <span className="sizes-muted">{t('sizes.suggestions.note')}</span>
      </div>
      {suggestions.map((s) => (
        <div
          key={s.path}
          className={`sizes-suggestion is-${s.level}`}
          onDoubleClick={() => onShow(s)}
          onContextMenu={(event) => onContextMenu(event, s)}
        >
          <span className="sizes-suggestion__badge">{s.level === 'safe' ? '✓' : '👁'}</span>
          <span className="sizes-suggestion__text">
            <strong>{t(`sizes.rules.${s.rule}.title`)}</strong>
            <span className="sizes-muted"> {s.name}</span>
            <span className="sizes-suggestion__path">{tildePath(s.path, home)}</span>
            <span className="sizes-muted">{t(`sizes.rules.${s.rule}.detail`)}</span>
          </span>
          <span className="sizes-num sizes-suggestion__size">{bytes(s.size)}</span>
          <button type="button" onClick={() => onShow(s)}>
            {t('sizes.suggestions.show')}
          </button>
        </div>
      ))}
    </div>
  );
}
