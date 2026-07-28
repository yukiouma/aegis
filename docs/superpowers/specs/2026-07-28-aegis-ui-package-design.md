# `@aegis/ui` Package & Sidebar Component — Design

**Date:** 2026-07-28
**Status:** Approved (pending spec review)
**Scope:** New pnpm workspace package `@aegis/ui` and a Sidebar component built with MUI.

---

## 1. Goals

1. Create a new pnpm workspace package `lib/packages/ui` named `@aegis/ui`.
2. Add MUI dependencies (`@mui/material`, `@emotion/react`, `@emotion/styled`, `@mui/icons-material`) and re-export them from the package.
3. Implement a `Sidebar` component matching the user's spec: collapsible (icons-only when closed, full menu when open), title bar with a close button, two-level menu with submenu expansion on click and navigation on leaf click.

---

## 2. Package structure

```
lib/packages/ui/
  package.json
  tsconfig.json
  vitest.config.ts
  vitest.setup.ts
  src/
    index.ts              # main barrel — re-exports Sidebar + types from ../components/Sidebar
    mui/
      index.ts            # re-exports @mui/material
    icons/
      index.ts            # re-exports @mui/icons-material
  components/
    Sidebar/
      index.ts            # public exports
      Sidebar.tsx         # implementation
      types.ts            # MenuItem, SubMenuItem, SidebarProps
      Sidebar.test.tsx    # vitest + RTL tests
      test-utils.tsx      # render helpers (theme wrapper)
```

### 2.1 Build & consumption

The package exports `.tsx`/`.ts` source files directly (no build step). The desktop app's existing Vite + `@vitejs/plugin-react` setup transpiles them on import. This keeps the package DX simple for a single-consumer internal monorepo.

`package.json` `exports` map:

```json
{
  ".":             "./src/index.ts",
  "./Sidebar":     "./components/Sidebar/index.ts",
  "./mui":         "./src/mui/index.ts",
  "./icons":       "./src/icons/index.ts"
}
```

### 2.2 Dependencies

**peerDependencies:**
- `react` ^19
- `react-dom` ^19
- `@emotion/react` ^11
- `@emotion/styled` ^11
- `@mui/material` ^9
- `@mui/icons-material` ^9

**devDependencies:** the same MUI/emotion packages (so the package builds and tests in isolation), `@types/react`, `@types/react-dom`, `typescript`, `vitest`, `@testing-library/react`, `@testing-library/jest-dom`, `jsdom`.

### 2.3 Scripts

- `test` → `vitest run`
- `test:watch` → `vitest`
- `typecheck` → `tsc --noEmit`

---

## 3. MUI re-exports

`src/mui/index.ts`:

```ts
export * from '@mui/material';
```

`src/icons/index.ts`:

```ts
export * from '@mui/icons-material';
```

Consumers import like:

```ts
import { Button, Drawer } from '@aegis/ui/mui';
import { Home, Settings } from '@aegis/ui/icons';
```

Naming collisions between re-exports and direct MUI imports are the consumer's responsibility — they should use named imports.

---

## 4. Sidebar API

### 4.1 Types (`components/Sidebar/types.ts`)

```ts
import type { ComponentType } from 'react';

export interface SubMenuItem {
  link: string;
  title: string;
  icon: ComponentType; // MUI icons satisfy ComponentType; if extra props (e.g. SvgIconProps) are needed, widen at the consumer side
}

export interface MenuItem extends SubMenuItem {
  subMenu?: SubMenuItem[];
}

export interface SidebarProps {
  title: string;
  menu: MenuItem[];
  open: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
  width?: number;          // expanded width, default 240
  collapsedWidth?: number; // default 56
}
```

### 4.2 Public exports (`components/Sidebar/index.ts`)

- `Sidebar` (named + default)
- Type re-exports: `MenuItem`, `SubMenuItem`, `SidebarProps`

### 4.3 Behavior

- **Controlled state:** parent owns `open`. The icon button calls `onToggle`.
- **Title bar:**
  - `IconButton` is always rendered (in both states). Icon is `FormatIndentDecrease` when `open=true`, `FormatIndentIncrease` when `open=false` (both from `@mui/icons-material`).
  - Title text (`Typography`) is rendered only when `open=true`. Positioned to the **right** of the icon button.
- **Menu list:** `List`/`ListItem`/`ListItemButton` from MUI.
  - **Collapsed (`open=false`):** only icons render. Titles appear in `Tooltip` on hover. Submenu items do not render (per spec).
  - **Expanded (`open=true`):** full text + icons.
- **Item click:**
  - If `item.subMenu` exists → toggle that item's submenu (local state: `Set<string>` of expanded keys, keyed by `link`).
  - Otherwise → call `onNavigate?.(item.link)`.
- **Submenu items:** nested `List` inside a `Collapse` transition, indent via `ListItemButton` `sx={{ pl: open ? 4 : 0 }}`.

### 4.4 Error handling

- Empty `menu` array → render an empty list (no crash, no placeholder).
- Missing `icon` on an item → render the menu item text without an icon.
- `onNavigate` undefined and leaf clicked → no-op (optional chaining).
- Invalid `link` → still call `onNavigate`; parent decides what to do.

---

## 5. Tests

`Sidebar.test.tsx` covers:

1. Renders title text when `open=true`.
2. Title close button calls `onClose` when clicked.
3. Collapsed state (`open=false`): icons render, title text and labels do not.
4. Menu item without `subMenu`: clicking calls `onNavigate(item.link)`.
5. Menu item with `subMenu`: clicking toggles submenu open/closed.
6. `onNavigate` is not called when expanding a submenu.
7. Custom `width` and `collapsedWidth` props are applied to the drawer — implementation applies these via `Drawer`'s `sx` prop (`width: open ? width : collapsedWidth`) and `variant="permanent"`.

Test setup uses `jsdom` + `@testing-library/jest-dom`. A small `test-utils.tsx` wraps renders in MUI's `<ThemeProvider>` so components have access to a theme.

---

## 6. Integration with the desktop app

- Add `"@aegis/ui": "workspace:*"` to `apps/desktop/aegis-desktop/package.json` dependencies.
- Vite + the React plugin already transpile workspace TypeScript sources; no Vite config changes required.
- If `tsc --noEmit` in the desktop app fails on cross-package imports, add the ui package's `src/` to the desktop app's `tsconfig.json` `references` array so types resolve.

---

## 7. Out of scope

- Multiple sidebars, drawer variants other than permanent, mobile/temporary overlay mode.
- Theming customization (consumers pass their own theme via `ThemeProvider`).
- Search/filter, pinning, drag-to-reorder, keyboard shortcuts.
- Router integration — navigation is via the `onNavigate` callback.
- Storybook / visual docs.