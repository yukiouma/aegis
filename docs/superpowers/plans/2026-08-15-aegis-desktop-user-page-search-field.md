# User Page Search Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `<Typography variant="h4">Users</Typography>` heading at the top of the User management page (`/_layout/users`) with a search TextField (leading `Search` icon + placeholder) that filters the visible rows by case-insensitive substring match on `name` OR `code`.

**Architecture:** Extract a thin `UserFilterBar` presentational component (mirrors the existing [ProjectFilterBar](apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx) pattern). `UserList` adds a `useState("")` query, passes it down, and filters `rows` further by trimmed lowercase substring on `code` OR `name` via `useMemo`. `UserTable` gains one optional `emptyMessage` prop that overrides the default `t("user.empty")` when the filtered list is empty AND a search is active. All filtering is client-side — the `list_users` endpoint stays unchanged.

**Tech Stack:** Tauri desktop app, React 19 + TypeScript, TanStack Router, TanStack Query v5, MUI v9 (re-exported through `@aegis/ui/mui` and `@aegis/ui/icons`), Vitest + Testing Library. i18n via `@aegis/ui/i18n`.

**Spec:** [2026-08-15-aegis-desktop-user-page-search-field-design.md](../specs/2026-08-15-aegis-desktop-user-page-search-field-design.md) — read it first; tasks below reference its sections.

## Global Constraints

These constraints apply to every task. If a task contradicts a constraint, the constraint wins.

- **i18n keys must be in lock-step**: every key added to `en.ts` must be added to `zhCN.ts` in the same commit. The typecheck fails otherwise (`zhCN.ts:3` uses `satisfies Record<keyof typeof en, string>`).
- **Test mock pattern**: every test file that uses Tauri commands must call `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` at the top (hoisted per-file). Use `mockCommands({...})` from `src/test/tauri-mock.ts` and `httpError(status, code, message)` for failures.
- **File naming**: TypeScript files use PascalCase for components/pages (`UserList.tsx`, `UserTable.tsx`, `UserFilterBar.tsx`). Test files mirror page names.
- **No new directory under `src/pages/`** — page files stay flat like `UserList.tsx`, `UserTable.tsx`, `UserFilterBar.tsx`.
- **No direct `invoke` calls from pages** — pages import hooks from `../data` (barrel); the data layer imports `api` from `../api`.
- **Existing patterns to follow**:
  - Controlled filter bar: `<TextField ... value={query} onChange={(e) => onQueryChange(e.target.value)} />` (see `ProjectFilterBar.tsx:32-38`).
  - MUI v9 adornment slot: `slotProps={{ input: { startAdornment: ... } }}` (matches `ProjectDrawer.tsx:134` `slotProps={{ paper: ... }}` pattern).
  - Loading/empty/error states: `<CircularProgress />` for spinner, `<Typography color="textSecondary">` for empty, `<Alert severity="error">` for errors (see `ProjectTable.tsx:56-72, 168-172`).
  - Icon imports: `import { Search as SearchIcon } from "@aegis/ui/icons";` — `@aegis/ui/icons` re-exports everything from `@mui/icons-material`.
- **No file may exceed ~250 lines** by the end of the work. If a component grows beyond that, split it.
- **Commit messages**: `<scope>(<area>): <verb> <description>` matching recent history. Use `feat` for new behavior, `fix` for bugs, `refactor` for restructuring, `test` for tests-only.

---

## Task 1: Update i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: existing `en` and `zhCN` exports
- Produces: in lock-step — `user.heading` removed from both files, `user.search.placeholder` and `user.noMatches` added to both files

- [ ] **Step 1: Remove `user.heading` and add `user.search.placeholder` + `user.noMatches` to `en.ts`**

Edit `lib/packages/ui/src/i18n/locales/en.ts`. Find the line `'user.heading': 'Users',` (currently line 106) and DELETE it.

Add these two lines immediately after `'user.empty': 'No users yet',` (currently line 107). The final `user.*` block should look like:

