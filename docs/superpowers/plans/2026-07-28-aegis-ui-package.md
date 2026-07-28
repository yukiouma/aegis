# `@aegis/ui` Package & Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `@aegis/ui` pnpm workspace package with MUI re-exports and a collapsible Sidebar component, integrated with the existing Tauri desktop app.

**Architecture:** Single workspace package `lib/packages/ui` consumes its own TypeScript source directly (no build step). Desktop app's existing Vite + React plugin transpiles on import. Sidebar built on MUI `Drawer` variant="permanent" + `List`/`ListItemButton`. State is controlled (parent owns `open`/`onToggle`); submenu expansion is local state.

**Tech Stack:** React 19, TypeScript 5.8, MUI 9.2, Emotion 11, Vitest, @testing-library/react, pnpm 10.33 workspaces.

**Spec:** [2026-07-28-aegis-ui-package-design.md](../specs/2026-07-28-aegis-ui-package-design.md)

---

## Global Constraints

These apply to every task. Do not deviate.

- React 19.x (`react`, `react-dom`).
- MUI peer deps use `^9` (installed 9.2.0). Emotion peer deps use `^11`.
- TypeScript strict mode (matches desktop app).
- Package name: `@aegis/ui`. Private. No build step — exports `.ts`/`.tsx` source.
- Source layout: `lib/packages/ui/{src,components}`; `src` holds barrels (`index.ts`, `mui/`, `icons/`); `components/Sidebar/` holds the component.
- TDD: every implementation step is preceded by a failing test step. Tests live alongside the implementation (`Sidebar.test.tsx` next to `Sidebar.tsx`).
- Commit messages: imperative mood, ≤72 chars subject, body explains "why".
- After each implementation task: run `pnpm -F @aegis/ui typecheck` AND `pnpm -F @aegis/ui test` before committing.

---

## File Structure

| Path | Responsibility |
| --- | --- |
| `lib/packages/ui/package.json` | Package manifest (name, exports, scripts, peer + dev deps). |
| `lib/packages/ui/tsconfig.json` | TS config — strict, jsx react-jsx, noEmit. |
| `lib/packages/ui/vitest.config.ts` | Vitest config — jsdom env, setup file. |
| `lib/packages/ui/vitest.setup.ts` | Imports `@testing-library/jest-dom`. |
| `lib/packages/ui/src/index.ts` | Main barrel — re-exports Sidebar + types + mui + icons. |
| `lib/packages/ui/src/mui/index.ts` | Re-exports `@mui/material`. |
| `lib/packages/ui/src/icons/index.ts` | Re-exports `@mui/icons-material`. |
| `lib/packages/ui/components/Sidebar/types.ts` | `MenuItem`, `SubMenuItem`, `SidebarProps`. |
| `lib/packages/ui/components/Sidebar/Sidebar.tsx` | Component implementation. |
| `lib/packages/ui/components/Sidebar/Sidebar.test.tsx` | Vitest + RTL tests. |
| `lib/packages/ui/components/Sidebar/test-utils.tsx` | `renderWithTheme` helper. |
| `lib/packages/ui/components/Sidebar/index.ts` | Sidebar barrel. |
| `apps/desktop/aegis-desktop/package.json` | Add `@aegis/ui: workspace:*` dep. |

---

## Task 1: Package scaffold & dependency install

**Files:**
- Create: `lib/packages/ui/package.json`
- Create: `lib/packages/ui/tsconfig.json`
- Create: `lib/packages/ui/vitest.config.ts`
- Create: `lib/packages/ui/vitest.setup.ts`

**Interfaces:**
- Produces: a pnpm-workspace package `@aegis/ui` whose `typecheck` and `test` scripts run from the repo root via `pnpm -F @aegis/ui <script>`.

- [ ] **Step 1: Create `lib/packages/ui/package.json`**

Write exactly:

