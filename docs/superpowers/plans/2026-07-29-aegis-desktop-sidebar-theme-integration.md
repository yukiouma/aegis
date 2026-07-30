# Aegis Desktop Sidebar + Theme Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `@aegis/ui`'s `Sidebar` and `AegisThemeProvider` / `useThemeMode` into `aegis-desktop` so the app has a Home page, a Settings page (with the theme switch), and a persistent light/dark theme.

**Architecture:** `AegisThemeProvider` mounts at the top of the React tree in `main.tsx`. `App.tsx` owns `page` and `sidebarOpen` state and renders `Sidebar` + the active page. `HomePage` is a placeholder with a "Test greet" button that calls the existing Tauri `greet` command. `SettingsPage` hosts a `Switch` bound to `useThemeMode()`. MUI primitives and icons are imported via `@aegis/ui/mui` and `@aegis/ui/icons` (peer-dep hoisting is not guaranteed in pnpm, so direct `@mui/material` imports in the desktop app would fail).

**Tech Stack:** React 19, TypeScript 5.8, MUI 9.2 (via `@aegis/ui`), Tauri 2, Vite 7, pnpm 10.33 workspaces.

**Spec:** [2026-07-29-aegis-desktop-sidebar-theme-integration-design.md](../specs/2026-07-29-aegis-desktop-sidebar-theme-integration-design.md)

---

## Global Constraints

These apply to every task. Do not deviate.

- React 19.x (`react`, `react-dom`).
- MUI primitives and icons are imported from `@aegis/ui/mui` and `@aegis/ui/icons`. Do **not** import from `@mui/material` or `@mui/icons-material` directly in the desktop app — those are peer deps of `@aegis/ui` only, not direct deps of `aegis-desktop`, and pnpm does not hoist peer deps to consumers.
- The package's own exports (`Sidebar`, types, `AegisThemeProvider`, `useThemeMode`, `ThemeMode`) come from `@aegis/ui` or `@aegis/ui/theme`.
- After every task, run `pnpm --filter aegis-desktop build` (which runs `tsc --noEmit` + Vite production bundle) to verify the change compiles. Fix any errors before committing.
- Commit messages: imperative mood, ≤72 chars subject, body explains "why".
- No new runtime dependencies. No changes to `App.css` (out of scope per spec).
- This is integration glue; no new automated tests. The spec lists "A vitest setup in the desktop app" as out of scope — do not add one.

---

## File Structure

| Path | Change | Responsibility |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/main.tsx` | Modify | Wrap `<App />` in `<AegisThemeProvider>`. |
| `apps/desktop/aegis-desktop/src/App.tsx` | Modify | Sidebar + page switcher + layout. |
| `apps/desktop/aegis-desktop/src/HomePage.tsx` | Create | Welcome placeholder + "Test greet" button. |
| `apps/desktop/aegis-desktop/src/SettingsPage.tsx` | Create | Theme switch bound to `useThemeMode()`. |

---

## Task 1: Create `HomePage.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/HomePage.tsx`

**Interfaces:**
- Produces: a default-style React component `<HomePage />` (named export) that renders a heading, a welcome line, and a "Test greet" button. The button calls `invoke<string>("greet", { name: "Aegis" })` and renders the response next to it.

- [ ] **Step 1: Create `HomePage.tsx`**

Write exactly:

```tsx
import { useState } from "react";
import { Box, Button, Stack, Typography } from "@aegis/ui/mui";
import { invoke } from "@tauri-apps/api/core";

