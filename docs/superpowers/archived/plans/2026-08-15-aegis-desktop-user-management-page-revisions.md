# User Management Page Revisions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply two revisions to the already-shipped User management page in the Aegis desktop app: (1) move the `Management` sidebar entry to sit between `Projects` and `Settings`, and (2) replace the read-only role `Chip` in the user table with a `Select` dropdown (admin/general options) that mutates the user's role through the existing `useUpdateUser` hook.

**Architecture:** The sidebar change is a single-line array reorder in `Layout.tsx`. The role dropdown replaces the `Chip` cell in `UserTable` (presentational) with a `Select` wrapped in a `<Tooltip>`, mirroring the existing self-disable + mutation-loading disable pattern on the `Switch`. `UserList` (orchestrator) gains one `handleRoleChange` callback. `useUpdateUser` is reused unchanged — its `UpdateUserBody` already accepts `{ role }`. One new i18n key (`user.cannotChangeOwnRole`) is added to both locales. Tests update the existing chip-style assertion and add 5 new cases for the Select behavior plus a role-mutation orchestrator case and a role-body data-layer case.

**Tech Stack:** Tauri desktop app, React 19 + TypeScript, TanStack Router, TanStack Query v5, MUI v9 (re-exported through `@aegis/ui/mui`), Vitest + Testing Library. i18n via `@aegis/ui/i18n`.

**Spec:** [2026-08-15-aegis-desktop-user-management-page-revisions-design.md](../specs/2026-08-15-aegis-desktop-user-management-page-revisions-design.md) — read it first; tasks below reference its sections.

## Global Constraints

These constraints apply to every task. If a task contradicts a constraint, the constraint wins.

- **i18n keys must be in lock-step**: every key added to `en.ts` must be added to `zhCN.ts` in the same commit. The typecheck fails otherwise (`zhCN.ts:3` uses `satisfies Record<keyof typeof en, string>`).
- **Test mock pattern**: every test file that uses Tauri commands must call `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` at the top (hoisted per-file). Use `mockCommands({...})` from `src/test/tauri-mock.ts` and `httpError(status, code, message)` for failures.
- **File naming**: TypeScript files use PascalCase for components/pages (`UserList.tsx`, `UserTable.tsx`). Test files mirror page names (`user-list.test.tsx`, `user-table.test.tsx`).
- **No new directory under `src/pages/`** — page files stay flat like `ProjectList.tsx` and `ProjectTable.tsx` (per commit `01b0483 refactor(desktop): rename page files to UpperCamelCase`).
- **No direct `invoke` calls from pages** — pages import hooks from `../data` (barrel); the data layer imports `api` from `../api`.
- **Existing patterns to follow**:
  - Switch control: `<FormControlLabel control={<Switch ... />} label="" sx={{ ml: 0 }} />` (see `ProjectDrawer.tsx:238-247`).
  - Tooltip + disabled control: `<Tooltip><span>{control}</span></Tooltip>` (see `ProjectTable.tsx:131-143`).
  - Loading/empty/error states: `<CircularProgress />` for spinner, `<Typography color="textSecondary">` for empty, `<Alert severity="error">` for errors (see `ProjectTable.tsx:56-72, 168-172`).
  - Mutation hook: mirror `useUpdateUser` in `src/data/user.ts` — `useUpdateUser` already accepts `{ role }` in `UpdateUserBody`.
- **MUI Select selectors in tests**: MUI v9 Select renders `<div role="combobox">` with the MenuItem label as visible text and a hidden `<input>` whose `value` is the MenuItem's `value` prop. Query the combobox by `getAllByRole("combobox")` and filter by text content (e.g. `combobox.textContent === "Admin"`); assert disabled state via `aria-disabled`. Do NOT use `getByDisplayValue("Admin")` — the hidden input's value is `"admin"`, not `"Admin"`.
- **No file may exceed ~250 lines** by the end of the work. If a component grows beyond that, split it.
- **Commit messages**: `<scope>(<area>): <verb> <description>` matching recent history (e.g. `feat(desktop): ...`, `feat(ui): ...`). Use `feat` for new behavior, `fix` for bugs, `refactor` for restructuring, `test` for tests-only.

