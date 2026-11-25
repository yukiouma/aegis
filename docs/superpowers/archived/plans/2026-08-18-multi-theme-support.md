# Multi-theme support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make all 8 themes selectable from the Aegis Settings page and keep cross-window broadcasting intact.

**Architecture:** Widen the `ThemeMode` union from 2 to 8 IDs, register the six new theme palettes in the existing registry, swap the Settings page Switch for a Select dropdown, and add the 6 new translation keys. The existing on-disk store + Tauri event pipeline (`aegis:settings-changed`) is reused — only the payload type widens.

**Tech Stack:** TypeScript, React 19, MUI v9, TanStack Router, `@tauri-apps/plugin-store`, `@tauri-apps/api/event`, Vitest, `@testing-library/react`.

## Global Constraints

- **Theme IDs** are kebab-cased strings matching the existing file names: `'light' | 'dark' | 'anya' | 'chihiro' | 'ntd' | 'sibly' | 'totoro' | 'xi'`.
- **No new persistence keys**: keep using `aegis:theme:mode` in `localStorage` and `theme` in `settings.bin` — backward compatible with values already on disk.
- **No new Tauri event names**: keep using `aegis:settings-changed`.
- **i18n**: every user-visible string uses `useI18n().t(...)` with translation keys under `settings.theme.*`. New keys are added to BOTH `en.ts` and `zhCN.ts`. `TranslationKey = keyof typeof en`, so the English catalog is the source of truth and `zhCN` must `satisfies Record<keyof typeof en, string>`.
- **Theme files are placeholders** for `'light'` and `'dark'`; do not modify `lib/packages/ui/src/theme/themes/light.ts` or `dark.ts`.
- **Broadcast pipeline is preserved**: `PersistentThemeProvider` continues to call `persistSettings({ theme })` then `emit('aegis:settings-changed', { theme })`. The listener (`useListenForSettingsChanges`) continues to call `setMode` on receipt. `useHydrateSettingsFromStore` continues to read `theme` from `settings.bin` once per mount.
- **Frequent commits**: one commit per task.

---

## File Structure

Files this plan touches:

| Path | Purpose |
| --- | --- |
| `lib/packages/ui/src/theme/types.ts` | `ThemeMode` union (widened) |
| `lib/packages/ui/src/theme/registry.ts` | ID → `Theme` map (now 8 entries) |
| `lib/packages/ui/src/theme/themes/index.ts` | Re-export all 8 theme constants |
| `lib/packages/ui/src/theme/AegisThemeProvider.tsx` | Type guard (widened) |
| `lib/packages/ui/src/theme/registry.test.ts` | Tests for the new 6 IDs |
| `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx` | Tests for new ID round-trip + invalid fallback |
| `lib/packages/ui/src/i18n/locales/en.ts` | 6 new `settings.theme.<id>` keys |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | 6 matching translations |
| `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx` | Switch → Select<ThemeMode> |
| `apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx` | New-theme round-trip test |

No new files are created. No files are split or restructured.

---

## Task 1: Widen `ThemeMode` union and the provider type guard

**Files:**
- Modify: `lib/packages/ui/src/theme/types.ts`
- Modify: `lib/packages/ui/src/theme/AegisThemeProvider.tsx`
- Test: `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`

**Interfaces:**
- Consumes: nothing (pure type change)
- Produces: `ThemeMode = 'light' | 'dark' | 'anya' | 'chihiro' | 'ntd' | 'sibly' | 'totoro' | 'xi'`

- [ ] **Step 1: Add the failing provider test for a new theme ID**

Open `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`. Insert a single new test immediately after the existing `'falls back to light on invalid stored value'` test (around line 101):

```tsx
  it('reads a non-binary theme ID like "totoro" from localStorage and writes it back', () => {
    localStorage.setItem(STORAGE_KEY, 'totoro');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    // The MUI theme palette.mode is not authoritative for character
    // themes — totoro's palette omits `mode`, so MUI defaults it to
    // 'light'. We assert the localStorage round-trip instead, which
    // is what the Settings page dropdown reads from.
    expect(localStorage.getItem(STORAGE_KEY)).toBe('totoro');
  });
```

Do not remove or change any existing tests in this file.

