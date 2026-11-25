# Project Workspace Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Tauri webview window at `/project/{project-code}` that reuses the main app, with a project-code sidebar header, Dashboard + Configuration menu items, a "Back to main" footer button, close-on-logout from the main window, and cross-window theme + language sync.

**Architecture:** A new pathless `_project` TanStack Router segment lives alongside the existing `_layout`. `WebviewWindow.new('project:<code>', { url, maximized: true })` opens the workspace; existing labels are focused instead of duplicated. Settings sync uses `tauri-plugin-store` for persistence + an `aegis:settings-changed` Tauri event for live updates, wired through `PersistentThemeProvider` / `PersistentI18nProvider` wrappers that hook into the providers' existing `onModeChange` / `onLocaleChange` callbacks. `useLogout` extends its success handler to close every `project:*` window before clearing the query cache.

**Tech Stack:** Tauri 2 (`@tauri-apps/api`, `WebviewWindow`, `getAllWebviewWindows`), `@tauri-apps/plugin-store`, `@tauri-apps/api/event`, TanStack Router, TanStack Query, `@aegis/ui` (Sidebar, theme, i18n), Vitest + Testing Library.

## Global Constraints

- Workspace lives entirely in `apps/desktop/aegis-desktop`. No `src-tauri/src/**` changes.
- One capability added: `core:window:allow-set-focus` (covers both focus-main button and close-on-logout sweep).
- Window labels: `project:<code>` for workspaces, `main` for the main window (made explicit in `tauri.conf.json`).
- Cross-window persistence file: `settings.bin` (separate from existing `auth.bin`).
- Settings sync event name: `aegis:settings-changed`. Payload shape: `{ theme?: ThemeMode; locale?: Locale }`.
- All commits follow the existing convention: `feat(desktop):` / `feat(ui):` / `docs(spec):` / etc., with `Co-Authored-By: Claude <noreply@anthropic.com>` trailers.
- Tests use Vitest + Testing Library. Existing helpers: `mockCommands` (`src/test/tauri-mock.ts`), `TestQueryProvider` (`src/test/test-query-provider.tsx`), `renderWithFullRouter` (`src/test/file-route-utils.tsx`).
- Existing i18n key contract: every key must exist in BOTH `lib/packages/ui/src/i18n/locales/en.ts` (with `as const`) and `zhCN.ts` (with `satisfies Record<keyof typeof en, string>`). Both files must stay in lock-step.
- Existing query-keys contract: pages consume `data/`, `data/` consumes `api/`, pages never reach `api/` directly.
- Existing Tauri-mock contract: `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` is required per test file using `mockInvoke`; for `@tauri-apps/api/webviewWindow` and `@tauri-apps/api/event`, mock those modules per-test similarly.

---

### Task 1: Tauri config — capability + main window label

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/capabilities/default.json`
- Modify: `apps/desktop/aegis-desktop/src-tauri/tauri.conf.json`

This task adds the new capability and makes the main window's label explicit. No tests — it's pure config.

- [ ] **Step 1: Add `core:window:allow-set-focus` to `default.json`**

Edit `apps/desktop/aegis-desktop/src-tauri/capabilities/default.json`. Find the `permissions` array and append the new permission. The file's current contents:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": [
    "main"
  ],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "opener:default",
    "store:default"
  ]
}
```

Change it to:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": [
    "main"
  ],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "opener:default",
    "store:default"
  ]
}
```

Why `core:window:allow-set-focus`: `setFocus()` is called both from the workspace window's "Back to main" footer button (in `ProjectWorkspaceLayout`) and from `useLogout`'s close-on-logout sweep. Tauri 2 requires explicit capabilities for window operations.

- [ ] **Step 2: Add explicit `"label": "main"` to `tauri.conf.json`**

Edit `apps/desktop/aegis-desktop/src-tauri/tauri.conf.json`. Find the `windows` array and update `windows[0]`:

```json
"app": {
  "windows": [
    {
      "label": "main",
      "title": "aegis-desktop",
      "width": 800,
      "height": 600,
      "maximized": true,
      "visible": false
    }
  ],
  ...
}
```

The default label for the first window is `"main"`, but we make it explicit so the `focusMainWindow` helper can hard-code it without ambiguity (and so any future Rust code can too).

- [ ] **Step 3: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src-tauri/capabilities/default.json src-tauri/tauri.conf.json
git commit -m "feat(desktop): label main window + allow setFocus" \
  -m "Adds core:window:allow-set-focus (workspace focus-main button" \
  -m "and close-on-logout sweep) and makes the main window's label" \
  -m "explicit so JS can hard-code it." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add `workspace.*` i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

Both locale files must be updated in lock-step. The English file uses `as const`; the Chinese file uses `satisfies Record<keyof typeof en, string>`. No tests — keys are exercised by every page test that renders workspace text.

- [ ] **Step 1: Read the existing locale files to find the right insertion point**

Read `lib/packages/ui/src/i18n/locales/en.ts` and find the `nav: { ... }` block (or wherever menu/navigation keys live). The new `workspace.*` keys should sit alongside other top-level namespaces.

The six new keys:

| Key | en | zh-CN |
|---|---|---|
| `workspace.menu.dashboard` | Dashboard | 仪表板 |
| `workspace.menu.configuration` | Configuration | 配置 |
| `workspace.dashboard.heading` | Dashboard — {projectCode} | 仪表板 — {projectCode} |
| `workspace.configuration.heading` | Configuration — {projectCode} | 配置 — {projectCode} |
| `workspace.placeholder` | Coming soon | 敬请期待 |
| `workspace.focusMain` | Back to main | 返回主窗口 |

- [ ] **Step 2: Add keys to `en.ts`**

In `lib/packages/ui/src/i18n/locales/en.ts`, add a new top-level key `workspace` next to the existing top-level keys:

```ts
workspace: {
  menu: {
    dashboard: "Dashboard",
    configuration: "Configuration",
  },
  dashboard: {
    heading: "Dashboard — {projectCode}",
  },
  configuration: {
    heading: "Configuration — {projectCode}",
  },
  placeholder: "Coming soon",
  focusMain: "Back to main",
},
```

The exact placement is up to the file's existing organization (alphabetical or grouped). Match the surrounding style.

- [ ] **Step 3: Add the same keys to `zhCN.ts`**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, mirror with the Chinese values:

```ts
workspace: {
  menu: {
    dashboard: "仪表板",
    configuration: "配置",
  },
  dashboard: {
    heading: "仪表板 — {projectCode}",
  },
  configuration: {
    heading: "配置 — {projectCode}",
  },
  placeholder: "敬请期待",
  focusMain: "返回主窗口",
},
```

- [ ] **Step 4: Type-check to confirm both files remain compatible**

```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS. `zhCN.ts`'s `satisfies Record<keyof typeof en, string>` constraint catches missing keys at compile time.

