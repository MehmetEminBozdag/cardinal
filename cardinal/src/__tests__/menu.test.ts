import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  menuItems: [] as Array<Record<string, unknown>>,
  submenuItems: [] as Array<Record<string, unknown>>,
  setAsAppMenu: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({ getName: vi.fn().mockResolvedValue('Cardinal') }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../utils/openPreferences', () => ({ openPreferences: vi.fn() }));
vi.mock('../i18n/config', () => ({
  default: {
    t: (key: string) => key,
    on: vi.fn(),
  },
}));
vi.mock('@tauri-apps/api/menu', () => ({
  Menu: {
    new: vi.fn().mockResolvedValue({ setAsAppMenu: mocks.setAsAppMenu }),
  },
  MenuItem: {
    new: vi.fn(async (options: Record<string, unknown>) => {
      mocks.menuItems.push(options);
      return options;
    }),
  },
  CheckMenuItem: {
    new: vi.fn(async (options: Record<string, unknown>) => {
      mocks.menuItems.push(options);
      return options;
    }),
  },
  PredefinedMenuItem: {
    new: vi.fn(async (options: Record<string, unknown>) => options),
  },
  Submenu: {
    new: vi.fn(async (options: Record<string, unknown>) => {
      mocks.submenuItems.push(options);
      return { ...options, setAsHelpMenuForNSApp: vi.fn().mockResolvedValue(undefined) };
    }),
  },
}));

describe('application menu', () => {
  beforeEach(() => {
    mocks.menuItems.length = 0;
    mocks.submenuItems.length = 0;
    mocks.setAsAppMenu.mockClear();
    window.localStorage.clear();
  });

  it('exposes search, workspace, indexing and View controls', async () => {
    const { initializeAppMenu } = await import('../menu');
    await initializeAppMenu();

    const ids = mocks.menuItems.map((item) => item.id);
    expect(ids).toEqual(
      expect.arrayContaining([
        'menu.focus_search',
        'menu.show_newest',
        'menu.workspace_saved',
        'menu.workspace_favorites',
        'menu.workspace_coverage',
        'menu.workspace_operations',
        'menu.workspace_duplicates',
        'menu.toggle_workspace_toolbar',
        'menu.rescan',
      ]),
    );
    expect(mocks.setAsAppMenu).toHaveBeenCalledTimes(1);
  });
});
