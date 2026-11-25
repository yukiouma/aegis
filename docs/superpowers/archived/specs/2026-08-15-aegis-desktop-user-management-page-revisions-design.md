# Reorder sidebar + role-dropdown for User management page

Date: 2026-08-15
Status: Approved (brainstorming)

## Goal

Two revisions to the User management page (already shipped under
[2026-08-15-aegis-desktop-user-management-page-design.md](2026-08-15-aegis-desktop-user-management-page-design.md)):

1. **Sidebar reorder** — move the `Management` menu entry so it sits
   between `Projects` and `Settings`, instead of trailing after
   `Settings`.
2. **Role Select dropdown** — replace the read-only `Chip` in the
   role column with a `Select` dropdown offering `admin` and
   `general` as options. Selecting a value calls `useUpdateUser` with
   `{ role: nextRole }`. The Select is disabled on the current user's
   own row (mirror of the existing self-disable rule on the active
   Switch) and while any mutation is in flight.

Both changes are scoped to the desktop app. No new data-layer code:
the existing `useUpdateUser` hook (added in the prior spec) already
threads `{ role }` through its `UpdateUserBody` shape, and
`api.updateUser(code, body)` on the server already accepts it.

## Approach

### Sidebar reorder

One-line edit in `src/pages/Layout.tsx`. Move the Management entry
in the `baseMenu` array from index 3 (after Settings) to index 2
(before Settings). The array currently is:

```
[ Home, Projects, Settings, Management (submenu: Users) ]
```

After:

```
[ Home, Projects, Management (submenu: Users), Settings ]
```

The `canManage === role === "root" || role === "admin"` gate still
controls whether the Management entry appears at all — only its
position in the array changes.

### Role Select dropdown

`UserTable` becomes a slightly thicker presentational component. The
new `onRoleChange(code, nextRole)` prop joins the existing
`onToggle(code, nextActive)` and `onRetry()` props. The Select sits
where the Chip was in the role cell, wrapped in the same
`<Tooltip><span>{control}</span></Tooltip>` pattern used by the
Switch so the tooltip can attach when `disabled`.

`UserList` adds one new `useCallback` for `handleRoleChange` and
passes it through. The `useUpdateUser` hook is unchanged.

## File layout

```
apps/desktop/aegis-desktop/src/
├── pages/
│   ├── UserList.tsx                          MODIFIED — add onRoleChange handler
│   └── UserTable.tsx                         MODIFIED — replace Chip with Select,
│                                              add onRoleChange prop + Tooltip
├── pages/Layout.tsx                          MODIFIED — reorder menu array
│
├── test/
│   ├── pages/user-list.test.tsx              MODIFIED — add role-mutation case
│   ├── pages/user-table.test.tsx             MODIFIED — chip → Select assertions,
│                                              add 4 new cases
│   └── data/user.test.tsx                    MODIFIED — add role-body mutation case
│
lib/packages/ui/src/i18n/locales/
├── en.ts                                     MODIFIED — add user.cannotChangeOwnRole
└── zhCN.ts                                   MODIFIED — mirror
```

No changes to: `src/api/`, `src/data/` (hooks), `src/components/`,
`src/routes/`, `src/pages/{Home,Settings,Project*}.tsx`,
`src-tauri/`, or the aegis-server.

## Sidebar entry (revised)

`src/pages/Layout.tsx`:

```tsx
const baseMenu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  {
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
  },
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];
```

The Management entry's parent `link: "#"` and `subMenu` definition
are unchanged — only the array index moves.

## Components

### `UserTable.tsx` — role cell revision

Replace the role cell:

```tsx
// before
<TableCell>
  <Chip
    variant="outlined"
    size="small"
    label={t(`user.role.${row.role}`)}
  />
</TableCell>

// after
<TableCell>
  <Tooltip
    title={isSelf ? t("user.cannotChangeOwnRole") : ""}
  >
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

Notes:

- `disabled = isSelf || mutationLoading` — same variable that gates
  the Switch, so both controls move together when the current user
  flips the active state of their own row, when another row's
  mutation is in flight, etc.
- `Tooltip title=""` (empty string) when not `isSelf` — MUI renders
  nothing for an empty title, so the wrapper doesn't add visual
  noise on the non-self rows.
- The root role is intentionally NOT a MenuItem — root users are
  filtered out of the table before they reach this component, so
  the option would never be the current value. If a future change
  ever lets admins view root users, the option reappears in this
  map.
- `value={row.role}` is a `Role` (which is the union `"root" | "admin"
  | "general"`). TypeScript will narrow the cast to `Role` at the
  call site; the dropdown only emits `"admin" | "general"`, so the
  cast is safe.

### `UserTableProps` (extended)

```ts
export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRoleChange: (code: string, nextRole: Role) => void;   // NEW
  onRetry: () => void;
}
```

### `UserList.tsx` — orchestrator addition

```tsx
import type { Role, UserView } from "../api";
// ...

