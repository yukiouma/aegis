# Add a User management page to Aegis desktop

Date: 2026-08-15
Status: Approved (brainstorming)

## Goal

Add a `User` page to the Aegis desktop app at `/_layout/users`,
reachable from a new `Management` sidebar entry whose only submenu
points at the user page. The page shows a read-only table of users
(code, name, role, active) and lets root or admin users flip the
`active` flag of any non-root user via a `Switch`. Root users are
never displayed in the table; the current user cannot deactivate
themselves. Both the `Management` sidebar entry and the page itself
are hidden from users whose role is not `root` or `admin`.

Back the page with two existing TanStack Query hooks (`useListUsers`,
`useCurrentUser`) plus one new hook (`useUpdateUser`) added to the
data layer to wrap `api.updateUser`. Reuse the existing PATCH
endpoint (`PATCH /api/user/{code}` with `{ active: true|false }`) for
the toggle; no new server route. Filter the root users out
client-side in the page orchestrator. Both gating rules are UI-only
— server authorization continues to live in `AuthClaims` middleware.

Today the desktop exposes `useListUsers` and `useCurrentUser` (per
the [2026-08-14 tanstack query refactor spec](2026-08-14-aegis-desktop-tanstack-query-refactor-design.md)
and [2026-08-13 domain-user-info-api spec](2026-08-13-aegis-desktop-domain-user-info-api-design.md)),
and `api.updateUser` exists in the transport layer. The Sidebar
component in `@aegis/ui` already supports nested `subMenu` entries
(per the
[2026-07-29 sidebar spec](2026-07-29-aegis-desktop-sidebar-theme-integration-design.md)).
The only data-layer addition is the `useUpdateUser` hook + its
re-export.

## Approach

Add a `_layout/users` file route, two page components (orchestrator
+ table), and one route file. Append a new `Management` entry to the
`AppLayout` Sidebar `menu` array with a `Users` submenu pointing at
the new route. Reuse the `FormControlLabel` + `Switch` control
pattern established in `Settings.tsx` and `ProjectDrawer.tsx`. Add
the i18n keys needed by the table to both locale files.

### Why an orchestrator + table split

A single-file page would mix the role gate, the data fetch, the
client-side root filter, the mutation handler, the error surfaces,
and the table rendering in one place. Splitting per concern keeps
each file under ~150 lines and lets each piece have a focused test
file (the convention in `src/test/pages/`).

### Why a flat URL `/users`, not `/management/users`

The sidebar visualizes the Management → Users hierarchy through
`subMenu`, but the URL stays flat at `/users`. This matches the
existing pattern (`/`, `/projects`, `/settings`) and keeps
TanStack Router's file-route generator output stable — adding a
nested path would force a route-tree reshuffle with no real benefit.

### Why client-side root filtering

The "do not display root users" rule is enforced in the page
orchestrator as a `useMemo` filter. This keeps the change scoped to
the desktop app (no server edits) and mirrors how the project page
filters on `Involve` and search. A direct API call to
`list_users` would still return root rows; the server-side gap is
documented and out of scope for this feature.

### Why prevent self-deactivation

Locking yourself out by accident is irreversible without database
access. The Switch is `disabled` on the row whose `code` matches
`currentUser.code`. Roots are already filtered out of the table, so
in practice this only matters if the rule changes; the defensive
check is cheap and documents intent.

## File layout

```
apps/desktop/aegis-desktop/src/
├── data/
│   ├── user.ts                               MODIFIED — add useUpdateUser
│   └── index.ts                              MODIFIED — re-export useUpdateUser
│
├── pages/
│   ├── UserList.tsx                          NEW — page orchestrator
│   ├── UserTable.tsx                         NEW — table + Switch toggle
│   └── layout.tsx                            MODIFIED — add Management menu
│                                              entry with Users submenu
├── routes/_layout/
│   └── users.tsx                             NEW — route file
│
├── test/
│   ├── data/user.test.tsx                    MODIFIED — add useUpdateUser cases
│   ├── pages/user-list.test.tsx              NEW
│   ├── pages/user-table.test.tsx             NEW
│   └── routes/users.test.tsx                 NEW
│
lib/packages/ui/src/i18n/locales/
├── en.ts                                     MODIFIED — add user.* + nav.management*
└── zhCN.ts                                   MODIFIED — mirror the same keys
```

