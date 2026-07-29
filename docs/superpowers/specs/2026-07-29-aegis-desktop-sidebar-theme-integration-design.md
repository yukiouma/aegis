# Aegis Desktop Sidebar + Theme Integration — Design

**Date:** 2026-07-29
**Status:** Approved (pending spec review)
**Scope:** Wire `@aegis/ui`'s `Sidebar` and `AegisThemeProvider` / `useThemeMode` into the `aegis-desktop` Tauri app. Replace the existing greet-form demo content with a two-page (Home, Settings) shell that exercises both components. The `invoke('greet')` Tauri command stays wired via a small "Test greet" button on Home.

---

## 1. Goals

1. Mount `<AegisThemeProvider>` once at the top of the desktop app's React tree.
2. Render a `<Sidebar>` with `Home` and `Settings` menu items to the left of the main content.
3. Switch between two pages — Home and Settings — based on the Sidebar's `onNavigate` callback. No router.
4. Place the theme switch (light/dark toggle) on the Settings page.
5. Preserve the existing `invoke('greet')` Tauri command binding by exposing a small "Test greet" button on Home.

---

## 2. Files added / changed

| Path | Change | Responsibility |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/main.tsx` | Modify | Wrap `<App />` in `<AegisThemeProvider>`. |
| `apps/desktop/aegis-desktop/src/App.tsx` | Modify | Render `Sidebar` + active page; own `page` and `sidebarOpen` state. |
| `apps/desktop/aegis-desktop/src/HomePage.tsx` | Create | Welcome placeholder + "Test greet" button (uses `invoke('greet')`). |
| `apps/desktop/aegis-desktop/src/SettingsPage.tsx` | Create | Theme switch (`Switch` bound to `useThemeMode`). |

No new runtime dependencies. The `@aegis/ui` workspace dep is already present.

---

## 3. Provider placement — `main.tsx`

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

- The provider is the outermost component under StrictMode so every descendant has access to `useThemeMode()`.
- The provider owns the localStorage key (`aegis:theme:mode`); the desktop app does not touch storage directly.

---

## 4. App layout — `App.tsx`

```tsx
import { useState } from "react";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home, Settings } from "@aegis/ui/icons";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";

const menu: MenuItem[] = [
  { link: "/home", title: "Home", icon: Home },
  { link: "/settings", title: "Settings", icon: Settings },
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
- `Sidebar` is `variant="permanent"`, so the right pane must reserve space with a matching `margin-left` (240 expanded / 56 collapsed). The 240/56 values come from the Sidebar's default `width` / `collapsedWidth`.
- Link-to-page mapping is centralised in `pageFromLink`. The default branch falls back to `"home"` for any unknown link (e.g. a future menu item whose page isn't built yet).
- The Sidebar component already exists in `@aegis/ui` and is fully tested there — no changes to the package.

---

## 5. Home page — `HomePage.tsx`

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
      <Stack direction="row" spacing={2} alignItems="center">
        <Button variant="contained" onClick={testGreet}>
          Test greet
        </Button>
        {greetMsg && <Typography variant="body2">{greetMsg}</Typography>}
      </Stack>
    </Box>
  );
}
```

- The "Test greet" button calls the existing Tauri `greet` command (defined in Rust; unchanged). The response renders next to the button.
- This is the only consumer of `invoke` in the desktop app, satisfying "keep invoke wired" without restoring the original demo form.

---

## 6. Settings page — `SettingsPage.tsx`

```tsx
import { Box, FormControlLabel, Switch, Typography } from "@aegis/ui/mui";
import { useThemeMode } from "@aegis/ui/theme";

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
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

- The `Switch` is checked when the current mode is `"dark"`; toggling calls `setMode` on the provider. The provider writes to localStorage; on the next app start the same mode is restored.
- `ThemeMode` is the type of the `mode` value; the page re-uses it implicitly through the hook's return type.
- The label is informational. A more elaborate control (icon + label, radio group, etc.) is a follow-up.

---

## 7. Import discipline

`aegis-desktop` depends on `@aegis/ui` only — it does **not** list `@mui/material` or `@mui/icons-material` as direct dependencies. Under pnpm, peer-dep hoisting is not guaranteed, so importing directly from `@mui/material` or `@mui/icons-material` in the desktop app can fail to resolve at build time. Always import MUI primitives and icons through the package's re-exports:

- `@aegis/ui/mui` for MUI components (`Box`, `Button`, `Stack`, `Typography`, `FormControlLabel`, `Switch`, etc.).
- `@aegis/ui/icons` for MUI icons (`Home`, `Settings`, etc.).
- `@aegis/ui` for the package's own exports (`Sidebar`, types, theme provider, theme hook).

This keeps the desktop app's `package.json` honest and avoids the "works on my machine because of hoisting" failure mode.

## 8. Behavior

- **Initial render:** Sidebar open, Home page selected. Mode from `localStorage` (`aegis:theme:mode`) or `"light"`.
- **Sidebar toggle:** clicking the title-bar icon button flips `sidebarOpen`; the main pane's `margin-left` animates to match.
- **Navigation:** clicking `Home` calls `onNavigate("/home")` → `setPage("home")`; clicking `Settings` calls `onNavigate("/settings")` → `setPage("settings")`. Leaf items only — no submenu.
- **Theme switch:** toggling the Switch updates the provider's `mode`; the MUI theme and `<CssBaseline />` inside the provider re-render. The choice persists in `localStorage` and survives an app restart.
- **Test greet:** clicking the button on Home calls the Tauri `greet` command and renders the response string. Errors from `invoke` (e.g. the Rust side is missing the command) propagate to the unhandled-rejection channel; the page does not catch them. (YAGNI for a try/catch in a demo button.)

---

## 9. Testing

This is integration glue; no new automated tests. The desktop app's `build` script is `tsc && vite build`, so a successful build is also a typecheck. Verification commands:

- `pnpm --filter aegis-desktop build` PASS (covers both `tsc --noEmit` and the Vite production bundle).
- (Optional) `pnpm --filter aegis-desktop dev` for a hot-reload smoke check during implementation.

Manual smoke (out of scope for CI, but the implementer should run `pnpm tauri dev`):

1. App starts on Home with Sidebar expanded.
2. Clicking the Sidebar toggle collapses the rail; main pane reflows.
3. Clicking `Settings` shows the Settings page with a Switch labelled `Theme: light`.
4. Toggling the switch flips the app to dark mode; `localStorage.getItem('aegis:theme:mode')` returns `"dark"`.
5. Restarting the app (or reloading the webview) restores dark mode on launch.
6. Clicking `Home` returns to the Home page.
7. Clicking "Test greet" shows the response string from the Rust `greet` command.

---

## 10. Out of scope

- Routing library (no react-router / wouter / hash router).
- More menu items or more pages.
- More settings content beyond the theme switch.
- Real theme tokens in `lib/packages/ui/src/theme/themes/{light,dark}.ts` (the user provides those later; the desktop app already works against the placeholder themes).
- Cleanup of leftover `.logo` / `.row` / demo button styles in `App.css`.
- A vitest setup in the desktop app.
- The `App.css` import in `App.tsx` — kept as-is for now; can be removed or rewritten in a follow-up.
- A Tauri-side change. The Rust `greet` command is untouched.
