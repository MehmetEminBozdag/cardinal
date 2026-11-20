import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => undefined) }));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts && 'name' in opts ? `${key}:${String(opts.name)}` : key,
    i18n: { language: 'en-US' },
  }),
}));

const { OperationHistory } = await import('../OperationHistory');

describe('OperationHistory', () => {
  it('lists logged operations with their trash destinations and failures', async () => {
    invokeMock.mockImplementation(async (cmd: string) =>
      cmd === 'get_operation_log'
        ? [
            {
              time: 1_795_000_000,
              source: 'appUninstall',
              app: 'Widget (com.acme.widget)',
              items: [
                {
                  path: '/Applications/Widget.app',
                  trashedTo: '/Users/me/.Trash/Widget (Cardinal 1-0).app',
                  size: 2_000_000,
                  admin: true,
                  ok: true,
                },
                { path: '/Library/Foo', admin: true, ok: false, error: 'admin_cancelled' },
              ],
            },
          ]
        : null,
    );
    render(<OperationHistory />);
    expect(await screen.findByText(/Widget \(com.acme.widget\)/)).toBeTruthy();
    expect(screen.getByText('/Applications/Widget.app')).toBeTruthy();
    expect(
      screen.getByText('workspace.operations.history.toTrash:Widget (Cardinal 1-0).app'),
    ).toBeTruthy();
    expect(screen.getByText('admin_cancelled')).toBeTruthy();
  });

  it('shows an empty note when the native log is unavailable', async () => {
    invokeMock.mockRejectedValue(new Error('no backend'));
    render(<OperationHistory />);
    expect(await screen.findByText('workspace.operations.history.empty')).toBeTruthy();
  });
});