---

## Task 1: Add `user.cannotChangeOwnRole` i18n key

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: existing `en` and `zhCN` exports
- Produces: one new key (`user.cannotChangeOwnRole`) in each locale, in lock-step

- [ ] **Step 1: Add the key to `en.ts`**

Edit `lib/packages/ui/src/i18n/locales/en.ts`. Add this line immediately after the existing `'user.cannotDeactivateSelf': 'You cannot deactivate yourself',` (currently line 111):

```ts
  'user.cannotChangeOwnRole': 'You cannot change your own role',
```

- [ ] **Step 2: Add the matching key to `zhCN.ts`**

Edit `lib/packages/ui/src/i18n/locales/zhCN.ts`. Add the same key immediately after the matching `'user.cannotDeactivateSelf'` line (currently line 109):

```ts
  'user.cannotChangeOwnRole': '无法修改自己的角色',
```

- [ ] **Step 3: Run typecheck to confirm lock-step**

Run from repo root:

```bash
pnpm --filter @aegis/ui exec tsc --noEmit
```

Expected: PASS. If it fails with a `Record<keyof typeof en, string>` mismatch, one of the locales is missing the key.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add user.cannotChangeOwnRole i18n key"
```

---

## Task 2: Replace role Chip with Select in UserTable

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`

**Interfaces:**
- Consumes: `Role` type from `apps/desktop/aegis-desktop/src/api/types.ts` (line 10: `export type Role = "root" | "admin" | "general";`)
- Produces:
  - `UserTableProps` extended with `onRoleChange: (code: string, nextRole: Role) => void`
  - `UserTable` renders `<Select>` (admin/general MenuItems) wrapped in a `<Tooltip>` with the same `disabled = isSelf || mutationLoading` rule that gates the Switch

- [ ] **Step 1: Write the failing tests**

Edit `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`. Replace the existing `'UserTable — rows'` describe block (lines 103-125) AND add a new `'UserTable — role Select'` describe block AFTER it. The final state of the `UserTable — rows` describe block should be:

```ts
describe("UserTable — rows", () => {
  it("renders one row per user with code, name, role select, and switch", () => {
    renderTable({ rows: [adminUser, generalUser, adminUser2] });
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("Alice")).toBeInTheDocument();
    // Each non-root user renders a Select with their current role as
    // the visible label. Two admins + one general.
    const selects = screen.getAllByRole("combobox");
    expect(selects).toHaveLength(3);
    expect(selects[0]).toHaveTextContent("Admin");   // alice
    expect(selects[1]).toHaveTextContent("General"); // bob
    expect(selects[2]).toHaveTextContent("Admin");   // carol
    expect(screen.getByText("bob")).toBeInTheDocument();
    expect(screen.getByText("carol")).toBeInTheDocument();
  });

  it("reflects active=true as a checked Switch", () => {
    renderTable({ rows: [adminUser] });
    const sw = getSwitches()[0];
    expect(sw.checked).toBe(true);
  });

  it("reflects active=false as an unchecked Switch", () => {
    renderTable({ rows: [generalUser] });
    const sw = getSwitches()[0];
    expect(sw.checked).toBe(false);
  });
});
```

Then ADD a new describe block after `'UserTable — rows'` and before the existing `'UserTable — self-disable'` block:

```ts
describe("UserTable — role Select", () => {
  /** Find the combobox whose visible text equals `label`. */
  function selectWithLabel(label: string): HTMLElement {
    const selects = screen.getAllByRole("combobox");
    const match = selects.find((s) => s.textContent === label);
    if (!match) throw new Error(`Select with label "${label}" not found`);
    return match;
  }

  it("Select on the self row is disabled", () => {
    renderTable({ rows: [adminUser], selfCode: "alice" });
    expect(selectWithLabel("Admin")).toHaveAttribute("aria-disabled", "true");
  });

  it("Select on non-self rows is enabled", () => {
    renderTable({ rows: [adminUser, generalUser], selfCode: "alice" });
    expect(selectWithLabel("Admin")).toHaveAttribute("aria-disabled", "true");
    expect(selectWithLabel("General")).toHaveAttribute("aria-disabled", "false");
  });

  it("dropdown options are admin and general only (no root)", async () => {
    renderTable({ rows: [adminUser] });
    await userEvent.click(selectWithLabel("Admin"));
    expect(screen.getByRole("option", { name: "Admin" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "General" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Root" })).not.toBeInTheDocument();
  });

  it("calls onRoleChange when a different role is picked", async () => {
    const onRoleChange = vi.fn();
    renderTable({ rows: [generalUser], onRoleChange });
    await userEvent.click(selectWithLabel("General"));
    await userEvent.click(screen.getByRole("option", { name: "Admin" }));
    expect(onRoleChange).toHaveBeenCalledWith("bob", "admin");
  });

  it("disables every Select while mutationLoading is true", () => {
    renderTable({ rows: [adminUser, generalUser], mutationLoading: true });
    const selects = screen.getAllByRole("combobox");
    expect(selects.every((s) => s.getAttribute("aria-disabled") === "true")).toBe(true);
  });
});
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run from `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run src/test/pages/user-table.test.tsx
```

Expected: FAIL. The `'UserTable — rows'` first case fails because there are still 0 combobox roles (no Select rendered yet) and the existing chip-style assertion (`screen.getAllByText("Admin").length`) may also break if the test order changes. The 5 new cases all fail with `Select with label "Admin" not found` or equivalent.

- [ ] **Step 3: Update `UserTable.tsx` — extend props + import Select/MenuItem**

Edit `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`. First, replace the import list (lines 1-18):

```tsx
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  FormControlLabel,
  MenuItem,
  Paper,
  Select,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { ApiError, Role, UserView } from "../api";
