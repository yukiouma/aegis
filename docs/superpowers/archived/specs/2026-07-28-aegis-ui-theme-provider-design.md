# `@aegis/ui` Theme Provider — Design

**Date:** 2026-07-28
**Status:** Approved (pending spec review)
**Scope:** Add a package-internal MUI theme provider and mode-switching hook to the `@aegis/ui` workspace package, so consumers can apply a theme without creating their own.

---

## 1. Goals

1. Centralize theme definitions inside `@aegis/ui` so other workspace packages cannot create their own.
2. Ship a `<AegisThemeProvider>` React component that wraps MUI's `ThemeProvider` + `CssBaseline`, owns the current mode, and persists it to `localStorage`.
3. Ship a `useThemeMode()` hook so consumers can read and change the mode.
4. Support the modes `'light'` and `'dark'` out of the box, with a clear extension path for additional modes added later by the package author.
5. Expose the provider and hook via both `@aegis/ui/theme` (focused) and `@aegis/ui` (root) subpaths.

---

## 2. Background — constraints from the user

- **Theme ownership**: `all the theme will be defined in this package, other package can not create their own theme`. Therefore `createTheme` and the underlying `Theme` objects are **not** part of the public surface. Consumers never receive a `Theme` value or a `createTheme` import from this package.
- **Modes**: `except light and dark, I may manually create other separated theme files later`. The package must make adding new modes a mechanical change (one file + one registry entry + one type-union member) without touching the provider.
- **Persistence**: the selected mode persists across reloads via `localStorage`.

---

## 3. Directory layout

```
lib/packages/ui/src/theme/
  types.ts                  # ThemeMode union (exported)
  themes/
    light.ts                # exports lightTheme: Theme  (placeholder body; user replaces later)
    dark.ts                 # exports darkTheme: Theme   (placeholder body; user replaces later)
    index.ts                # internal barrel
  registry.ts               # maps ThemeMode -> Theme   (internal)
  AegisThemeProvider.tsx
  useThemeMode.ts
  AegisThemeProvider.test.tsx
  index.ts                  # public barrel
```

The MUI-side peer dependencies already pinned in the package are sufficient — no new runtime dependencies are needed.

---

## 4. Public API

### 4.1 `ThemeMode` (types.ts)

```ts
export type ThemeMode = 'light' | 'dark';
```

This union is exported. Adding a new mode later = extend the union, add a theme file, add one line to the registry. The provider code does not need to change.

### 4.2 `AegisThemeProvider` (AegisThemeProvider.tsx)

```ts
interface AegisThemeProviderProps {
  children: React.ReactNode;
  onModeChange?: (mode: ThemeMode) => void;
}
```

- **State**: holds the current mode internally (`useState<ThemeMode>`).
- **Initial value**: reads `localStorage.getItem('aegis:theme:mode')`. If the value is a valid `ThemeMode`, use it; otherwise default to `'light'`.
- **Effect on `mode` change**: writes the new mode to `localStorage` under the same key, then calls `onModeChange?.(mode)` if provided.
- **Render**: wraps children in MUI's `<ThemeProvider theme={getTheme(mode)}>` followed by `<CssBaseline />`. The reset styles apply to the whole subtree.
- **No controlled prop**: the provider is intentionally uncontrolled. Consumers influence the mode via `useThemeMode().setMode(...)`. (YAGNI: a controlled `mode` prop adds a second ownership path with no current consumer.)

### 4.3 `useThemeMode` (useThemeMode.ts)

```ts
function useThemeMode(): { mode: ThemeMode; setMode: (mode: ThemeMode) => void };
```

- Implemented via a small React context (`AegisThemeModeContext`) that the provider creates and the hook consumes.
- The `setMode` it returns is stable (memoized with `useCallback`) and updates the same internal state the provider owns.
- Called outside a provider → throws with a clear message: `"useThemeMode must be used inside <AegisThemeProvider>"`.

### 4.4 Exports

Add to `package.json` `exports`:

```json
"./theme": "./src/theme/index.ts"
```

`src/theme/index.ts` (public barrel):

```ts
export { AegisThemeProvider } from './AegisThemeProvider';
export type { AegisThemeProviderProps } from './AegisThemeProvider';
export { useThemeMode } from './useThemeMode';
export type { ThemeMode } from './types';
```

Re-export the same symbols from the package root (`src/index.ts`) for convenience (mirrors how `Sidebar` is re-exported today):

```ts
export { AegisThemeProvider, useThemeMode } from './theme';
export type { AegisThemeProviderProps, ThemeMode } from './theme';
```

