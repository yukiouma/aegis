import { describe, expect, it } from 'vitest';
import * as Mui from '@mui/material';
import * as Icons from '@mui/icons-material';
import {
  AegisI18nProvider as PackageI18nProvider,
  useI18n as packageUseI18n,
} from '@aegis/ui/i18n';
import type {
  Locale as PackageLocale,
  TranslationKey as PackageTranslationKey,
} from '@aegis/ui/i18n';
import {
  AegisI18nProvider as FocusedI18nProvider,
  useI18n as focusedUseI18n,
} from './i18n';
import {
  AegisI18nProvider,
  icons,
  mui,
  useI18n,
} from './index';
import type {
  Locale as RootLocale,
  TranslationKey as RootTranslationKey,
} from './index';

describe('barrel re-exports', () => {
  it('mui barrel re-exports @mui/material', () => {
    expect(mui.Button).toBe(Mui.Button);
  });

  it('icons barrel re-exports @mui/icons-material', () => {
    expect(icons.Home).toBe(Icons.Home);
  });

  it('root barrel re-exports the focused i18n API', () => {
    expect(AegisI18nProvider).toBe(FocusedI18nProvider);
    expect(useI18n).toBe(focusedUseI18n);
  });

  it('@aegis/ui/i18n resolves to the focused i18n API', () => {
    expect(PackageI18nProvider).toBe(FocusedI18nProvider);
    expect(packageUseI18n).toBe(focusedUseI18n);
  });

  it('root and focused entry points expose matching i18n types', () => {
    const rootLocale: RootLocale = 'zh-CN';
    const packageLocale: PackageLocale = rootLocale;
    const rootKey: RootTranslationKey = 'language.english';
    const packageKey: PackageTranslationKey = rootKey;

    expect([packageLocale, packageKey]).toEqual([
      'zh-CN',
      'language.english',
    ]);
  });
});