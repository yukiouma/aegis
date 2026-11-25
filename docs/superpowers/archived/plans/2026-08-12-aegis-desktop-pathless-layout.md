# Aegis Desktop — Pathless Layout Route — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the Sidebar + main-`<Box>` chrome currently in `src/routes/__root.tsx` into a TanStack Router **pathless layout route** (`_layout/route.tsx`). `__root.tsx` becomes a bare `<Outlet />`. The two page routes (`/`, `/settings`) move into the `_layout/` directory so they are children of the new pathless layout. Rendered DOM, behavior, and test assertions are unchanged.

**Architecture:** TanStack Router file-based routing with `@tanstack/router-plugin/vite` already configured. The directory-based pathless-layout convention uses `route.tsx` *inside* the `_layout/` directory as the layout file — children placed alongside (`index.tsx`, `settings.tsx`) are wrapped by that layout but contribute no URL segment. The plugin regenerates `routeTree.gen.ts` on `vite dev`/`vite build`; the regenerated file is committed so `tsc --noEmit` works without running Vite first.

**Tech Stack:** React 19, TanStack Router v1.170, Vite v7, Vitest v2.1, Testing Library v16. Workspace dep: `@aegis/ui`.

---

## Global Constraints

- File-based routing mode: routes live in `src/routes/`. The plugin-generated `routeTree.gen.ts` is committed to the repo.
- All MUI imports go through `@aegis/ui/mui`, `@aegis/ui/icons`, `@aegis/ui/i18n`, `@aegis/ui/theme` — never direct `@mui/material` / `@mui/icons-material`.
- TypeScript `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true` — every step must satisfy these.
- The pathless layout file is `route.tsx` *inside* the `_layout/` directory (TanStack Router directory convention) — **not** `_layout.tsx` at the parent level.
- Existing test cases (`renderWithFullRouter`, `renderInRouter`, all assertions) carry over verbatim apart from the `describe` label rename in Task 2. No behaviour change.
- Tests use Vitest + Testing Library (`@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom/vitest`).
- Commit messages use a conventional prefix: `refactor(desktop)` for the implementation, `test(desktop)` for test scaffolding.
- Verification commands after every task: `pnpm --filter aegis-desktop typecheck` and `pnpm --filter aegis-desktop test`. The route restructure task also runs `pnpm --filter aegis-desktop build` to trigger plugin regen.
- Do not modify `vite.config.ts` — the `@tanstack/router-plugin/vite` is already wired.

---

### Task 1: Restructure routes into the `_layout/` directory

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_layout/route.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_layout/index.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_layout/settings.tsx`
- Modify: `apps/desktop/aegis-desktop/src/routes/__root.tsx`
- Delete: `apps/desktop/aegis-desktop/src/routes/index.tsx`
- Delete: `apps/desktop/aegis-desktop/src/routes/settings.tsx`
- Regenerate: `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (via `vite build`)

**Interfaces:**
- `src/routes/_layout/route.tsx` exports `Route = createFileRoute("/_layout")({ component: AppLayout })` and a default-exported `AppLayout` function (renamed from `RootLayout`) that owns `sidebarOpen` state and renders `<Sidebar/>` + `<main><Outlet/></main>`.
- `src/routes/_layout/index.tsx` exports `Route = createFileRoute("/_layout/")({ component: HomePage })`. Imports `HomePage` from `"../../pages/home"`.
- `src/routes/_layout/settings.tsx` exports `Route = createFileRoute("/_layout/settings")({ component: SettingsPage })`. Imports `SettingsPage` from `"../../pages/settings"`.
- `src/routes/__root.tsx` becomes `createRootRoute({ component: () => <Outlet /> })` — no other imports, no JSX chrome.

This task is one atomic change: creating the new files alongside the old ones would produce duplicate `/` and `/settings` routes in the generated tree. The steps below create the new files, slim `__root.tsx`, delete the old page files, then regenerate `routeTree.gen.ts` via a Vite build before verifying.

- [ ] **Step 1: Create `src/routes/_layout/route.tsx`**