- [ ] **Step 5: Commit**

```bash
cd ../..
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add workspace.* i18n keys (en + zh-CN)" \
  -m "Six keys for the project workspace window: menu entries," \
  -m "Dashboard/Configuration headings, placeholder text, and" \
  -m "Back-to-main button label." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: `data/settings.ts` — store hydration + listener + persist

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/settings.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/settings.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/data/index.ts` (re-export)

This module holds three pieces: a singleton store handle, a hook that hydrates theme + locale from the store on mount, a hook that subscribes to `aegis:settings-changed` events, and an imperative `persistSettings` used by the provider wrappers in Task 4.

TDD order: failing tests → implementation → green tests → commit.

- [ ] **Step 1: Write the failing test file**

Create `apps/desktop/aegis-desktop/src/test/data/settings.test.tsx`:

```tsx
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, act } from "@testing-library/react";
import { AegisThemeProvider, useThemeMode } from "@aegis/ui/theme";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";

// Mock the store BEFORE importing the module under test so the
// singleton store handle is constructed against the mock loader.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () => Promise.resolve({
    get: <T>(k: string) => Promise.resolve(store.get(k) as T | undefined),
    set: (k: string, v: unknown) => { store.set(k, v); return Promise.resolve(); },
    save: () => Promise.resolve(),
  }),
}));

// In-memory pub-sub for events. Captures listeners so tests can fire
// payloads directly.
type Handler = (e: { payload: unknown }) => void;
const handlers: Handler[] = [];
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_name: string, h: Handler) => {
    handlers.push(h);
    return Promise.resolve(() => {
      const idx = handlers.indexOf(h);
      if (idx >= 0) handlers.splice(idx, 1);
    });
  },
  emit: vi.fn(),
}));

// Now import the module under test — its top-level singleton is built
// against the mocked loader.
import {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "../../data/settings";

function HydrateProbe() {
  useHydrateSettingsFromStore();
  return null;
}

function ListenProbe() {
  useListenForSettingsChanges();
  return null;
}

function ThemeProbe({ label }: { label: string }) {
  const { mode } = useThemeMode();
  return <span data-testid={label}>{mode}</span>;
}

function LocaleProbe({ label }: { label: string }) {
  const { locale } = useI18n();
  return <span data-testid={label}>{locale}</span>;
}

beforeEach(() => {
  store.clear();
  handlers.length = 0;
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() { return 0; },
  });
});
afterEach(() => cleanup());

describe("useHydrateSettingsFromStore", () => {
  it("calls setMode and setLocale when the store has values that differ", async () => {
    store.set("theme", "dark");
    store.set("locale", "zh-CN");

    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <HydrateProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("zh-CN");
    });
  });

  it("leaves providers alone when the store is empty", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <HydrateProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    // Wait one tick so the hook's effect runs.
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.getByTestId("mode").textContent).toBe("light");
    expect(screen.getByTestId("locale").textContent).toBe("en");
  });
});

describe("useListenForSettingsChanges", () => {
  it("calls setMode and setLocale when an event fires", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ListenProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    // Wait for the listen() effect to register its handler.
    await waitFor(() => expect(handlers.length).toBe(1));

    await act(async () => {
      handlers[0]({ payload: { theme: "dark", locale: "zh-CN" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("zh-CN");
    });
  });

  it("only applies the keys present in the payload", async () => {
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
      handlers[0]({ payload: { theme: "dark" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("en");
    });
  });
});

describe("persistSettings", () => {
  it("writes both keys when both are provided", async () => {
    await persistSettings({ theme: "dark", locale: "zh-CN" });
    expect(store.get("theme")).toBe("dark");
    expect(store.get("locale")).toBe("zh-CN");
  });

  it("writes only the patch key when only one is provided", async () => {
    store.set("locale", "en");
    await persistSettings({ theme: "dark" });
    expect(store.get("theme")).toBe("dark");
    expect(store.get("locale")).toBe("en"); // unchanged
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/data/settings.test.tsx
```

Expected: FAIL — `data/settings.ts` doesn't exist, so the import errors.

- [ ] **Step 3: Write the minimal implementation**

Create `apps/desktop/aegis-desktop/src/data/settings.ts`:

```ts
import { useEffect } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import { useThemeMode, type ThemeMode } from "@aegis/ui/theme";
import { useI18n, type Locale } from "@aegis/ui/i18n";

/**
 * Lazy singleton over the `settings.bin` store. The store lives on
 * disk at the app-config level, so every Tauri window — main window
 * and every `project:*` workspace window — sees the same file.
 */
let storePromise: Promise<Store> | null = null;
async function getStore(): Promise<Store> {
  if (!storePromise) storePromise = load("settings.bin");
  return storePromise;
}

/**
 * Read theme + locale from the on-disk settings store and apply them
 * to the React providers. Mounted once per window inside the bridge
 * component (see SettingsSyncBridge.tsx) so every window picks up
 * the user's last choice before its first paint. Intentionally
 * single-fire per mount — subsequent changes flow through the
 * `aegis:settings-changed` event listener.
 */
export function useHydrateSettingsFromStore() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale } = useI18n();

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const store = await getStore();
      const theme = await store.get<ThemeMode>("theme");
      const loc = await store.get<Locale>("locale");
      if (cancelled) return;
      if (theme && theme !== mode) setMode(theme);
      if (loc && loc !== locale) setLocale(loc);
    })();
    return () => {
      cancelled = true;
    };
    // Run once per window mount; deps intentionally omitted.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}

/**
 * Subscribe to `aegis:settings-changed` events from other windows.
 * The event fires from PersistentThemeProvider / PersistentI18nProvider
 * (defined in SettingsSyncBridge.tsx) when the user toggles a setting
 * in the main window. The local main window also receives its own
 * emit — that's a no-op because setMode/setLocale already ran.
 */
export function useListenForSettingsChanges() {
  const { setMode } = useThemeMode();
  const { setLocale } = useI18n();
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const u = await listen<{ theme?: ThemeMode; locale?: Locale }>(
        "aegis:settings-changed",
        ({ payload }) => {
          if (payload.theme) void setMode(payload.theme);
          if (payload.locale) void setLocale(payload.locale);
        },
      );
      unlisten = u;
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [setMode, setLocale]);
}

/**
 * Imperative write used by PersistentThemeProvider /
 * PersistentI18nProvider to persist a setting change and trigger
 * the cross-window broadcast. Only the keys present in `patch` are
 * written — passing only `theme` does not clobber `locale` and vice
 * versa.
 */
export async function persistSettings(patch: {
  theme?: ThemeMode;
  locale?: Locale;
}) {
  const store = await getStore();
  if (patch.theme !== undefined) await store.set("theme", patch.theme);
  if (patch.locale !== undefined) await store.set("locale", patch.locale);
  await store.save();
}
```