```ts
  'user.empty': 'No users yet',
  'user.noMatches': 'No matching users',
  'user.loadFailed': 'Failed to load users: {message}',
  'user.active': 'Active',
  'user.inactive': 'Inactive',
  'user.cannotDeactivateSelf': 'You cannot deactivate yourself',
  'user.cannotChangeOwnRole': 'You cannot change your own role',
  'user.field.code': 'Code',
  'user.field.name': 'Name',
  'user.field.role': 'Role',
  'user.field.active': 'Active',
  'user.role.root': 'Root',
  'user.role.admin': 'Admin',
  'user.role.general': 'General',
  'user.search.placeholder': 'Search by name or code',
} as const;
```

- [ ] **Step 2: Mirror the changes in `zhCN.ts`**

Edit `lib/packages/ui/src/i18n/locales/zhCN.ts`. Find `'user.heading': '用户',` (currently line 104) and DELETE it.

Add these two lines immediately after `'user.empty': '暂无用户',` (currently line 105). The final `user.*` block should look like:

```ts
  'user.empty': '暂无用户',
  'user.noMatches': '无匹配用户',
  'user.loadFailed': '加载用户失败：{message}',
  'user.active': '已启用',
  'user.inactive': '未启用',
  'user.cannotDeactivateSelf': '无法停用自己的账号',
  'user.cannotChangeOwnRole': '无法修改自己的角色',
  'user.field.code': '账号',
  'user.field.name': '姓名',
  'user.field.role': '角色',
  'user.field.active': '已启用',
  'user.role.root': 'Root',
  'user.role.admin': '管理员',
  'user.role.general': '普通用户',
  'user.search.placeholder': '按姓名或账号搜索',
} satisfies Record<keyof typeof en, string>;
```

- [ ] **Step 3: Run typecheck to confirm lock-step**

From repo root:

```bash
pnpm --filter @aegis/ui exec tsc --noEmit
```

Expected: PASS. The `satisfies Record<keyof typeof en, string>` check enforces that both files have the exact same key set. If it fails, one of the locales is missing a key — match counts.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add user.search.placeholder + user.noMatches, drop user.heading"
```

---

## Task 2: Add `emptyMessage` prop to `UserTable`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`

**Interfaces:**
- Consumes: existing `UserTableProps`
- Produces: `UserTableProps.emptyMessage?: string` — when provided, the empty-state Typography renders it instead of `t("user.empty")`

- [ ] **Step 1: Write the failing test**

Edit `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`. Add a new `it` block inside the existing `'UserTable — rendering states'` describe block, after the existing empty-state test (around line 81):

```ts
  it("renders emptyMessage when provided and rows is empty", () => {
    renderTable({ rows: [], emptyMessage: "Nothing matched" });
    expect(screen.getByText("Nothing matched")).toBeInTheDocument();
    expect(screen.queryByText(/no users yet/i)).not.toBeInTheDocument();
  });
```

- [ ] **Step 2: Run the test to verify it fails**

From `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run src/test/pages/user-table.test.tsx
```

Expected: FAIL with a TypeScript-like error: `emptyMessage` is not assignable to `UserTableProps` (the `renderTable` helper types via `Partial<React.ComponentProps<typeof UserTable>>`). This is acceptable — the failure indicates the prop doesn't exist yet.

- [ ] **Step 3: Add the `emptyMessage` prop to `UserTable.tsx`**

Edit `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`. Extend the `UserTableProps` interface (currently lines 24-32):

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
  emptyMessage?: string;
}
```

Extend the function signature destructuring (currently lines 40-49):

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
  emptyMessage,
}: UserTableProps) {
```

Replace the empty-state `<Typography>` block (currently lines 130-134):

```tsx
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{t("user.empty")}</Typography>
          </Box>
        )}
```

with:

```tsx
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">
              {emptyMessage ?? t("user.empty")}
            </Typography>
          </Box>
        )}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
pnpm exec vitest run src/test/pages/user-table.test.tsx
```

Expected: PASS for all 17 tests (the 16 existing + the 1 new).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserTable.tsx apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx
git commit -m "feat(desktop): add optional emptyMessage prop to UserTable"
```

---

## Task 3: Create `UserFilterBar` component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/UserFilterBar.tsx`

**Interfaces:**
- Consumes: `useI18n` from `@aegis/ui/i18n`, MUI components `InputAdornment` + `TextField` from `@aegis/ui/mui`, `Search` icon from `@aegis/ui/icons`
- Produces: `UserFilterBar({ query, onQueryChange })` — controlled TextField with leading `Search` icon and `user.search.placeholder` placeholder

