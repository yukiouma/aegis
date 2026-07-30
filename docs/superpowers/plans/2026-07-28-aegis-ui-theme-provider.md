# `@aegis/ui` Theme Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a package-internal MUI theme provider (`<AegisThemeProvider>`) and a `useThemeMode()` hook to `@aegis/ui`, with light/dark mode, localStorage persistence, and a public export path so other workspace packages can apply the theme without creating their own.

**Architecture:** Package owns theme definitions in `src/theme/themes/*.ts` (light/dark placeholders that the user replaces later). An internal registry maps `ThemeMode` → `Theme`. The provider owns the `mode` state and persists to `localStorage`; it wraps MUI's `ThemeProvider` + `CssBaseline`. The hook exposes `{ mode, setMode }` via a React context. Consumers never receive `createTheme` or `Theme` values from this package.

**Tech Stack:** React 19, TypeScript 5.8, MUI 9.2, Emotion 11, Vitest, @testing-library/react, pnpm 10.33 workspaces.

**Spec:** [2026-07-28-aegis-ui-theme-provider-design.md](../specs/2026-07-28-aegis-ui-theme-provider-design.md)

---

## Global Constraints

These apply to every task. Do not deviate.

- React 19.x (`react`, `react-dom`).
- MUI peer deps use `^9` (installed 9.2.0). Emotion peer deps use `^11`.
- TypeScript strict mode (matches the package's existing `tsconfig.json`).
- Package name: `@aegis/ui`. Private. No build step — exports `.ts`/`.tsx` source.
- TDD: every implementation step is preceded by a failing test step. Tests live alongside the implementation (`AegisThemeProvider.test.tsx` next to `AegisThemeProvider.tsx`).
- Commit messages: imperative mood, ≤72 chars subject, body explains "why".
- After each implementation task: run `pnpm -F @aegis/ui typecheck` AND `pnpm -F @aegis/ui test` before committing.
- The package owns the theme. **Do not** export `createTheme`, the `Theme` type, theme objects, or the registry from this package. Public surface is exactly: `AegisThemeProvider`, `AegisThemeProviderProps`, `useThemeMode`, `ThemeMode`.
- `localStorage` access is guarded by `typeof window !== 'undefined'` so importing the module in non-browser contexts (e.g. SSR fixtures) does not throw.

---

## File Structure

| Path | Responsibility |
| --- | --- |
| `lib/packages/ui/src/theme/types.ts` | `ThemeMode` union (exported). |
| `lib/packages/ui/src/theme/themes/light.ts` | `lightTheme: Theme` (placeholder body; user replaces later). |
| `lib/packages/ui/src/theme/themes/dark.ts` | `darkTheme: Theme` (placeholder body; user replaces later). |
| `lib/packages/ui/src/theme/themes/index.ts` | Internal barrel for the registry. |
| `lib/packages/ui/src/theme/registry.ts` | `getTheme(mode)` — internal, not exported from package. |
| `lib/packages/ui/src/theme/AegisThemeProvider.tsx` | Component + `AegisThemeModeContext` (context is a module-local named export, not re-exported from the public barrel). |
| `lib/packages/ui/src/theme/useThemeMode.ts` | `useThemeMode()` hook. |
| `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx` | Vitest + RTL tests for both the provider and the hook. |
| `lib/packages/ui/src/theme/index.ts` | Public barrel — `AegisThemeProvider`, `AegisThemeProviderProps`, `useThemeMode`, `ThemeMode`. |
| `lib/packages/ui/package.json` | Add `"./theme"` to `exports`. |
| `lib/packages/ui/src/index.ts` | Re-export `AegisThemeProvider`, `useThemeMode`, `AegisThemeProviderProps`, `ThemeMode` from the root. |

No new runtime dependencies. No changes to `tsconfig.json` or `vitest.config.ts` (existing `include` already covers `src/`).

---

## Task 1: Theme files, `ThemeMode`, and registry (TDD)

**Files:**
- Create: `lib/packages/ui/src/theme/types.ts`
- Create: `lib/packages/ui/src/theme/themes/light.ts`
- Create: `lib/packages/ui/src/theme/themes/dark.ts`
- Create: `lib/packages/ui/src/theme/themes/index.ts`
- Create: `lib/packages/ui/src/theme/registry.ts`
- Create: `lib/packages/ui/src/theme/registry.test.ts`

**Interfaces:**
- Produces:
  - `ThemeMode = 'light' | 'dark'` (exported from `types.ts`).
  - `lightTheme: Theme` and `darkTheme: Theme` (placeholder bodies).
  - `getTheme(mode: ThemeMode): Theme` — internal function used by the provider.

- [ ] **Step 1: Write the failing registry test**

Create `lib/packages/ui/src/theme/registry.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { getTheme } from './registry';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';

describe('theme registry', () => {
  it('light mode returns the light theme', () => {
    expect(getTheme('light')).toBe(lightTheme);
  });

  it('dark mode returns the dark theme', () => {
    expect(getTheme('dark')).toBe(darkTheme);
  });

  it('returned light theme has palette.mode === "light"', () => {
    expect(getTheme('light').palette.mode).toBe('light');
  });

  it('returned dark theme has palette.mode === "dark"', () => {
    expect(getTheme('dark').palette.mode).toBe('dark');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm -F @aegis/ui test
```

Expected: FAIL — `./registry` module not found.

- [ ] **Step 3: Create `src/theme/types.ts`**

```ts
export type ThemeMode = 'light' | 'dark';
```

- [ ] **Step 4: Create `src/theme/themes/light.ts`**

```ts
import { createTheme, type Theme } from '@mui/material/styles';

// Placeholder. The user replaces this body with the real theme tokens
// (palette, typography, component overrides, etc.) later. The provider,
// registry, and public API do not depend on the contents of this file.
export const lightTheme: Theme = createTheme({
  palette: { mode: 'light' },
});
```

- [ ] **Step 5: Create `src/theme/themes/dark.ts`**

```ts
import { createTheme, type Theme } from '@mui/material/styles';

// Placeholder. The user replaces this body with the real theme tokens
// (palette, typography, component overrides, etc.) later. The provider,
// registry, and public API do not depend on the contents of this file.
export const darkTheme: Theme = createTheme({
  palette: { mode: 'dark' },
});
```

- [ ] **Step 6: Create `src/theme/themes/index.ts`**

```ts
export { lightTheme } from './light';
export { darkTheme } from './dark';
```

- [ ] **Step 7: Create `src/theme/registry.ts`**

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

- [ ] **Step 8: Run the test to verify it passes**

```bash
pnpm -F @aegis/ui test
```

Expected: 4 registry tests PASS.

- [ ] **Step 9: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/theme/
git commit -m "feat(ui): add theme types, placeholder light/dark themes, and registry"
```

---

## Task 2: `<AegisThemeProvider>` component (TDD)

**Files:**
- Create: `lib/packages/ui/src/theme/AegisThemeProvider.tsx`
- Create: `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`

**Interfaces:**
- Produces:
  - `AegisThemeProvider({ children, onModeChange? })` — wraps children in MUI's `ThemeProvider` (theme from registry) + `CssBaseline`. Owns `mode` state. Persists `mode` to `localStorage` under key `aegis:theme:mode`. Fires `onModeChange?.(mode)` on every mode change.
  - Module-local `AegisThemeModeContext` (exported as a named export from `AegisThemeProvider.tsx` for the hook to consume; NOT re-exported from the public barrel).

- [ ] **Step 1: Write the failing provider tests**

Create `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`:

```tsx
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useTheme } from '@mui/material/styles';
import { AegisThemeProvider } from './AegisThemeProvider';
import { useThemeMode } from './useThemeMode';

