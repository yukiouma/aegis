# Aegis Desktop i18n Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mount the existing `@aegis/ui` i18n module inside `aegis-desktop`, translate every user-visible desktop string into English and Simplified Chinese, and add a language switcher to the Settings page that drives the active locale and updates `document.documentElement.lang`.

**Architecture:** Extend the package's existing flat catalogs with namespaced desktop keys (`app.*`, `nav.*`, `home.*`, `settings.*`). Mount `<AegisI18nProvider>` inside the existing `<AegisThemeProvider>` in `main.tsx`, with a tiny `<DocumentLangSync />` component beside `<App />` that mirrors the active locale onto `<html lang>`. Replace hardcoded strings in `App.tsx`, `HomePage.tsx`, and `SettingsPage.tsx` with `t(...)` calls. The Settings page also gains an MUI `Select<Locale>` switcher. Desktop test infrastructure (Vitest + jsdom + React Testing Library) is added so the switcher and `<DocumentLangSync />` are testable.

**Tech Stack:** TypeScript 5.8, React 19, MUI 9, Vitest 2.1, React Testing Library 16, jsdom 25, pnpm 10.33.

## Global Constraints

- Implement the approved design in `docs/superpowers/specs/2026-07-30-aegis-desktop-i18n-integration-design.md`.
- Add no runtime dependencies to either the package or the desktop app; only `devDependencies` may grow.
- Translate every user-visible desktop string in this change: sidebar app title, both sidebar menu items, the entire Home page, and the entire Settings page.
- Use the existing flat namespaced key pattern (`nav.*`, `home.*`, `settings.*`, `app.*`) added to the package's English and Simplified Chinese catalogs.
- Keep `<AegisI18nProvider>` inside `<AegisThemeProvider>` in `main.tsx`; render `<DocumentLangSync />` as a sibling of `<App />`.
- Use the package's persisted locale via the existing `aegis:i18n:locale` storage key — do not introduce a second storage path.
- Update `document.documentElement.lang` whenever the active locale changes.
- Use `tsconfig` and the package's existing test setup unchanged.
- Do not modify Rust, Tauri, or `index.html`.
- Run implementation in a worktree based on `feat/ui`, because the package i18n module only exists on `feat/ui`.
- Preserve unrelated working-tree changes and stage only the files listed by each task.
- Follow strict TDD: add the focused failing test, observe the expected failure, implement the minimum behavior, then run focused and full verification.

---

## File Structure

### Files to create

- `apps/desktop/aegis-desktop/vitest.config.ts` — Vitest config: jsdom environment + setup file.
- `apps/desktop/aegis-desktop/vitest.setup.ts` — `@testing-library/jest-dom` matchers.
- `apps/desktop/aegis-desktop/src/smoke.test.tsx` — TDD-driven smoke test used only to validate the test infrastructure in Task 2; deleted at the end of Task 2.
- `apps/desktop/aegis-desktop/src/DocumentLangSync.tsx` — `useI18n`-driven `useEffect` that mirrors the active locale onto `document.documentElement.lang`.
- `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx` — verifies the initial `<html lang>` value and the effect after a locale change.
- `apps/desktop/aegis-desktop/src/SettingsPage.test.tsx` — verifies heading, theme label, and switcher option labels appear in both locales.

### Files to modify

- `lib/packages/ui/src/i18n/locales/en.ts` — add the new English keys.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add the matching Simplified Chinese keys under the existing `satisfies` clause.
- `apps/desktop/aegis-desktop/package.json` — add Vitest + testing-library devDependencies, a `test` script, and a `typecheck` script.
- `apps/desktop/aegis-desktop/src/main.tsx` — wrap `App` with `AegisI18nProvider` and `DocumentLangSync`.
- `apps/desktop/aegis-desktop/src/App.tsx` — translate `menu` titles and `title` prop via `useI18n`.
- `apps/desktop/aegis-desktop/src/HomePage.tsx` — translate heading, welcome text, and button label via `useI18n`.
- `apps/desktop/aegis-desktop/src/SettingsPage.tsx` — translate heading, theme label, and add the language switcher.

### Files intentionally untouched