```json
{
  "name": "@aegis/ui",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": "./src/index.ts",
    "./Sidebar": "./components/Sidebar/index.ts",
    "./mui": "./src/mui/index.ts",
    "./icons": "./src/icons/index.ts"
  },
  "scripts": {
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "peerDependencies": {
    "react": "^19",
    "react-dom": "^19",
    "@emotion/react": "^11",
    "@emotion/styled": "^11",
    "@mui/material": "^9",
    "@mui/icons-material": "^9"
  },
  "devDependencies": {
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "@emotion/react": "^11",
    "@emotion/styled": "^11",
    "@mui/material": "9.2.0",
    "@mui/icons-material": "9.2.0",
    "@types/react": "^19.1.8",
    "@types/react-dom": "^19.1.6",
    "typescript": "~5.8.3",
    "vitest": "^2.1.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/jest-dom": "^6.5.0",
    "@testing-library/user-event": "^14.5.0",
    "jsdom": "^25.0.0"
  }
}
```

Pin MUI to exact `9.2.0` in devDependencies so peer + installed match.

- [ ] **Step 2: Create `lib/packages/ui/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src", "components", "vitest.setup.ts"]
}
```

- [ ] **Step 3: Create `lib/packages/ui/vitest.config.ts`**

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
    include: ['**/*.test.{ts,tsx}'],
  },
});
```

- [ ] **Step 4: Create `lib/packages/ui/vitest.setup.ts`**

```ts
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 5: Install dependencies**

From repo root:

```bash
pnpm install
```

Expected: pnpm detects the new `@aegis/ui` workspace package and links it. No errors.

- [ ] **Step 6: Verify scaffold works**

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected:
- `typecheck` PASS (no source files, but config must load).
- `test` exits 0 with "No test files found" or similar. Some vitest versions exit 1 on no tests — that's acceptable, proceed.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/package.json lib/packages/ui/tsconfig.json lib/packages/ui/vitest.config.ts lib/packages/ui/vitest.setup.ts pnpm-lock.yaml
git commit -m "feat(ui): scaffold @aegis/ui package"
```

---

## Task 2: MUI barrel re-exports & main barrel

**Files:**
- Create: `lib/packages/ui/src/mui/index.ts`
- Create: `lib/packages/ui/src/icons/index.ts`
- Create: `lib/packages/ui/src/index.ts`
- Create: `lib/packages/ui/src/index.test.ts`

**Interfaces:**
- Produces: `import { mui, icons } from '@aegis/ui'` resolves with `mui.Button === Mui.Button` and `icons.Home === Icons.Home`.

- [ ] **Step 1: Write the failing smoke test**

Create `lib/packages/ui/src/index.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import * as Mui from '@mui/material';
import * as Icons from '@mui/icons-material';
import { mui, icons } from './index';