No changes to `src/api/` (the `api.updateUser` transport and
`UpdateUserBody` type already exist). No changes to
`src/components/`. The Tauri Rust backend (`src-tauri/`) is
untouched. The aegis-server is untouched.

## Routing

### `src/routes/_layout/users.tsx`

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { UserListPage } from "../../pages/UserList";

export const Route = createFileRoute("/_layout/users")({
  component: UserListPage,
});
```

`routeTree.gen.ts` regenerates automatically via
`@tanstack/router-plugin` (already in `package.json`). No manual
edits to the generated file.

### Sidebar entry

In `src/pages/layout.tsx`, append a `Management` menu item after
Settings. The entry is hidden entirely from users whose role is
neither `root` nor `admin`, mirroring the page-visibility rule:

```tsx
import {
  AdminPanelSettings as AdminPanelSettingsIcon,
  Home as HomeIcon,
  People as PeopleIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
import { useCurrentUser } from "../data";

const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;
const ManagementMenuIcon = () => <AdminPanelSettingsIcon />;
const UsersMenuIcon = () => <PeopleIcon />;

export function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);
  const currentUser = useCurrentUser();

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";

  const baseMenu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const managementEntry: MenuItem = {
    link: "#",
    title: t("nav.management"),
    icon: ManagementMenuIcon,
    subMenu: [
      {
        link: "/users",
        title: t("nav.management.users"),
        icon: UsersMenuIcon,
      },
    ],
  };

  const menu: MenuItem[] = canManage
    ? [...baseMenu, managementEntry]
    : baseMenu;

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: <UserFooter sidebarOpen={sidebarOpen} />,
  };

  // ...render unchanged
}
```

`AdminPanelSettings` and `People` are both part of
`@mui/icons-material` v9, re-exported by `@aegis/ui/icons`. The
`link: "#"` on the parent entry is intentional: clicking it just
expands the submenu (the existing Sidebar `handleClick` in
`lib/packages/ui/src/components/Sidebar/Sidebar.tsx:107-110` does
not call `onNavigate` for items with a `subMenu`).

While `useCurrentUser` is still loading, `role` is `undefined`,
`canManage` is `false`, and the Management entry is hidden. Once
the role resolves, the entry appears. The brief flicker on first
mount for a root/admin user is acceptable and matches how the
footer already behaves.

The `UserFooter` already renders below the menu via the
existing `footer` prop on `Sidebar`; no change there.

## Role gating

The page is reachable by any authenticated user (the auth gate in
`routes/_layout/route.tsx` covers login only). The role check is
inside the page orchestrator:

```ts
const role = currentUser.data?.role;
const canManage = role === "root" || role === "admin";

if (!canManage) return null;
```

This mirrors `ProjectList.tsx:24-25`'s `canEdit = role === "root" ||
role === "admin"` pattern. A `general` user who navigates to
`/users` via URL sees an empty `<Outlet>` area inside `AppLayout`.
The auth boundary is server-side; UI gating prevents affordances
from being exercised by accident, but a determined user can still
issue `update_user` directly. That gap exists today for the project
write flow and is out of scope here.

## New hook — `useUpdateUser`

Append to `src/data/user.ts` (mirror of `useUpdateProject` in
`src/data/project.ts:67-80`):

```ts
import { api, type ApiError, type UpdateUserBody, type UserView } from "../api";