- `lib/packages/ui/src/i18n/registry.ts`
- `lib/packages/ui/src/i18n/AegisI18nProvider.tsx`
- `lib/packages/ui/src/i18n/useI18n.ts`
- `lib/packages/ui/src/i18n/index.ts`
- `lib/packages/ui/src/i18n/types.ts`
- `lib/packages/ui/src/i18n/locales/index.ts`
- `lib/packages/ui/tsconfig.json`
- `lib/packages/ui/vitest.config.ts`
- `lib/packages/ui/vitest.setup.ts`
- `apps/desktop/aegis-desktop/vite.config.ts`
- `apps/desktop/aegis-desktop/tsconfig.json`
- `apps/desktop/aegis-desktop/tsconfig.node.json`
- `apps/desktop/aegis-desktop/index.html`
- `apps/desktop/aegis-desktop/src-tauri/**`
- Any Rust files.

---

### Task 1: Extend package catalogs with desktop namespaced keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: existing keys in `en` (`language.english`, `language.simplifiedChinese`, `language.current`).
- Produces: 11 new keys added to `en` and the same 11 keys added to `zhCN` under the existing `satisfies Record<keyof typeof en, string>` clause.

- [ ] **Step 1: Run the existing parity test to confirm baseline**

Run:

```bash
pnpm -F @aegis/ui exec vitest run src/i18n/registry.test.ts
```

Expected: PASS, 8 tests. The existing `keeps both catalogs on the same key set` test must currently pass — it is the safety net for this task.

- [ ] **Step 2: Add the new English keys**

Modify `lib/packages/ui/src/i18n/locales/en.ts` to read exactly:

```ts
export const en = {
  'language.english': 'English',
  'language.simplifiedChinese': 'Simplified Chinese',
  'language.current': 'Language: {name}',

  'app.title': 'Aegis',
  'nav.home': 'Home',
  'nav.settings': 'Settings',
  'home.heading': 'Home',
  'home.welcome': 'Welcome to Aegis.',
  'home.testGreet': 'Test greet',
  'settings.heading': 'Settings',
  'settings.theme.label': 'Theme: {mode}',
  'settings.theme.dark': 'Dark',
  'settings.theme.light': 'Light',
  'settings.language.label': 'Language',
} as const;
```

- [ ] **Step 3: Add the matching Simplified Chinese keys**

Modify `lib/packages/ui/src/i18n/locales/zhCN.ts` to read exactly:

```ts
import { en } from './en';

export const zhCN = {
  'language.english': '英语',
  'language.simplifiedChinese': '简体中文',
  'language.current': '当前语言：{name}',

  'app.title': 'Aegis',
  'nav.home': '首页',
  'nav.settings': '设置',
  'home.heading': '首页',
  'home.welcome': '欢迎使用 Aegis。',
  'home.testGreet': '测试问候',
  'settings.heading': '设置',
  'settings.theme.label': '主题：{mode}',
  'settings.theme.dark': '深色',
  'settings.theme.light': '浅色',
  'settings.language.label': '语言',
} satisfies Record<keyof typeof en, string>;
```

The `satisfies` expression must remain: deleting or misspelling any English key must produce a TypeScript error.

- [ ] **Step 4: Run the parity test again to confirm the catalogs remain in sync**

Run:

```bash
pnpm -F @aegis/ui exec vitest run src/i18n/registry.test.ts
```

Expected: PASS, 8 tests. The `keeps both catalogs on the same key set` assertion confirms both files expose the same key set.

- [ ] **Step 5: Translate the new keys to confirm registry lookup works**

Run:

```bash
pnpm -F @aegis/ui exec vitest run -t "translates messages in both locales"
```

Expected: PASS. This proves the new keys resolve in both locales.

- [ ] **Step 6: Run package typecheck and the full existing UI suite**

Run:

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected: typecheck exits 0; test exits 0 with the same 51 tests still passing.

- [ ] **Step 7: Commit the catalog extension**

```bash
git add -- \
  lib/packages/ui/src/i18n/locales/en.ts \
  lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add desktop i18n keys" -m "Extend the package catalogs with namespaced keys for the desktop app: app.*, nav.*, home.*, settings.*.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Desktop test infrastructure

**Files:**
- Create: `apps/desktop/aegis-desktop/vitest.config.ts`
- Create: `apps/desktop/aegis-desktop/vitest.setup.ts`
- Create: `apps/desktop/aegis-desktop/src/smoke.test.tsx`
- Modify: `apps/desktop/aegis-desktop/package.json`

**Interfaces:**
- Consumes: the existing `lib/packages/ui/vitest.config.ts` for reference on jsdom + setup file conventions.
- Produces: a Vitest configuration that uses `jsdom`, loads `vitest.setup.ts`, and respects `@/` style imports if any (none yet). Also produces scripts `typecheck` and `test` in the desktop `package.json`.

- [ ] **Step 1: Write the failing smoke test**

Create `apps/desktop/aegis-desktop/src/smoke.test.tsx`:

```tsx
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