Note the dynamic `import("@tauri-apps/api/event")` inside the listener
effect. That keeps the synchronous module-load path free of the Tauri
runtime requirement — the listener only mounts in a real window.

- [ ] **Step 4: Re-export the new symbols from `data/index.ts`**

Edit `apps/desktop/aegis-desktop/src/data/index.ts`. Add a new
re-export line after the existing `user` block:

```ts
export {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "./settings";
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/data/settings.test.tsx
```

Expected: PASS — all four describe blocks green.

- [ ] **Step 6: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/data/settings.ts \
        apps/desktop/aegis-desktop/src/data/index.ts \
        apps/desktop/aegis-desktop/src/test/data/settings.test.tsx
git commit -m "feat(desktop): data/settings module — store hydration + listener" \
  -m "Adds useHydrateSettingsFromStore (reads theme + locale from" \
  -m "settings.bin on mount), useListenForSettingsChanges (subscribes" \
  -m "to aegis:settings-changed), and persistSettings (writes the" \
  -m "settings.bin store). Covered by 4 test cases." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: `SettingsSyncBridge.tsx` — provider wrappers + glue component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/SettingsSyncBridge.tsx`

Three named exports: `PersistentThemeProvider` (wraps `AegisThemeProvider` with the persist+broadcast `onModeChange`), `PersistentI18nProvider` (same for i18n), and `SettingsSyncBridge` (mounts the two hooks from Task 3). No new tests — the bridge's behavior is exercised through integration; the persist+emit path is verified by the listener hook in Task 3.

- [ ] **Step 1: Create the file**

Create `apps/desktop/aegis-desktop/src/SettingsSyncBridge.tsx`:

```tsx
import type { ReactNode } from "react";
import { emit } from "@tauri-apps/api/event";
import { AegisThemeProvider, type ThemeMode } from "@aegis/ui/theme";
import { AegisI18nProvider, type Locale } from "@aegis/ui/i18n";

