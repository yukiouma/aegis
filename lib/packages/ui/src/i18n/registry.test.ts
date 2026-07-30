import { describe, expect, it } from 'vitest';
import { en, zhCN } from './locales';
import { getCatalog, resolveMessage, translate } from './registry';
import type { TranslationKey } from './types';

describe('i18n registry', () => {
  it('returns the English catalog', () => {
    expect(getCatalog('en')).toBe(en);
  });

  it('returns the Simplified Chinese catalog', () => {
    expect(getCatalog('zh-CN')).toBe(zhCN);
  });

  it('keeps both catalogs on the same key set', () => {
    expect(Object.keys(zhCN).sort()).toEqual(Object.keys(en).sort());
  });

  it('translates messages in both locales', () => {
    expect(translate('en', 'language.simplifiedChinese')).toBe(
      'Simplified Chinese',
    );
    expect(translate('zh-CN', 'language.simplifiedChinese')).toBe('简体中文');
  });

  it('interpolates string and number variables', () => {
    expect(
      translate('en', 'language.current', {
        name: 'English',
        unused: 'ignored',
      }),
    ).toBe('Language: English');
    expect(translate('en', 'language.current', { name: 42 })).toBe(
      'Language: 42',
    );
  });

  it('leaves missing interpolation variables visible', () => {
    expect(translate('en', 'language.current')).toBe('Language: {name}');
  });

  it('falls back to English when the active catalog lacks a key', () => {
    expect(resolveMessage({}, 'language.english')).toBe('English');
  });

  it('falls back to the key when no catalog contains it', () => {
    const missingKey = 'missing.key' as TranslationKey;
    expect(resolveMessage({}, missingKey)).toBe('missing.key');
  });
});