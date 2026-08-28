import { useState } from 'react';
import type { AppLifecycleStatus } from '../types/ipc';
import type { FavoritePath, SavedSearch } from '../hooks/useWorkspaceCollections';
import { useTranslation } from 'react-i18next';
import { splitPath } from '../utils/path';
import type { FileOperationUpdate } from '../hooks/useContextMenu';
import type { DuplicateGroup, DuplicateScanError } from '../hooks/useDuplicateFiles';

export type WorkspaceSection = 'saved' | 'favorites' | 'coverage' | 'operations' | 'duplicates';

type CoverageInfo = {
  watchRoot: string;
  ignorePaths: string[];
  includePaths: string[];
  fullDiskAccessGranted: boolean;
  lifecycleState: AppLifecycleStatus;
  scannedFiles: number;
  currentQuery: string;
  currentDirectoryQuery: string;
};

type OperationInfo = {
  indexingPaused: boolean;
  lifecycleState: AppLifecycleStatus;
  activity: 'idle' | 'low' | 'medium' | 'high';
  scannedFiles: number;
  fileOperations: FileOperationUpdate[];
  isSearching: boolean;
  isSorting: boolean;
  onToggleIndexing: () => void;
  onCancelSearch: () => void;
  onCancelSort: () => void;
  onRequestRescan: () => void;
};

type WorkspacePanelProps = {
  activeSection: WorkspaceSection;
  onClose: () => void;
  savedSearches: SavedSearch[];
  favorites: FavoritePath[];
  selectedPaths: string[];
  onSaveSearch: (name: string) => void;
  onApplySavedSearch: (savedSearch: SavedSearch) => void;
  onRemoveSavedSearch: (id: string) => void;
  onAddSelectedFavorites: () => void;
  onOpenFavorite: (path: string) => void;
  onRevealFavorite: (path: string) => void;
  onRemoveFavorite: (path: string) => void;
  onIncludePath: (path: string) => void;
  coverage: CoverageInfo;
  operations: OperationInfo;
  onOpenPreferences: () => void;
  duplicates: {
    state: 'idle' | 'scanning' | 'ready' | 'error';
    groups: DuplicateGroup[];
    scannedFiles: number;
    skippedFiles: number;
    limited: boolean;
    error: DuplicateScanError | null;
    onScan: () => void;
  };
};

const SECTION_KEYS: WorkspaceSection[] = [
  'saved',
  'favorites',
  'coverage',
  'operations',
  'duplicates',
];

export function WorkspaceToolbar({
  activeSection,
  onSelect,
}: {
  activeSection: WorkspaceSection | null;
  onSelect: (section: WorkspaceSection) => void;
}) {
  const { t } = useTranslation();
  return (
    <nav className="workspace-toolbar" aria-label={t('workspace.toolbarLabel')}>
      {SECTION_KEYS.map((section) => (
        <button
          key={section}
          type="button"
          className={activeSection === section ? 'is-active' : ''}
          aria-pressed={activeSection === section}
          onClick={() => onSelect(section)}
        >
          <span className="workspace-toolbar__icon" aria-hidden="true">
            {section === 'saved'
              ? '⌕'
              : section === 'favorites'
                ? '★'
                : section === 'coverage'
                  ? '?'
                  : section === 'operations'
                    ? '◷'
                    : '≋'}
          </span>
          {t(`workspace.sections.${section}`)}
        </button>
      ))}
    </nav>
  );
}