import {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "./data/settings";

/**
 * Glue component mounted once per window inside both provider
 * wrappers. Hydrates theme + locale from the on-disk settings store
 * and subscribes to live changes broadcast by other windows.
 */
export function SettingsSyncBridge({ children }: { children: ReactNode }) {
  useHydrateSettingsFromStore();
  useListenForSettingsChanges();
  return <>{children}</>;
}

/**
 * Wraps AegisThemeProvider so every setMode call in any window is
 * persisted to `settings.bin` AND broadcast to other windows as an
 * `aegis:settings-changed` event. The provider's existing
 * onModeChange callback fires after setMode, so local state has
 * already updated by the time we persist + emit.
 */
export function PersistentThemeProvider({ children }: { children: ReactNode }) {
  const handleChange = async (mode: ThemeMode) => {
    await persistSettings({ theme: mode });
    await emit("aegis:settings-changed", { theme: mode });
  };
  return (
    <AegisThemeProvider onModeChange={handleChange}>
      {children}
    </AegisThemeProvider>
  );
}

/**
 * Wraps AegisI18nProvider with the same persist+broadcast pattern.
 * The default locale falls through to AegisI18nProvider's default
 * ("en") when not supplied.
 */
export function PersistentI18nProvider({
  children,
  defaultLocale,
}: {
  children: ReactNode;
  defaultLocale?: Locale;
}) {
  const handleChange = async (locale: Locale) => {
    await persistSettings({ locale });
    await emit("aegis:settings-changed", { locale });
  };
  return (
    <AegisI18nProvider
      onLocaleChange={handleChange}
      defaultLocale={defaultLocale}
    >
      {children}
    </AegisI18nProvider>
  );
}
```

Notes:
- The bridge must live INSIDE both provider wrappers at runtime because
  the two hooks consume `useThemeMode` and `useI18n` from those
  contexts. The wiring in Task 5 places the bridge correctly.
- `emit` from `@tauri-apps/api/event` resolves successfully even with
  zero listeners; no throw path.
- `persistSettings` and `emit` are awaited sequentially — first write
  to disk, then broadcast. If `persistSettings` rejects, the broadcast
  is skipped (correct ordering: don't tell other windows about a
  value we couldn't persist).

- [ ] **Step 2: Type-check**

```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/SettingsSyncBridge.tsx
git commit -m "feat(desktop): SettingsSyncBridge + persistent provider wrappers" \
  -m "PersistentThemeProvider/PersistentI18nProvider hook into the" \
  -m "ui providers' existing onModeChange/onLocaleChange callbacks" \
  -m "to persist every change to settings.bin and broadcast" \
  -m "aegis:settings-changed to other windows." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Wire `SettingsSyncBridge` into `main.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`

Mounts the bridge inside both persistent providers. No new tests — the existing `routes/*.test.tsx` suite exercises the full app tree with `renderWithFullRouter`, which already wraps with `AegisThemeProvider` + `AegisI18nProvider`. Those tests must keep passing because they DO NOT use the persistent wrappers — verify in Step 3 that the existing suite is still green.

- [ ] **Step 1: Read the current `main.tsx`**

The current file (already read in exploration) has:
- A window-show effect using `getCurrentWindow`
- The `App` function rendering `<AegisThemeProvider><QueryProvider><AegisI18nProvider>...</AegisI18nProvider></QueryProvider></AegisThemeProvider>`
- Imports the `AegisThemeProvider` from `@aegis/ui/theme` and `AegisI18nProvider` from `@aegis/ui/i18n`

- [ ] **Step 2: Update imports and the tree**

Edit `apps/desktop/aegis-desktop/src/main.tsx`. Add three imports:

```ts
import {
  PersistentThemeProvider,
  PersistentI18nProvider,
  SettingsSyncBridge,
} from "./SettingsSyncBridge";
```

Replace the two provider import lines:

```ts
// REMOVE these two:
// import { AegisThemeProvider } from "@aegis/ui/theme";
// import { AegisI18nProvider } from "@aegis/ui/i18n";
```

Update the `App` function's return tree. The new ordering places the bridge inside both providers so its hooks can resolve both contexts:

```tsx
return (
  <React.StrictMode>
    <PersistentThemeProvider>
      <QueryProvider>
        <PersistentI18nProvider>
          <SettingsSyncBridge>
            <DocumentLangSync />
            <RouterProvider router={router} />
            {import.meta.env.DEV && (
              <TanStackRouterDevtools
                router={router}
                position="bottom-right"
              />
            )}
          </SettingsSyncBridge>
        </PersistentI18nProvider>
      </QueryProvider>
    </PersistentThemeProvider>
  </React.StrictMode>
);
```

The `bootstrap` URL rewrite at the top of the file (the `replaceState` / `router.navigate` block for `/bootstrap`) is unchanged — workspace windows land on `/project/<code>` and skip the rewrite because the pathname check is `=== "/" || === "/index.html"`.

- [ ] **Step 3: Run the full test suite to confirm no regression**

```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS — every existing test green. The test suites wrap their components with `AegisThemeProvider` + `AegisI18nProvider` directly (not the persistent wrappers), so the bridge hooks never run in jsdom. The bridge's runtime behavior is exercised manually via `pnpm tauri dev` once the implementation lands.

- [ ] **Step 4: Type-check**

```bash
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "feat(desktop): mount SettingsSyncBridge inside persistent providers" \
  -m "Replaces the bare AegisThemeProvider + AegisI18nProvider with" \
  -m "PersistentThemeProvider + PersistentI18nProvider, and mounts" \
  -m "SettingsSyncBridge inside both so its hooks resolve both" \
  -m "provider contexts. No behavior change for existing tests;" \
  -m "production windows now persist + sync theme and locale." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: `api.openProjectWorkspace` — open + dedupe + maximize

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/api/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/api/open-project-workspace.test.ts`

Adds `openProjectWorkspace(code)` that opens a new `WebviewWindow` with label `project:<code>`, or focuses the existing one if present. TDD: failing test → implementation → green.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/api/open-project-workspace.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockNew = vi.fn();
const mockGetByLabel = vi.fn();
const mockShow = vi.fn();
const mockSetFocus = vi.fn();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: {
    new: (...args: unknown[]) => mockNew(...args),
    getByLabel: (...args: unknown[]) => mockGetByLabel(...args),
  },
}));

import { api } from "../../api";

beforeEach(() => {
  mockNew.mockReset();
  mockGetByLabel.mockReset();
  mockShow.mockReset();
  mockSetFocus.mockReset();

  mockGetByLabel.mockResolvedValue(null);
  mockNew.mockResolvedValue({ show: mockShow, setFocus: mockSetFocus });
  mockShow.mockResolvedValue(undefined);
  mockSetFocus.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.clearAllMocks();
});

describe("api.openProjectWorkspace", () => {
  it("creates a new maximized window when no window with that label exists", async () => {
    await api.openProjectWorkspace("DEMO-001");
    expect(mockGetByLabel).toHaveBeenCalledWith("project:DEMO-001");
    expect(mockNew).toHaveBeenCalledWith("project:DEMO-001", {
      url: "/project/DEMO-001",
      title: "DEMO-001",
      width: 1100,
      height: 720,
      minWidth: 720,
      minHeight: 480,
      maximized: true,
    });
  });

  it("focuses the existing window instead of creating a duplicate", async () => {
    const existing = { show: mockShow, setFocus: mockSetFocus };
    mockGetByLabel.mockResolvedValue(existing);

    await api.openProjectWorkspace("DEMO-001");

    expect(mockNew).not.toHaveBeenCalled();
    expect(mockShow).toHaveBeenCalledTimes(1);
    expect(mockSetFocus).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/api/open-project-workspace.test.ts
```

Expected: FAIL — `api.openProjectWorkspace` is undefined.

- [ ] **Step 3: Implement the function**

Edit `apps/desktop/aegis-desktop/src/api/index.ts`. Add a new import at the top:

```ts
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
```

Add a new entry to the `api` object, alongside the existing entries:

```ts
export const api = {
  // ... existing entries ...
  openProjectWorkspace: async (code: string): Promise<void> => {
    const label = `project:${code}`;
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }
    await WebviewWindow.new(label, {
      url: `/project/${code}`,
      title: code,
      width: 1100,
      height: 720,
      minWidth: 720,
      minHeight: 480,
      maximized: true,
    });
  },
} as const;
```

The `url` uses a leading slash (`/project/<code>`) so TanStack Router
treats it as a route within the same Vite app, not a Tauri asset path.

- [ ] **Step 4: Re-run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/api/open-project-workspace.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/api/index.ts \
        apps/desktop/aegis-desktop/src/test/api/open-project-workspace.test.ts
git commit -m "feat(desktop): api.openProjectWorkspace — new or focus existing" \
  -m "Tauri webview window labelled project:<code>, opened" \
  -m "maximized with url /project/<code>. If a window with that" \
  -m "label already exists, focuses it instead of duplicating." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Wire `OpenInNew` → `onOpenWorkspace` in `ProjectTable` + `ProjectList`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/pages/ProjectList.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`

Three coupled changes:
1. `ProjectTable` gains an `onOpenWorkspace(code)` prop; the row's `OpenInNew` button calls it instead of being disabled.
2. `ProjectList` creates the handler and passes it down.
3. The existing `project-table.test.tsx` has assertions that the OpenInNew button is `disabled` — relax those and add a click-calls-handler assertion.

TDD order: failing test update → table change → list change → green.

- [ ] **Step 1: Read the existing test file's relevant assertions**

The existing file at `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx` has two relevant assertions in the `ProjectTable — operation column role gating` describe block:
- Line 124-131: "hides Add and Edit when canEdit=false but still renders OpenInNew as disabled" — this assertion must be REMOVED.
- The "renders Add, Edit, and OpenInNew when canEdit=true" assertion (line 117-122) stays.

The `renderTable` helper at line 39-67 also needs a new `onOpenWorkspace` optional prop.

- [ ] **Step 2: Update the test file with the new behavior**

Edit `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`.

Replace the `renderTable` helper's signature and add the new prop. The new helper:

```tsx
function renderTable(props: {
  rows?: ProjectView[];
  loading?: boolean;
  error?: ApiError | null;
  canEdit?: boolean;
  onOpenCreate?: () => void;
  onOpenEdit?: (code: string) => void;
  onOpenWorkspace?: (code: string) => void;   // NEW
} = {}) {
  const onOpenCreate = props.onOpenCreate ?? vi.fn();
  const onOpenEdit = props.onOpenEdit ?? vi.fn();
  const onOpenWorkspace = props.onOpenWorkspace ?? vi.fn();   // NEW
  return {
    onOpenCreate,
    onOpenEdit,
    onOpenWorkspace,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectTable
            rows={props.rows ?? [baseRow]}
            loading={props.loading ?? false}
            error={props.error ?? null}
            canEdit={props.canEdit ?? true}
            onOpenCreate={onOpenCreate}
            onOpenEdit={onOpenEdit}
            onOpenWorkspace={onOpenWorkspace}   // NEW
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}
```

Replace the `canEdit=false` block. The new assertion: OpenInNew is rendered (NOT disabled) regardless of `canEdit`, since opening a project workspace is a read action available to every authenticated user:

```tsx
it("hides Add and Edit when canEdit=false but still renders OpenInNew enabled", () => {
  renderTable({ canEdit: false });
  expect(screen.queryByRole("button", { name: /add project/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /edit project/i })).not.toBeInTheDocument();
  const openBtn = screen.getByRole("button", { name: /open project/i });
  expect(openBtn).toBeInTheDocument();
  expect(openBtn).not.toBeDisabled();
});
```

Append a new describe block at the end of the file:

```tsx
describe("ProjectTable — OpenInNew workspace action", () => {
  it("calls onOpenWorkspace(row.code) when OpenInNew is clicked", async () => {
    const { onOpenWorkspace } = renderTable();
    await userEvent.click(screen.getByRole("button", { name: /open project/i }));
    expect(onOpenWorkspace).toHaveBeenCalledTimes(1);
    expect(onOpenWorkspace).toHaveBeenCalledWith("alpha");
  });

  it("OpenInNew is enabled regardless of canEdit", () => {
    renderTable({ canEdit: false });
    const openBtn = screen.getByRole("button", { name: /open project/i });
    expect(openBtn).not.toBeDisabled();
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/pages/project-table.test.tsx
```

Expected: FAIL — `ProjectTable` doesn't accept `onOpenWorkspace`.

- [ ] **Step 4: Update `ProjectTable`**

Edit `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`.

Add `onOpenWorkspace` to `ProjectTableProps`:

```tsx
export interface ProjectTableProps {
  rows: ProjectView[];
  loading: boolean;
  error: ApiError | null;
  canEdit: boolean;
  onOpenCreate: () => void;
  onOpenEdit: (code: string) => void;
  onOpenWorkspace: (code: string) => void;
}
```

Add it to the destructured args at the top of the function:

```tsx
export function ProjectTable({
  rows,
  loading,
  error,
  canEdit,
  onOpenCreate,
  onOpenEdit,
  onOpenWorkspace,
}: ProjectTableProps) {
```

Replace the disabled OpenInNew button (in the row's actions cell) with a wired one:

```tsx
<IconButton
  aria-label={t("project.open")}
  onClick={() => onOpenWorkspace(row.code)}
>
  <OpenInNew />
</IconButton>
```

(The existing code already has this IconButton — just remove the `disabled` attribute and add the `onClick`.)

- [ ] **Step 5: Update `ProjectList` to pass the handler**

Edit `apps/desktop/aegis-desktop/src/pages/ProjectList.tsx`.

Add the import:

```tsx
import { useCallback } from "react";
import { api } from "../api";
```

(If `useMemo` is already imported from React, just add `useCallback` to the same import line.)

Inside `ProjectListPage`, before `return`, add:

```tsx
const handleOpenWorkspace = useCallback((code: string) => {
  void api.openProjectWorkspace(code);
}, []);
```

In the `<ProjectTable>` JSX, add the new prop:

```tsx
<ProjectTable
  rows={filteredRows}
  loading={projects.isLoading}
  error={projects.error}
  canEdit={canEdit}
  onOpenCreate={() => setDrawer({ mode: "create", code: null })}
  onOpenEdit={(code) => setDrawer({ mode: "edit", code })}
  onOpenWorkspace={handleOpenWorkspace}
/>
```

- [ ] **Step 6: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/pages/project-table.test.tsx src/test/pages/project-list.test.tsx
```

Expected: PASS — both updated test files green.

- [ ] **Step 7: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx \
        apps/desktop/aegis-desktop/src/pages/ProjectList.tsx \
        apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx
git commit -m "feat(desktop): wire OpenInNew to openProjectWorkspace" \
  -m "Adds onOpenWorkspace prop to ProjectTable; ProjectList wires" \
  -m "it to api.openProjectWorkspace(code). OpenInNew is now an" \
  -m "always-enabled read action available to every authenticated" \
  -m "user, regardless of canEdit." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Extend `useLogout` to close every `project:*` window

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/data/user.ts`
- Modify: `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`

The success handler now does: enumerate windows → close project-prefixed ones → clear cache. Tests must verify the close order, the prefix filter, and the main-window skip.

- [ ] **Step 1: Add the failing test cases to `user.test.tsx`**

Edit `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`. At the top, after the `vi.mock("@tauri-apps/api/core", ...)` line, add a second mock:

```ts
const mockGetAll = vi.fn();
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getAllWebviewWindows: (...args: unknown[]) => mockGetAll(...args),
}));
```

In the `beforeEach` block (around line 50), add a reset:

```ts
beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  mockGetAll.mockReset();
});
```

Inside the existing `describe("useLogout", ...)` block (around line 167), append three new test cases:

```tsx
it("closes every project:* window on success and skips the main window", async () => {
  const mainClose = vi.fn();
  const project1Close = vi.fn();
  const project2Close = vi.fn();
  mockGetAll.mockResolvedValue([
    { label: "main", close: mainClose },
    { label: "project:DEMO-001", close: project1Close },
    { label: "project:DEMO-002", close: project2Close },
  ]);
  mockCommands({ logout: () => undefined });
  const { client } = renderWithQueryClient(<LogoutHarness />);
  const clearSpy = vi.spyOn(client, "clear");

  await userEvent.click(screen.getByRole("button", { name: "logout" }));

  await waitFor(() => {
    expect(project1Close).toHaveBeenCalledTimes(1);
    expect(project2Close).toHaveBeenCalledTimes(1);
    expect(mainClose).not.toHaveBeenCalled();
    expect(clearSpy).toHaveBeenCalled();
  });
});

it("does not call any window.close when only the main window exists", async () => {
  const mainClose = vi.fn();
  mockGetAll.mockResolvedValue([{ label: "main", close: mainClose }]);
  mockCommands({ logout: () => undefined });
  const { client } = renderWithQueryClient(<LogoutHarness />);
  const clearSpy = vi.spyOn(client, "clear");

  await userEvent.click(screen.getByRole("button", { name: "logout" }));

  await waitFor(() => expect(clearSpy).toHaveBeenCalled());
  expect(mainClose).not.toHaveBeenCalled();
});

it("closes project windows BEFORE clearing the cache", async () => {
  // Deferred close promise — lets the test observe the ordering.
  let closeProject!: () => void;
  const project1Close = vi.fn(
    () => new Promise<void>((resolve) => { closeProject = resolve; }),
  );
  mockGetAll.mockResolvedValue([
    { label: "main", close: vi.fn() },
    { label: "project:DEMO-001", close: project1Close },
  ]);
  mockCommands({ logout: () => undefined });
  const { client } = renderWithQueryClient(<LogoutHarness />);
  const clearSpy = vi.spyOn(client, "clear");

  // Fire the logout click; the close promise is pending, so we can
  // check intermediate state.
  const clickPromise = userEvent.click(
    screen.getByRole("button", { name: "logout" }),
  );

  // Give the event loop a tick for the handler to start awaiting.
  await new Promise((r) => setTimeout(r, 0));
  expect(project1Close).toHaveBeenCalledTimes(1);
  expect(clearSpy).not.toHaveBeenCalled();

  closeProject();
  await clickPromise;

  await waitFor(() => expect(clearSpy).toHaveBeenCalled());
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/data/user.test.tsx
```

Expected: FAIL — the close calls aren't made yet.

- [ ] **Step 3: Update `useLogout`**

Edit `apps/desktop/aegis-desktop/src/data/user.ts`. Add an import:

```ts
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";
```

(Add it next to the existing `useMutation`, `useQueryClient`, etc., imports.)

Update the `useLogout` function body. The new `onSuccess`:

```ts
export function useLogout() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, void>({
    mutationFn: () => api.logout(),
    onSuccess: async () => {
      // Close every project workspace window BEFORE clearing the
      // cache. Workspace windows have their own React tree and their
      // own query client — closing them first means the cached data
      // is never read again, and there is no ordering window during
      // which a workspace page could issue a stale fetch.
      const all = await getAllWebviewWindows();
      await Promise.all(
        all
          .filter((w) => w.label.startsWith("project:"))
          .map((w) => w.close()),
      );
      qc.clear();
    },
  });
}
```

If `Promise.all` rejects because one close fails, `qc.clear()` is in
a separate `await` line so the rest of the function still proceeds.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/data/user.test.tsx
```

Expected: PASS — all 5 `useLogout` cases green (the 2 existing + 3 new).

- [ ] **Step 5: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/data/user.ts \
        apps/desktop/aegis-desktop/src/test/data/user.test.tsx
git commit -m "feat(desktop): close all project:* windows on logout" \
  -m "useLogout's success handler now enumerates webview windows," \
  -m "closes every label prefixed project:, and only THEN clears" \
  -m "the query cache. The main window is never closed by this" \
  -m "sweep — that's the UserFooter's responsibility on confirm." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: `_project` route segment — `route.tsx`, `index.tsx`, `dashboard.tsx`, `configuration.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_project/route.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_project/index.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_project/dashboard.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_project/configuration.tsx`
- Modify: `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (regenerated)

The pathless segment matches URLs like `/project/<code>`, `/project/<code>/dashboard`, `/project/<code>/configuration`. The auth guard is the same shape as `_layout/route.tsx`. The `index` redirects to the dashboard. The router plugin regenerates `routeTree.gen.ts` automatically; if it doesn't, run the dev server once to trigger the codegen.

- [ ] **Step 1: Create `src/routes/_project/route.tsx`**

```ts
import { createFileRoute, redirect } from "@tanstack/react-router";

import { api } from "../../api";
import { ProjectWorkspaceLayout } from "../../pages/ProjectWorkspaceLayout";

export const Route = createFileRoute("/_project")({
  beforeLoad: async ({ params }) => {
    let loggedIn = false;
    try {
      loggedIn = await api.isLoggedIn();
    } catch {
      loggedIn = false;
    }
    if (!loggedIn) {
      throw redirect({ to: "/login" });
    }
    return { projectCode: params.projectCode };
  },
  component: ProjectWorkspaceLayout,
});
```

- [ ] **Step 2: Create `src/routes/_project/index.tsx`**

```ts
import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_project/")({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: "/project/$projectCode/dashboard",
      params: { projectCode: params.projectCode },
    });
  },
});
```

- [ ] **Step 3: Create `src/routes/_project/dashboard.tsx`**

```ts
import { createFileRoute } from "@tanstack/react-router";
import { ProjectDashboardPage } from "../../pages/ProjectDashboard";