describe('desktop test infrastructure', () => {
  it('renders a div with the test infrastructure available', () => {
    render(<div data-testid="smoke">ready</div>);
    expect(screen.getByTestId('smoke')).toHaveTextContent('ready');
  });
});
```

- [ ] **Step 2: Run the smoke test and verify it fails**

Run:

```bash
pnpm --filter aegis-desktop exec vitest run src/smoke.test.tsx 2>&1 | tail -10
```

Expected: FAIL because `vitest` is not installed in the desktop project, or the `test` script does not exist.

- [ ] **Step 3: Create the Vitest setup file**

Create `apps/desktop/aegis-desktop/vitest.setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 4: Create the Vitest configuration**

Create `apps/desktop/aegis-desktop/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    setupFiles: ['./vitest.setup.ts'],
    globals: false,
  },
});
```

- [ ] **Step 5: Add devDependencies and scripts to the desktop package.json**

Modify `apps/desktop/aegis-desktop/package.json` to include the following `devDependencies`:

```json
"devDependencies": {
  "@tauri-apps/cli": "^2",
  "@testing-library/jest-dom": "^6.5.0",
  "@testing-library/react": "^16.0.0",
  "@testing-library/user-event": "^14.5.0",
  "@types/react": "^19.1.8",
  "@types/react-dom": "^19.1.6",
  "@vitejs/plugin-react": "^4.6.0",
  "jsdom": "^25.0.0",
  "typescript": "~5.8.3",
  "vite": "^7.0.4",
  "vitest": "^2.1.0"
}
```

Also add to the `scripts` block:

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "tauri": "tauri",
  "typecheck": "tsc --noEmit",
  "test": "vitest run",
  "test:watch": "vitest"
}
```

- [ ] **Step 6: Install the new devDependencies**

Run:

```bash
pnpm install
```

Expected: exit 0. The new devDependencies resolve cleanly.

- [ ] **Step 7: Run the smoke test and verify it passes**

Run:

```bash
pnpm --filter aegis-desktop test 2>&1 | tail -10
```

Expected: PASS, 1 test. The desktop test infrastructure is functional.

- [ ] **Step 8: Run the desktop typecheck to confirm scripts work**

Run:

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: exit 0.

- [ ] **Step 9: Remove the smoke test**

Delete `apps/desktop/aegis-desktop/src/smoke.test.tsx`. It has served its purpose; keeping it would add noise.

- [ ] **Step 10: Run the desktop test script one more time to confirm it still exits 0 with no tests**

Run:

```bash
pnpm --filter aegis-desktop test 2>&1 | tail -10
```

Expected: exit 0. Vitest reports zero tests (or zero test files) but does not fail.

- [ ] **Step 11: Commit the desktop test infrastructure**

```bash
git add -- \
  apps/desktop/aegis-desktop/vitest.config.ts \
  apps/desktop/aegis-desktop/vitest.setup.ts \
  apps/desktop/aegis-desktop/package.json
git commit -m "chore(desktop): add Vitest test infrastructure" -m "Configure jsdom and React Testing Library so the desktop app can host component tests for the i18n integration.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: DocumentLangSync component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/DocumentLangSync.tsx`
- Create: `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx`

**Interfaces:**
- Consumes: `useI18n().locale` from `@aegis/ui/i18n`.
- Produces: a named export `DocumentLangSync(): null` that renders nothing and writes `document.documentElement.lang` whenever the locale changes.

- [ ] **Step 1: Write the failing `DocumentLangSync` test**