describe('barrel re-exports', () => {
  it('mui barrel re-exports @mui/material', () => {
    expect(mui.Button).toBe(Mui.Button);
  });

  it('icons barrel re-exports @mui/icons-material', () => {
    expect(icons.Home).toBe(Icons.Home);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm -F @aegis/ui test
```

Expected: FAIL — `mui` is not exported from `./index`.

- [ ] **Step 3: Create `src/mui/index.ts`**

```ts
export * from '@mui/material';
```

- [ ] **Step 4: Create `src/icons/index.ts`**

```ts
export * from '@mui/icons-material';
```

- [ ] **Step 5: Create `src/index.ts`**

```ts
export * as mui from './mui';
export * as icons from './icons';
```

`mui` and `icons` are namespaces so consumers use `mui.Button` without naming collisions.

- [ ] **Step 6: Run the test to verify it passes**

```bash
pnpm -F @aegis/ui test
```

Expected: PASS — both barrels resolve.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/
git commit -m "feat(ui): add MUI and icons barrel re-exports"
```

---

## Task 3: Sidebar types, test utilities, & placeholder component

**Files:**
- Create: `lib/packages/ui/components/Sidebar/types.ts`
- Create: `lib/packages/ui/components/Sidebar/test-utils.tsx`
- Create: `lib/packages/ui/components/Sidebar/Sidebar.tsx` (placeholder)
- Create: `lib/packages/ui/components/Sidebar/index.ts`
- Modify: `lib/packages/ui/src/index.ts`

**Interfaces:**
- Produces:
  - Types `MenuItem`, `SubMenuItem`, `SidebarProps` exported from `components/Sidebar/types.ts` and re-exported from both barrels.
  - `renderWithTheme(ui)` test helper that wraps renders in `ThemeProvider`.
  - `Sidebar` named export available from `@aegis/ui` (placeholder body — real impl in Tasks 4–5).

- [ ] **Step 1: Create `components/Sidebar/types.ts`**

```ts
import type { ComponentType } from 'react';

export interface SubMenuItem {
  link: string;
  title: string;
  icon: ComponentType;
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
  width?: number;
  collapsedWidth?: number;
}
```

- [ ] **Step 2: Create `components/Sidebar/test-utils.tsx`**

```tsx
import type { ReactElement } from 'react';
import { render, type RenderOptions } from '@testing-library/react';
import { ThemeProvider, createTheme } from '@mui/material/styles';

const theme = createTheme();

export function renderWithTheme(ui: ReactElement, options?: RenderOptions) {
  return render(<ThemeProvider theme={theme}>{ui}</ThemeProvider>, options);
}

export * from '@testing-library/react';
export { default as userEvent } from '@testing-library/user-event';
```

- [ ] **Step 3: Create placeholder `components/Sidebar/Sidebar.tsx`**

```tsx
// Placeholder — implementation lands in Tasks 4–5.
export function Sidebar(): null {
  return null;
}
```

- [ ] **Step 4: Create `components/Sidebar/index.ts`**

```ts
export { Sidebar } from './Sidebar';
export type { MenuItem, SubMenuItem, SidebarProps } from './types';
```

- [ ] **Step 5: Update `src/index.ts` to re-export Sidebar**

Replace `src/index.ts` contents:

```ts
export * as mui from './mui';
export * as icons from './icons';

export { Sidebar } from '../components/Sidebar';
export type { MenuItem, SubMenuItem, SidebarProps } from '../components/Sidebar';
```

- [ ] **Step 6: Verify typecheck and tests**

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected:
- `typecheck` PASS (types + stub Sidebar both compile).
- `test` PASS (the barrel smoke test from Task 2 still passes; no Sidebar tests yet).

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/
git commit -m "feat(ui): add Sidebar types, test utilities, and barrel"
```

---

## Task 4: Sidebar title bar (TDD)

**Files:**
- Create: `lib/packages/ui/components/Sidebar/Sidebar.test.tsx`
- Modify: `lib/packages/ui/components/Sidebar/Sidebar.tsx`

**Interfaces:**
- Consumes: `SidebarProps.title`, `SidebarProps.open`, `SidebarProps.onToggle`, `SidebarProps.width`, `SidebarProps.collapsedWidth`.
- Produces: a `<Sidebar>` that renders a `Drawer` with a toggle `IconButton` always visible (icon is `FormatIndentDecrease` when `open=true`, `FormatIndentIncrease` when `open=false`) on the **left**, and a `Typography` of `title` on the **right** (only when `open=true`). Clicking the icon button calls `onToggle`. Drawer width follows `open ? width : collapsedWidth`.

**Tests covered in this task:** (1) renders title when open=true, (2) hides title when open=false, (3) toggle button calls onToggle, (4) toggle icon is FormatIndentDecrease when open=true, (5) toggle icon is FormatIndentIncrease when open=false.

- [ ] **Step 1: Write the failing tests**

Create `components/Sidebar/Sidebar.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from './Sidebar';
import type { MenuItem } from './types';
import { renderWithTheme } from './test-utils';

const Icon = () => <svg data-testid="mock-icon" />;

const baseMenu: MenuItem[] = [
  { link: '/home', title: 'Home', icon: Icon },
  {
    link: '/settings',
    title: 'Settings',
    icon: Icon,
    subMenu: [{ link: '/settings/profile', title: 'Profile', icon: Icon }],
  },
];

const defaultProps = {
  title: 'My App',
  menu: baseMenu,
  open: true,
  onToggle: () => {},
};

describe('Sidebar', () => {
  it('renders title when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByText('My App')).toBeInTheDocument();
  });

  it('hides title when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.queryByText('My App')).not.toBeInTheDocument();
  });

  it('toggle button calls onToggle when clicked', async () => {
    const onToggle = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onToggle={onToggle} />);
    await userEvent.click(screen.getByLabelText('toggle sidebar'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('toggle icon is FormatIndentDecrease when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByTestId('FormatIndentDecreaseIcon')).toBeInTheDocument();
  });

  it('toggle icon is FormatIndentIncrease when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.getByTestId('FormatIndentIncreaseIcon')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the three tests to verify they fail**

```bash
pnpm -F @aegis/ui test -t "Sidebar"
```

Expected: 3 tests FAIL — the placeholder Sidebar returns `null`.

- [ ] **Step 3: Implement the title bar**

Replace `components/Sidebar/Sidebar.tsx`:

```tsx
import { Drawer, Box, Typography, IconButton, Divider } from '@mui/material';
import { FormatIndentDecrease, FormatIndentIncrease } from '@mui/icons-material';
import type { SidebarProps } from './types';

export function Sidebar({
  title,
  open,
  onToggle,
  width = 240,
  collapsedWidth = 56,
}: SidebarProps) {
  const drawerWidth = open ? width : collapsedWidth;

  return (
    <Drawer
      variant="permanent"
      data-testid="sidebar"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          boxSizing: 'border-box',
          transition: 'width 0.3s',
          overflowX: 'hidden',
        },
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', p: 1, minHeight: 56 }}>
        <IconButton onClick={onToggle} aria-label="toggle sidebar" edge="start">
          {open ? <FormatIndentDecrease /> : <FormatIndentIncrease />}
        </IconButton>
        {open && (
          <Typography variant="h6" sx={{ ml: 1 }} noWrap>
            {title}
          </Typography>
        )}
      </Box>
      <Divider />
    </Drawer>
  );
}
```

- [ ] **Step 4: Run the five tests to verify they pass**

```bash
pnpm -F @aegis/ui test -t "Sidebar"
```

Expected: 5 tests PASS.

- [ ] **Step 5: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/components/Sidebar/Sidebar.tsx lib/packages/ui/components/Sidebar/Sidebar.test.tsx
git commit -m "feat(ui): render Sidebar toggle button with conditional icon"
```

---

## Task 5: Sidebar menu list, leaf navigation, submenu toggle, collapsed mode, width props (TDD)

**Files:**
- Modify: `lib/packages/ui/components/Sidebar/Sidebar.test.tsx` (add five more tests)
- Modify: `lib/packages/ui/components/Sidebar/Sidebar.tsx`

**Interfaces:**
- Consumes: `SidebarProps.menu`, `SidebarProps.onNavigate`.
- Produces: a `<List>` of menu items. Clicking a leaf (no `subMenu`) calls `onNavigate(link)`. Clicking a parent toggles its `Collapse`d submenu and does NOT call `onNavigate`. In collapsed mode (`open=false`), menu text is hidden, icons remain inside `Tooltip`s, and the submenu block does not render. Drawer width follows `open ? width : collapsedWidth` (already wired in Task 4).

**Tests covered in this task:** (4) leaf click calls `onNavigate`, (5) parent click toggles submenu, (6) parent click does NOT call `onNavigate`, (7) collapsed mode renders only icons, (8) custom width/collapsedWidth props apply.

- [ ] **Step 1: Append five more failing tests to `Sidebar.test.tsx`**

Append to the existing `describe('Sidebar', ...)` block, before the closing `});`:

```tsx
  it('clicking a leaf menu item calls onNavigate with its link', async () => {
    const onNavigate = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onNavigate={onNavigate} />);
    await userEvent.click(screen.getByText('Home'));
    expect(onNavigate).toHaveBeenCalledWith('/home');
  });

  it('clicking a parent menu toggles its submenu open', async () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    await userEvent.click(screen.getByText('Settings'));
    expect(screen.getByText('Profile')).toBeInTheDocument();
    await userEvent.click(screen.getByText('Settings'));
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
  });

  it('clicking a parent menu does NOT call onNavigate', async () => {
    const onNavigate = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onNavigate={onNavigate} />);
    await userEvent.click(screen.getByText('Settings'));
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('collapsed mode renders only icons, hides menu text', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.queryByText('Home')).not.toBeInTheDocument();
    expect(screen.queryByText('Settings')).not.toBeInTheDocument();
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    expect(screen.getAllByTestId('mock-icon').length).toBeGreaterThan(0);
  });

  it('applies custom width and collapsedWidth without crashing', () => {
    expect(() =>
      renderWithTheme(
        <Sidebar {...defaultProps} width={300} collapsedWidth={64} />,
      ),
    ).not.toThrow();
  });
```

- [ ] **Step 2: Run the full Sidebar test suite to verify the new five fail**

```bash
pnpm -F @aegis/ui test -t "Sidebar"
```

Expected: 5 of the 8 tests FAIL (the title-bar tests from Task 4 still pass; the new menu tests fail because no menu is rendered yet).

- [ ] **Step 3: Implement the menu list, submenu, collapsed mode, and width props**

Replace `components/Sidebar/Sidebar.tsx`:

```tsx
import { useState } from 'react';
import {
  Drawer,
  Box,
  Typography,
  IconButton,
  Divider,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Collapse,
  Tooltip,
} from '@mui/material';
import { FormatIndentDecrease, FormatIndentIncrease } from '@mui/icons-material';
import type { SidebarProps, MenuItem } from './types';

export function Sidebar({
  title,
  menu,
  open,
  onToggle,
  onNavigate,
  width = 240,
  collapsedWidth = 56,
}: SidebarProps) {
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(new Set());
  const drawerWidth = open ? width : collapsedWidth;

  const toggleExpanded = (link: string) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(link)) next.delete(link);
      else next.add(link);
      return next;
    });
  };

  return (
    <Drawer
      variant="permanent"
      data-testid="sidebar"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          boxSizing: 'border-box',
          transition: 'width 0.3s',
          overflowX: 'hidden',
        },
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', p: 1, minHeight: 56 }}>
        <IconButton onClick={onToggle} aria-label="toggle sidebar" edge="start">
          {open ? <FormatIndentDecrease /> : <FormatIndentIncrease />}
        </IconButton>
        {open && (
          <Typography variant="h6" sx={{ ml: 1 }} noWrap>
            {title}
          </Typography>
        )}
      </Box>
      <Divider />
      <List>
        {menu.map((item) => (
          <SidebarMenuItem
            key={item.link}
            item={item}
            open={open}
            expanded={expandedKeys.has(item.link)}
            onToggle={() => toggleExpanded(item.link)}
            onNavigate={onNavigate}
          />
        ))}
      </List>
    </Drawer>
  );
}