export const Route = createFileRoute("/_project/dashboard")({
  component: ProjectDashboardPage,
});
```

- [ ] **Step 4: Create `src/routes/_project/configuration.tsx`**

```ts
import { createFileRoute } from "@tanstack/react-router";
import { ProjectConfigurationPage } from "../../pages/ProjectConfiguration";

export const Route = createFileRoute("/_project/configuration")({
  component: ProjectConfigurationPage,
});
```

- [ ] **Step 5: Regenerate `routeTree.gen.ts`**

The plugin regenerates the file on Vite startup. Two ways to trigger it:
- Run `pnpm dev` once (the dev server boots and writes the file).
- Or, if there's a separate codegen command, run that.

If neither is feasible, manually run the typecheck — the generated file may show type errors until regenerated:

```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS after regeneration.

If the file is not regenerated automatically, inspect it and add the new entries following the existing pattern. Each entry follows the shape:

```ts
export const Route = <RouteImport>.update({
  id: '/_project',
  path: '/project/$projectCode',
  getParentRoute: () => rootRouteImport,
} as any)
// ... and similar for /_project/, /_project/dashboard, /_project/configuration
```

The plugin also adds entries to the `FileRoutesByFullPath`, `FileRoutesByTo`, `FileRoutesById`, `FileRouteTypes`, `RootRouteChildren`, and `declare module '@tanstack/react-router'` blocks. Easiest path: run `pnpm dev` briefly.

