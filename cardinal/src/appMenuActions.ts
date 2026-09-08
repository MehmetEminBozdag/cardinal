export const APP_MENU_ACTION_EVENT = 'cardinal:menu-action';

export type AppMenuAction =
  | 'focus-search'
  | 'show-newest'
  | 'show-files'
  | 'show-events'
  | 'workspace-saved'
  | 'workspace-favorites'
  | 'workspace-coverage'
  | 'workspace-operations'
  | 'toggle-workspace-toolbar'
  | 'rescan';

export function dispatchAppMenuAction(action: AppMenuAction): void {
  window.dispatchEvent(new CustomEvent(APP_MENU_ACTION_EVENT, { detail: { action } }));
}

export function subscribeToAppMenuActions(listener: (action: AppMenuAction) => void): () => void {
  const handleEvent = (event: Event) => {
    const action = (event as CustomEvent<{ action?: AppMenuAction }>).detail?.action;
    if (action) listener(action);
  };
  window.addEventListener(APP_MENU_ACTION_EVENT, handleEvent);
  return () => window.removeEventListener(APP_MENU_ACTION_EVENT, handleEvent);
}