Create the file `apps/desktop/aegis-desktop/src/routes/_layout/route.tsx` with the following content. This is the layout moved verbatim from `__root.tsx`, with the function renamed `RootLayout` → `AppLayout`:

```tsx
import React from "react";
import { createFileRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

export const Route = createFileRoute("/_layout")({
  component: AppLayout,
});

export default function AppLayout() {
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

- [ ] **Step 2: Create `src/routes/_layout/index.tsx`**

Create the file `apps/desktop/aegis-desktop/src/routes/_layout/index.tsx`. The body is byte-identical to the existing `src/routes/index.tsx` except the import path moves from `"../pages/home"` to `"../../pages/home"` to account for the deeper directory:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { HomePage } from "../../pages/home";

export const Route = createFileRoute("/_layout/")({
  component: HomePage,
});
```

- [ ] **Step 3: Create `src/routes/_layout/settings.tsx`**

Create the file `apps/desktop/aegis-desktop/src/routes/_layout/settings.tsx`. The body is byte-identical to the existing `src/routes/settings.tsx` except the import path moves from `"../pages/settings"` to `"../../pages/settings"`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { SettingsPage } from "../../pages/settings";

export const Route = createFileRoute("/_layout/settings")({
  component: SettingsPage,
});
```

- [ ] **Step 4: Slim `src/routes/__root.tsx` to `<Outlet />`**

Replace the entire content of `apps/desktop/aegis-desktop/src/routes/__root.tsx` with:

```tsx
import { createRootRoute, Outlet } from "@tanstack/react-router";

export const Route = createRootRoute({
  component: () => <Outlet />,
});
```

This removes the `Sidebar`, `Box`, `useI18n`, `useNavigate`, `React.useState`, and the `RootLayout` function. The route stays named `__root.tsx` — only its component shrinks.

- [ ] **Step 5: Delete the old page route files**

```bash
rm apps/desktop/aegis-desktop/src/routes/index.tsx
rm apps/desktop/aegis-desktop/src/routes/settings.tsx
```

These are now superseded by `src/routes/_layout/index.tsx` and `src/routes/_layout/settings.tsx`. Leaving them in place would cause the plugin to emit duplicate routes for `/` and `/settings`.

- [ ] **Step 6: Regenerate `routeTree.gen.ts`**

Run a Vite build to let `@tanstack/router-plugin/vite` walk the new directory layout and rewrite `routeTree.gen.ts`:

```bash
cd apps/desktop/aegis-desktop && pnpm exec vite build
```

Expected: build succeeds. `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` now references `_layout/route`, `_layout/index`, `_layout/settings` as the children of `__root__`. (The `tsc` step in `pnpm build` is bypassed here because we want to trigger the plugin without a successful prior typecheck — the route tree is generated first, then verified.)

Inspect the file to confirm the new directory layout is referenced:

```bash
grep -E "_layout/(route|index|settings)" apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
```

Expected: at least three matches, one per new file (`_layout/route`, `_layout/index`, `_layout/settings`). The exact variable names depend on the plugin version (`LayoutRouteImport`, `LayoutIndexRouteImport`, `LayoutSettingsRouteImport`, etc.) — only the source paths matter.

If the file still references `./index` and `./settings` at the top level with no `_layout/` entries, the plugin has not regenerated — delete `routeTree.gen.ts` and re-run the build to force a fresh emit.

- [ ] **Step 7: Verify typecheck, build, and tests pass**

Run:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three PASS.

- `typecheck` confirms the regenerated `routeTree.gen.ts` types are valid (pathless layout + two children).
- `build` runs `tsc && vite build` end-to-end.
- `test` exercises the full route tree via `renderWithFullRouter` (in `src/test/routes/__root.test.tsx`) and the leaf routes via `renderInRouter` (in `src/test/routes/index.test.tsx` and `settings.test.tsx`).

If `__root.test.tsx` fails with "Sidebar not found" or "no element with role heading name=/home/i", the route tree has not been regenerated — repeat Step 6 and re-run.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_layout apps/desktop/aegis-desktop/src/routes/__root.tsx apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git rm apps/desktop/aegis-desktop/src/routes/index.tsx apps/desktop/aegis-desktop/src/routes/settings.tsx
git commit -m "refactor(desktop): move layout to _layout/ pathless route; slim __root to <Outlet />"
```