- [ ] **Step 6: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/routes/_project \
        apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git commit -m "feat(desktop): _project route segment (auth-gated)" \
  -m "Adds the pathless segment behind /project/<code>. route.tsx" \
  -m "re-runs the isLoggedIn guard and reads :projectCode into" \
  -m "context; index.tsx redirects /project/<code> to /dashboard;" \
  -m "dashboard.tsx and configuration.tsx mount placeholder pages" \
  -m "that arrive in Task 10. routeTree.gen.ts regenerated." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Workspace pages — `ProjectWorkspaceLayout`, `ProjectDashboard`, `ProjectConfiguration`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectWorkspaceLayout.tsx`
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectDashboard.tsx`
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectConfiguration.tsx`

Three new page files. The layout reuses `@aegis/ui`'s `Sidebar` with project-specific title, menu, and a focus-main footer button. The dashboard / configuration pages render placeholders.

- [ ] **Step 1: Create `ProjectDashboard.tsx`**

```tsx
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

/**
 * Placeholder Dashboard page for a project workspace window.
 * Real content (charts, KPIs, recent activity) is out of scope for
 * the workspace-window feature and arrives in a later spec.
 */
export function ProjectDashboardPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.dashboard.heading", { projectCode })}
      </Typography>
      <Typography color="textSecondary">
        {t("workspace.placeholder")}
      </Typography>
    </Box>
  );
}
```

- [ ] **Step 2: Create `ProjectConfiguration.tsx`**

Identical to `ProjectDashboard.tsx` with `configuration` substituted:

```tsx
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