---

## 5. Theme file contract

Each theme file in `src/theme/themes/` exports a single `Theme`:

```ts
// src/theme/themes/light.ts
import { createTheme, type Theme } from '@mui/material/styles';

export const lightTheme: Theme = createTheme({
  palette: { mode: 'light' },
  // real tokens land here later
});
```

The same shape applies to `dark.ts` with `palette: { mode: 'dark' }`.

**This spec scaffolds the two files with the minimal bodies above** so the package builds and tests run today. The user will replace the bodies with the real theme files later. The provider, registry, types, and public API do not depend on the file contents.

### 5.1 Registry (registry.ts, internal)

```ts
import type { Theme } from '@mui/material/styles';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import type { ThemeMode } from './types';

const themes: Record<ThemeMode, Theme> = {
  light: lightTheme,
  dark: darkTheme,
};

export function getTheme(mode: ThemeMode): Theme {
  return themes[mode];
}
```

The registry and the theme files are **not** exported from the package. This is what enforces "other packages cannot create their own theme" — consumers have no path to a `Theme` object from this package.

---

## 6. Behavior

### 6.1 Mode resolution order

1. `localStorage` value at key `aegis:theme:mode`, if it is a valid `ThemeMode`.
2. Else `'light'`.

`typeof window` is checked before any `localStorage` access so the module is safe to import in a non-browser environment (e.g. SSR, tests with `jsdom` always provides `window`, but defensive code is cheap).

### 6.2 Persistence

- Key: `aegis:theme:mode` (literal, scoped to the package by prefix).
- Value: the mode string (`'light'` | `'dark'`).
- Writes happen inside a `useEffect` that fires on mode change. Provider mount does not write — only changes do.
- Read failures (SecurityError, quota errors) are silently ignored; the default mode applies.

### 6.3 `onModeChange`

- Fires after the localStorage write and after the state has settled.
- Receives the new mode.
- If the consumer's callback throws, the error propagates uncaught — the provider doesn't swallow it.

---

## 7. Tests

`src/theme/AegisThemeProvider.test.tsx` (vitest + jsdom + testing-library, already configured). Cases:

1. Renders children.
2. Default mode is `'light'` when `localStorage` is empty.
3. Reads the initial mode from `localStorage` on mount.
4. `useThemeMode().setMode('dark')` updates the mode and writes `'dark'` to `localStorage`.
5. Invalid stored value (`'purple'`) falls back to `'light'` and does not throw.
6. `onModeChange` fires with the new mode when the mode changes.
7. `useThemeMode` called outside a provider throws with the documented message.
8. After render, MUI's `useTheme()` called inside the subtree returns the theme from the registry for the current mode (i.e. the provider actually wires MUI's context, not just renders children).

Each test resets `localStorage` in `beforeEach`.

---

## 8. Files added / changed

**New:**
- `lib/packages/ui/src/theme/types.ts`
- `lib/packages/ui/src/theme/themes/light.ts`
- `lib/packages/ui/src/theme/themes/dark.ts`
- `lib/packages/ui/src/theme/themes/index.ts`
- `lib/packages/ui/src/theme/registry.ts`
- `lib/packages/ui/src/theme/AegisThemeProvider.tsx`
- `lib/packages/ui/src/theme/useThemeMode.ts`
- `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`
- `lib/packages/ui/src/theme/index.ts`

**Edited:**
- `lib/packages/ui/package.json` — add `"./theme"` to `exports`.
- `lib/packages/ui/src/index.ts` — re-export `AegisThemeProvider`, `useThemeMode`, `AegisThemeProviderProps`, `ThemeMode`.

No new runtime dependencies. No changes to `tsconfig.json` or `vitest.config.ts` (the existing `include` already covers `src/`).

---

## 9. Out of scope

- The actual theme tokens (palette, typography, component overrides) — the user provides those as separated theme files later.
- Wiring `aegis-desktop` to use the provider — separate change, after the real theme files exist.
- OS color-scheme detection (`prefers-color-scheme`).
- High-contrast / brand / any other modes — to be added later by extending the `ThemeMode` union and adding a theme file.
- CSS-variables theme (`theme.cssVariables`) — not needed for a Tauri desktop app.
- SSR / `InitColorSchemeScript` — Tauri is CSR-only.
- A controlled `mode` prop on the provider, and a `storageKey` prop — YAGNI; add when a consumer actually needs them.
- A toggle / switch UI component — consumers build their own with `useThemeMode()`.