export function HomePage() {
  const [greetMsg, setGreetMsg] = useState("");

  async function testGreet() {
    setGreetMsg(await invoke<string>("greet", { name: "Aegis" }));
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>Home</Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>
        Welcome to Aegis.
      </Typography>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center" }}>
        <Button variant="contained" onClick={testGreet}>
          Test greet
        </Button>
        {greetMsg && <Typography variant="body2">{greetMsg}</Typography>}
      </Stack>
    </Box>
  );
}
```

Notes:
- `@aegis/ui/mui` re-exports everything from `@mui/material` (see `lib/packages/ui/src/mui/index.ts`), so the named imports work.
- `@tauri-apps/api/core` is already a direct dep of `aegis-desktop` — no new deps added.
- The `invoke` call may reject at runtime if the Rust side is missing the `greet` command; the spec calls this acceptable for a demo button.

- [ ] **Step 2: Verify the build**

```bash
pnpm --filter aegis-desktop build
```

Expected: PASS. (The file is not yet imported by `App.tsx`, but the bundler will still typecheck and emit. The result is `App.tsx` is unchanged so the existing greet form is still rendered; this step is purely a syntax / typecheck gate on the new file.)

- [ ] **Step 3: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/HomePage.tsx
git commit -m "feat(desktop): add HomePage with Test greet button"
```

---

## Task 2: Create `SettingsPage.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/SettingsPage.tsx`

**Interfaces:**
- Produces: a named export `<SettingsPage />` that renders a heading and a `FormControlLabel` containing a `Switch` bound to `useThemeMode()`. The `Switch` is checked when `mode === "dark"`; toggling calls `setMode`.

- [ ] **Step 1: Create `SettingsPage.tsx`**

Write exactly:

```tsx
import type { ChangeEvent } from "react";
import { Box, FormControlLabel, Switch, Typography } from "@aegis/ui/mui";
import { useThemeMode } from "@aegis/ui/theme";

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? "dark" : "light");
  };

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4" gutterBottom>Settings</Typography>
      <FormControlLabel
        control={<Switch checked={mode === "dark"} onChange={handleChange} />}
        label={`Theme: ${mode}`}
      />
    </Box>
  );
}
```

Notes:
- `useThemeMode` throws if called outside a provider. The provider will be mounted in Task 3 before the page becomes reachable, so this is safe.
- `useThemeMode` already writes to `localStorage` and fires `onModeChange` internally — the page does not touch storage.
- The label is plain text (`Theme: light` / `Theme: dark`) per the spec.

- [ ] **Step 2: Verify the build**

```bash
pnpm --filter aegis-desktop build
```

Expected: PASS. The file is not yet imported anywhere; this is a typecheck-only gate.

- [ ] **Step 3: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/SettingsPage.tsx
git commit -m "feat(desktop): add SettingsPage with theme switch"
```

---

## Task 3: Mount `<AegisThemeProvider>` in `main.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`

**Interfaces:**
- Produces: `<App />` rendered inside `<AegisThemeProvider>` (which is itself inside `<React.StrictMode>`). The provider wraps the entire app so `useThemeMode()` works in any descendant.

- [ ] **Step 1: Replace `main.tsx`**

Replace `apps/desktop/aegis-desktop/src/main.tsx` contents with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { AegisThemeProvider } from "@aegis/ui/theme";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <App />
    </AegisThemeProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 2: Verify the build**

```bash
pnpm --filter aegis-desktop build
```

Expected: PASS. `App.tsx` is still the greet demo, so the page renders, but the entire tree is now inside a theme provider. The provider's MUI theme is applied; the placeholder themes (light/dark) use the MUI defaults. The `useThemeMode` consumers in the not-yet-rendered `SettingsPage` will work the moment `App.tsx` is updated in Task 4.

- [ ] **Step 3: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "feat(desktop): mount AegisThemeProvider in main.tsx"
```

---

## Task 4: Replace `App.tsx` with Sidebar + page switcher

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/App.tsx`

**Interfaces:**
- Produces: a default-exported `App` component that:
  - Owns `page: "home" | "settings"` (default `"home"`) and `sidebarOpen: boolean` (default `true`) state.
  - Renders a flex row: `<Sidebar />` on the left, `<Box component="main">` on the right with a matching `margin-left` (240 expanded / 56 collapsed, the Sidebar's defaults).
  - The Sidebar's `onNavigate` maps `/settings` → `"settings"`, anything else → `"home"`.
  - The main pane renders `<SettingsPage />` or `<HomePage />` based on `page`.

- [ ] **Step 1: Replace `App.tsx`**

Replace `apps/desktop/aegis-desktop/src/App.tsx` contents with:

```tsx
import { useState } from "react";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";