---

### Task 2: Rename `__root.test.tsx` → `_layout.test.tsx` and update the describe block

**Files:**
- Rename: `apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx` → `apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx` (describe block only)

**Interfaces:**
- The renamed test file uses `renderWithFullRouter` to mount the full routeTree (now includes the `_layout` pathless route as the parent of `/` and `/settings`). All three assertions stay identical: Sidebar renders at `/`, menu items navigate, content swaps.
- The `describe("RootLayout", ...)` label becomes `describe("AppLayout", ...)` to match the renamed component.

- [ ] **Step 1: Rename the test file via `git mv`**

```bash
git mv apps/desktop/aegis-desktop/src/test/routes/__root.test.tsx apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx
```

Using `git mv` (not `mv` + `git add`) preserves rename history in the commit.

- [ ] **Step 2: Update the `describe` label**

In `apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx`, change:

```tsx
describe("RootLayout", () => {
```

to:

```tsx
describe("AppLayout", () => {
```

No other lines change. The assertions (`getByTestId("sidebar")`, `getByText("Settings")`, `getByText("Home")`, `getByRole("heading", ...)`) are byte-identical.

- [ ] **Step 3: Run the renamed test and verify it passes**

Run:

```bash
pnpm --filter aegis-desktop test
```

Expected: PASS. Vitest now reports the renamed file at `src/test/routes/_layout.test.tsx` with three tests (Sidebar + Home render, navigate to Settings, navigate back to Home). All other test files (`index.test.tsx`, `settings.test.tsx`, `document-lang-sync.test.tsx`) continue to pass.

If any test reports `getByText("Settings")` matching multiple elements (unlikely, but possible if Settings text appears elsewhere on the page), tighten the selector to `screen.getByRole("link", { name: "Settings" })` or scope it to the Sidebar via `within(screen.getByTestId("sidebar")).getByText("Settings")`.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx
git commit -m "test(desktop): rename __root.test.tsx to _layout.test.tsx; describe AppLayout"
```

---

### Task 3: Final smoke verification

**Files:** none changed.

- [ ] **Step 1: Run the full verification gauntlet**

From the repo root, run all three commands:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
pnpm --filter aegis-desktop test
```

Expected: all three PASS.

- `typecheck` confirms the regenerated route tree types are still valid after the test rename.
- `build` runs `tsc && vite build` end-to-end.
- `test` exercises four test files (document-lang-sync, routes/_layout, routes/index, routes/settings).

- [ ] **Step 2: Confirm the directory layout matches the spec**

From the repo root, list the desktop app's `src/routes/` and `src/test/routes/` trees:

```bash
find apps/desktop/aegis-desktop/src/routes apps/desktop/aegis-desktop/src/test/routes -type f | sort
```

Expected output (file names only):

```
apps/desktop/aegis-desktop/src/routes/__root.tsx
apps/desktop/aegis-desktop/src/routes/_layout/index.tsx
apps/desktop/aegis-desktop/src/routes/_layout/route.tsx
apps/desktop/aegis-desktop/src/routes/_layout/settings.tsx
apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx
apps/desktop/aegis-desktop/src/test/routes/index.test.tsx
apps/desktop/aegis-desktop/src/test/routes/settings.test.tsx
```

No top-level `index.tsx` or `settings.tsx` should remain under `src/routes/`. No `__root.test.tsx` should remain under `src/test/routes/`.

- [ ] **Step 3: (Optional, manual) Boot the Tauri app**

```bash
pnpm tauri dev --filter aegis-desktop
```

Smoke checks:

1. App boots at `/`; Sidebar open; Home heading + login form render.
2. Click `Settings` in the Sidebar → URL/state at `/settings`, Settings heading + theme switch + language select render.
3. Click `Home` → returns to `/`.
4. Toggle the Sidebar collapse icon → main pane's `margin-left` animates (240 ↔ 56); route content stays put.
5. Theme + language toggles in Settings behave identically to before.

If any smoke check fails, file an issue and fix before declaring the refactor complete.