const STORAGE_KEY = 'aegis:theme:mode';

function createMemoryStorage(): Storage {
  // jsdom 25 does not expose a usable `localStorage` global and throws on
  // `new Storage()`. Provide a minimal Storage-shaped shim so provider tests
  // can read/write to it.
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

function ReadThemeMode() {
  const theme = useTheme();
  return <span data-testid="theme-mode">{theme.palette.mode}</span>;
}

function ReadAndSetMode() {
  const { mode, setMode } = useThemeMode();
  return (
    <>
      <span data-testid="hook-mode">{mode}</span>
      <button onClick={() => setMode('dark')}>set-dark</button>
    </>
  );
}

function ReadHookMode() {
  const { mode } = useThemeMode();
  return <span data-testid="hook-mode">{mode}</span>;
}

beforeEach(() => {
  // jsdom 25 leaves `localStorage` as an empty `{}` by default; install a
  // fresh in-memory shim so provider tests can read/write to it.
  vi.stubGlobal('localStorage', createMemoryStorage());
  vi.restoreAllMocks();
});

describe('AegisThemeProvider', () => {
  it('renders children', () => {
    render(
      <AegisThemeProvider>
        <span data-testid="child">child</span>
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('default mode is light when localStorage is empty', () => {
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
  });

  it('reads initial mode from localStorage on mount', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('dark');
  });

  it('falls back to light on invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, 'purple');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
  });

  it('writes the current mode to localStorage on mount', () => {
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
  });

  it('mirrors mode changes into the MUI theme', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('dark');
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('dark');
  });

  it('writes the new mode to localStorage when setMode is called', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });

  it('fires onModeChange with the new mode when mode changes', async () => {
    const onModeChange = vi.fn();
    render(
      <AegisThemeProvider onModeChange={onModeChange}>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    await userEvent.click(screen.getByText('set-dark'));
    expect(onModeChange).toHaveBeenCalledWith('dark');
  });

  it('useThemeMode throws when called outside a provider', () => {
    // Suppress the React error boundary noise from the expected throw.
    const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => render(<ReadAndSetMode />)).toThrow(
      /useThemeMode must be used inside <AegisThemeProvider>/,
    );
    errSpy.mockRestore();
  });

  it('useThemeMode returns the current mode', () => {
    render(
      <AegisThemeProvider>
        <ReadHookMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
  });

  it('useThemeMode.setMode is stable across renders', () => {
    const seen: Set<unknown> = new Set();
    function Capture() {
      const { setMode } = useThemeMode();
      seen.add(setMode);
      return null;
    }
    const { rerender } = render(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    rerender(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    expect(seen.size).toBe(1);
  });

  it('useThemeMode.setMode("dark") updates mode and writes to localStorage', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('dark');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm -F @aegis/ui test
```

Expected: FAIL — the test file imports `./useThemeMode` and `./AegisThemeProvider`, neither of which exists. Vitest reports a module-resolution failure.

- [ ] **Step 3: Create a stub `useThemeMode.ts` (so the test file compiles)**

Create `lib/packages/ui/src/theme/useThemeMode.ts`:

```ts
import { useContext } from 'react';
import { AegisThemeModeContext } from './AegisThemeProvider';

export function useThemeMode() {
  const ctx = useContext(AegisThemeModeContext);
  if (!ctx) {
    throw new Error('useThemeMode must be used inside <AegisThemeProvider>');
  }
  return ctx;
}
```

This file imports `AegisThemeModeContext` from the provider, which still doesn't exist. The test file will still fail to compile. Proceed to Step 4.

- [ ] **Step 4: Create a stub `AegisThemeProvider.tsx` (so the test file compiles)**

Create `lib/packages/ui/src/theme/AegisThemeProvider.tsx`:

```tsx
import { createContext, type ReactNode } from 'react';
import type { ThemeMode } from './types';

export interface AegisThemeModeContextValue {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}

export const AegisThemeModeContext = createContext<AegisThemeModeContextValue | null>(null);

export interface AegisThemeProviderProps {
  children: ReactNode;
  onModeChange?: (mode: ThemeMode) => void;
}

// Stub — full implementation lands in Step 6.
export function AegisThemeProvider(_props: AegisThemeProviderProps): null {
  return null;
}
```

- [ ] **Step 5: Run the test to verify the partial failure**

```bash
pnpm -F @aegis/ui test
```

Expected: 8 of 9 tests FAIL. The "useThemeMode throws when called outside a provider" test PASSES because the stub already implements the throw. All other tests fail because the stub provider returns `null` (no children, no MUI theme, no localStorage write).

- [ ] **Step 6: Implement the full provider**

Replace `lib/packages/ui/src/theme/AegisThemeProvider.tsx`:

```tsx
import { createContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { ThemeProvider } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import type { ThemeMode } from './types';
import { getTheme } from './registry';

const STORAGE_KEY = 'aegis:theme:mode';

function isThemeMode(value: string | null): value is ThemeMode {
  return value === 'light' || value === 'dark';
}

function readInitialMode(): ThemeMode {
  if (typeof window === 'undefined') return 'light';
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (isThemeMode(stored)) return stored;
  } catch {
    // localStorage may throw in private modes / sandboxed contexts.
  }
  return 'light';
}

export interface AegisThemeModeContextValue {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}

export const AegisThemeModeContext = createContext<AegisThemeModeContextValue | null>(null);

export interface AegisThemeProviderProps {
  children: ReactNode;
  onModeChange?: (mode: ThemeMode) => void;
}

export function AegisThemeProvider({ children, onModeChange }: AegisThemeProviderProps) {
  const [mode, setModeState] = useState<ThemeMode>(readInitialMode);

  const setMode = useCallback((next: ThemeMode) => {
    setModeState(next);
  }, []);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem(STORAGE_KEY, mode);
      } catch {
        // ignore write failures (quota, private mode)
      }
    }
    onModeChange?.(mode);
  }, [mode, onModeChange]);

  return (
    <AegisThemeModeContext.Provider value={{ mode, setMode }}>
      <ThemeProvider theme={getTheme(mode)}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </AegisThemeModeContext.Provider>
  );
}
```

- [ ] **Step 7: Run the test to verify it passes**

```bash
pnpm -F @aegis/ui test
```

Expected: 9 tests PASS.

- [ ] **Step 8: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/theme/AegisThemeProvider.tsx lib/packages/ui/src/theme/AegisThemeProvider.test.tsx lib/packages/ui/src/theme/useThemeMode.ts
git commit -m "feat(ui): render AegisThemeProvider with localStorage persistence"
```

---

## Task 3: `useThemeMode` hook — focused coverage (TDD)

The provider test in Task 2 already exercises the hook end-to-end. This task adds direct, focused tests for the hook's contract and asserts that no extension surface for "create your own theme" exists.

**Files:**
- Modify: `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx` (append `useThemeMode` block)

**Interfaces:**
- Produces: `useThemeMode()` returns `{ mode, setMode }` where the `setMode` reference is stable across renders (memoized).

- [ ] **Step 1: Append the focused hook tests**

Append at the end of `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`, before the file's final `});` close (the outer `describe('AegisThemeProvider', ...)` block):

```tsx
  it('useThemeMode returns the current mode', () => {
    render(
      <AegisThemeProvider>
        <ReadHookMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
  });

  it('useThemeMode.setMode is stable across renders', () => {
    const seen: Set<unknown> = new Set();
    function Capture() {
      const { setMode } = useThemeMode();
      seen.add(setMode);
      return null;
    }
    const { rerender } = render(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    rerender(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    expect(seen.size).toBe(1);
  });

  it('useThemeMode.setMode("dark") updates mode and writes to localStorage', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('dark');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });
```

Add a small helper at the top of the file (next to the existing helpers):

```tsx
function ReadHookMode() {
  const { mode } = useThemeMode();
  return <span data-testid="hook-mode">{mode}</span>;
}
```

(TypeScript will complain about the existing `screen.getByTestId('hook-mode')` references in Task 2 tests if `ReadHookMode` is not defined at the top. Add it now.)

- [ ] **Step 2: Run the test to verify the new tests pass**

```bash
pnpm -F @aegis/ui test
```

Expected: all 12 tests PASS (the 3 new ones plus the 9 from Task 2). Since the implementation already exists from Task 2, these tests are written after the code — they formalize the contract that the implementation already satisfies. If any fail, the implementation in Task 2 is wrong.

The new tests are written **after** the implementation exists, so they should pass immediately. This is acceptable here because the hook has only one behavior — read state and return a stable setter — and Task 2 already covered the integration paths. The task exists to lock the hook's contract with direct assertions.

- [ ] **Step 3: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/theme/AegisThemeProvider.test.tsx
git commit -m "test(ui): add focused useThemeMode contract tests"
```

---

## Task 4: Public exports and package wiring

**Files:**
- Create: `lib/packages/ui/src/theme/index.ts`
- Modify: `lib/packages/ui/src/index.ts`
- Modify: `lib/packages/ui/package.json`

**Interfaces:**
- Produces:
  - `import { AegisThemeProvider, useThemeMode, type ThemeMode, type AegisThemeProviderProps } from '@aegis/ui/theme'` resolves.
  - `import { AegisThemeProvider, useThemeMode, type ThemeMode, type AegisThemeProviderProps } from '@aegis/ui'` resolves.
  - `createTheme`, `Theme`, `lightTheme`, `darkTheme`, `getTheme`, and `AegisThemeModeContext` are **not** importable from either subpath.

- [ ] **Step 1: Create `src/theme/index.ts` (public barrel)**

```ts
export { AegisThemeProvider } from './AegisThemeProvider';
export type { AegisThemeProviderProps } from './AegisThemeProvider';
export { useThemeMode } from './useThemeMode';
export type { ThemeMode } from './types';
```

**Important**: do not export `AegisThemeModeContext`, `getTheme`, `lightTheme`, `darkTheme`, `createTheme`, or `Theme` from this barrel. Those are internal.

- [ ] **Step 2: Verify the context is not reachable from the barrel**

No code change — verification only. Run:

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS. The barrel's `export` clause has no entries for the context.

- [ ] **Step 3: Re-export from the package root**

Replace `lib/packages/ui/src/index.ts` contents:

```ts
export * as mui from './mui';
export * as icons from './icons';

export { Sidebar } from '../components/Sidebar';
export type { MenuItem, SubMenuItem, SidebarProps } from '../components/Sidebar';

export { AegisThemeProvider, useThemeMode } from './theme';
export type { AegisThemeProviderProps, ThemeMode } from './theme';
```

- [ ] **Step 4: Add `./theme` to the package's exports map**

Modify `lib/packages/ui/package.json`. The `exports` field currently reads:

```json
"exports": {
  ".": "./src/index.ts",
  "./Sidebar": "./components/Sidebar/index.ts",
  "./mui": "./src/mui/index.ts",
  "./icons": "./src/icons/index.ts"
}
```

Add a `"./theme"` entry. Final shape:

```json
"exports": {
  ".": "./src/index.ts",
  "./Sidebar": "./components/Sidebar/index.ts",
  "./mui": "./src/mui/index.ts",
  "./icons": "./src/icons/index.ts",
  "./theme": "./src/theme/index.ts"
}
```

- [ ] **Step 5: Verify the barrel smoke test still passes**

The existing `src/index.test.ts` checks `mui.Button` and `icons.Home`. Run the suite:

```bash
pnpm -F @aegis/ui test
```

Expected: 14 tests PASS (2 barrel + 4 registry + 8 provider/hook).

- [ ] **Step 6: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/theme/index.ts lib/packages/ui/src/index.ts lib/packages/ui/package.json
git commit -m "feat(ui): expose theme provider via @aegis/ui and @aegis/ui/theme"
```

---

## Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Run the full check suite**

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected:
- `typecheck` PASS.
- `test` PASS — 14 tests total, all green.

- [ ] **Step 2: Spot-check the public surface**

Run a one-off typecheck that imports the public symbols from both subpaths. From the repo root:

```bash
cd d:/projects/rusty/aegis
node -e "import('./lib/packages/ui/src/index.ts').then(m => { console.log('root:', Object.keys(m).sort().join(',')); })"
```

Expected output (alphabetical):

```
root: AegisThemeProvider,MenuItem,Sidebar,SidebarProps,SubMenuItem,icons,mui,useThemeMode
```

Note the absence of `useTheme`, `Theme`, `createTheme`, `lightTheme`, `darkTheme`, `getTheme`, `AegisThemeModeContext`, or `AegisThemeModeContextValue` — none of those should be reachable from the package root.

- [ ] **Step 3: Confirm the spec's "no consumer theme creation" guarantee**

Spot-check that the package does not re-export `createTheme`:

```bash
cd d:/projects/rusty/aegis
grep -RIn "createTheme" lib/packages/ui/src
```

Expected output: matches only in `src/theme/themes/light.ts` and `src/theme/themes/dark.ts` (the placeholder theme files). No matches in `src/theme/index.ts`, `src/theme/AegisThemeProvider.tsx`, `src/theme/useThemeMode.ts`, `src/theme/registry.ts`, or `src/index.ts`.

- [ ] **Step 4: Commit (only if Step 2 or 3 surfaced a deviation that was fixed)**

If everything matches, no commit.

---

## Done Criteria

- [ ] All 5 tasks committed on the current branch.
- [ ] `pnpm -F @aegis/ui typecheck` PASS.
- [ ] `pnpm -F @aegis/ui test` PASS — 14 tests total (2 barrel + 4 registry + 8 provider/hook), all green.
- [ ] `import { AegisThemeProvider, useThemeMode, type ThemeMode } from '@aegis/ui/theme'` resolves.
- [ ] `import { AegisThemeProvider, useThemeMode, type ThemeMode } from '@aegis/ui'` resolves.
- [ ] `createTheme`, `Theme`, `lightTheme`, `darkTheme`, `getTheme`, and `AegisThemeModeContext` are NOT importable from the package.
- [ ] No new runtime dependencies added to `package.json`.