// MUI icon components require SvgIconProps; the Sidebar's `icon` slot is
// typed as the no-required-props `ComponentType`. Wrap each icon in a
// no-arg function so the assignment type-checks.
const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

const menu: MenuItem[] = [
  { link: "/home", title: "Home", icon: HomeMenuIcon },
  { link: "/settings", title: "Settings", icon: SettingsMenuIcon },
];

type Page = "home" | "settings";

function pageFromLink(link: string): Page {
  return link === "/settings" ? "settings" : "home";
}

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const sidebarProps: SidebarProps = {
    title: "Aegis",
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

Notes:
- `Home` and `Settings` from `@aegis/ui/icons` are the MUI icon components. The Sidebar's `MenuItem.icon` accepts a `ComponentType` (see `lib/packages/ui/components/Sidebar/types.ts`), so the icon components are passed directly.
- The `margin-left` values 240 / 56 are the Sidebar's default `width` / `collapsedWidth`. They are duplicated here for layout because the Sidebar is `variant="permanent"` and does not push siblings.
- `pageFromLink` defaults unknown links to `"home"`. Future menu items whose page isn't built yet will not crash the app — they will land on the Home page.

- [ ] **Step 2: Verify the build**

```bash
pnpm --filter aegis-desktop build
```

Expected: PASS. The Vite production bundle emits to `apps/desktop/aegis-desktop/dist/`.

- [ ] **Step 3: Sanity-check the import discipline**

```bash
grep -RIn "@mui/material\|@mui/icons-material" apps/desktop/aegis-desktop/src
```

Expected: no matches. All MUI imports in the desktop app route through `@aegis/ui/*` subpaths.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/App.tsx
git commit -m "feat(desktop): render Sidebar with Home and Settings pages"
```

---

## Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Run the full build**

```bash
pnpm --filter aegis-desktop build
```

Expected: PASS. The desktop app's TypeScript + Vite production bundle both succeed.

- [ ] **Step 2: Confirm the package's typecheck + tests still pass**

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected: PASS — same 28 tests green as before. The integration does not touch the package's source.

- [ ] **Step 3: Manual smoke checklist**

`pnpm --filter aegis-desktop dev` (or `pnpm tauri dev` from `apps/desktop/aegis-desktop` if Tauri is set up) and walk through:

1. App starts on Home with Sidebar expanded.
2. Sidebar toggle collapses the rail; main pane reflows.
3. Clicking `Settings` shows the Settings page with a `Switch` labelled `Theme: light`.
4. Toggling the Switch flips the app to dark mode; `localStorage.getItem('aegis:theme:mode')` returns `"dark"`.
5. Reload (or app restart) restores dark mode on launch.
6. Clicking `Home` returns to the Home page.
7. Clicking "Test greet" shows the response string from the Rust `greet` command.

If any step fails, fix the relevant file and re-run the build. Steps 4–5 require `localStorage` in the WebView (Tauri WebView2 / WKWebView); the spec assumes this is available.

- [ ] **Step 4: Commit (only if Step 3 surfaced a deviation that was fixed)**

If everything matches, no commit.

---

## Done Criteria

- [ ] All 5 tasks committed on the current branch.
- [ ] `pnpm --filter aegis-desktop build` PASS.
- [ ] `pnpm -F @aegis/ui typecheck` and `pnpm -F @aegis/ui test` still PASS (28 tests).
- [ ] No `import` from `@mui/material` or `@mui/icons-material` in `apps/desktop/aegis-desktop/src` (verified by `grep`).
- [ ] Manual smoke checklist passes (or any deviations are fixed and committed).
- [ ] No new runtime dependencies added to `aegis-desktop/package.json`.
- [ ] `App.css` is unchanged.
