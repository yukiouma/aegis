# Aegis Desktop — TanStack Router File-Based Routing Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the manual `useState<Page>` page-switching in `aegis-desktop`'s `App.tsx` with `@tanstack/react-router` file-based routing; reorganise routes into `src/routes/` and tests into `src/test/` per the TanStack official testing guide.

**Architecture:** The `@tanstack/router-plugin/vite` (already installed) generates `src/routes/routeTree.gen.ts` on each `vite dev` / `vite build`. Three route files (`__root.tsx`, `index.tsx`, `settings.tsx`) replace the existing `App.tsx`/`HomePage.tsx`/`SettingsPage.tsx`. A new `src/test/file-route-utils.tsx` exports `renderInRouter` (leaf component) and `renderWithFullRouter` (full layout) helpers; all tests move to `src/test/`. The generated `routeTree.gen.ts` is committed so `tsc --noEmit` can type-check the app without a separate generate step.

**Tech Stack:** React 19, TanStack Router v1.170, TanStack Router Devtools v1.167, Vite v7, Vitest v2.1, Testing Library v16. Workspace dep: `@aegis/ui`.

---

## Global Constraints

- File-based routing mode: routes live in `src/routes/`; the plugin-generated `routeTree.gen.ts` is committed to the repo (so `tsc --noEmit` and CI typecheck can find it without running Vite first).
- All MUI imports go through `@aegis/ui/mui`, `@aegis/ui/icons`, `@aegis/ui/i18n`, `@aegis/ui/theme` — never direct `@mui/material` / `@mui/icons-material`.
- TypeScript `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true` — every step must satisfy these.
- Existing test cases for `SettingsPage` and `DocumentLangSync` carry over verbatim (same assertions, same setup, same stubs). No behaviour change.
- Tests use Vitest + Testing Library (`@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom/vitest`).
- Commit messages use a conventional prefix: `refactor(desktop)` for the implementation, `test(desktop)` for test scaffolding, `chore(desktop)` for config / cleanup.
- Verification commands after every task: `pnpm --filter aegis-desktop typecheck` and `pnpm --filter aegis-desktop test`. The implementation tasks (4–6) also run `pnpm --filter aegis-desktop build`.
- Do not modify `vite.config.ts` — the `@tanstack/router-plugin/vite` is already wired with `target: 'react'` and `autoCodeSplitting: true`.

---

### Task 1: Scaffold the `src/test/` infrastructure

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/setup.ts`
- Create: `apps/desktop/aegis-desktop/src/test/file-route-utils.tsx`
- Modify: `apps/desktop/aegis-desktop/vitest.config.ts`

**Interfaces:**
- `renderInRouter(ui, options?) => { ...renderResult, router }` — minimal router at `/` with only the page-under-test mounted; no real layout.
- `renderWithFullRouter(options?) => { ...renderResult, router }` — mounts the real `routeTree` (including `__root.tsx`) with an in-memory history; used by navigation tests.
- Both helpers accept `initialEntries?: string[]` (default `['/']`) and forward remaining `RenderOptions` to `@testing-library/react`'s `render`.

- [ ] **Step 1: Create `src/test/setup.ts`**

Create the file `apps/desktop/aegis-desktop/src/test/setup.ts` with:

```ts
import "@testing-library/jest-dom/vitest";
```

This is byte-equivalent to the existing `apps/desktop/aegis-desktop/vitest.setup.ts` at the project root (which we will delete in Task 2).

- [ ] **Step 2: Update `vitest.config.ts` to point at the new setup file**

In `apps/desktop/aegis-desktop/vitest.config.ts`, replace the `setupFiles` line:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: false,
    passWithNoTests: true,
  },
});
```

Leave the old root `vitest.setup.ts` in place for now — both files exist briefly. Verification at the end of this task runs `pnpm --filter aegis-desktop test` to prove Vitest picks up the new setup file (the existing test files at `src/SettingsPage.test.tsx` and `src/DocumentLangSync.test.tsx` should still pass because they import nothing from the setup file directly).

- [ ] **Step 3: Create `src/test/file-route-utils.tsx`**

Create the file `apps/desktop/aegis-desktop/src/test/file-route-utils.tsx`:

```tsx
import { ReactNode } from "react";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render, type RenderOptions } from "@testing-library/react";
import { routeTree } from "../routes/routeTree.gen";

interface RenderInRouterOptions extends Omit<RenderOptions, "wrapper"> {
  initialEntries?: string[];
}

/**
 * Render a component in a minimal router at "/" — for testing a single page
 * in isolation (no real layout, no Sidebar). Use `renderWithFullRouter` to
 * exercise the full `__root.tsx` layout and navigation.
 */
export function renderInRouter(
  ui: ReactNode,
  { initialEntries = ["/"], ...renderOptions }: RenderInRouterOptions = {},
) {
  const history = createMemoryHistory({ initialEntries });

  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });

  const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <>{ui}</>,
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([indexRoute]),
    history,
  });

  return {
    ...render(<RouterProvider router={router} />, renderOptions),
    router,
  };
}

interface RenderWithFullRouterOptions extends Omit<RenderOptions, "wrapper"> {
  initialEntries?: string[];
}

/**
 * Render the full app routeTree (including `__root.tsx` layout) with an
 * in-memory history. Use this for tests that exercise the Sidebar, layout,
 * or navigation between real routes.
 */
export function renderWithFullRouter({
  initialEntries = ["/"],
  ...renderOptions
}: RenderWithFullRouterOptions = {}) {
  const history = createMemoryHistory({ initialEntries });
  const router = createRouter({ routeTree, history });

  return {
    ...render(<RouterProvider router={router} />, renderOptions),
    router,
  };
}
```

- [ ] **Step 4: Verify existing tests still pass**

Run:

```bash
pnpm --filter aegis-desktop test
```

Expected: PASS. The existing `src/SettingsPage.test.tsx` and `src/DocumentLangSync.test.tsx` still run and pass; Vitest now uses `src/test/setup.ts` (proven by `--reporter=verbose` showing the setup file path, or simply by the tests passing — if setup failed, every `toBeInTheDocument()` assertion would error).

If the test command complains about `routeTree.gen.ts` being missing, that is expected at this stage — the helper imports it but the file doesn't exist yet. Skip the import until Task 3. **Workaround for now:** temporarily comment out the `import { routeTree } from "../routes/routeTree.gen";` line and the body of `renderWithFullRouter` (return `{ ...render(<></>), router: undefined as never }`). Restore both in Task 3 after the route files are created.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/setup.ts apps/desktop/aegis-desktop/src/test/file-route-utils.tsx apps/desktop/aegis-desktop/vitest.config.ts
git commit -m "test(desktop): scaffold src/test/ with renderInRouter + renderWithFullRouter"
```

---

### Task 2: Migrate existing tests into `src/test/`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/document-lang-sync.test.tsx`
- Delete: `apps/desktop/aegis-desktop/src/SettingsPage.test.tsx`
- Delete: `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx`
- Delete: `apps/desktop/aegis-desktop/vitest.setup.ts`

**Interfaces:**
- `src/test/routes/settings.test.tsx` uses `renderInRouter(<SettingsPage/>)`. The import path points at the soon-to-exist `src/routes/settings.tsx` via `../../routes/settings`; until Task 3 creates that file, the test imports from the existing `src/SettingsPage.tsx` (path `../../SettingsPage`) and gets retargeted in Task 3.
- `src/test/document-lang-sync.test.tsx` is a pure RTL render — no router helper needed.

- [ ] **Step 1: Create `src/test/document-lang-sync.test.tsx` (verbatim relocation)**

Create the file `apps/desktop/aegis-desktop/src/test/document-lang-sync.test.tsx`. The content is byte-equivalent to today's `apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";
import { DocumentLangSync } from "../../DocumentLangSync";

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
  vi.stubGlobal("localStorage", createMemoryStorage());
  document.documentElement.lang = "en";
});

afterEach(() => {
  cleanup();
});

function Switcher() {
  const { setLocale } = useI18n();
  return <button onClick={() => setLocale("zh-CN")}>set-zh-CN</button>;
}