/**
 * Placeholder Configuration page for a project workspace window.
 * Real content (project settings, member management, integrations)
 * is out of scope for the workspace-window feature.
 */
export function ProjectConfigurationPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.configuration.heading", { projectCode })}
      </Typography>
      <Typography color="textSecondary">
        {t("workspace.placeholder")}
      </Typography>
    </Box>
  );
}
```

- [ ] **Step 3: Create `ProjectWorkspaceLayout.tsx`**

```tsx
import React from "react";
import { Outlet, useNavigate, useParams } from "@tanstack/react-router";
import { Box, Button } from "@aegis/ui/mui";
import {
  Sidebar,
  type MenuItem,
  type SidebarProps,
} from "@aegis/ui";
import {
  Dashboard as DashboardIcon,
  Settings as SettingsIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";

const DashboardMenuIcon = () => <DashboardIcon />;
const ConfigMenuIcon = () => <SettingsIcon />;

/**
 * Workspace window shell. Sidebar header is the project code; menu
 * has Dashboard + Configuration entries only; footer is a "Back to
 * main" button that focuses the main window. Mounted by the
 * `_project/route.tsx` layout.
 */
export function ProjectWorkspaceLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    {
      link: `/project/${projectCode}/dashboard`,
      title: t("workspace.menu.dashboard"),
      icon: DashboardMenuIcon,
    },
    {
      link: `/project/${projectCode}/configuration`,
      title: t("workspace.menu.configuration"),
      icon: ConfigMenuIcon,
    },
  ];

  async function focusMainWindow() {
    const all = await getAllWebviewWindows();
    const mainWin = all.find((w) => w.label === "main");
    if (mainWin) {
      await mainWin.setFocus();
      await mainWin.show();
    }
  }

  const sidebarProps: SidebarProps = {
    title: projectCode,
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: (
      <Button
        size="small"
        variant="outlined"
        fullWidth
        onClick={() => void focusMainWindow()}
      >
        {t("workspace.focusMain")}
      </Button>
    ),
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{ flexGrow: 1, transition: "margin 0.3s" }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}
```

- [ ] **Step 4: Type-check**

```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS — the new pages compile.

- [ ] **Step 5: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/pages/ProjectWorkspaceLayout.tsx \
        apps/desktop/aegis-desktop/src/pages/ProjectDashboard.tsx \
        apps/desktop/aegis-desktop/src/pages/ProjectConfiguration.tsx
git commit -m "feat(desktop): project workspace layout + placeholder pages" \
  -m "ProjectWorkspaceLayout renders the Sidebar with the project" \
  -m "code as title and Dashboard + Configuration menu items, plus" \
  -m "a focus-main footer Button that calls setFocus on the window" \
  -m "whose label is main. Dashboard and Configuration pages render" \
  -m "a heading + Coming-soon placeholder." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 11: Route tests for `/project/*`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/routes/project-workspace.test.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/project-workspace-layout.test.tsx`

Two new test files covering the full router mount and the layout in isolation. Uses the existing `mockCommands` + `renderWithFullRouter` harness.

- [ ] **Step 1: Write the layout test file**

Create `apps/desktop/aegis-desktop/src/test/pages/project-workspace-layout.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";

const mockGetAll = vi.fn();
const mockFocus = vi.fn();
const mockShow = vi.fn();
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getAllWebviewWindows: (...args: unknown[]) => mockGetAll(...args),
}));

import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { ProjectWorkspaceLayout } from "../../pages/ProjectWorkspaceLayout";

afterEach(() => cleanup());
beforeEach(() => {
  mockGetAll.mockReset();
  mockFocus.mockReset();
  mockShow.mockReset();
  mockFocus.mockResolvedValue(undefined);
  mockShow.mockResolvedValue(undefined);
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() { return 0; },
  });
});

