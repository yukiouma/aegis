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

describe('splash and register catalogs', () => {
  const splashAndRegisterKeys = [
    'splash.title',
    'splash.step.health',
    'splash.step.method',
    'splash.method.account',
    'splash.method.domain',
    'splash.field.code',
    'splash.field.password',
    'splash.action.login',
    'splash.action.register',
    'splash.hint.notFound',
    'splash.hint.inactive',
    'splash.log.healthCheck.start',
    'splash.log.healthCheck.ok',
    'splash.log.healthCheck.failed',
    'splash.log.method.selected',
    'splash.log.login.start',
    'splash.log.login.ok',
    'splash.log.login.failed',
    'splash.log.login.notFound',
    'splash.log.login.inactive',
    'register.title',
    'register.field.userCode',
    'register.field.domainName',
    'register.field.hostname',
    'register.field.sid',
    'register.field.userName',
    'register.field.password',
    'register.action.register',
    'register.hint.contactAdmin',
    'register.log.identity.start',
    'register.log.identity.ok',
    'register.log.identity.failed',
    'register.log.register.start',
    'register.log.register.ok',
    'register.log.register.failed',
  ] as const;

  it.each(splashAndRegisterKeys)(
    'has a non-empty en and zh-CN message for %s',
    (key) => {
      expect(translate('en', key)).not.toBe(key);
      expect(translate('en', key).length).toBeGreaterThan(0);
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('zh-CN', key).length).toBeGreaterThan(0);
    },
  );

  it('interpolates the message variable in a failure log line', () => {
    expect(
      translate('en', 'splash.log.login.failed', { message: 'boom' }),
    ).toContain('boom');
    expect(
      translate('zh-CN', 'splash.log.login.failed', { message: 'boom' }),
    ).toContain('boom');
  });
});