import { errorMessage } from "../api/error";
```

(`Chip` is removed from the imports; `Select` and `MenuItem` added; `Role` added to the api type import.)

Then extend `UserTableProps` (currently lines 24-32):

```tsx
export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRoleChange: (code: string, nextRole: Role) => void;
  onRetry: () => void;
}
```

Then extend the destructuring in the function signature (currently lines 40-48):

```tsx
export function UserTable({
  rows,
  loading,
  mutationLoading,
  error,
  selfCode,
  onToggle,
  onRoleChange,
  onRetry,
}: UserTableProps) {
```

- [ ] **Step 4: Replace the role cell with the Select**

In the same file, replace the role `<TableCell>` block (currently lines 92-98):

```tsx
<TableCell>
  <Chip
    variant="outlined"
    size="small"
    label={t(`user.role.${row.role}`)}
  />
</TableCell>
```

with:

```tsx
<TableCell>
  <Tooltip title={isSelf ? t("user.cannotChangeOwnRole") : ""}>
    <span>
      <Select
        size="small"
        value={row.role}
        disabled={disabled}
        onChange={(e) =>
          onRoleChange(row.code, e.target.value as Role)
        }
        sx={{ minWidth: 120 }}
      >
        <MenuItem value="admin">{t("user.role.admin")}</MenuItem>
        <MenuItem value="general">{t("user.role.general")}</MenuItem>
      </Select>
    </span>
  </Tooltip>
</TableCell>
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
pnpm exec vitest run src/test/pages/user-table.test.tsx
```

Expected: PASS for all `UserTable — rows`, `UserTable — self-disable`, `UserTable — mutation loading`, `UserTable — toggle interaction`, and the new `UserTable — role Select` cases.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserTable.tsx apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx
git commit -m "feat(desktop): replace role Chip with Select in UserTable"
```

---

## Task 3: Wire `handleRoleChange` in UserList and add role-mutation tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/UserList.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`

**Interfaces:**
- Consumes: existing `useUpdateUser` (returns TanStack Query mutation with `mutate({ code, body })` where `body: UpdateUserBody = { active?, role?, ... }`)
- Produces:
  - `UserList` passes `onRoleChange={handleRoleChange}` to `UserTable`
  - `handleRoleChange = useCallback((code, nextRole) => updateUser.mutate({ code, body: { role: nextRole } }), [updateUser])`

- [ ] **Step 1: Write the failing orchestrator test**

Edit `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`. Add a new describe block AFTER the existing `'UserListPage — toggle calls update_user'` describe block (line 118), keeping it before the `'UserListPage — self-disable'` block (line 120):

```ts
describe("UserListPage — role change", () => {
  /** Find the Select on a row whose visible text equals `label`. */
  function selectWithLabel(label: string): HTMLElement {
    const selects = screen.getAllByRole("combobox");
    const match = selects.find((s) => s.textContent === label);
    if (!match) throw new Error(`Select with label "${label}" not found`);
    return match;
  }

  it("calls update_user with { code, body: { role } } when a role is picked", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("bob");
    await userEvent.click(selectWithLabel("General"));
    await userEvent.click(screen.getByRole("option", { name: "Admin" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { role: "admin" },
      });
    });
  });
});
```

Also update the existing `'UserListPage — self-disable'` describe block (lines 120-128) to assert the Select is disabled on alice's row in addition to the Switch:

```ts
describe("UserListPage — self-disable", () => {
  it("disables both the Switch and the Select on the current user's row", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const switches = getSwitches();
    expect(switches[0].disabled).toBe(true); // alice Switch

    const selects = screen.getAllByRole("combobox");
    const aliceSelect = selects.find((s) => s.textContent === "Admin");
    expect(aliceSelect).toBeDefined();
    expect(aliceSelect).toHaveAttribute("aria-disabled", "true");
  });
});
```

- [ ] **Step 2: Write the failing data-layer test**

Edit `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`. Add a second harness and a new describe block AFTER the existing `'useUpdateUser'` describe block (line 280):

```ts
function UpdateUserRoleHarness() {
  const m = useUpdateUser();
  return (
    <button
      onClick={() => {
        m.mutate({ code: "bob", body: { role: "admin" } });
      }}
    >
      promote
    </button>
  );
}

describe("useUpdateUser — role body", () => {
  it("invokes api.update_user with body: { role }", async () => {
    mockCommands({ update_user: () => userView });
    renderWithQueryClient(<UpdateUserRoleHarness />);
    await userEvent.click(screen.getByRole("button", { name: "promote" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { role: "admin" },
      });
    });
  });
});
```

- [ ] **Step 3: Run the new tests to verify they fail**

Run from `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run src/test/pages/user-list.test.tsx src/test/data/user.test.tsx
```

Expected: FAIL. The role-change orchestrator test fails because `UserList` doesn't pass `onRoleChange` (the Select is rendered but clicking does nothing, so `invoke` is never called with `{ role: ... }`). The data-layer test fails because `UpdateUserRoleHarness` references the same `useUpdateUser` that already exists — actually it should PASS for the invoke shape (the existing hook already handles `{ role }` in its body). Verify the data-layer test passes; the orchestrator test is the one that should fail. If the data-layer test fails, re-check the harness matches the existing `useUpdateUser` signature in `src/data/user.ts`.

- [ ] **Step 4: Add `handleRoleChange` to `UserList.tsx`**

Edit `apps/desktop/aegis-desktop/src/pages/UserList.tsx`. The `Role` type import on line 6 (`import type { UserView } from "../api";`) needs to include `Role`:

```tsx
import type { Role, UserView } from "../api";
```

Then add a `handleRoleChange` callback immediately after the existing `handleToggle` block (currently lines 30-35):

```tsx
  const handleRoleChange = useCallback(
    (code: string, nextRole: Role) => {
      updateUser.mutate({ code, body: { role: nextRole } });
    },
    [updateUser],
  );
```

Then update the `<UserTable>` JSX (currently lines 42-50) to pass the new prop:

```tsx
      <UserTable
        rows={rows}
        loading={users.isLoading}
        mutationLoading={updateUser.isPending}
        error={users.error ?? updateUser.error}
        selfCode={selfCode}
        onToggle={handleToggle}
        onRoleChange={handleRoleChange}
        onRetry={users.refetch}
      />
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
pnpm exec vitest run src/test/pages/user-list.test.tsx src/test/data/user.test.tsx
```

Expected: PASS. All `UserListPage — root filter`, `UserListPage — role gate`, `UserListPage — toggle calls update_user`, `UserListPage — self-disable`, `UserListPage — error surfaces`, and the new `UserListPage — role change` cases pass. All `useUpdateUser` cases plus the new `useUpdateUser — role body` case pass.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserList.tsx apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx apps/desktop/aegis-desktop/src/test/data/user.test.tsx
git commit -m "feat(desktop): wire role-change handler in UserList"
```

---

## Task 4: Reorder sidebar + full-suite verification

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/Layout.tsx`

**Interfaces:**
- Consumes: existing `MenuItem[]` array
- Produces: the `Management` entry moves from index 3 (after `Settings`) to index 2 (before `Settings`)

- [ ] **Step 1: Reorder the menu array**

Edit `apps/desktop/aegis-desktop/src/pages/Layout.tsx`. The `baseMenu` array (currently lines 37-41) is:

```tsx
  const baseMenu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];
```

The `managementEntry` block (lines 43-54) is then conditionally appended via the spread at line 56-58. Change the spread at line 56-58 from:

```tsx
  const menu: MenuItem[] = canManage
    ? [...baseMenu, managementEntry]
    : baseMenu;
```

to:

```tsx
  const menu: MenuItem[] = canManage
    ? [
        ...baseMenu.slice(0, 2),     // Home, Projects
        managementEntry,             // Management (submenu: Users)
        ...baseMenu.slice(2),        // Settings
      ]
    : baseMenu;
```

(Alternative: refactor the array so `Settings` is moved up and `managementEntry` is conditionally inserted via `slice` — both shapes produce the same DOM order. Use the version above to minimize the diff.)

Verify the resulting order when `canManage === true` is `[Home, Projects, Management (Users), Settings]` and when `canManage === false` is `[Home, Projects, Settings]` (unchanged).

- [ ] **Step 2: Run the full desktop test suite**

From `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run
```

Expected: PASS for all suites. Pay particular attention to:
- `src/test/routes/users.test.tsx` — the "shows the Management entry for an admin", "shows the Management entry for a root user", "hides the Management entry for a general user", "expands the Users submenu when Management is clicked", and "navigates from /settings to /users when Users submenu is clicked" cases all still pass after the reorder.
- `src/test/pages/user-table.test.tsx` — all rows + Select cases pass.
- `src/test/pages/user-list.test.tsx` — root filter, role gate, toggle, role change, self-disable, error surfaces pass.
- `src/test/data/user.test.tsx` — all hooks including the new role-body case pass.

If any test fails, fix the root cause (not the test) and re-run.

- [ ] **Step 3: Run the full repo verification (typecheck + test for the UI package)**

From repo root:

```bash
pnpm -r exec tsc --noEmit
pnpm --filter @aegis/ui exec vitest run
pnpm --filter aegis-desktop exec vitest run
```

Expected: PASS for all three. The typecheck confirms i18n lock-step and the new `onRoleChange` prop typechecks across `UserList` → `UserTable` → its test files.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/Layout.tsx
git commit -m "refactor(desktop): move Management menu before Settings in sidebar"
```

---

## Out of scope (deferred, per spec)

- Confirmation dialog before promoting / demoting. Same YAGNI call as the active Switch.
- A dedicated "change role" hook. `useUpdateUser` covers it.
- Allowing admins to assign / revoke the `root` role from the page. The dropdown only offers `admin` / `general`; root assignment is server-bootstrap only.
- Optimistic updates on the Select. Server is the source of truth (same as the Switch).
- Sorting, filtering, or any other UX beyond this revision.