function renderLayout(initialPath: string) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <MemoryRouter initialEntries={[initialPath]}>
          <Routes>
            <Route
              path="/project/:projectCode/*"
              element={<ProjectWorkspaceLayout />}
            >
              <Route path="dashboard" element={<div>dashboard-slot</div>} />
              <Route
                path="configuration"
                element={<div>configuration-slot</div>}
              />
            </Route>
          </Routes>
        </MemoryRouter>
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectWorkspaceLayout", () => {
  it("renders the projectCode as the sidebar title", () => {
    renderLayout("/project/DEMO-001/dashboard");
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByText("DEMO-001")).toBeInTheDocument();
  });

  it("renders Dashboard and Configuration menu entries", () => {
    renderLayout("/project/DEMO-001/dashboard");
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Configuration")).toBeInTheDocument();
  });

  it("renders the focus-main footer Button", () => {
    renderLayout("/project/DEMO-001/dashboard");
    expect(
      screen.getByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();
  });

  it("clicking the focus-main Button calls setFocus + show on the main window", async () => {
    mockGetAll.mockResolvedValue([
      { label: "main", setFocus: mockFocus, show: mockShow },
      { label: "project:DEMO-001", setFocus: vi.fn(), show: vi.fn() },
    ]);
    renderLayout("/project/DEMO-001/dashboard");
    await userEvent.click(screen.getByRole("button", { name: /back to main/i }));
    await waitFor(() => {
      expect(mockGetAll).toHaveBeenCalled();
      expect(mockFocus).toHaveBeenCalledTimes(1);
      expect(mockShow).toHaveBeenCalledTimes(1);
    });
  });

  it("does nothing when no main window is present", async () => {
    mockGetAll.mockResolvedValue([
      { label: "project:DEMO-001", setFocus: vi.fn(), show: vi.fn() },
    ]);
    renderLayout("/project/DEMO-001/dashboard");
    await userEvent.click(screen.getByRole("button", { name: /back to main/i }));
    await waitFor(() => expect(mockGetAll).toHaveBeenCalled());
    expect(mockFocus).not.toHaveBeenCalled();
    expect(mockShow).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the layout test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/pages/project-workspace-layout.test.tsx
```

Expected: PASS. The layout component is self-contained — no auth guard mocking needed because we mount it inside a stub `MemoryRouter` and skip the route file entirely.

- [ ] **Step 3: Write the full-router test file**

Create `apps/desktop/aegis-desktop/src/test/routes/project-workspace.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { mockCommands } from "../tauri-mock";
import { TestQueryProvider } from "../test-query-provider";

afterEach(() => cleanup());
beforeEach(() => {
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() { return 0; },
  });
});

function renderRoot(initialEntries: string[]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("/project/DEMO-001/dashboard — authenticated", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the sidebar with the project code as title", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(await screen.findByTestId("sidebar")).toBeInTheDocument();
    expect(await screen.findByText("DEMO-001")).toBeInTheDocument();
  });

  it("renders the Dashboard heading with the project code", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(
      await screen.findByRole("heading", {
        name: /Dashboard — DEMO-001/,
      }),
    ).toBeInTheDocument();
  });

  it("renders the focus-main footer Button", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(
      await screen.findByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();
  });
});

describe("/project/DEMO-001/configuration — authenticated", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the Configuration heading with the project code", async () => {
    await renderRoot(["/project/DEMO-001/configuration"]);
    expect(
      await screen.findByRole("heading", {
        name: /Configuration — DEMO-001/,
      }),
    ).toBeInTheDocument();
  });
});

describe("/project/DEMO-001 — authenticated redirect", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("redirects bare /project/<code> to /project/<code>/dashboard", async () => {
    const { router } = await renderRoot(["/project/DEMO-001"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe(
        "/project/DEMO-001/dashboard",
      ),
    );
  });
});

describe("/project/DEMO-001/dashboard — unauthenticated", () => {
  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/project/DEMO-001/dashboard"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run the route test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm test src/test/routes/project-workspace.test.tsx
```

Expected: PASS — all 5 cases green. The full router with the regenerated `routeTree.gen.ts` recognizes the new `_project` segment and the auth guard redirects correctly.

- [ ] **Step 5: Run the full test suite**

```bash
pnpm test
```

Expected: PASS — every existing test still green plus the new layout + route tests.

- [ ] **Step 6: Commit**

```bash
cd ../..
git add apps/desktop/aegis-desktop/src/test/pages/project-workspace-layout.test.tsx \
        apps/desktop/aegis-desktop/src/test/routes/project-workspace.test.tsx
git commit -m "test(desktop): workspace layout + /project/* route tests" \
  -m "5 layout cases (sidebar title, menu, focus-main button, both" \
  -m "happy and no-main-window paths) and 5 route cases covering" \
  -m "authenticated dashboard, authenticated configuration, bare" \
  -m "redirect, and unauthenticated redirect-to-login." \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 12: Final verification — full test suite + manual smoke

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS — every test file green, no skipped tests.

- [ ] **Step 2: Type-check**

```bash
pnpm typecheck
```

Expected: PASS — no type errors.

- [ ] **Step 3: Lint**

```bash
pnpm exec eslint src --max-warnings=0
```

Expected: PASS. If the lint script isn't wired in `package.json`, skip this step.

- [ ] **Step 4: Manual smoke (desktop dev)**

Start the Tauri dev server:

```bash
cd apps/desktop/aegis-desktop
pnpm tauri dev
```

Walk through the user-facing flow:

1. Log in (or register + log in).
2. Navigate to `/projects`. Click the OpenInNew icon on a project row.
3. Confirm a new Tauri window opens, maximized, with the sidebar header showing the project code, the Dashboard heading showing `Dashboard — <code>`, and a "Back to main" footer button.
4. Click "Configuration" in the sidebar — page navigates to `/project/<code>/configuration`, heading shows `Configuration — <code>`.
5. Click "Back to main" — focus returns to the main window.
6. In the main window, open Settings, toggle theme from light → dark. The workspace window's theme updates immediately (no reload).
7. Toggle language from English → Chinese (zh-CN). Both windows update immediately.
8. Close the workspace window manually (X button). Main window unaffected.
9. Open the same project again. A new workspace window opens (because the previous one was closed).
10. Log out from the main window (UserFooter → confirm). Both the main window AND any open workspace window close; the main window navigates to `/login`.

- [ ] **Step 5: Final commit (only if any manual fix-ups were needed)**

If the smoke surfaced a fix-up, commit it with an appropriate `fix(desktop):` message.

```bash
cd ../..
git add -A
git commit -m "fix(desktop): <describe>" \
  -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

If no fix-ups, skip this step.

- [ ] **Step 6: Tag the milestone**

```bash
git tag feat/aegis-desktop_project-workspace-window
```

(Use a tag name that matches any existing convention — `feat/<scope>_<short-name>` if such tags already exist.)