Create `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider, useI18n } from '@aegis/ui/i18n';
import { DocumentLangSync } from './DocumentLangSync';

function createMemoryStorage(): Storage {
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

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal('localStorage', createMemoryStorage());
  document.documentElement.lang = 'en';
});

function Switcher() {
  const { setLocale } = useI18n();
  return <button onClick={() => setLocale('zh-CN')}>set-zh-CN</button>;
}

describe('DocumentLangSync', () => {
  it('mirrors the initial locale onto <html lang>', () => {
    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <DocumentLangSync />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe('zh-CN');
  });

  it('updates <html lang> when the active locale changes', async () => {
    render(
      <AegisI18nProvider>
        <DocumentLangSync />
        <Switcher />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe('en');

    await userEvent.click(screen.getByText('set-zh-CN'));

    expect(document.documentElement.lang).toBe('zh-CN');
  });
});
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
pnpm --filter aegis-desktop exec vitest run src/DocumentLangSync.test.tsx 2>&1 | tail -10
```

Expected: FAIL because `./DocumentLangSync` does not exist.

- [ ] **Step 3: Implement `DocumentLangSync`**

Create `apps/desktop/aegis-desktop/src/DocumentLangSync.tsx`:

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

- [ ] **Step 4: Run the test and verify it passes**

Run:

```bash
pnpm --filter aegis-desktop exec vitest run src/DocumentLangSync.test.tsx 2>&1 | tail -10
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Run the desktop typecheck and the full desktop test suite**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck exits 0; test exits 0.

- [ ] **Step 6: Commit `DocumentLangSync`**

```bash
git add -- \
  apps/desktop/aegis-desktop/src/DocumentLangSync.tsx \
  apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx
git commit -m "feat(desktop): mirror locale onto <html lang>" -m "Add DocumentLangSync component that follows the active i18n locale and updates document.documentElement.lang, helping assistive technology announce the correct language.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Settings page translation and language switcher

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/SettingsPage.tsx`
- Create: `apps/desktop/aegis-desktop/src/SettingsPage.test.tsx`

**Interfaces:**
- Consumes: `useI18n`, `Locale` from `@aegis/ui/i18n`; `FormControl`, `InputLabel`, `MenuItem`, `Select`, `FormControlLabel`, `Switch`, `Box`, `Typography` from `@aegis/ui/mui`; `useThemeMode` from `@aegis/ui/theme`.
- Produces: a `SettingsPage` component whose heading, theme label, and language switcher all read from the active locale; a `Select<Locale>` that calls `setLocale` on change.

- [ ] **Step 1: Write the failing `SettingsPage` test**

Create `apps/desktop/aegis-desktop/src/SettingsPage.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { AegisThemeProvider } from '@aegis/ui/theme';
import { SettingsPage } from './SettingsPage';

function createMemoryStorage(): Storage {
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

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal('localStorage', createMemoryStorage());
});

function renderSettings(defaultLocale: 'en' | 'zh-CN' = 'en') {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <SettingsPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe('SettingsPage', () => {
  it('renders English copy by default', () => {
    renderSettings();

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent(
      'Settings',
    );
    expect(screen.getByLabelText(/Theme: Light/i)).toBeInTheDocument();
    expect(screen.getByLabelText('Language')).toHaveTextContent('English');
  });

  it('renders Simplified Chinese copy when the default locale is zh-CN', () => {
    renderSettings('zh-CN');

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent('设置');
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
    expect(screen.getByLabelText('语言')).toHaveTextContent('简体中文');
  });

  it('switches locale, headings, and theme label when the user picks zh-CN', async () => {
    renderSettings('en');

    await userEvent.click(screen.getByLabelText('Language'));
    await userEvent.click(screen.getByRole('option', { name: '简体中文' }));

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent('设置');
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
pnpm --filter aegis-desktop exec vitest run src/SettingsPage.test.tsx 2>&1 | tail -10
```

Expected: FAIL because `./SettingsPage` still renders the old English copy with no switcher.

- [ ] **Step 3: Rewrite `SettingsPage` to use `useI18n` and add the language switcher**

Replace the contents of `apps/desktop/aegis-desktop/src/SettingsPage.tsx` with:

```tsx
import type { ChangeEvent } from 'react';
import {
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Switch,
  Typography,
  type SelectChangeEvent,
} from '@aegis/ui/mui';
import { useI18n, type Locale } from '@aegis/ui/i18n';
import { useThemeMode } from '@aegis/ui/theme';

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale, t } = useI18n();

  const handleThemeChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? 'dark' : 'light');
  };

  const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
    setLocale(event.target.value as Locale);
  };

  const themeLabel = t('settings.theme.label', {
    mode: t(
      mode === 'dark' ? 'settings.theme.dark' : 'settings.theme.light',
    ),
  });

  return (
    <Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography variant="h4" gutterBottom>
        {t('settings.heading')}
      </Typography>
      <FormControlLabel
        control={<Switch checked={mode === 'dark'} onChange={handleThemeChange} />}
        label={themeLabel}
      />
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
    </Box>
  );
}
```

- [ ] **Step 4: Run the test and verify it passes**

Run:

```bash
pnpm --filter aegis-desktop exec vitest run src/SettingsPage.test.tsx 2>&1 | tail -10
```

Expected: PASS, 3 tests.

- [ ] **Step 5: Run the desktop typecheck and the full desktop test suite**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck exits 0; test exits 0 with 5 tests total (2 from `DocumentLangSync` + 3 from `SettingsPage`).

- [ ] **Step 6: Commit the `SettingsPage` change**

```bash
git add -- \
  apps/desktop/aegis-desktop/src/SettingsPage.tsx \
  apps/desktop/aegis-desktop/src/SettingsPage.test.tsx