describe("DocumentLangSync", () => {
  it("mirrors the initial locale onto <html lang>", () => {
    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <DocumentLangSync />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe("zh-CN");
  });

  it("updates <html lang> when the active locale changes", async () => {
    render(
      <AegisI18nProvider>
        <DocumentLangSync />
        <Switcher />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe("en");

    await userEvent.click(screen.getByText("set-zh-CN"));

    expect(document.documentElement.lang).toBe("zh-CN");
  });
});
```

Note the import path is now `../../DocumentLangSync` (two levels up from `src/test/`).

- [ ] **Step 2: Create `src/test/routes/settings.test.tsx` using `renderInRouter`**

Create the file `apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx`. It carries over the three assertions from the existing `SettingsPage.test.tsx` and uses the new `renderInRouter` helper. The SettingsPage component lives in `src/SettingsPage.tsx` for now; we re-target the import in Task 3 once `src/routes/settings.tsx` exists.

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { SettingsPage } from "../../SettingsPage";
import { renderInRouter } from "../file-route-utils";

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
  vi.stubGlobal("localStorage", createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderSettings(defaultLocale: "en" | "zh-CN" = "en") {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <SettingsPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("SettingsPage", () => {
  it("renders English copy by default", () => {
    renderSettings();

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent(
      "Settings",
    );
    expect(screen.getByLabelText(/Theme: Light/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Language")).toHaveTextContent("English");
  });

  it("renders Simplified Chinese copy when the default locale is zh-CN", () => {
    renderSettings("zh-CN");

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent("设置");
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
    expect(screen.getByLabelText("语言")).toHaveTextContent("简体中文");
  });

  it("switches locale, headings, and theme label when the user picks zh-CN", async () => {
    renderSettings("en");

    await userEvent.click(screen.getByLabelText("Language"));
    await userEvent.click(
      screen.getByRole("option", { name: "Simplified Chinese" }),
    );

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent("设置");
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
  });
});
```

Note the import path `../../SettingsPage` resolves to `src/SettingsPage.tsx` (still present at this stage). We retarget to `../../routes/settings` in Task 3.

- [ ] **Step 3: Delete the old test files and the old setup file**

```bash
rm apps/desktop/aegis-desktop/src/SettingsPage.test.tsx
rm apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx
rm apps/desktop/aegis-desktop/vitest.setup.ts
```

- [ ] **Step 4: Verify only the new tests run, and they pass**

Run:

```bash
pnpm --filter aegis-desktop test
```

Expected: PASS. Vitest reports exactly two test files: `src/test/document-lang-sync.test.tsx` (2 tests) and `src/test/routes/settings.test.tsx` (3 tests). All 5 tests pass.

If `renderInRouter` reports an error about `routeTree.gen.ts` (because Task 3 hasn't run yet and the helper still imports it), confirm the workaround from Task 1 Step 4 is in place — the import is temporarily commented out, or `renderInRouter` is using a minimal in-line tree. If `renderInRouter` itself doesn't import `routeTree.gen.ts` (it doesn't — only `renderWithFullRouter` does), this is fine.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx apps/desktop/aegis-desktop/src/test/document-lang-sync.test.tsx
git rm apps/desktop/aegis-desktop/src/SettingsPage.test.tsx apps/desktop/aegis-desktop/src/DocumentLangSync.test.tsx apps/desktop/aegis-desktop/vitest.setup.ts
git commit -m "test(desktop): relocate tests to src/test/ and use renderInRouter"
```

---

### Task 3: Create the `src/routes/` directory with three route files

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/__root.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/index.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/settings.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (generated by the plugin)
- Retarget: `apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx` (import `../../routes/settings` instead of `../../SettingsPage`)

**Interfaces:**
- `src/routes/__root.tsx` exports a default component `RootLayout` that owns `sidebarOpen` state, renders `<Sidebar/>` + `<Outlet/>`, and passes `navigate({ to: link })` as the Sidebar's `onNavigate`.
- `src/routes/index.tsx` exports `Route = createFileRoute("/")({ component: HomePage })` and a `HomePage` named export.
- `src/routes/settings.tsx` exports `Route = createFileRoute("/settings")({ component: SettingsPage })` and a `SettingsPage` named export.

- [ ] **Step 1: Create `src/routes/__root.tsx`**

Create `apps/desktop/aegis-desktop/src/routes/__root.tsx`:

```tsx
import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

export default function RootLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
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
        <Outlet />
      </Box>
    </Box>
  );
}
```

- [ ] **Step 2: Create `src/routes/index.tsx`**

Create `apps/desktop/aegis-desktop/src/routes/index.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { Box, Button, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { invoke } from "@tauri-apps/api/core";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
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

- [ ] **Step 3: Create `src/routes/settings.tsx`**

Create `apps/desktop/aegis-desktop/src/routes/settings.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import type { ChangeEvent } from "react";
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
} from "@aegis/ui/mui";
import { useI18n, type Locale } from "@aegis/ui/i18n";
import { useThemeMode } from "@aegis/ui/theme";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale, t } = useI18n();

  const handleThemeChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? "dark" : "light");
  };
  const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
    setLocale(event.target.value as Locale);
  };

  const themeLabel = t("settings.theme.label", {
    mode: t(mode === "dark" ? "settings.theme.dark" : "settings.theme.light"),
  });

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4" gutterBottom>
        {t("settings.heading")}
      </Typography>
      <FormControlLabel
        control={
          <Switch checked={mode === "dark"} onChange={handleThemeChange} />
        }
        label={themeLabel}
      />
      <FormControl size="small" sx={{ minWidth: 160 }}>
        <InputLabel id="language-label">
          {t("settings.language.label")}
        </InputLabel>
        <Select<Locale>
          labelId="language-label"
          value={locale}
          label={t("settings.language.label")}
          onChange={handleLanguageChange}
        >
          <MenuItem value="en">{t("language.english")}</MenuItem>
          <MenuItem value="zh-CN">{t("language.simplifiedChinese")}</MenuItem>
        </Select>
      </FormControl>
    </Box>
  );
}
```

- [ ] **Step 4: Generate `routeTree.gen.ts`**

Run a single Vite build (or dev) to let `@tanstack/router-plugin/vite` walk the new `src/routes/` directory and emit `routeTree.gen.ts`:

```bash
cd apps/desktop/aegis-desktop && pnpm exec vite build
```

Expected: build succeeds. `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` now exists and references `__root.tsx`, `index.tsx`, `settings.tsx`. The build will fail at the `tsc` step (which runs first via `pnpm build`) if `routeTree.gen.ts` does not yet exist — that's why we use `pnpm exec vite build` directly, bypassing `tsc`.

If you prefer the dev server:

```bash
cd apps/desktop/aegis-desktop && timeout 15 pnpm exec vite dev || true
```

This boots Vite for 15 seconds, lets the plugin emit `routeTree.gen.ts`, then exits. Inspect the file to confirm it exists.

- [ ] **Step 5: Retarget `src/test/routes/settings.test.tsx` import**

In `apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx`, change:

```tsx
import { SettingsPage } from "../../SettingsPage";
```

to:

```tsx
import { SettingsPage } from "../../routes/settings";
```

If you kept the workaround in `file-route-utils.tsx` from Task 1 (commented-out `routeTree.gen.ts` import), restore it now:

```tsx
import { routeTree } from "../routes/routeTree.gen";
```

…and restore the body of `renderWithFullRouter` to use `routeTree`.

- [ ] **Step 6: Verify typecheck and tests pass**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck PASS (no errors), tests PASS. The settings test now exercises the route file at `src/routes/settings.tsx` via `renderInRouter`. The DocumentLangSync test continues to pass unchanged.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/__root.tsx apps/desktop/aegis-desktop/src/routes/index.tsx apps/desktop/aegis-desktop/src/routes/settings.tsx apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx apps/desktop/aegis-desktop/src/test/file-route-utils.tsx
git commit -m "refactor(desktop): create src/routes/ with __root, index, settings"
```

---

### Task 4: Wire `main.tsx` to `RouterProvider`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`
- (Old `src/App.tsx` is no longer imported but stays on disk until Task 5 deletes it.)

**Interfaces:**
- `main.tsx` exports nothing; its side effect is `ReactDOM.createRoot(...).render(...)`.
- Inside the tree: `<AegisThemeProvider><AegisI18nProvider><DocumentLangSync /><RouterProvider router={router} /></AegisI18nProvider></AegisThemeProvider>`.
- `router` is created at module load from `routeTree`; `Register` augmentation enables typed `Link` / `useNavigate`.

- [ ] **Step 1: Replace the contents of `src/main.tsx`**

Replace the entire body of `apps/desktop/aegis-desktop/src/main.tsx` with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { routeTree } from "./routes/routeTree.gen";
import { DocumentLangSync } from "./DocumentLangSync";

const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DocumentLangSync />
        <RouterProvider router={router} />
      </AegisI18nProvider>
    </AegisThemeProvider>
  </React.StrictMode>,
);
```

`App` is no longer imported; `src/App.tsx` becomes dead code on disk and is removed in Task 5.

- [ ] **Step 2: Verify typecheck, build, and tests all pass**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three commands PASS.

- `typecheck` confirms the `Register` augmentation compiles and `routeTree.gen.ts` types are valid.
- `build` confirms `tsc --noEmit && vite build` works end-to-end.
- `test` confirms the existing migrated tests still pass with the new entry point.

If `build` complains that `routeTree.gen.ts` is missing (because someone wiped it), re-run `pnpm exec vite build` once to regenerate, then re-run the full `pnpm build`.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "refactor(desktop): mount RouterProvider in main.tsx"
```

---

### Task 5: Delete the old `App.tsx`, `HomePage.tsx`, `SettingsPage.tsx`, `App.css`

**Files:**
- Delete: `apps/desktop/aegis-desktop/src/App.tsx`
- Delete: `apps/desktop/aegis-desktop/src/HomePage.tsx`
- Delete: `apps/desktop/aegis-desktop/src/SettingsPage.tsx`
- Delete: `apps/desktop/aegis-desktop/src/App.css`

- [ ] **Step 1: Confirm no remaining references to the deleted files**

Run from the repo root:

```bash
grep -rn --include='*.ts' --include='*.tsx' -E "from ['\"]\./App['\"]|from ['\"]\./HomePage['\"]|from ['\"]\./SettingsPage['\"]|App\.css" apps/desktop/aegis-desktop/src
```

Expected: no matches. `App.tsx`, `HomePage.tsx`, `SettingsPage.tsx` are not imported anywhere now that `main.tsx` mounts `<RouterProvider/>` and the routes live in `src/routes/`.

- [ ] **Step 2: Delete the files**

```bash
rm apps/desktop/aegis-desktop/src/App.tsx
rm apps/desktop/aegis-desktop/src/HomePage.tsx
rm apps/desktop/aegis-desktop/src/SettingsPage.tsx
rm apps/desktop/aegis-desktop/src/App.css
```

- [ ] **Step 3: Verify typecheck, build, and tests still pass**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three PASS. The deletion is invisible to the test/build pipeline because nothing references the deleted files.

- [ ] **Step 4: Commit**

```bash
git rm apps/desktop/aegis-desktop/src/App.tsx apps/desktop/aegis-desktop/src/HomePage.tsx apps/desktop/aegis-desktop/src/SettingsPage.tsx apps/desktop/aegis-desktop/src/App.css
git commit -m "refactor(desktop): remove legacy App.tsx/HomePage.tsx/SettingsPage.tsx/App.css"
```

---

### Task 6: Add new route tests — `index.test.tsx` and `__root.test.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx`

**Interfaces:**
- `index.test.tsx` mocks `@tauri-apps/api/core`'s `invoke` so `testGreet()` can be exercised without a real Rust runtime; asserts the button triggers `invoke('greet', { name: 'Aegis' })` and the response renders.
- `__root.test.tsx` uses `renderWithFullRouter({ initialEntries: ['/'] })` to mount the full layout and asserts: the Sidebar renders, Home content is visible at `/`, clicking the Settings menu item navigates to `/settings`, and Settings content renders.

- [ ] **Step 1: Create `src/test/routes/index.test.tsx`**

Create `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`. Vitest hoists `vi.mock(...)` to the top of the file, so the `import { HomePage } from "../../routes/index"` is mocked correctly:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../file-route-utils";
import { HomePage } from "../../routes/index";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

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
  vi.stubGlobal("localStorage", createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderHome(defaultLocale: "en" | "zh-CN" = "en") {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <HomePage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("HomePage", () => {
  it("renders the welcome heading", () => {
    renderHome();

    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });

  it("invokes the Tauri greet command and shows the response", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      "Greetings, Aegis!",
    );

    renderHome();

    await userEvent.click(
      screen.getByRole("button", { name: /test greet/i }),
    );

    expect(invoke).toHaveBeenCalledWith("greet", { name: "Aegis" });
    expect(await screen.findByText("Greetings, Aegis!")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Create `src/test/routes/__root.test.tsx`**

Create `apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderWithFullRouter } from "../file-route-utils";

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
  vi.stubGlobal("localStorage", createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderRoot(initialEntries: string[] = ["/"]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("RootLayout", () => {
  it("renders the Sidebar and the Home page content at /", () => {
    const { router } = renderRoot(["/"]);

    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/");
  });

  it("navigates to /settings when the Settings menu item is clicked", async () => {
    const { router } = renderRoot(["/"]);

    await userEvent.click(screen.getByText("Settings"));

    expect(router.state.location.pathname).toBe("/settings");
    expect(
      screen.getByRole("heading", { level: 4, name: /settings/i }),
    ).toBeInTheDocument();
  });

  it("navigates back to / when the Home menu item is clicked", async () => {
    const { router } = renderRoot(["/settings"]);

    await userEvent.click(screen.getByText("Home"));

    expect(router.state.location.pathname).toBe("/");
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});
```

Note: `renderWithFullRouter` from `file-route-utils.tsx` accepts a `wrapper` option via `RenderOptions` (Testing Library's wrapper). The `wrapper` wraps the rendered tree above the `RouterProvider`, so the providers (`AegisThemeProvider`, `AegisI18nProvider`) apply globally to every test.

- [ ] **Step 3: Run the new tests and verify they pass**

Run:

```bash
pnpm --filter aegis-desktop test
```

Expected: PASS. Vitest reports four test files, eight tests in total:

- `src/test/document-lang-sync.test.tsx` (2)
- `src/test/routes/index.test.tsx` (2)
- `src/test/routes/settings.test.tsx` (3)
- `src/test/routes/__root.test.tsx` (3)

If `__root.test.tsx` fails because clicking the menu text matches multiple elements (e.g. "Home" appears in both a menu item and somewhere in the page), tighten the selector to `screen.getByRole("button", { name: "Home" })` or use `screen.getAllByText("Home")[0]!` and assert on the rendered menu.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/routes/index.test.tsx apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx
git commit -m "test(desktop): add Home and RootLayout route tests"
```

---

### Task 7: Add `TanStackRouterDevtools` (dev-only)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`

- [ ] **Step 1: Add the devtools import + render guard**

In `apps/desktop/aegis-desktop/src/main.tsx`, add the import near the top of the file:

```tsx
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
```

Inside the `render(...)` call, immediately after `<RouterProvider router={router} />`, add:

```tsx
{import.meta.env.DEV && <TanStackRouterDevtools position="bottom-right" />}
```

The full `main.tsx` becomes:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { routeTree } from "./routes/routeTree.gen";
import { DocumentLangSync } from "./DocumentLangSync";

const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DocumentLangSync />
        <RouterProvider router={router} />
        {import.meta.env.DEV && (
          <TanStackRouterDevtools position="bottom-right" />
        )}
      </AegisI18nProvider>
    </AegisThemeProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 2: Verify typecheck, build, and tests pass**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three PASS. Vite replaces `import.meta.env.DEV` at build time, so the devtools are stripped from the production bundle.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "feat(desktop): add TanStackRouterDevtools (dev-only)"
```

---

### Task 8: Final smoke verification

**Files:** none changed.

- [ ] **Step 1: Run the full verification gauntlet**

From the repo root, run all three commands:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three PASS.

- [ ] **Step 2: Confirm the directory layout matches the spec**

From the repo root, list the desktop app's `src/` tree:

```bash
find apps/desktop/aegis-desktop/src -type f -not -path '*/node_modules/*' | sort
```

Expected output (file names only — generated content aside):

```
apps/desktop/aegis-desktop/src/DocumentLangSync.tsx
apps/desktop/aegis-desktop/src/main.tsx
apps/desktop/aegis-desktop/src/routes/__root.tsx
apps/desktop/aegis-desktop/src/routes/index.tsx
apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
apps/desktop/aegis-desktop/src/routes/settings.tsx
apps/desktop/aegis-desktop/src/test/document-lang-sync.test.tsx
apps/desktop/aegis-desktop/src/test/file-route-utils.tsx
apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx
apps/desktop/aegis-desktop/src/test/routes/index.test.tsx
apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx
apps/desktop/aegis-desktop/src/test/setup.ts
```

No `App.tsx`, `HomePage.tsx`, `SettingsPage.tsx`, `App.css`, root `vitest.setup.ts`, or old test files should remain.

- [ ] **Step 3: (Optional, manual) Boot the Tauri app**

```bash
pnpm tauri dev --filter aegis-desktop
```

Smoke checks:
1. App boots at `/`; Sidebar open; Home heading + greet button render.
2. Click `Settings` in the Sidebar → URL/state at `/settings`, Settings heading + theme switch + language select render.
3. Click `Home` → returns to `/`.
4. Toggle theme on Settings → MUI theme flips; persists across reload.
5. Change language to `zh-CN` → all copy translates; `<html lang>` updates.
6. Click `Test greet` on Home → response string from Rust `greet` command appears.
7. TanStack Router Devtools panel appears bottom-right (dev build only).
8. Production build (`pnpm tauri build --filter aegis-desktop`) produces a bundle without the devtools.

If any smoke check fails, file an issue and fix before declaring the refactor complete.