- [ ] **Step 2: Run the new test to verify it fails**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test -- AegisThemeProvider
```
Expected: FAIL. The pre-test calls `localStorage.setItem(STORAGE_KEY, 'totoro')`, but the provider's mount effect overwrites it with `'light'` (the fallback) because the narrow `isThemeMode` type guard does not yet accept `'totoro'`. So `localStorage.getItem(STORAGE_KEY)` returns `'light'`, not `'totoro'`.

- [ ] **Step 3: Widen `ThemeMode` in `types.ts`**

In `lib/packages/ui/src/theme/types.ts`, replace the body with:

```ts
export type ThemeMode =
  | 'light'
  | 'dark'
  | 'anya'
  | 'chihiro'
  | 'ntd'
  | 'sibly'
  | 'totoro'
  | 'xi';
```

- [ ] **Step 4: Widen `isThemeMode` in `AegisThemeProvider.tsx`**

In `lib/packages/ui/src/theme/AegisThemeProvider.tsx`, replace the existing `isThemeMode` function:

```ts
function isThemeMode(value: string | null): value is ThemeMode {
  return (
    value === 'light' ||
    value === 'dark' ||
    value === 'anya' ||
    value === 'chihiro' ||
    value === 'ntd' ||
    value === 'sibly' ||
    value === 'totoro' ||
    value === 'xi'
  );
}
```

No other provider changes — storage key stays `'aegis:theme:mode'` and the rest of the effect chain is unchanged.

- [ ] **Step 5: Run the test to verify it passes**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test -- AegisThemeProvider
```
Expected: PASS. Existing tests still pass; the new `'reads the new theme ID "totoro" from localStorage on mount and writes it back'` test passes.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis && git add lib/packages/ui/src/theme/types.ts lib/packages/ui/src/theme/AegisThemeProvider.tsx lib/packages/ui/src/theme/AegisThemeProvider.test.tsx
git commit -m "feat(theme): widen ThemeMode union to 8 IDs and update type guard"
```

---

## Task 2: Register the 6 new themes in the registry

**Files:**
- Modify: `lib/packages/ui/src/theme/themes/index.ts`
- Modify: `lib/packages/ui/src/theme/registry.ts`
- Test: `lib/packages/ui/src/theme/registry.test.ts`

**Interfaces:**
- Consumes: `ThemeMode` from Task 1; 6 new theme constants (`anyaTheme`, `chihiroTheme`, `ntdTheme`, `siblyTheme`, `totoroTheme`, `xiTheme`) exported from `themes/*Theme.ts`
- Produces: `getTheme(id: ThemeMode): Theme` returns the correct theme for all 8 IDs

- [ ] **Step 1: Write the failing registry tests**

Open `lib/packages/ui/src/theme/registry.test.ts`. Replace the file body with:

```ts
import { describe, it, expect } from 'vitest';
import { getTheme } from './registry';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import { anyaTheme } from './themes/anyaTheme';
import { chihiroTheme } from './themes/chihiroTheme';
import { ntdTheme } from './themes/ntdTheme';
import { siblyTheme } from './themes/siblyTheme';
import { totoroTheme } from './themes/totoroTheme';
import { xiTheme } from './themes/xiTheme';

describe('theme registry', () => {
  it('light mode returns the light theme', () => {
    expect(getTheme('light')).toBe(lightTheme);
  });

  it('dark mode returns the dark theme', () => {
    expect(getTheme('dark')).toBe(darkTheme);
  });

  it('anya returns the anya theme (light palette)', () => {
    expect(getTheme('anya')).toBe(anyaTheme);
    expect(getTheme('anya').palette.mode).toBe('light');
  });

  it('chihiro returns the chihiro theme (light palette)', () => {
    expect(getTheme('chihiro')).toBe(chihiroTheme);
    expect(getTheme('chihiro').palette.mode).toBe('light');
  });

  it('ntd returns the ntd theme (dark palette)', () => {
    expect(getTheme('ntd')).toBe(ntdTheme);
    expect(getTheme('ntd').palette.mode).toBe('dark');
  });

  it('sibly returns the sibly theme (dark palette)', () => {
    expect(getTheme('sibly')).toBe(siblyTheme);
    expect(getTheme('sibly').palette.mode).toBe('dark');
  });

  it('totoro returns the totoro theme (palette.mode defaults to light)', () => {
    expect(getTheme('totoro')).toBe(totoroTheme);
    expect(getTheme('totoro').palette.mode).toBe('light');
  });

  it('xi returns the xi theme (dark palette)', () => {
    expect(getTheme('xi')).toBe(xiTheme);
    expect(getTheme('xi').palette.mode).toBe('dark');
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test -- registry
```
Expected: FAIL — `themes/index.ts` does not export the new constants, and `registry.ts` does not map them.

- [ ] **Step 3: Re-export the 6 new themes from `themes/index.ts`**

Replace `lib/packages/ui/src/theme/themes/index.ts` body with:

```ts
export { lightTheme } from './light';
export { darkTheme } from './dark';
export { anyaTheme } from './anyaTheme';
export { chihiroTheme } from './chihiroTheme';
export { ntdTheme } from './ntdTheme';
export { siblyTheme } from './siblyTheme';
export { totoroTheme } from './totoroTheme';
export { xiTheme } from './xiTheme';
```

- [ ] **Step 4: Map the 6 new themes in `registry.ts`**

Replace `lib/packages/ui/src/theme/registry.ts` body with:

```ts
import type { Theme } from '@mui/material/styles';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import {
  anyaTheme,
  chihiroTheme,
  ntdTheme,
  siblyTheme,
  totoroTheme,
  xiTheme,
} from './themes';
import type { ThemeMode } from './types';

const themes: Record<ThemeMode, Theme> = {
  light: lightTheme,
  dark: darkTheme,
  anya: anyaTheme,
  chihiro: chihiroTheme,
  ntd: ntdTheme,
  sibly: siblyTheme,
  totoro: totoroTheme,
  xi: xiTheme,
};

export function getTheme(mode: ThemeMode): Theme {
  return themes[mode];
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test -- registry
```
Expected: PASS for all 8 cases.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis && git add lib/packages/ui/src/theme/themes/index.ts lib/packages/ui/src/theme/registry.ts lib/packages/ui/src/theme/registry.test.ts
git commit -m "feat(theme): register 6 new character themes in the theme registry"
```

---

## Task 3: Add the 6 new translation keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: `TranslationKey = keyof typeof en` — adding a key to `en.ts` is the source of truth
- Produces: New keys `settings.theme.anya`, `settings.theme.chihiro`, `settings.theme.ntd`, `settings.theme.sibly`, `settings.theme.totoro`, `settings.theme.xi` resolvable in both locales

- [ ] **Step 1: Run typecheck to confirm current state is clean**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm typecheck
```
Expected: clean. If not, fix any preexisting errors first and re-run.

- [ ] **Step 2: Add the 6 keys to `en.ts`**

In `lib/packages/ui/src/i18n/locales/en.ts`, immediately after the existing `'settings.theme.light': 'Light',` line (around line 15), insert:

```ts
  'settings.theme.anya': 'Anya',
  'settings.theme.chihiro': 'Chihiro',
  'settings.theme.ntd': 'NTD',
  'settings.theme.sibly': 'Sibly',
  'settings.theme.totoro': 'Totoro',
  'settings.theme.xi': 'XI',
```

- [ ] **Step 3: Add the 6 keys to `zhCN.ts`**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, immediately after the existing `'settings.theme.light': '浅色',` line (around line 17), insert:

```ts
  'settings.theme.anya': '安雅',
  'settings.theme.chihiro': '千寻',
  'settings.theme.ntd': 'NTD',
  'settings.theme.sibly': 'Sibly',
  'settings.theme.totoro': '龙猫',
  'settings.theme.xi': 'XI',
```

- [ ] **Step 4: Run typecheck**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm typecheck
```
Expected: clean. The `satisfies Record<keyof typeof en, string>` clause in `zhCN.ts` will fail if any English key is missing a Chinese counterpart.

- [ ] **Step 5: Run all UI package tests to confirm nothing broke**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test
```
Expected: PASS. (i18n catalog tests do not exist today, but type safety + the existing tests should hold.)

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis && git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add labels for the 6 new themes in en and zh-CN"
```

---

## Task 4: Replace the Settings page Switch with a Select dropdown

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx`

**Interfaces:**
- Consumes: `useThemeMode()` returning `{ mode: ThemeMode; setMode: (mode: ThemeMode) => void }`, `useI18n()` returning `{ t: (key, vars?) => string }`, `ThemeMode` widened union from Task 1, new translation keys from Task 3
- Produces: A `<Select<ThemeMode>>` with 8 `<MenuItem>`s that updates `mode` via `setMode`

- [ ] **Step 1: Update the imports**

In `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx`, remove `Switch` from the `@aegis/ui/mui` import and add `type ThemeMode` to the `@aegis/ui/theme` import. The import block at the top becomes:

```tsx
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  TextField,
  Typography,
  type SelectChangeEvent,
} from "@aegis/ui/mui";
import { useI18n, type Locale } from "@aegis/ui/i18n";
import { useThemeMode, type ThemeMode } from "@aegis/ui/theme";
```

(Removing `Switch` from the named imports; adding `ThemeMode` to the type import.)

Also remove the unused `ChangeEvent` import from `"react"` if it is no longer referenced. Replace the existing line `import { useState, type ChangeEvent } from "react";` with:

```tsx
import { useState } from "react";
```

- [ ] **Step 2: Define the theme options and remove the Switch handler**

After the existing `useLogout` and `updatePassword` declarations, add:

```tsx
const THEME_OPTIONS: readonly ThemeMode[] = [
  "light",
  "dark",
  "anya",
  "chihiro",
  "ntd",
  "sibly",
  "totoro",
  "xi",
];
```

Remove the `handleThemeChange` function and the `themeLabel` derivation (the two existing blocks that read `event.target.checked` and the `'Theme: {mode}'` interpolation).

- [ ] **Step 3: Add a typed `handleThemeSelect` handler**

Add the handler (sibling to `handleLanguageChange`):

```tsx
const handleThemeSelect = (event: SelectChangeEvent<ThemeMode>) => {
  setMode(event.target.value as ThemeMode);
};
```

- [ ] **Step 4: Replace the Switch with a Select**

Locate the `<FormControlLabel>` block that wraps the `<Switch>` (around lines 105–110). Replace the entire block — including the `<FormControlLabel>` — with:

```tsx
      <FormControl size="small" sx={{ minWidth: 160 }}>
        <InputLabel id="theme-label">
          {t("settings.theme.label")}
        </InputLabel>
        <Select<ThemeMode>
          labelId="theme-label"
          value={mode}
          label={t("settings.theme.label")}
          onChange={handleThemeSelect}
        >
          {THEME_OPTIONS.map((id) => (
            <MenuItem key={id} value={id}>
              {t(`settings.theme.${id}`)}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
```

- [ ] **Step 5: Run typecheck for the desktop app**

Run:
```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop && pnpm typecheck
```
Expected: clean. `TranslationKey` resolves all 8 `settings.theme.<id>` keys via `keyof typeof en`.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx
git commit -m "feat(settings): replace theme Switch for Select<ThemeMode> with all 8 themes"
```

---

## Task 5: Add a round-trip test for a new theme ID through the broadcast

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx`

**Interfaces:**
- Consumes: The mocking pattern already in the file (`@tauri-apps/plugin-store` and `@tauri-apps/api/event`), `ThemeMode` widened union
- Produces: A test that asserts `setMode('totoro')` triggered by an inbound `aegis:settings-changed` event is visible through `useThemeMode()`

- [ ] **Step 1: Add the failing test**

Open `apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx`. Locate the `describe("useListenForSettingsChanges"...)` block (around line 113). Add a new test after the existing `'only applies the keys present in the payload'` test (after line 159):

```tsx
  it("applies a new character theme ID broadcast from another window", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ListenProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => expect(handlers.length).toBe(1));

    await act(async () => {
      handlers[0]({ payload: { theme: "totoro" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("totoro");
    });
    expect(screen.getByTestId("locale").textContent).toBe("en");
  });
```

- [ ] **Step 2: Run the test to verify it passes**

Run:
```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop && pnpm test -- settings-persist
```
Expected: PASS. The broadcast pipeline was already widened via `ThemeMode` in Task 1; this test confirms a new ID flows end-to-end through the existing listener.

- [ ] **Step 3: Run the full UI package tests too (no regressions)**

Run:
```bash
cd d:/projects/rusty/aegis/lib/packages/ui && pnpm test
```
Expected: PASS.

- [ ] **Step 4: Run the full desktop test suite**

Run:
```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop && pnpm test
```
Expected: PASS — no regressions.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx
git commit -m "test(settings): cover theme ID round-trip through the cross-window broadcast"
```

---

## Self-Review

After completing the tasks, verify against the spec checklist:

- [ ] `ThemeMode` union is exactly the 8 IDs (no more, no less).
- [ ] All 8 themes are registered in `registry.ts` with `Record<ThemeMode, Theme>` for compile-time completeness.
- [ ] `isThemeMode` in the provider spells out all 8 IDs.
- [ ] `themes/index.ts` re-exports all 8 theme constants.
- [ ] Both `en.ts` and `zhCN.ts` declare all 6 new keys; `zhCN.ts` still satisfies `Record<keyof typeof en, string>`.
- [ ] Settings page renders a Select (not a Switch) with 8 MenuItems; the Switch import is gone.
- [ ] Storage keys (`aegis:theme:mode`, `theme`) are unchanged.
- [ ] Tauri event name (`aegis:settings-changed`) is unchanged.
- [ ] No `light.ts` or `dark.ts` file content was modified.
- [ ] All existing tests still pass; the new registry tests (8 total) and the new broadcast test pass.

If any item above fails, fix it inline before declaring the plan complete.