interface SidebarMenuItemProps {
  item: MenuItem;
  open: boolean;
  expanded: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
}

function SidebarMenuItem({
  item,
  open,
  expanded,
  onToggle,
  onNavigate,
}: SidebarMenuItemProps) {
  const hasSubmenu = !!item.subMenu?.length;
  const Icon = item.icon;

  const handleClick = () => {
    if (hasSubmenu) onToggle();
    else onNavigate?.(item.link);
  };

  const button = (
    <ListItemButton onClick={handleClick}>
      <ListItemIcon>
        <Icon />
      </ListItemIcon>
      {open && <ListItemText primary={item.title} />}
    </ListItemButton>
  );

  return (
    <>
      <ListItem disablePadding>
        {open ? (
          button
        ) : (
          <Tooltip title={item.title} placement="right">
            <span>{button}</span>
          </Tooltip>
        )}
      </ListItem>
      {hasSubmenu && open && (
        <Collapse in={expanded} unmountOnExit>
          <List disablePadding>
            {item.subMenu!.map((sub) => {
              const SubIcon = sub.icon;
              return (
                <ListItem key={sub.link} disablePadding sx={{ pl: 2 }}>
                  <ListItemButton onClick={() => onNavigate?.(sub.link)}>
                    <ListItemIcon>
                      <SubIcon />
                    </ListItemIcon>
                    <ListItemText primary={sub.title} />
                  </ListItemButton>
                </ListItem>
              );
            })}
          </List>
        </Collapse>
      )}
    </>
  );
}
```

- [ ] **Step 4: Run the full test suite**

```bash
pnpm -F @aegis/ui test
```

Expected: all tests PASS — 2 barrel tests + 8 Sidebar tests = 10 PASS.

- [ ] **Step 5: Run typecheck**

```bash
pnpm -F @aegis/ui typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/components/Sidebar/Sidebar.tsx lib/packages/ui/components/Sidebar/Sidebar.test.tsx
git commit -m "feat(ui): render Sidebar menu, submenu, collapsed mode, and width props"
```

---

## Task 6: Desktop app integration

**Files:**
- Modify: `apps/desktop/aegis-desktop/package.json`
- Modify (smoke test, then revert): `apps/desktop/aegis-desktop/src/App.tsx`
- Modify (only if resolution fails): `apps/desktop/aegis-desktop/tsconfig.json` and/or `apps/desktop/aegis-desktop/vite.config.ts`

**Interfaces:**
- Produces: `import { Sidebar } from '@aegis/ui'` resolves inside the desktop app.

- [ ] **Step 1: Add workspace dependency**

From repo root:

```bash
pnpm --filter aegis-desktop add '@aegis/ui@workspace:*'
```

Expected: pnpm updates `apps/desktop/aegis-desktop/package.json` to include `"@aegis/ui": "workspace:*"` and refreshes `pnpm-lock.yaml`.

- [ ] **Step 2: Smoke-import Sidebar from the desktop app**

Temporarily replace `apps/desktop/aegis-desktop/src/App.tsx` with:

```tsx
import { useState } from "react";
import { Sidebar } from "@aegis/ui";