const handleRoleChange = useCallback(
  (code: string, nextRole: Role) => {
    updateUser.mutate({ code, body: { role: nextRole } });
  },
  [updateUser],
);

// ... pass to <UserTable>
<UserTable
  rows={rows}
  loading={users.isLoading}
  mutationLoading={updateUser.isPending}
  error={users.error ?? updateUser.error}
  selfCode={selfCode}
  onToggle={handleToggle}
  onRoleChange={handleRoleChange}      // NEW
  onRetry={users.refetch}
/>
```

## Data flow for a role change

1. User clicks the Select → MUI menu opens with two MenuItems
   labelled via `t("user.role.admin")` and `t("user.role.general")`.
2. User picks a different value → `Select.onChange` fires
   `onRoleChange(row.code, "admin" | "general")`.
3. Orchestrator calls `updateUser.mutate({ code, body: { role } })`
   — same hook as the active toggle.
4. While `updateUser.isPending`, every Switch and every Select on
   the page is `disabled` (same `mutationLoading` prop). The
   `UserTable` `disabled` variable covers both controls.
5. On success, `useUpdateUser` invalidates
   `queryKeys.user.list()` (and `user.current()`); the table
   re-renders with the new role, the Select snaps to the new
   selected value.
6. On error, `updateUser.error` surfaces in the existing `<Alert>`
   slot (same path as the active-toggle failure). Rows remain
   visible.

## Edge cases

- **Self-select same role** — MUI Select's `onChange` only fires on
  value change, not on re-clicking the current option. No spurious
  mutation.
- **Promote general → admin** — works; `UpdateUserBody` already
  supports `role` on the server side
  (`src-tauri/src/http/user.rs:35-46`).
- **Demote admin → general** — works the same way; both directions
  flow through the same code.
- **No confirmation dialog** — same YAGNI call as the active Switch.
  Reversible by another admin.
- **Currently selected value can never be `"root"`** — root users
  are filtered before reaching the table. The Select still types
  as `Role` (the union includes `root`), but `e.target.value` is
  narrowed to `"admin" | "general"` by the MenuItem values.

## Error handling summary

| Failure | Surface |
|---|---|
| `update_user` throws (role change) | Same `<Alert severity="error">` slot as list / active-toggle errors. Rows remain visible. |
| `list_users` throws | Already handled — `<Alert>` with Retry. |
| `useCurrentUser` fails | Already handled — page returns `null`. |

No new error paths. The role change reuses the existing mutation's
error surface.

## i18n keys

Add to both `lib/packages/ui/src/i18n/locales/en.ts` (with
`as const`) and `zhCN.ts` (with
`satisfies Record<keyof typeof en, string>`). Both files must stay
in lock-step.

| Key | en | zh-CN |
|---|---|---|
| `user.cannotChangeOwnRole` | You cannot change your own role | 无法修改自己的角色 |

All other keys (`user.role.admin`, `user.role.general`,
`user.field.role`, `nav.management*`, etc.) already exist.

## Tests

### `src/test/pages/user-table.test.tsx`

Replace the chip-style role assertion in the "renders one row per
user" test with a Select assertion:

```ts
it("renders one Select per non-root user with the current role preselected", () => {
  renderTable({ rows: [adminUser, generalUser, adminUser2] });
  expect(screen.getByDisplayValue("Admin")).toBeInTheDocument();
  expect(screen.getByDisplayValue("General")).toBeInTheDocument();
});
```

Add new cases:

```ts
describe("UserTable — role Select", () => {
  it("Select on the self row is disabled", () => {
    renderTable({ rows: [adminUser], selfCode: "alice" });
    const select = screen.getByDisplayValue("Admin") as HTMLInputElement;
    expect(select.disabled).toBe(true);
  });

  it("Select on non-self rows is enabled", () => {
    renderTable({ rows: [adminUser, generalUser], selfCode: "alice" });
    // Two selects: alice (admin, self-disabled) and bob (general, enabled).
    const selects = Array.from(
      document.querySelectorAll('[role="combobox"]'),
    );
    expect(selects.length).toBe(2);
    expect((selects[0] as HTMLElement).getAttribute("aria-disabled")).toBe("true");
    expect((selects[1] as HTMLElement).getAttribute("aria-disabled")).toBe("false");
  });

  it("dropdown options are admin and general only (no root)", async () => {
    renderTable({ rows: [adminUser] });
    await userEvent.click(screen.getByDisplayValue("Admin"));
    expect(screen.getByRole("option", { name: "Admin" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "General" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Root" })).not.toBeInTheDocument();
  });

  it("calls onRoleChange when a different role is picked", async () => {
    const onRoleChange = vi.fn();
    renderTable({ rows: [generalUser], onRoleChange });
    await userEvent.click(screen.getByDisplayValue("General"));
    await userEvent.click(screen.getByRole("option", { name: "Admin" }));
    expect(onRoleChange).toHaveBeenCalledWith("bob", "admin");
  });

  it("disables every Select while mutationLoading is true", () => {
    renderTable({ rows: [adminUser, generalUser], mutationLoading: true });
    const selects = Array.from(
      document.querySelectorAll('[role="combobox"]'),
    ) as HTMLElement[];
    expect(
      selects.every((s) => s.getAttribute("aria-disabled") === "true"),
    ).toBe(true);
  });
});
```

The existing chip-style assertion in the basic-rendering test is
replaced by the Select assertion above; the rest of the file
(loading/empty/error states, Switch behavior, self-disable on
Switch, mutation loading on Switch, toggle interaction) is
unchanged.

### `src/test/pages/user-list.test.tsx`

Add a role-mutation case:

```ts
describe("UserListPage — role change", () => {
  it("calls update_user with { code, body: { role } }", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("bob");
    // Open bob's Select (general → admin).
    const bobSelect = screen.getByDisplayValue("General");
    await userEvent.click(bobSelect);
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

Update the existing self-disable test to assert the Select is
disabled on alice's row, in addition to the Switch:

```ts
it("disables both the Switch and the Select on the current user's row", async () => {
  await renderPage(adminUser, [adminUser, generalUser]);
  await screen.findByText("alice");
  const switches = getSwitches();
  expect(switches[0].disabled).toBe(true); // alice Switch

  const aliceSelect = screen.getByDisplayValue("Admin");
  expect(
    (aliceSelect as HTMLElement).getAttribute("aria-disabled"),
  ).toBe("true");
});
```

### `src/test/data/user.test.tsx`

Add a second harness and one more case to the `useUpdateUser`
describe block:

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

### `src/test/routes/users.test.tsx`

No new tests. The existing "shows the Management entry for an
admin" test still passes after the reorder. Optional cheap
addition: assert "Management" appears before "Settings" in DOM
order — but not required since the existing presence tests cover
both entries independently.

## Out of scope (deferred)

- Confirmation dialog before promoting / demoting. Same YAGNI call as
  the active Switch.
- A dedicated "change role" hook. `useUpdateUser` covers it.
- Allowing admins to assign / revoke the `root` role from the page.
  Today the dropdown only offers `admin` / `general`; root
  assignment is server-bootstrap only.
- Optimistic updates on the Select. Server is the source of truth
  (same as the Switch).
- Sorting, filtering, or any other UX beyond this revision.

## File changes summary

**Modified files**

- `apps/desktop/aegis-desktop/src/pages/Layout.tsx` — reorder menu
  array
- `apps/desktop/aegis-desktop/src/pages/UserTable.tsx` — replace
  Chip with Select, add `onRoleChange` prop, add Tooltip on the
  Select
- `apps/desktop/aegis-desktop/src/pages/UserList.tsx` — add
  `handleRoleChange` callback, pass it to `UserTable`
- `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx` —
  replace chip assertion with Select assertion; add 5 new
  Select-specific cases
- `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx` —
  add role-mutation case; update self-disable test
- `apps/desktop/aegis-desktop/src/test/data/user.test.tsx` — add
  `body: { role }` case to `useUpdateUser`
- `lib/packages/ui/src/i18n/locales/en.ts` — add
  `user.cannotChangeOwnRole`
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — mirror

**Untouched**

- `apps/desktop/aegis-desktop/src/api/**`
- `apps/desktop/aegis-desktop/src/data/**` (hooks)
- `apps/desktop/aegis-desktop/src/components/**`
- `apps/desktop/aegis-desktop/src/routes/**`
- `apps/desktop/aegis-desktop/src/pages/{Home,Settings,Project*,
  UserFooter,Layout}.tsx` (Layout modified for reorder only;
  layout structure unchanged)
- `apps/desktop/aegis-desktop/src/main.tsx`
- `apps/desktop/aegis-desktop/src-tauri/**`
- `apps/server/aegis-server/**`
- `lib/crates/**`
- `lib/packages/ui/src/components/**`
- All other tests, vitest config, package.json