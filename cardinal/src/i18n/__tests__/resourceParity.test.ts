import { describe, expect, it } from 'vitest';
import arSA from '../resources/ar-SA.json';
import deDE from '../resources/de-DE.json';
import enUS from '../resources/en-US.json';
import esES from '../resources/es-ES.json';
import frFR from '../resources/fr-FR.json';
import hiIN from '../resources/hi-IN.json';
import itIT from '../resources/it-IT.json';
import jaJP from '../resources/ja-JP.json';
import koKR from '../resources/ko-KR.json';
import ptBR from '../resources/pt-BR.json';
import ruRU from '../resources/ru-RU.json';
import trTR from '../resources/tr-TR.json';
import ukUA from '../resources/uk-UA.json';
import zhCN from '../resources/zh-CN.json';
import zhTW from '../resources/zh-TW.json';

const flattenKeys = (value: unknown, prefix = ''): string[] => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    flattenKeys(child, prefix ? `${prefix}.${key}` : key),
  );
};

const resources = { arSA, deDE, esES, frFR, hiIN, itIT, jaJP, koKR, ptBR, ruRU, trTR, ukUA, zhCN, zhTW };

describe('translation resources', () => {
  it.each(Object.entries(resources))('%s has every English translation key', (_locale, resource) => {
    expect(flattenKeys(resource).sort()).toEqual(flattenKeys(enUS).sort());
  });
});
