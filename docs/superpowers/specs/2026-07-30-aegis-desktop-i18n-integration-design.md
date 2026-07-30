# Aegis Desktop i18n Integration Design

**Date:** 2026-07-30
**Status:** Approved for specification
**Depends on:** `2026-07-29-aegis-ui-i18n-design.md` (implemented)

## 1. Goal

Mount the existing `@aegis/ui` i18n module inside the desktop app, translate every user-visible desktop string into English and Simplified Chinese, and add a language switcher to the Settings page. The result is a single persisted locale that controls translations across the sidebar, both pages, and the document language announcement.

This change uses the package's existing provider, hook, catalogs, and persistence. It does not modify the package's public API, registry, or persistence semantics. It does not introduce a second i18n system.

## 2. Constraints

- Build on the package i18n module exactly as designed; do not generalize it to accept host catalogs or expose new internals.
- Mount the provider inside the existing `AegisThemeProvider` so theme and i18n are independent.
- Add no runtime dependencies to either the package or the desktop app.
- Translate every user-visible desktop string in this change: sidebar app title, both sidebar menu items, the entire Home page, and the entire Settings page (including the new switcher).
- Use the existing flat namespaced key pattern (`nav.*`, `home.*`, `settings.*`, `app.*`).
- Update `document.documentElement.lang` whenever the active locale changes.
- Persist the active locale via the package's existing storage key (`aegis:i18n:locale`).
- Use `tsconfig` and the package's test setup unchanged.
- Do not modify Rust, Tauri, or `index.html` document title.
- Add desktop-level tests for the Settings page switcher and the document-language effect.

## 3. Architecture

### 3.1 Provider Mount

`apps/desktop/aegis-desktop/src/main.tsx` wraps the app in both providers. The i18n provider is mounted inside the theme provider so each provider manages its own concern:

```tsx
<React.StrictMode>
  <AegisThemeProvider>
    <AegisI18nProvider>
      <DocumentLangSync />
      <App />
    </AegisI18nProvider>
  </AegisThemeProvider>
</React.StrictMode>
```

### 3.2 Document Language Sync

A new file `apps/desktop/aegis-desktop/src/DocumentLangSync.tsx` exports a zero-render component that reads `useI18n().locale` and synchronizes `document.documentElement.lang` via a `useEffect`:

```tsx
import { useEffect } from 'react';
import { useI18n } from '@aegis/ui/i18n';

export function DocumentLangSync(): null {
  const { locale } = useI18n();
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return null;
}
```

The component returns `null` so it adds no visible DOM. It must be rendered inside `AegisI18nProvider` so the `useI18n` call resolves.

### 3.3 Component Integration

Each desktop component calls `useI18n()` and replaces its hardcoded strings with `t(key)` calls. Strings that interpolate the theme mode pass through the existing interpolation helper:

- `App.tsx`: move the `menu` array and `sidebarProps` construction into the render body so they read translated titles via `useI18n().t`. The Sidebar's `title` prop receives the translated app title.
- `HomePage.tsx`: replace the heading, welcome paragraph, and button label with `t(...)`.
- `SettingsPage.tsx`: replace the heading and theme label, render a new MUI `Select` for language switching.

## 4. Catalog Extension

Add the following keys to `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`. Existing keys (`language.english`, `language.simplifiedChinese`, `language.current`) remain unchanged.

| Key | en | zh-CN |
| --- | --- | --- |
| `app.title` | `Aegis` | `Aegis` |
| `nav.home` | `Home` | `首页` |
| `nav.settings` | `Settings` | `设置` |
| `home.heading` | `Home` | `首页` |
| `home.welcome` | `Welcome to Aegis.` | `欢迎使用 Aegis。` |
| `home.testGreet` | `Test greet` | `测试问候` |
| `settings.heading` | `Settings` | `设置` |
| `settings.theme.label` | `Theme: {mode}` | `主题：{mode}` |
| `settings.theme.dark` | `Dark` | `深色` |
| `settings.theme.light` | `Light` | `浅色` |
| `settings.language.label` | `Language` | `语言` |

The `satisfies Record<keyof typeof en, string>` clause in `zhCN.ts` continues to enforce parity at compile time. The `registry.test.ts` parity assertion continues to verify the same property at runtime.

The catalog namespace prefix carries the desktop owner's intent (`nav.*`, `home.*`, `settings.*`, `app.*`) but lives in the package because the package's `TranslationKey` is closed over its `en` catalog. This trade-off is explicit: package-owned catalogs and a single source of truth take precedence over physically separating desktop strings.

## 5. Settings Page Switcher

The new switcher renders inside an MUI `FormControl` with an `InputLabel` and a `Select<Locale>`:

```tsx
import {
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  type SelectChangeEvent,
} from '@aegis/ui/mui';
import { useI18n, type Locale } from '@aegis/ui';

const { locale, setLocale, t } = useI18n();

const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
  setLocale(event.target.value as Locale);
};

// ...
<FormControl size="small" sx={{ minWidth: 160 }}>
  <InputLabel id="language-label">{t('settings.language.label')}</InputLabel>
  <Select<Locale>
    labelId="language-label"
    value={locale}
    label={t('settings.language.label')}
    onChange={handleLanguageChange}
  >
    <MenuItem value="en">{t('language.english')}</MenuItem>
    <MenuItem value="zh-CN">{t('language.simplifiedChinese')}</MenuItem>
  </Select>
</FormControl>
```