export function WorkspacePanel(props: WorkspacePanelProps): React.JSX.Element {
  const { t } = useTranslation();
  const [savedSearchName, setSavedSearchName] = useState('');
  const { activeSection, coverage, operations } = props;

  const saveCurrentSearch = () => {
    const name = savedSearchName.trim();
    if (!name) return;
    props.onSaveSearch(name);
    setSavedSearchName('');
  };

  return (
    <aside className="workspace-panel" aria-label={t(`workspace.sections.${activeSection}`)}>
      <header className="workspace-panel__header">
        <h2>{t(`workspace.sections.${activeSection}`)}</h2>
        <button type="button" aria-label={t('workspace.close')} onClick={props.onClose}>
          ×
        </button>
      </header>

      <div className="workspace-panel__body">
        {activeSection === 'saved' ? (
          <>
            <div className="workspace-inline-form">
              <label htmlFor="saved-search-name">{t('workspace.saved.nameLabel')}</label>
              <div>
                <input
                  id="saved-search-name"
                  value={savedSearchName}
                  onChange={(event) => setSavedSearchName(event.target.value)}
                  onKeyDown={(event) => event.key === 'Enter' && saveCurrentSearch()}
                />
                <button
                  type="button"
                  onClick={saveCurrentSearch}
                  disabled={!savedSearchName.trim()}
                >
                  {t('workspace.saved.saveCurrent')}
                </button>
              </div>
            </div>
            <WorkspaceEmpty
              visible={props.savedSearches.length === 0}
              text={t('workspace.saved.empty')}
            />
            <ul className="workspace-list">
              {props.savedSearches.map((savedSearch) => (
                <li key={savedSearch.id}>
                  <button
                    type="button"
                    className="workspace-list__primary"
                    onClick={() => props.onApplySavedSearch(savedSearch)}
                  >
                    <strong>{savedSearch.name}</strong>
                    <span>{savedSearch.query || t('workspace.saved.allFiles')}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.remove')}
                    onClick={() => props.onRemoveSavedSearch(savedSearch.id)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : null}

        {activeSection === 'favorites' ? (
          <>
            <button
              type="button"
              className="workspace-primary-action"
              onClick={props.onAddSelectedFavorites}
              disabled={props.selectedPaths.length === 0}
            >
              {t('workspace.favorites.addSelection')}
            </button>
            <WorkspaceEmpty
              visible={props.favorites.length === 0}
              text={t('workspace.favorites.empty')}
            />
            <ul className="workspace-list">
              {props.favorites.map((favorite) => (
                <li key={favorite.path}>
                  <button
                    type="button"
                    className="workspace-list__primary"
                    aria-label={t('workspace.favorites.open')}
                    onClick={() => props.onOpenFavorite(favorite.path)}
                  >
                    <strong>{splitPath(favorite.path).name || favorite.path}</strong>
                    <span>{favorite.path}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.favorites.reveal')}
                    onClick={() => props.onRevealFavorite(favorite.path)}
                  >
                    ↗
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.remove')}
                    onClick={() => props.onRemoveFavorite(favorite.path)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : null}

        {activeSection === 'coverage' ? (
          <div className="coverage-details">
            <CoverageRow
              label={t('workspace.coverage.permission')}
              value={
                coverage.fullDiskAccessGranted
                  ? t('workspace.coverage.granted')
                  : t('workspace.coverage.denied')
              }
            />
            <CoverageRow label={t('workspace.coverage.state')} value={coverage.lifecycleState} />
            <CoverageRow
              label={t('workspace.coverage.indexed')}
              value={coverage.scannedFiles.toLocaleString()}
            />
            <CoverageRow label={t('workspace.coverage.root')} value={coverage.watchRoot} />
            <PathGroup
              title={t('workspace.coverage.excluded')}
              paths={coverage.ignorePaths}
              empty={t('workspace.coverage.none')}
              actionLabel={t('workspace.coverage.includeInSearch')}
              onAction={props.onIncludePath}
            />
            <PathGroup
              title={t('workspace.coverage.included')}
              paths={coverage.includePaths}
              empty={t('workspace.coverage.allUnderRoot')}
            />
            {coverage.currentDirectoryQuery ? (
              <CoverageRow
                label={t('workspace.coverage.directoryScope')}
                value={coverage.currentDirectoryQuery}
              />
            ) : null}
            <p className="workspace-note">{t('workspace.coverage.explanation')}</p>
            <div className="workspace-actions">
              <button type="button" onClick={props.onOpenPreferences}>
                {t('workspace.coverage.openSettings')}
              </button>
              <button type="button" onClick={operations.onRequestRescan}>
                {t('workspace.operations.rescan')}
              </button>
            </div>
          </div>
        ) : null}

        {activeSection === 'operations' ? (
          <div className="operation-list">
            <OperationRow
              label={t('workspace.operations.activity')}
              state={`${t(`workspace.operations.activityLevels.${operations.activity}`)} · ${t('workspace.operations.indexedCount', { count: operations.scannedFiles })}`}
            />
            <OperationRow
              label={t('workspace.operations.indexing')}
              state={
                operations.indexingPaused
                  ? t('workspace.operations.paused')
                  : operations.lifecycleState
              }
              actionLabel={
                operations.indexingPaused
                  ? t('workspace.operations.resumeIndexing')
                  : t('workspace.operations.pauseIndexing')
              }
              onAction={operations.onToggleIndexing}
            />
            {operations.isSearching ? (
              <OperationRow
                label={t('workspace.operations.search')}
                state={t('workspace.operations.running')}
                actionLabel={t('workspace.operations.cancelSearch')}
                onAction={operations.onCancelSearch}
              />
            ) : null}
            {operations.isSorting ? (
              <OperationRow
                label={t('workspace.operations.sorting')}
                state={t('workspace.operations.running')}
                actionLabel={t('workspace.operations.cancelSort')}
                onAction={operations.onCancelSort}
              />
            ) : null}
            {operations.fileOperations.map((operation) => (
              <OperationRow
                key={operation.id}
                label={t('workspace.operations.trash')}
                state={t(`workspace.operations.fileStates.${operation.state}`, {
                  count: operation.itemCount,
                })}
              />
            ))}
            {!operations.isSearching &&
            !operations.isSorting &&
            operations.lifecycleState === 'Ready' ? (
              <WorkspaceEmpty visible text={t('workspace.operations.idle')} />
            ) : null}
            <button
              type="button"
              className="workspace-primary-action"
              onClick={operations.onRequestRescan}
              disabled={operations.indexingPaused}
            >
              {t('workspace.operations.rescan')}
            </button>
          </div>
        ) : null}

        {activeSection === 'duplicates' ? (
          <div className="duplicate-details">
            <p className="workspace-note">{t('workspace.duplicates.explanation')}</p>
            <button
              type="button"
              className="workspace-primary-action"
              onClick={props.duplicates.onScan}
              disabled={props.duplicates.state === 'scanning'}
            >
              {props.duplicates.state === 'scanning'
                ? t('workspace.duplicates.scanning')
                : t('workspace.duplicates.scan')}
            </button>
            {props.duplicates.error ? (
              <p className="workspace-error" role="alert">
                {t(`workspace.duplicates.${props.duplicates.error}`)}
              </p>
            ) : null}
            {props.duplicates.state === 'ready' ? (
              <>
                <p className="workspace-note">
                  {t('workspace.duplicates.scanned', { count: props.duplicates.scannedFiles })}
                </p>
                {props.duplicates.skippedFiles > 0 || props.duplicates.limited ? (
                  <p className="workspace-warning" role="status">
                    {t('workspace.duplicates.incomplete', {
                      count: props.duplicates.skippedFiles,
                    })}
                  </p>
                ) : null}
              </>
            ) : null}
            <WorkspaceEmpty
              visible={props.duplicates.state === 'ready' && props.duplicates.groups.length === 0}
              text={t('workspace.duplicates.empty')}
            />
            <div className="duplicate-groups">
              {props.duplicates.groups.map((group, groupIndex) => (
                <section key={`${group.size}-${groupIndex}`} className="duplicate-group">
                  <h3>{t('workspace.duplicates.groupSize', { size: formatBytes(group.size) })}</h3>
                  <ul>
                    {group.files.map((file) => (
                      <li key={file.path}>
                        <span title={file.path}>{file.path}</span>
                        <strong>{t(`workspace.duplicates.${file.storage}`)}</strong>
                      </li>
                    ))}
                  </ul>
                </section>
              ))}
            </div>
          </div>
        ) : null}
      </div>
    </aside>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`;
}

function WorkspaceEmpty({ visible, text }: { visible: boolean; text: string }) {
  return visible ? <p className="workspace-empty">{text}</p> : null;
}

function CoverageRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="coverage-row">
      <span>{label}</span>
      <strong title={value}>{value}</strong>
    </div>
  );
}

function PathGroup({
  title,
  paths,
  empty,
  actionLabel,
  onAction,
}: {
  title: string;
  paths: string[];
  empty: string;
  actionLabel?: string;
  onAction?: (path: string) => void;
}) {
  return (
    <section className="coverage-paths">
      <h3>{title}</h3>
      {paths.length ? (
        <ul>
          {paths.map((path) => (
            <li key={path} title={path}>
              <span>{path}</span>
              {actionLabel && onAction ? (
                <button type="button" aria-label={actionLabel} onClick={() => onAction(path)}>
                  {actionLabel}
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      ) : (
        <p>{empty}</p>
      )}
    </section>
  );
}

function OperationRow({
  label,
  state,
  actionLabel,
  onAction,
}: {
  label: string;
  state: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <div className="operation-row">
      <div>
        <strong>{label}</strong>
        <span>{state}</span>
      </div>
      {actionLabel && onAction ? (
        <button type="button" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}