git commit -m "feat(desktop): translate Settings page and add language switcher" -m "Replace hardcoded strings with t() lookups and add an MUI Select that drives AegisI18nProvider's setLocale.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: App.tsx sidebar translations

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/App.tsx`

**Interfaces:**
- Consumes: `useI18n().t` from `@aegis/ui/i18n`.
- Produces: a `menu` array whose `title` strings are translated; a `sidebarProps.title` whose value is translated. No new exports.

- [ ] **Step 1: Rewrite `App.tsx` to translate menu and sidebar title**

Replace the contents of `apps/desktop/aegis-desktop/src/App.tsx` with:

```tsx
import { useState } from "react";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";

// MUI icon components require SvgIconProps; the Sidebar's `icon` slot is
// typed as the no-required-props `ComponentType`. Wrap each icon in a
// no-arg function so the assignment type-checks.
const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

type Page = "home" | "settings";

function pageFromLink(link: string): Page {
  return link === "/settings" ? "settings" : "home";
}

export default function App() {
  const { t } = useI18n();
  const [page, setPage] = useState<Page>("home");
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const menu: MenuItem[] = [
    { link: "/home", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => setPage(pageFromLink(link)),
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          ml: `${sidebarOpen ? 240 : 56}px`,
          transition: "margin 0.3s",
        }}
      >
        {page === "settings" ? <SettingsPage /> : <HomePage />}
      </Box>
    </Box>
  );
}
```

- [ ] **Step 2: Run the desktop typecheck**

Run:

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: exit 0. The `Sidebar` props accept opaque string titles, so this is a type-only change.

- [ ] **Step 3: Run the desktop test suite to confirm nothing regressed**

Run:

```bash
pnpm --filter aegis-desktop test
```

Expected: exit 0. The existing `DocumentLangSync` and `SettingsPage` tests still pass.

- [ ] **Step 4: Commit the `App.tsx` change**

```bash
git add apps/desktop/aegis-desktop/src/App.tsx
git commit -m "feat(desktop): translate sidebar titles via i18n" -m "Replace hardcoded sidebar menu and app titles with translations read from AegisI18nProvider.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: HomePage translations

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/HomePage.tsx`

**Interfaces:**
- Consumes: `useI18n().t` from `@aegis/ui/i18n`.
- Produces: a `HomePage` component whose heading, welcome paragraph, and button label read from the active locale.

- [ ] **Step 1: Rewrite `HomePage.tsx` to use `t(...)`**

Replace the contents of `apps/desktop/aegis-desktop/src/HomePage.tsx` with:

```tsx
import { useState } from "react";
import { Box, Button, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { invoke } from "@tauri-apps/api/core";

export function HomePage() {
  const { t } = useI18n();
  const [greetMsg, setGreetMsg] = useState("");

  async function testGreet() {
    setGreetMsg(await invoke<string>("greet", { name: "Aegis" }));
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("home.heading")}
      </Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>
        {t("home.welcome")}
      </Typography>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center" }}>
        <Button variant="contained" onClick={testGreet}>
          {t("home.testGreet")}
        </Button>
        {greetMsg && <Typography variant="body2">{greetMsg}</Typography>}
      </Stack>
    </Box>
  );
}
```

- [ ] **Step 2: Run the desktop typecheck and full test suite**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck exits 0; test exits 0.

- [ ] **Step 3: Commit the `HomePage` change**

```bash
git add apps/desktop/aegis-desktop/src/HomePage.tsx
git commit -m "feat(desktop): translate Home page via i18n" -m "Replace hardcoded heading, welcome text, and button label with translations read from AegisI18nProvider.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Mount AegisI18nProvider and DocumentLangSync in main.tsx

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`

**Interfaces:**
- Consumes: `AegisI18nProvider` from `@aegis/ui/i18n`; `AegisThemeProvider` already in use.
- Produces: a render tree of `<AegisThemeProvider><AegisI18nProvider><DocumentLangSync /><App /></AegisI18nProvider></AegisThemeProvider>`.

- [ ] **Step 1: Update `main.tsx` to wrap with both providers and the lang-sync component**

Replace the contents of `apps/desktop/aegis-desktop/src/main.tsx` with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DocumentLangSync } from "./DocumentLangSync";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DocumentLangSync />
        <App />
      </AegisI18nProvider>
    </AegisThemeProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 2: Run the desktop typecheck and full test suite**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck exits 0; test exits 0. Both providers compose without errors.

- [ ] **Step 3: Run the desktop build**

Run:

```bash
pnpm --filter aegis-desktop build
```

Expected: exit 0.

- [ ] **Step 4: Commit the `main.tsx` change**

```bash
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "feat(desktop): mount AegisI18nProvider and DocumentLangSync" -m "Wire the i18n provider inside AegisThemeProvider and add DocumentLangSync as a sibling of <App /> so the document language follows the active locale.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Final verification and scope audit