The theme switcher remains unchanged in behavior. Its label uses interpolation to surface the localized mode name:

```tsx
<FormControlLabel
  control={<Switch checked={mode === 'dark'} onChange={handleChange} />}
  label={t('settings.theme.label', {
    mode: t(mode === 'dark' ? 'settings.theme.dark' : 'settings.theme.light'),
  })}
/>
```

## 6. Testing

### 6.1 Package Tests

No changes are required. The catalog extension is verified by the existing `lib/packages/ui/src/i18n/registry.test.ts` suite:

- The parity test (`keeps both catalogs on the same key set`) continues to assert that `zhCN` exposes every English key.
- The translation tests continue to assert that `language.english` and `language.simplifiedChinese` resolve in both locales.
- New keys are exercised implicitly by the typecheck — the `satisfies` clause in `zhCN.ts` fails compilation if a key is added to `en.ts` but not `zhCN.ts`, or vice versa.

### 6.2 Desktop Tests

Add the following tests under `apps/desktop/aegis-desktop/src/`:

- `SettingsPage.test.tsx`
  - Renders `<SettingsPage />` inside `<AegisI18nProvider>` with `defaultLocale="en"`.
  - Asserts the heading, theme label, and switcher label appear in English.
  - Selects `简体中文` and asserts the heading, theme label, and switcher option labels appear in Simplified Chinese.
  - Asserts that the active menu items in the Sidebar are not part of this test (they live in `App.tsx`).
- `DocumentLangSync.test.tsx`
  - Mounts `<DocumentLangSync />` inside `<AegisI18nProvider>`.
  - Asserts `document.documentElement.lang === 'en'` after initial render.
  - Calls `setLocale('zh-CN')` via `useI18n` and asserts `document.documentElement.lang === 'zh-CN'` after the effect runs.

The existing desktop project has no test setup. The tests use Vitest's `jsdom` environment directly, matching the package's existing tooling. The desktop `package.json` will add Vitest, `jsdom`, `@testing-library/react`, `@testing-library/user-event`, and `@testing-library/jest-dom` as dev dependencies. A `vitest.config.ts` and `vitest.setup.ts` mirror the package's configuration.

### 6.3 Verification Commands

After implementation, the following commands must each exit 0:

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

## 7. File Scope

### Files to modify

- `lib/packages/ui/src/i18n/locales/en.ts` — add the new English keys.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add the matching Simplified Chinese keys under the existing `satisfies` clause.
- `apps/desktop/aegis-desktop/src/main.tsx` — wrap `App` with `AegisI18nProvider` and `DocumentLangSync`.
- `apps/desktop/aegis-desktop/src/App.tsx` — translate `menu` titles and `title` prop via `useI18n`.
- `apps/desktop/aegis-desktop/src/HomePage.tsx` — translate heading, welcome text, and button label via `useI18n`.
- `apps/desktop/aegis-desktop/src/SettingsPage.tsx` — translate heading, theme label, and add the language switcher.
- `apps/desktop/aegis-desktop/package.json` — add Vitest + testing-library devDependencies, a `test` script, and a `typecheck` script.
- `apps/desktop/aegis-desktop/vite.config.ts` — leave unchanged (existing test runner is independent of Vite's build).
- `apps/desktop/aegis-desktop/tsconfig.json` — leave unchanged.

### Files to create

- `apps/desktop/aegis-desktop/src/DocumentLangSync.tsx` — `useI18n`-driven `useEffect` that mirrors the active locale onto `document.documentElement.lang`.
- `apps/desktop/aegis-desktop/src/SettingsPage.test.tsx` — switcher + label tests.
- `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx` — document-lang sync tests.
- `apps/desktop/aegis-desktop/vitest.config.ts` — jsdom environment + setup file.
- `apps/desktop/aegis-desktop/vitest.setup.ts` — `@testing-library/jest-dom` matchers.

### Files intentionally untouched

- `lib/packages/ui/src/i18n/registry.ts`
- `lib/packages/ui/src/i18n/AegisI18nProvider.tsx`
- `lib/packages/ui/src/i18n/useI18n.ts`
- `lib/packages/ui/src/i18n/index.ts`
- `apps/desktop/aegis-desktop/index.html`
- `apps/desktop/aegis-desktop/src-tauri/**`
- Any Rust files.

## 8. Out of Scope

- Generalizing the package i18n module to accept host-provided catalogs.
- Translating `index.html` `<title>`.
- Translating Tauri's native menu or window title.
- Adding locales beyond `en` and `zh-CN`.
- Browser/OS locale detection.
- Pluralization, ICU MessageFormat, dates, numbers, or rich text.
- Migrating the Sidebar's internal strings (it accepts opaque titles via props; the desktop app supplies the translated strings).
- Changes to the package's `AegisI18nProvider`, `useI18n`, registry, persistence, or storage key.