/**
 * Update an existing user. On success: invalidates the user list
 * cache so the management page reflects the new active state on the
 * next render. Also invalidates `user.current()` since the current
 * user's own row could be the one being updated by a sibling admin.
 */
export function useUpdateUser() {
  const qc = useQueryClient();
  return useMutation<
    UserView,
    ApiError,
    { code: string; body: UpdateUserBody }
  >({
    mutationFn: ({ code, body }) => api.updateUser(code, body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.user.list() });
      qc.invalidateQueries({ queryKey: queryKeys.user.current() });
    },
  });
}
```

Add to `src/data/index.ts`:

```ts
export {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useRegisterUser,
  useLogout,
  useUpdateUser,            // NEW
} from "./user";
```

Rationale for invalidating `user.current()`:

- A root/admin can flip the `active` flag on any row including
  their own future session row; invalidating the current-user cache
  keeps the footer's display in sync if the row happens to be the
  active session.
- The mutation invalidation target is keyed by tuple, so this is
  one extra `invalidateQueries` call — cheap, and avoids a stale
  footer showing the wrong name/role/active state.

## Components

### `src/pages/UserList.tsx` — orchestrator

```tsx
import { useCallback, useMemo } from "react";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListUsers, useUpdateUser } from "../data";
import type { UserView } from "../api";
import { UserTable } from "./UserTable";

/**
 * User management page. Lists non-root users with code, name, role,
 * and an active-state Switch. Filtering (root users hidden) and the
 * role gate live here; the table is a pure presentational component.
 */
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

  if (!canManage) return null;

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
        onRetry={users.refetch}
      />
    </Box>
  );
}
```

Notes:

- `useUpdateUser` invalidates both `queryKeys.user.list()` and
  `queryKeys.user.current()` on success (see "New hook"
  section above), so the table re-renders with the new active state
  without a manual refetch.
- Mutation error and list error share the same surface inside the
  table; both flow through `errorMessage` from `src/api/error.ts`.
- The heading uses `t("user.heading")`, mirroring
  `project.heading` even though the project page omits its
  heading — having an explicit page title is friendlier for a
  management surface users arrive at from a sidebar rather than
  the home screen.

### `src/pages/UserTable.tsx` — presentational

```tsx
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  FormControlLabel,
  Paper,
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

import type { ApiError, UserView } from "../api";
import { errorMessage } from "../api/error";

export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRetry: () => void;
}

/**
 * Renders the user list as a MUI Table. The Switch in the active
 * column is disabled on the row matching `selfCode` (cannot
 * deactivate yourself) and on every row while a mutation is in
 * flight.
 */
