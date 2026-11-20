import { describe, expect, it } from 'vitest';
import { defaultSelection } from '../useInstalledApps';
import type { AppPlan } from '../useInstalledApps';

const item = (path: string, confidence: 'certain' | 'likely' | 'possible') => ({
  path,
  kind: 'support' as const,
  confidence,
  size: 1,
  needsAdmin: false,
  launchItem: false,
});

describe('defaultSelection', () => {
  it('preselects certain and likely leftovers but never possible ones', () => {
    const plan: AppPlan = {
      name: 'Widget',
      path: '/Applications/Widget.app',
      bundleId: 'com.acme.widget',
      version: '1.0',
      running: false,
      receipts: [],
      items: [
        item('/Applications/Widget.app', 'certain'),
        item('/Users/me/Library/Application Support/Widget', 'likely'),
        item('/Users/me/Library/Caches/com.acme.other', 'possible'),
      ],
    };
    expect([...defaultSelection(plan)].sort()).toEqual([
      '/Applications/Widget.app',
      '/Users/me/Library/Application Support/Widget',
    ]);
  });
});