- [ ] **Step 1: Create `UserFilterBar.tsx`**

Create `apps/desktop/aegis-desktop/src/pages/UserFilterBar.tsx` with this exact content:

```tsx
import { InputAdornment, TextField } from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

export interface UserFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
}

/**
 * Search field for the User management page. Pure controlled
 * component — `UserList` owns the query state. The leading Search
 * icon signals purpose; the placeholder hints at the match
 * semantics (name or code, case-insensitive).
 */
export function UserFilterBar({
  query,
  onQueryChange,
}: UserFilterBarProps) {
  const { t } = useI18n();
  return (
    <TextField
      size="small"
      placeholder={t("user.search.placeholder")}
      value={query}
      onChange={(e) => onQueryChange(e.target.value)}
      slotProps={{
        input: {
          startAdornment: (
            <InputAdornment position="start">
              <SearchIcon fontSize="small" />
            </InputAdornment>
          ),
        },
      }}
      sx={{ minWidth: 320 }}
    />
  );
}
```

- [ ] **Step 2: Run the desktop typecheck**

From `apps/desktop/aegis-desktop`:

```bash
pnpm exec tsc --noEmit
```

Expected: PASS. The new file is type-correct against MUI v9 (`@aegis/ui/mui` re-exports `@mui/material`, which has `slotProps.input.startAdornment`; `@aegis/ui/icons` re-exports `@mui/icons-material`).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserFilterBar.tsx
git commit -m "feat(desktop): add UserFilterBar search component"
```

---

## Task 4: Wire search state into `UserList` + add search tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/UserList.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`

**Interfaces:**
- Consumes: `UserFilterBar` (just created) with `{ query, onQueryChange }`; `UserTable`'s new `emptyMessage?` prop
- Produces: `UserListPage` adds `const [search, setSearch] = useState("")`, renders `<UserFilterBar />` in place of the heading, filters `rows` via `useMemo`, branches `emptyMessage` based on whether `trimmedQuery` is non-empty

- [ ] **Step 1: Write the failing tests**

Edit `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`. Add a new describe block at the END of the file:

```ts
describe("UserListPage — search", () => {
  it("renders a TextField with a Search icon in place of the heading", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    expect(input).toBeInTheDocument();
    // The Search icon is wired via InputAdornment with position="start".
    // MUI renders an svg inside the adornment — assert presence loosely
    // (no role="img" on MUI icons in v9).
    expect(input.parentElement?.querySelector("svg")).not.toBeNull();
  });

  it("filters rows by code substring (case-insensitive)", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "BO");
    // Only bob (code "bob") remains — alice (code "alice") does not
    // contain "bo".
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("filters rows by name substring", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "bob");
    // "bob" matches generalUser's code AND name.
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("shows 'no matches' empty state when query yields zero rows", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "xyz");
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.queryByText("bob")).not.toBeInTheDocument();
    expect(screen.getByText(/no matching users/i)).toBeInTheDocument();
  });

  it("clearing the query restores the full list", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "bob");
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    await userEvent.clear(input);
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the new tests to verify they fail**

From `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run src/test/pages/user-list.test.tsx
```

Expected: FAIL for all 5 new cases. The "renders a TextField..." case fails because `screen.getByPlaceholderText(...)` returns nothing (no TextField in `UserList`). The other 4 cases fail because the test never gets to interact with the TextField (it doesn't exist) — likely `TestingLibraryElementError: Unable to find an element with the placeholder`.

- [ ] **Step 3: Update `UserList.tsx` — add search state + filter + render `UserFilterBar`**

Edit `apps/desktop/aegis-desktop/src/pages/UserList.tsx`. Update the import line (currently line 1):

```tsx
import { useCallback, useMemo, useState } from "react";
```

(`useState` is added; `useCallback` and `useMemo` were already imported.)

Add the `UserFilterBar` import (currently line 7):

```tsx
import { UserFilterBar } from "./UserFilterBar";
import { UserTable } from "./UserTable";
```

(`UserTable` import stays where it is; `UserFilterBar` is added just above it.)

In the `UserListPage` function body, add the search state and update the `rows` memo. The current body (lines 14-37) is:

```tsx
export function UserListPage() {
  const { t } = useI18n();
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";
  const selfCode = currentUser.data?.code ?? null;

  // Root users are never shown. Filter is single-pass over the list.
  const rows = useMemo<UserView[]>(
    () => (users.data ?? []).filter((u) => u.role !== "root"),
    [users.data],
  );

  const handleToggle = useCallback(
    (code: string, nextActive: boolean) => {
      updateUser.mutate({ code, body: { active: nextActive } });
    },
    [updateUser],
  );

  const handleRoleChange = useCallback(
    (code: string, nextRole: Role) => {
      updateUser.mutate({ code, body: { role: nextRole } });
    },
    [updateUser],
  );

  if (!canManage) return null;
```

Replace the lines from `const users = useListUsers();` through the closing of the `rows` `useMemo` with:

```tsx
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();
  const [search, setSearch] = useState("");

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";
  const selfCode = currentUser.data?.code ?? null;

  // Trim + lowercase once so the memo dependency is a stable string
  // and trailing-whitespace-only edits don't trigger a re-render.
  const trimmedQuery = search.trim().toLowerCase();

  // Root users are never shown. Search (when present) filters by
  // case-insensitive substring on code OR name.
  const rows = useMemo<UserView[]>(() => {
    const list = (users.data ?? []).filter((u) => u.role !== "root");
    if (!trimmedQuery) return list;
    return list.filter(
      (u) =>
        u.code.toLowerCase().includes(trimmedQuery) ||
        u.name.toLowerCase().includes(trimmedQuery),
    );
  }, [users.data, trimmedQuery]);
```

(Leave the two `useCallback` blocks for `handleToggle` and `handleRoleChange` unchanged.)

Now replace the JSX block (currently lines 47-58):

```tsx
  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4">{t("user.heading")}</Typography>
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
    </Box>
  );