export function UserTable({
  rows,
  loading,
  mutationLoading,
  error,
  selfCode,
  onToggle,
  onRetry,
}: UserTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("user.loadFailed", { message: errorMessage(error) })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>{t("user.field.code")}</TableCell>
              <TableCell>{t("user.field.name")}</TableCell>
              <TableCell>{t("user.field.role")}</TableCell>
              <TableCell>{t("user.field.active")}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => {
              const isSelf = row.code === selfCode;
              const disabled = isSelf || mutationLoading;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>{row.code}</TableCell>
                  <TableCell>{row.name}</TableCell>
                  <TableCell>
                    <Chip
                      variant="outlined"
                      size="small"
                      label={t(`user.role.${row.role}`)}
                    />
                  </TableCell>
                  <TableCell>
                    <Tooltip
                      title={
                        isSelf
                          ? t("user.cannotDeactivateSelf")
                          : t(row.active ? "user.active" : "user.inactive")
                      }
                    >
                      <span>
                        <FormControlLabel
                          sx={{ ml: 0 }}
                          control={
                            <Switch
                              size="small"
                              checked={row.active}
                              disabled={disabled}
                              onChange={(e) =>
                                onToggle(row.code, e.target.checked)
                              }
                            />
                          }
                          label=""
                        />
                      </span>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{t("user.empty")}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}
```

Behavior:

- **Loading state** — when `loading && rows.length === 0`, render a
  centered `<CircularProgress />` in place of the table body. While
  a mutation is in flight (`mutationLoading`) the existing rows
  stay rendered and every Switch is `disabled`, matching
  `ProjectTable`'s stale-while-revalidate approach.
- **Error state** — an `<Alert severity="error">` above the table;
  the table itself is not rendered. Includes a `Retry` button that
  calls `onRetry` (`users.refetch` from `useListUsers`).
- **Empty state** — no rows + not loading + no error: a centered
  `<Typography>{t("user.empty")}</Typography>` inside the table
  area.
- **Self-disable** — Switch on the row whose `code` matches
  `selfCode` is `disabled`. The `Tooltip` title changes to
  `t("user.cannotDeactivateSelf")` to make the rule discoverable.
  Rows disabled only because `mutationLoading` is true keep the
  normal Active/Inactive tooltip — the disabled state itself is
  enough feedback.
- **Role chip** — uses the same outlined Chip pattern as
  `ProjectTable`'s leader chips, with the label sourced from
  `t("user.role.admin")` or `t("user.role.general")` (root labels
  are unused in the table — they would only render if the filter
  regressed; still defined to keep the i18n set exhaustive).

The Switch sits inside a `<span>` so the `Tooltip` wrapper can
attach even when the Switch itself is `disabled` (matching the
project table's `<Tooltip><span>{icon}</span></Tooltip>` pattern at
`ProjectTable.tsx:132-143`).

## i18n keys

Add to both `lib/packages/ui/src/i18n/locales/en.ts` (with
`as const`) and `zhCN.ts` (with
`satisfies Record<keyof typeof en, string>`). Both files must stay
in lock-step.

| Key | en | zh-CN |
|---|---|---|
| `nav.management` | Management | 管理 |
| `nav.management.users` | Users | 用户 |
| `common.retry` | Retry | 重试 |
| `user.heading` | Users | 用户 |
| `user.empty` | No users yet | 暂无用户 |
| `user.loadFailed` | Failed to load users: {message} | 加载用户失败：{message} |
| `user.active` | Active | 已启用 |
| `user.inactive` | Inactive | 未启用 |
| `user.cannotDeactivateSelf` | You cannot deactivate yourself | 无法停用自己的账号 |
| `user.field.code` | Code | 账号 |
| `user.field.name` | Name | 姓名 |
| `user.field.role` | Role | 角色 |
| `user.field.active` | Active | 已启用 |
| `user.role.root` | Root | Root |
| `user.role.admin` | Admin | 管理员 |
| `user.role.general` | General | 普通用户 |

Notes on existing keys:

- `app.user.role.root` / `app.user.role.admin` already exist
  (used by `UserFooter`). The new `user.role.*` keys are a separate
  set used only by the table. Keeping them separate means the
  footer copy and the table copy can diverge if the product wants
  one to be more verbose than the other.
- `common.cancel` already exists. `common.retry` is new.

## Data flow

1. Mount → `useListUsers` fires `GET /api/user` → Tauri `list_users`
   → server returns all users (root + non-root).
2. `useCurrentUser` provides the current user's code (for the
   self-disable) and role (for the visibility gate).
3. `useMemo` strips root users before they reach the table.
4. Toggle Switch → `useUpdateUser.mutate({ code, body: { active } })`
   → Tauri `update_user` → `PATCH /api/user/{code}`.
5. On success, `useUpdateUser` invalidates
   `queryKeys.user.list()` → `useListUsers` refetches → table
   re-renders with the new state.
6. Failure surfaces via the table's `<Alert>` (both list and
   mutation errors share the slot).

The page does no optimistic updates. The Switch stays where the
user put it visually until the refetch lands, then snaps to the
server's truth. While `updateUser.isPending`, every Switch is
`disabled` so the user cannot fire a second mutation against the
same list.

## Error handling summary

| Failure | Surface |
|---|---|
| `list_users` throws | `<Alert severity="error">` above the table; Retry button calls `refetch`. |
| `update_user` throws | `<Alert severity="error">` above the table (same slot as list error); rows remain visible. |
| `getCurrentUser` fails / not loaded | Page returns `null` (no heading, no table); a `general` user sees nothing, a `root`/`admin` user sees nothing until `useCurrentUser` resolves. If the call fails the user sees an empty `<Outlet>`. |

Error narrowing goes through `toApiError` / `errorMessage` /
`httpCode` from `src/api/error.ts` — already the convention; no
new helpers.

## Tests

### `src/test/data/user.test.tsx`

Append cases to the existing test file (which already covers
`useCurrentUser`, `useDomainUserInfo`, `useRegisterUser`,
`useLogout`, `useListUsers`):

- `useUpdateUser`:
  - calls `update_user` with `{ code, body }`.
  - on success: invalidates both `queryKeys.user.list()` and
    `queryKeys.user.current()`.
  - on error: does NOT invalidate either cache.

### `src/test/pages/user-list.test.tsx`

Wraps in `AegisThemeProvider > TestQueryProvider > AegisI18nProvider`,
mocks `current_user` and `list_users` via `mockCommands`. Uses
`renderInRouter` (no full router needed — the orchestrator does not
rely on layout state beyond `useCurrentUser`).

Coverage:

- **Basic rendering** — admin user with a list of 3 users (1 root +
  2 non-root): assert root user is NOT in the document; the 2
  non-root users are.
- **Role gate** — `generalUser` with a list of 1 non-root user:
  assert no rows are rendered (page returns `null`).
- **Columns** — assert each row renders Code, Name, Role chip, and
  the Switch with the correct `checked` value.
- **Self-deactivation prevention** — current user's row Switch has
  `disabled` attribute set; tooltip text matches
  `t("user.cannotDeactivateSelf")`. Other rows are not disabled.
- **Toggle calls mutation** — click a non-self Switch, assert
  `invoke("update_user", { code: "<row.code>", body: { active:
  !prev } })` was called exactly once.
- **Mutation loading disables all Switches** — fire a slow mutation
  handler; while pending, every Switch on the page has `disabled`.
- **List failure → Alert + Retry** — `list_users` rejects with
  `httpError(401, "token_verification_failed")`; assert Alert text
  matches `t("user.loadFailed", { message: ... })` and the Retry
  button is present; clicking Retry fires a second
  `invoke("list_users")` call.
- **Mutation failure → Alert** — first call to `list_users`
  succeeds; `update_user` rejects with `httpError(403, "forbidden")`;
  assert Alert text + the rows are still visible.

### `src/test/pages/user-table.test.tsx`

Pure component tests. No router, no `useListUsers` — pass `rows`,
`loading`, `mutationLoading`, `error`, `selfCode`, `onToggle`,
`onRetry` props directly.

Coverage:

- Empty rows + `loading=true` → `<CircularProgress />` renders; no
  table.
- Empty rows + `loading=false` → empty-state copy renders.
- 3 rows render with correct code, name, role chip, and Switch
  `checked` state.
- `selfCode` matches one row's `code` → that row's Switch is
  `disabled`; the Tooltip title text matches `user.cannotDeactivateSelf`.
- Other rows' Switches are NOT disabled.
- Click a Switch → `onToggle(code, nextChecked)` called exactly
  once with the right args.
- `mutationLoading=true` → every Switch has `disabled`.
- `error` set → `<Alert severity="error">` with the message; Retry
  button present; clicking Retry calls `onRetry` once.

### `src/test/routes/users.test.tsx`

Full router test with `renderWithFullRouter` (same shape as
`src/test/routes/projects.test.tsx`).

Coverage:

- **Authenticated + admin** — `is_logged_in: () => true`,
  `current_user: { role: "admin", ... }`, `list_users: () => [user]`
  → `screen.getByTestId("sidebar")` is present; the table renders.
- **Authenticated + root** — same as above with `role: "root"` →
  page renders (root has visibility).
- **Authenticated + general** — `role: "general"` → page renders
  nothing inside `<Outlet>` (the orchestrator returns `null`).
- **Sidebar shows the Management entry with Users submenu** —
  with an admin current user, `screen.getByText("Management")`
  present; clicking it expands the submenu; `screen.getByText("Users")`
  then appears.
- **Sidebar hides the Management entry for general users** — with
  `role: "general"`, `screen.queryByText("Management")` returns
  `null`; the sidebar still shows Home, Projects, and Settings.
- **Sidebar shows the Management entry for root users** — with
  `role: "root"`, `screen.getByText("Management")` present.
- **Navigation** — starting at `/settings`, click `Users` in the
  sidebar submenu; `router.state.location.pathname` becomes
  `/users`.
- **Unauthenticated** — `is_logged_in: () => false` → redirects to
  `/login`; sidebar not present.

## File changes summary

**New files**

- `apps/desktop/aegis-desktop/src/pages/UserList.tsx`
- `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`
- `apps/desktop/aegis-desktop/src/routes/_layout/users.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/routes/users.test.tsx`

**Modified files**

- `apps/desktop/aegis-desktop/src/data/user.ts` — add
  `useUpdateUser` mutation hook
- `apps/desktop/aegis-desktop/src/data/index.ts` — re-export
  `useUpdateUser`
- `apps/desktop/aegis-desktop/src/data/user.test.tsx` — add
  `useUpdateUser` cases (per the "New hook" section)
- `apps/desktop/aegis-desktop/src/pages/layout.tsx` — add
  `AdminPanelSettings` / `People` icon imports; add `useCurrentUser`
  import; build the menu array dynamically so the Management entry
  is only included for root/admin users
- `lib/packages/ui/src/i18n/locales/en.ts` — add `user.*`,
  `nav.management*`, `common.retry`
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — mirror the same keys
- `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` —
  regenerated by `@tanstack/router-plugin`

**Untouched**

- `apps/desktop/aegis-desktop/src/api/**` — `api.updateUser` and
  `UpdateUserBody` already exist
- `apps/desktop/aegis-desktop/src/data/queryKeys.ts` —
  `queryKeys.user.list()` already exists
- `apps/desktop/aegis-desktop/src/components/**`
- `apps/desktop/aegis-desktop/src/routes/__root.tsx`,
  `_layout/route.tsx`, `_layout/index.tsx`, `_layout/projects.tsx`,
  `_layout/settings.tsx`
- `apps/desktop/aegis-desktop/src/main.tsx`
- `apps/desktop/aegis-desktop/src/pages/{Home,Settings,
  ProjectList,ProjectTable,ProjectFilterBar,ProjectDrawer,
  UserFooter}.tsx`
- `apps/desktop/aegis-desktop/src-tauri/**`
- `apps/server/aegis-server/**`
- `lib/crates/**`
- `lib/packages/ui/src/components/**`
- All other tests, vitest config, package.json

## Out of scope (deferred to a later feature)

- Server-side role enforcement on the user endpoints (today's
  handlers ignore claims). Defense-in-depth would live in
  `apps/server/aegis-server/src/transport/http/user/handlers.rs`.
- A dedicated `PATCH /api/user/{code}/active` endpoint. The
  general PATCH endpoint is sufficient today.
- Editing user fields (code, name, role) from the page. The page is
  read-only except for the active flag.
- Creating users from the page (the register flow handles account
  creation; admin-issued invites / role assignment would be a
  separate feature).
- Sorting, pagination, filtering by active / role from the UI.
  YAGNI for the first cut.
- Confirmation dialog when flipping the Switch. The mutation is
  reversible; no confirm.
- Optimistic updates on the Switch. Server is the source of truth.