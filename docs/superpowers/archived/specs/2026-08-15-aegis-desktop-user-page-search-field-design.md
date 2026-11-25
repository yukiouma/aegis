# Search field on User management page

Date: 2026-08-15
Status: Approved (brainstorming)

## Goal

Replace the `<Typography variant="h4">Users</Typography>` heading at
the top of the User management page (`/_layout/users`) with a
search TextField (leading `Search` icon + placeholder) that filters
the visible rows by case-insensitive substring match on `name` OR
`code`.

Filter is client-side — the existing `list_users` endpoint stays
unchanged. No new server routes.

## Approach

Extract a new `UserFilterBar` component that mirrors the existing
[ProjectFilterBar](../..//apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx)
pattern (pure controlled, orchestrator owns state). Add a `query`
state to `UserList` and pass it down. Filter `rows` further by the
query before passing to `UserTable`.

When the query is non-empty AND the filtered list is empty, render
a "no matches" message instead of the existing "no users yet" empty
state.

## File layout

```
apps/desktop/aegis-desktop/src/
├── pages/
│   ├── UserFilterBar.tsx                  NEW — search TextField + icon
│   ├── UserList.tsx                       MODIFIED — add search state, render filter bar
│   └── UserTable.tsx                      UNCHANGED
│
├── test/
│   └── pages/user-list.test.tsx           MODIFIED — add 4 search test cases
│
lib/packages/ui/src/i18n/locales/
├── en.ts                                  MODIFIED — drop user.heading, add
│                                          user.search.placeholder + user.noMatches
└── zhCN.ts                                MODIFIED — mirror
```

No changes to: `src/api/`, `src/data/`, `src/components/`, `src/routes/`,
`src-tauri/`, or the aegis-server.

## Components

### `UserFilterBar.tsx` (new)

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
 * component — `UserList` owns the query state. Leading Search icon
 * signals purpose at a glance; placeholder hints at the match
 * semantics (name or code, case-insensitive).
 */
export function UserFilterBar({ query, onQueryChange }: UserFilterBarProps) {
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

Notes:
- `slotProps.input.startAdornment` is the MUI v9 spelling for the
  leading-icon slot (consistent with the `slotProps={{ paper: ... }}`
  pattern already used in [ProjectDrawer.tsx:134](../..//apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx)).
  The legacy `InputProps={{ startAdornment: ... }}` spelling still
  works but is not used in this codebase.
- No clear button — out of scope. Backspace / select-all-delete
  is one keystroke.

### `UserList.tsx` (modified)

```tsx
import { useCallback, useMemo, useState } from "react";
import { Box } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListUsers, useUpdateUser } from "../data";
import type { Role, UserView } from "../api";
import { UserFilterBar } from "./UserFilterBar";
import { UserTable } from "./UserTable";

export function UserListPage() {
  const { t } = useI18n();
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();
  const [search, setSearch] = useState("");

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";
  const selfCode = currentUser.data?.code ?? null;

  const trimmedQuery = search.trim().toLowerCase();

  const rows = useMemo<UserView[]>(() => {
    const list = (users.data ?? []).filter((u) => u.role !== "root");
    if (!trimmedQuery) return list;
    return list.filter(
      (u) =>
        u.code.toLowerCase().includes(trimmedQuery) ||
        u.name.toLowerCase().includes(trimmedQuery),
    );
  }, [users.data, trimmedQuery]);

  // ... existing handleToggle + handleRoleChange unchanged ...

  if (!canManage) return null;

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
        emptyMessage={
          trimmedQuery ? t("user.noMatches") : undefined
        }
      />
    </Box>
  );
}
```

Notes:
- `useMemo` dependency on `trimmedQuery` (a stable string) is
  preferred over `search` so a trailing-whitespace-only edit
  doesn't re-render.
- `UserTable` gains one new optional prop `emptyMessage`. When
  provided, the table renders that string instead of `t("user.empty")`
  when rows is empty. The existing `user.empty` key is unchanged —
  it's still used when there's no search query and the server
  returned an empty list.

### `UserTable.tsx` (one-prop addition)

Extend `UserTableProps`:

```ts
export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRoleChange: (code: string, nextRole: Role) => void;
  onRetry: () => void;
  emptyMessage?: string;          // NEW
}
```

In the empty-state branch (currently:

```tsx
{!showSpinner && rows.length === 0 && (
  <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
    <Typography color="textSecondary">{t("user.empty")}</Typography>
  </Box>
)}
```

):

```tsx
{!showSpinner && rows.length === 0 && (
  <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
    <Typography color="textSecondary">
      {emptyMessage ?? t("user.empty")}
    </Typography>
  </Box>
)}
```

## Data flow

1. User types in the TextField → `onChange` fires
   `onQueryChange(e.target.value)`.
2. `UserList` updates local `search` state.
3. `useMemo` recomputes `rows` with the trimmed, lowercased query.
4. `UserTable` re-renders with the filtered list.
5. If filtered list is empty AND `trimmedQuery` is non-empty, the
   empty-state Typography shows `user.noMatches`.
6. Mutations (Switch toggle, Select change) still flow through the
   existing `updateUser.mutate(...)` path; on success the list
   cache invalidates and the search filter reapplies to the new
   data.

## Edge cases

- **Empty / whitespace-only query** — `trimmedQuery === ""`,
  short-circuits, returns full list. No filter applied.
- **Query matches self** — self-row still renders, still disabled
  on both controls. No special-casing needed.
- **Query matches no rows** — empty-state shows `user.noMatches`.
- **Mutation changes a row that no longer matches the query** —
  the row disappears from the table but the cache still holds it
  (server is source of truth). Clearing the query brings it back.
- **Case sensitivity** — always case-insensitive on both `code`
  and `name`.
- **Match anywhere in the string** — `includes`, not `startsWith`.

## Error handling

No new error paths. The existing `<Alert>` slot still covers
`list_users` and `update_user` failures.

## i18n keys

Add to both `en.ts` (with `as const`) and `zhCN.ts` (with
`satisfies Record<keyof typeof en, string>`). Both files must stay
in lock-step.

| Key | en | zh-CN |
|---|---|---|
| `user.search.placeholder` | Search by name or code | 按姓名或账号搜索 |
| `user.noMatches` | No matching users | 无匹配用户 |

Remove `user.heading` from both files (its only consumer, the
`<Typography variant="h4">` heading, is deleted).

All other existing keys (`user.role.*`, `user.field.*`,
`user.empty`, `user.cannotDeactivateSelf`, `user.cannotChangeOwnRole`,
`nav.management*`, etc.) are untouched.

## Tests

### `src/test/pages/user-list.test.tsx`

Add a new describe block at the end:

```ts
describe("UserListPage — search", () => {
  it("renders a TextField with a Search icon in place of the heading", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    expect(input).toBeInTheDocument();
  });

  it("filters rows by code substring (case-insensitive)", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "BO");
    // Only bob remains.
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("filters rows by name substring", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "bob");
    // "bob" matches the code AND the name of generalUser.
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

The existing tests (root filter, role gate, toggle calls
update_user, role change, self-disable, error surfaces) remain
unchanged and should still pass — the heading removal and search
default-to-empty do not affect them.

## Out of scope (deferred)

- Server-side search (new `?q=` param on `list_users`).
- URL persistence (`?q=bob` survives reload).
- Debouncing keystrokes (no network call yet, so no need).
- Clear button adornment.
- Highlighting matched substrings.
- Sort / multi-field filtering.

## File changes summary

**Modified files**

- `apps/desktop/aegis-desktop/src/pages/UserList.tsx` — add
  search state, replace heading with `UserFilterBar`, filter
  `rows`, branch empty-state message
- `apps/desktop/aegis-desktop/src/pages/UserTable.tsx` — add
  optional `emptyMessage` prop
- `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`
  — add 5 search test cases
- `lib/packages/ui/src/i18n/locales/en.ts` — drop `user.heading`,
  add `user.search.placeholder` + `user.noMatches`
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — mirror

**New files**

- `apps/desktop/aegis-desktop/src/pages/UserFilterBar.tsx`

**Untouched**

- `apps/desktop/aegis-desktop/src/api/**`
- `apps/desktop/aegis-desktop/src/data/**`
- `apps/desktop/aegis-desktop/src/components/**`
- `apps/desktop/aegis-desktop/src/routes/**`
- `apps/desktop/aegis-desktop/src/pages/{Home,Settings,Project*,
  UserFooter,Layout}.tsx`
- `apps/desktop/aegis-desktop/src/main.tsx`
- `apps/desktop/aegis-desktop/src-tauri/**`
- `apps/server/aegis-server/**`
- `lib/crates/**`
- `lib/packages/ui/src/components/**`
- All other tests, vitest config, package.json