```

with:

```tsx
  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <UserFilterBar query={search} onQueryChange={setSearch} />
      <UserTable
        rows={rows}
        loading={users.isLoading}
        mutationLoading={updateUser.isPending}
        error={users.error ?? updateUser.error}
        selfCode={selfCode}
        onToggle={handleToggle}
        onRoleChange={handleRoleChange}
        onRetry={users.refetch}
        emptyMessage={trimmedQuery ? t("user.noMatches") : undefined}
      />
    </Box>
  );
```

The `Typography` import from `@aegis/ui/mui` (currently line 2) is no longer used. Remove it from the import list:

```tsx
import { Box } from "@aegis/ui/mui";
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
pnpm exec vitest run src/test/pages/user-list.test.tsx
```

Expected: PASS for all 13 tests (the 8 existing + the 5 new).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserList.tsx apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx
git commit -m "feat(desktop): wire search filter into UserList page"
```

---

## Task 5: Full verification

- [ ] **Step 1: Run the full desktop test suite**

From `apps/desktop/aegis-desktop`:

```bash
pnpm exec vitest run
```

Expected: PASS for all suites. Particularly:
- `src/test/pages/user-list.test.tsx` — 13 tests pass (8 existing + 5 search)
- `src/test/pages/user-table.test.tsx` — 17 tests pass (16 existing + 1 emptyMessage)
- All other suites unchanged

If any test fails, fix the root cause (not the test) and re-run.

- [ ] **Step 2: Run the full repo typecheck + UI test suite**

From repo root:

```bash
pnpm -r exec tsc --noEmit
pnpm --filter @aegis/ui exec vitest run
```

Expected: PASS for all. The typecheck confirms `user.heading` removal didn't break any other consumer (only `UserList.tsx` referenced it) and that `UserFilterBar`'s `UserFilterBarProps` interface is exported correctly.

- [ ] **Step 3: Confirm the file count**

```bash
git diff --stat main..HEAD
```

Expected: 3 new files (this plan's spec already exists at `docs/superpowers/specs/2026-08-15-aegis-desktop-user-page-search-field-design.md`), 5 modified files (`en.ts`, `zhCN.ts`, `UserTable.tsx`, `UserList.tsx`, `user-table.test.tsx`, `user-list.test.tsx`) plus the new `UserFilterBar.tsx`.

---

## Out of scope (deferred, per spec)

- Server-side search (new `?q=` param on `list_users`).
- URL persistence (`?q=bob` survives reload).
- Debouncing keystrokes (no network call, no need).
- Clear button adornment.
- Highlighting matched substrings.
- Sort / multi-field filtering.
