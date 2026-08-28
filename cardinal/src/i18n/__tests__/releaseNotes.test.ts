import { describe, expect, it } from 'vitest';
import notes from '../../../../docs/release-notes/0.1.24.md?raw';

const supportedLocales = [
  'ar-SA',
  'de-DE',
  'en-US',
  'es-ES',
  'fr-FR',
  'hi-IN',
  'it-IT',
  'ja-JP',
  'ko-KR',
  'pt-BR',
  'ru-RU',
  'tr-TR',
  'uk-UA',
  'zh-CN',
  'zh-TW',
];

describe('localized release notes', () => {
  it('contains a section for every supported locale', () => {
    for (const locale of supportedLocales) expect(notes).toContain(`## ${locale}`);
  });
});