**Files:**
- Verify only; no files should change.

**Interfaces:**
- Consumes: the completed changes from Tasks 1–7.
- Produces: evidence that the implementation satisfies the design without changing Rust, Tauri, `index.html`, or unrelated files.

- [ ] **Step 1: Run all required verification commands from the repository root**

Run:

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

Expected: all five commands exit 0.

- [ ] **Step 2: Confirm the final changed-file scope**

Run:

```bash
git status --short
git diff --name-only HEAD~7..HEAD
```

Expected committed implementation paths (in addition to the prior `@aegis/ui` i18n files already on `feat/ui`):

```text
apps/desktop/aegis-desktop/package.json
apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx
apps/desktop/aegis-desktop/src/DocumentLangSync.tsx
apps/desktop/aegis-desktop/src/HomePage.tsx
apps/desktop/aegis-desktop/src/SettingsPage.test.tsx
apps/desktop/aegis-desktop/src/SettingsPage.tsx
apps/desktop/aegis-desktop/src/main.tsx
apps/desktop/aegis-desktop/vitest.config.ts
apps/desktop/aegis-desktop/vitest.setup.ts
lib/packages/ui/src/i18n/locales/en.ts
lib/packages/ui/src/i18n/locales/zhCN.ts
```

No implementation commit may contain files under `apps/desktop/aegis-desktop/src-tauri/**`, `apps/desktop/aegis-desktop/index.html`, `lib/packages/ui/src/i18n/registry.ts`, `lib/packages/ui/src/i18n/AegisI18nProvider.tsx`, `lib/packages/ui/src/i18n/useI18n.ts`, `lib/packages/ui/src/i18n/index.ts`, `lib/packages/ui/src/i18n/types.ts`, or `lib/packages/ui/src/i18n/locales/index.ts`. In an isolated worktree, `git status --short` should be empty after the final commit.

- [ ] **Step 3: Verify no runtime dependencies were added to either package**

Run:

```bash
git diff HEAD~7..HEAD -- apps/desktop/aegis-desktop/package.json | grep -E '"[a-zA-Z@].*":\s*"[\^~]?[0-9]'
git diff HEAD~7..HEAD -- lib/packages/ui/package.json | grep -E '"[a-zA-Z@].*":\s*"[\^~]?[0-9]'
```

Expected: the first command may show additions under `devDependencies` only; the second command shows no `dependencies` additions for the package. If either command reports an addition under `dependencies` (not `devDependencies`), stop and fix.

- [ ] **Step 4: Report verification evidence**

Record the exact pass counts from Vitest and the successful typecheck + build exits. If any command fails, report the command and failure output without claiming completion; use `superpowers:systematic-debugging` before changing implementation.