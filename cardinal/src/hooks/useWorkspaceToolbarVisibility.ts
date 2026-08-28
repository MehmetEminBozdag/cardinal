import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

export const WORKSPACE_TOOLBAR_VISIBILITY_KEY = 'cardinal.workspace.toolbarVisible';

export function useWorkspaceToolbarVisibility() {
  const [visible, setVisible] = useStoredState<boolean>({
    key: WORKSPACE_TOOLBAR_VISIBILITY_KEY,
    defaultValue: true,
    read: (raw) => (raw === 'true' ? true : raw === 'false' ? false : null),
    write: String,
    readErrorMessage: 'Unable to read workspace toolbar visibility',
    writeErrorMessage: 'Unable to save workspace toolbar visibility',
  });

  const toggle = useCallback(() => setVisible(!visible), [setVisible, visible]);
  return { visible, setVisible, toggle };
}