function App() {
  const [open, setOpen] = useState(true);
  return (
    <div style={{ display: "flex" }}>
      <Sidebar
        title="Aegis"
        menu={[{ link: "/", title: "Home", icon: () => <span>🏠</span> }]}
        open={open}
        onToggle={() => setOpen((o) => !o)}
      />
      <main style={{ flex: 1, padding: 16 }}>
        <h1>Aegis</h1>
      </main>
    </div>
  );
}

export default App;
```

- [ ] **Step 3: Verify desktop typecheck and build**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
```

Expected: both PASS.

**If typecheck fails** with a module-not-found error for `@aegis/ui`, the desktop app's TS scope does not include the workspace source. Fix in `apps/desktop/aegis-desktop/tsconfig.json` by adding to `compilerOptions`:

```json
"baseUrl": ".",
"paths": {
  "@aegis/ui": ["../../lib/packages/ui/src/index.ts"],
  "@aegis/ui/*": ["../../lib/packages/ui/src/*", "../../lib/packages/ui/components/*"]
}
```

**If `vite build` fails** with a similar module-not-found error, add to `apps/desktop/aegis-desktop/vite.config.ts`:

```ts
resolve: {
  alias: {
    '@aegis/ui': fileURLToPath(new URL('../../lib/packages/ui/src/index.ts', import.meta.url)),
  },
},
```

(You will also need `import { fileURLToPath } from 'node:url';` at the top of the vite config.)

- [ ] **Step 4: Revert the smoke test**

Restore `apps/desktop/aegis-desktop/src/App.tsx` to its original contents (the file as it exists in HEAD before this task began — the version from the initial repo commit with `useState`/`invoke("greet")`).

- [ ] **Step 5: Final verification**

```bash
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
```

Expected: all four commands exit 0.

- [ ] **Step 6: Commit**

If only the dep was added:

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/package.json pnpm-lock.yaml
git commit -m "feat(desktop): depend on @aegis/ui workspace package"
```

If TS/Vite config also had to change:

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/package.json apps/desktop/aegis-desktop/tsconfig.json apps/desktop/aegis-desktop/vite.config.ts pnpm-lock.yaml
git commit -m "feat(desktop): depend on @aegis/ui workspace package"
```

---

## Done Criteria

- [ ] All 6 tasks committed on the current branch.
- [ ] `pnpm -F @aegis/ui typecheck` PASS.
- [ ] `pnpm -F @aegis/ui test` PASS — 2 barrel tests + 8 Sidebar tests = 10 tests, all green.
- [ ] `pnpm --filter aegis-desktop typecheck` PASS.
- [ ] `pnpm --filter aegis-desktop build` PASS.
- [ ] Desktop app's `App.tsx` is back to its original contents (no smoke-test residue).