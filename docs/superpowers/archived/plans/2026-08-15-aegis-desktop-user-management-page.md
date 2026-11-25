# User Management Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a User management page to the Aegis desktop app at `/_layout/users`, reachable from a new `Management` sidebar entry. Show a table of non-root users (code, name, role, active) and let root/admin flip the active flag via a `Switch`. Hide both the page and the sidebar entry from users whose role is not `root` or `admin`.

**Architecture:** Two page components (`UserList` orchestrator + `UserTable` presentational) backed by one new TanStack Query mutation hook (`useUpdateUser`) plus the existing `useListUsers` and `useCurrentUser`. Role gating lives in `UserList` (returns `null` for non-admin) and in the sidebar menu array (Management entry appended only when `canManage === role === "root" || role === "admin"`). Reuse the existing `api.updateUser` PATCH endpoint with `{ active: true|false }`. No server-side changes.

**Tech Stack:** Tauri desktop app, React 19 + TypeScript, TanStack Router (file-based routes via `@tanstack/router-plugin/vite`), TanStack Query v5, MUI v9 (re-exported through `@aegis/ui/mui` and `@aegis/ui/icons`), Vitest + Testing Library. i18n via `@aegis/ui/i18n` using `en.ts` (`as const`) and `zhCN.ts` (`satisfies Record<keyof typeof en, string>`).

**Spec:** [2026-08-15-aegis-desktop-user-management-page-design.md](../specs/2026-08-15-aegis-desktop-user-management-page-design.md) — read it first; tasks below reference its sections.

## Global Constraints

These constraints apply to every task. If a task contradicts a constraint, the constraint wins.

- **i18n keys must be in lock-step**: every key added to `en.ts` must be added to `zhCN.ts` in the same commit. The typecheck fails otherwise (`zhCN.ts:3` uses `satisfies Record<keyof typeof en, string>`).
- **Test mock pattern**: every test file that uses Tauri commands must call `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` at the top (hoisted per-file). Use `mockCommands({...})` from `src/test/tauri-mock.ts` and `httpError(status, code, message)` for failures.
- **File naming**: TypeScript files use PascalCase for components/pages (`UserList.tsx`, `UserTable.tsx`). Test files mirror page names (`user-list.test.tsx`, `user-table.test.tsx`).
- **No new directory under `src/pages/`** — page files stay flat like `ProjectList.tsx` and `ProjectTable.tsx` (per commit `01b0483 refactor(desktop): rename page files to UpperCamelCase`).
- **No direct `invoke` calls from pages** — pages import hooks from `../data` (barrel); the data layer imports `api` from `../api`.
- **Props interface naming**: `ComponentNameProps`, exported alongside the component, with JSDoc on every exported symbol (heavy-comment convention from `ProjectTable.tsx`).
- **Existing patterns to follow**:
  - Switch control: `<FormControlLabel control={<Switch ... />} label="" sx={{ ml: 0 }} />` (see `ProjectDrawer.tsx:238-247`, `Settings.tsx:36-41`).
  - Tooltip + disabled control: `<Tooltip><span>{control}</span></Tooltip>` (see `ProjectTable.tsx:131-143`).
  - Loading/empty/error states: `<CircularProgress />` for spinner, `<Typography color="textSecondary">` for empty, `<Alert severity="error">` for errors (see `ProjectTable.tsx:56-72, 168-172`).
  - Mutation hook: mirror `useUpdateProject` in `src/data/project.ts:67-80`.
- **No file may exceed ~250 lines** by the end of the work. If a component grows beyond that, split it.
- **Commit messages**: `<scope>(<area>): <verb> <description>` matching recent history (e.g. `feat(desktop): ...`, `feat(ui): ...`, `docs(desktop): ...`). Use `feat` for new behavior, `fix` for bugs, `refactor` for restructuring, `test` for tests-only, `docs` for docs.

---

## Task 1: Add i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: existing `en` and `zhCN` exports
- Produces: 16 new keys per file (listed below). Both files' type signatures must remain compatible — `zhCN` is `satisfies Record<keyof typeof en, string>`, so typecheck enforces parity.

**Keys to add** (one commit, both files):

```
nav.management        | nav.management.users  | common.retry
user.heading          | user.empty            | user.loadFailed
user.active           | user.inactive         | user.cannotDeactivateSelf
user.field.code       | user.field.name       | user.field.role
user.field.active     | user.role.root        | user.role.admin
user.role.general
```

Copy the English and Chinese strings verbatim from the i18n table in the spec under "i18n keys".

- [ ] **Step 1: Add keys to `en.ts`**

Edit `lib/packages/ui/src/i18n/locales/en.ts`. Add the following 16 entries just before the trailing `} as const;` on line 101:

```ts
  'nav.management': 'Management',
  'nav.management.users': 'Users',
  'common.retry': 'Retry',
  'user.heading': 'Users',
  'user.empty': 'No users yet',
  'user.loadFailed': 'Failed to load users: {message}',
  'user.active': 'Active',
  'user.inactive': 'Inactive',
  'user.cannotDeactivateSelf': 'You cannot deactivate yourself',
  'user.field.code': 'Code',
  'user.field.name': 'Name',
  'user.field.role': 'Role',
  'user.field.active': 'Active',
  'user.role.root': 'Root',
  'user.role.admin': 'Admin',
  'user.role.general': 'General',
```

- [ ] **Step 2: Add matching keys to `zhCN.ts`**

Edit `lib/packages/ui/src/i18n/locales/zhCN.ts`. Add the same 16 keys just before the trailing `} satisfies Record<keyof typeof en, string>;` on line 100:

```ts
  'nav.management': '管理',
  'nav.management.users': '用户',
  'common.retry': '重试',
  'user.heading': '用户',
  'user.empty': '暂无用户',
  'user.loadFailed': '加载用户失败：{message}',
  'user.active': '已启用',
  'user.inactive': '未启用',
  'user.cannotDeactivateSelf': '无法停用自己的账号',
  'user.field.code': '账号',
  'user.field.name': '姓名',
  'user.field.role': '角色',
  'user.field.active': '已启用',
  'user.role.root': 'Root',
  'user.role.admin': '管理员',
  'user.role.general': '普通用户',
```

- [ ] **Step 3: Typecheck**

Run: `cd lib/packages/ui && pnpm typecheck`
Expected: PASS, zero errors. If it fails, the most likely cause is a typo or missing entry — re-check the 16 keys are identical between the two files.

- [ ] **Step 4: Commit**

```bash
cd d:/project/aegis
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add user.* and nav.management* i18n keys"
```

---

## Task 2: Add `useUpdateUser` mutation hook

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/data/user.ts` (append `useUpdateUser`)
- Modify: `apps/desktop/aegis-desktop/src/data/index.ts` (re-export `useUpdateUser`)
- Modify: `apps/desktop/aegis-desktop/src/test/data/user.test.tsx` (append `useUpdateUser` tests)

**Interfaces:**
- Consumes: `api.updateUser(code, body)` from `src/api/index.ts:59-60` (returns `Promise<UserView>`), `UpdateUserBody` and `UserView` types from `src/api/types.ts`, `queryKeys.user.list()` and `queryKeys.user.current()` from `src/data/queryKeys.ts`.
- Produces: `useUpdateUser()` returning a `useMutation` whose `mutationFn` takes `{ code: string; body: UpdateUserBody }` and whose `onSuccess` invalidates `queryKeys.user.list()` and `queryKeys.user.current()`.

- [ ] **Step 1: Append failing test cases to `src/test/data/user.test.tsx`**

Open `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`. Add this block at the end of the imports section (after the `useLogout` import on line 13):

```ts
import {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useLogout,
  useRegisterUser,
  useUpdateUser,            // NEW
} from "../../data/user";
```

Then append a probe and a `describe` block at the bottom of the file:

```ts
function UpdateUserHarness() {
  const m = useUpdateUser();
  return (
    <>
      <button
        onClick={() => {
          m.mutate({ code: "bob", body: { active: false } });
        }}
      >
        toggle
      </button>
      <span data-testid="pending">{m.isPending ? "yes" : "no"}</span>
    </>
  );
}

describe("useUpdateUser", () => {
  it("invokes api.update_user with { code, body }", async () => {
    mockCommands({ update_user: () => userView });
    renderWithQueryClient(<UpdateUserHarness />);
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { active: false },
      });
    });
  });

  it("invalidates user.list and user.current on success", async () => {
    mockCommands({ update_user: () => userView });
    const { client } = renderWithQueryClient(<UpdateUserHarness />);
    // Seed both cache entries so we can assert they get cleared.
    client.setQueryData(queryKeys.user.list(), usersList);
    client.setQueryData(queryKeys.user.current(), userView);

    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith({ queryKey: queryKeys.user.list() });
      expect(spy).toHaveBeenCalledWith({ queryKey: queryKeys.user.current() });
    });
  });

  it("does not invalidate any query on error", async () => {
    mockCommands({ update_user: () => Promise.reject({ kind: "http", status: 403, code: "forbidden", message: "nope" }) });
    const { client } = renderWithQueryClient(<UpdateUserHarness />);
    client.setQueryData(queryKeys.user.list(), usersList);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", expect.anything());
    });
    expect(spy).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/data/user.test.tsx`
Expected: FAIL with "Failed to resolve import `../../data/user`" (because `useUpdateUser` does not exist yet) — Vitest reports the unresolved import on the line of the new `import` block.

- [ ] **Step 3: Add `useUpdateUser` to `src/data/user.ts`**

Open `apps/desktop/aegis-desktop/src/data/user.ts`. Add to the existing `api` import on line 3 (which currently imports `RegisterUserInput`, `RegisterUserResponse`, `Identity`, `UserView`, `ApiError`):

```ts
import { api, type ApiError, type Identity, type RegisterUserInput, type RegisterUserResponse, type UpdateUserBody, type UserView } from "../api";
```

Append at the bottom of the file:

```ts
/**
 * Update an existing user. On success: invalidates the user list cache
 * so the management page reflects the new active state on the next
 * render. Also invalidates `user.current()` since the current user's
 * own row could be the one being updated by a sibling admin and the
 * `UserFooter` reads the same cache entry.
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

- [ ] **Step 4: Add `useUpdateUser` to the data barrel**

Open `apps/desktop/aegis-desktop/src/data/index.ts`. Update the `user` re-export block (lines 11-17) to include the new hook:

```ts
export {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useRegisterUser,
  useLogout,
  useUpdateUser,
} from "./user";
```

- [ ] **Step 5: Run the new tests to verify they pass**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/data/user.test.tsx`
Expected: PASS for all `useUpdateUser` describe-block cases plus all existing tests in the file.

- [ ] **Step 6: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/data/user.ts \
        apps/desktop/aegis-desktop/src/data/index.ts \
        apps/desktop/aegis-desktop/src/test/data/user.test.tsx
git commit -m "feat(desktop): add useUpdateUser mutation hook"
```

---

## Task 3: Add `UserTable` component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/UserTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx`

**Interfaces:**
- Consumes: `UserView` and `ApiError` types from `../api`, `errorMessage` helper from `../api/error`, `useI18n` hook from `@aegis/ui/i18n`, MUI components from `@aegis/ui/mui`. No hooks from `../data` — this is a pure presentational component.
- Produces: `UserTable` component and exported `UserTableProps` interface:

```ts
export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRetry: () => void;
}
```

- [ ] **Step 1: Create the failing test file**

Create `apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx` with this content:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { UserView } from "../../api";
import { UserTable } from "../../pages/UserTable";

function renderTable(props: Partial<React.ComponentProps<typeof UserTable>> = {}) {
  const baseProps = {
    rows: [],
    loading: false,
    mutationLoading: false,
    error: null,
    selfCode: null,
    onToggle: vi.fn(),
    onRetry: vi.fn(),
  };
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <UserTable {...baseProps} {...props} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const adminUser: UserView = {
  id: 1, code: "alice", name: "Alice", role: "admin", active: true,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};
const generalUser: UserView = {
  id: 2, code: "bob", name: "Bob", role: "general", active: false,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};
const adminUser2: UserView = {
  id: 3, code: "carol", name: "Carol", role: "admin", active: true,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};

afterEach(() => cleanup());

describe("UserTable — rendering states", () => {
  it("renders the spinner when loading with no rows", () => {
    renderTable({ rows: [], loading: true });
    expect(screen.getByRole("progressbar")).toBeInTheDocument();
  });

  it("renders the empty-state copy when no rows and not loading", () => {
    renderTable({ rows: [], loading: false });
    expect(screen.getByText(/no users yet/i)).toBeInTheDocument();
  });

  it("renders the error Alert when error is set", () => {
    renderTable({ error: { kind: "http", status: 500, code: "boom", message: "boom" } });
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });

  it("calls onRetry when the Retry button is clicked", async () => {
    const onRetry = vi.fn();
    renderTable({ error: { kind: "http", status: 500, code: "boom", message: "boom" }, onRetry });
    await userEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});

describe("UserTable — rows", () => {
  it("renders one row per user with code, name, role chip, and switch", () => {
    renderTable({ rows: [adminUser, generalUser, adminUser2] });
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("Alice")).toBeInTheDocument();
    expect(screen.getByText("Admin")).toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
    expect(screen.getByText("General")).toBeInTheDocument();
    expect(screen.getByText("carol")).toBeInTheDocument();
  });

  it("reflects active=true as a checked Switch", () => {
    renderTable({ rows: [adminUser] });
    const sw = screen.getByRole("checkbox", { name: "" }) as HTMLInputElement;
    expect(sw.checked).toBe(true);
  });

  it("reflects active=false as an unchecked Switch", () => {
    renderTable({ rows: [generalUser] });
    const sw = screen.getByRole("checkbox", { name: "" }) as HTMLInputElement;
    expect(sw.checked).toBe(false);
  });
});

describe("UserTable — self-disable", () => {
  it("disables the Switch on the row whose code matches selfCode", () => {
    renderTable({ rows: [adminUser], selfCode: "alice" });
    const sw = screen.getByRole("checkbox", { name: "" }) as HTMLInputElement;
    expect(sw.disabled).toBe(true);
  });

  it("does NOT disable the Switch on other rows", () => {
    renderTable({ rows: [adminUser, generalUser], selfCode: "alice" });
    const switches = screen.getAllByRole("checkbox", { name: "" }) as HTMLInputElement[];
    expect(switches[0].disabled).toBe(true);   // alice
    expect(switches[1].disabled).toBe(false);  // bob
  });
});

describe("UserTable — mutation loading", () => {
  it("disables every Switch while mutationLoading is true", () => {
    renderTable({ rows: [adminUser, generalUser], mutationLoading: true });
    const switches = screen.getAllByRole("checkbox", { name: "" }) as HTMLInputElement[];
    expect(switches.every((s) => s.disabled)).toBe(true);
  });
});

describe("UserTable — toggle interaction", () => {
  it("calls onToggle with the row's code and the new checked value", async () => {
    const onToggle = vi.fn();
    renderTable({ rows: [generalUser], onToggle });
    const sw = screen.getByRole("checkbox", { name: "" }) as HTMLInputElement;
    await userEvent.click(sw);
    await waitFor(() => {
      expect(onToggle).toHaveBeenCalledWith("bob", true);
    });
  });
});
```

- [ ] **Step 2: Run the test file to verify it fails**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/pages/user-table.test.tsx`
Expected: FAIL with "Failed to resolve import `../../pages/UserTable`" — the file doesn't exist yet.

- [ ] **Step 3: Create `src/pages/UserTable.tsx`**

Create `apps/desktop/aegis-desktop/src/pages/UserTable.tsx` with this content:

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

- [ ] **Step 4: Run the test file to verify it passes**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/pages/user-table.test.tsx`
Expected: PASS for all 13 tests. If the Switch selector `getByRole("checkbox", { name: "" })` does not find the switch, switch the selector to `getAllByRole("checkbox")` or use a label — adjust the test to match what the actual DOM looks like (debug with `screen.debug()` if needed).

- [ ] **Step 5: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/pages/UserTable.tsx \
        apps/desktop/aegis-desktop/src/test/pages/user-table.test.tsx
git commit -m "feat(desktop): add UserTable component"
```

---

## Task 4: Add `UserList` page orchestrator

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/UserList.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx`

**Interfaces:**
- Consumes: `useCurrentUser`, `useListUsers`, `useUpdateUser` from `../data`, `UserView` from `../api`, `UserTable` from `./UserTable`, `useI18n` from `@aegis/ui/i18n`, MUI from `@aegis/ui/mui`.
- Produces: `UserListPage` named export (no props) — page orchestrator.

- [ ] **Step 1: Create the failing test file**

Create `apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx` with this content:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { UserListPage } from "../../pages/UserList";
import type { UserView } from "../../api";
import { mockCommands, httpError } from "../tauri-mock";
import { renderInRouter } from "../file-route-utils";
import { TestQueryProvider } from "../test-query-provider";

const rootUser: UserView = {
  id: 1, code: "root", name: "Root", role: "root", active: true,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};
const adminUser: UserView = {
  id: 2, code: "alice", name: "Alice", role: "admin", active: true,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};
const generalUser: UserView = {
  id: 3, code: "bob", name: "Bob", role: "general", active: false,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

async function renderPage(current: UserView, list: UserView[]) {
  mockCommands({
    current_user: () => current,
    list_users: () => list,
    update_user: () => list.find((u) => u.code === "bob") ?? generalUser,
  });
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <UserListPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("UserListPage — root filter", () => {
  it("does NOT render users whose role is root", async () => {
    await renderPage(adminUser, [rootUser, adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.queryByText("root")).not.toBeInTheDocument();
  });
});

describe("UserListPage — role gate", () => {
  it("renders nothing for a general user", async () => {
    const { container } = await renderPage(generalUser, [adminUser]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("current_user");
    });
    expect(container.textContent).not.toContain("alice");
    expect(screen.queryByText(/users/i)).not.toBeInTheDocument();
  });

  it("renders the table for an admin user", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("renders the table for a root user (viewing other non-root users)", async () => {
    await renderPage(rootUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.getByText("bob")).toBeInTheDocument();
  });
});

describe("UserListPage — toggle calls update_user", () => {
  it("calls update_user with { code, body: { active: !prev } }", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("bob");
    // Find Bob's row Switch.
    const switches = screen.getAllByRole("checkbox", { name: "" }) as HTMLInputElement[];
    const bobSwitch = switches[1]; // alice (self, disabled) is first
    await userEvent.click(bobSwitch);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { active: true },
      });
    });
  });
});

describe("UserListPage — self-disable", () => {
  it("disables the Switch on the current user's own row", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const switches = screen.getAllByRole("checkbox", { name: "" }) as HTMLInputElement[];
    expect(switches[0].disabled).toBe(true); // alice = self
    expect(switches[1].disabled).toBe(false); // bob
  });
});

describe("UserListPage — error surfaces", () => {
  it("renders an Alert when list_users fails", async () => {
    mockCommands({
      current_user: () => adminUser,
      list_users: () => Promise.reject(httpError(500, "boom", "boom")),
    });
    await renderInRouter(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <UserListPage />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test file to verify it fails**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/pages/user-list.test.tsx`
Expected: FAIL with "Failed to resolve import `../../pages/UserList`".

- [ ] **Step 3: Create `src/pages/UserList.tsx`**

Create `apps/desktop/aegis-desktop/src/pages/UserList.tsx` with this content:

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

- [ ] **Step 4: Run the test file to verify it passes**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/pages/user-list.test.tsx`
Expected: PASS for all tests. If Switch selectors behave differently than the UserTable test (because the orchestrator wraps them in extra layout), debug with `screen.debug()` and adjust the role/name selectors accordingly.

- [ ] **Step 5: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/pages/UserList.tsx \
        apps/desktop/aegis-desktop/src/test/pages/user-list.test.tsx
git commit -m "feat(desktop): add UserList page orchestrator"
```

---

## Task 5: Sidebar Management entry + `/users` route

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/Layout.tsx` (add icon imports, `useCurrentUser` import, gate Management entry by role)
- Create: `apps/desktop/aegis-desktop/src/routes/_layout/users.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/routes/users.test.tsx`

**Interfaces:**
- Consumes: `useCurrentUser` from `../data`, `MenuItem` / `SidebarProps` from `@aegis/ui`, `AppLayout` is the existing export of `Layout.tsx`.
- Produces: `Layout.tsx` still exports `AppLayout` (signature unchanged); the menu array now omits the Management entry when `canManage === false`. The new route file `routes/_layout/users.tsx` exports `Route` with `path: "/_layout/users"`.

- [ ] **Step 1: Create the failing test file**

Create `apps/desktop/aegis-desktop/src/test/routes/users.test.tsx` with this content:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { mockCommands, mockInvoke } from "../tauri-mock";
import { TestQueryProvider } from "../test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() { return data.size; },
    clear() { data.clear(); },
    getItem(key) { return data.has(key) ? data.get(key)! : null; },
    key(index) { return Array.from(data.keys())[index] ?? null; },
    removeItem(key) { data.delete(key); },
    setItem(key, value) { data.set(key, value); },
  } as unknown as Storage;
}

function renderRoot(initialEntries: string[] = ["/users"]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

const adminUser = {
  id: 1, code: "alice", name: "Alice", role: "admin", active: true,
  createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
};
const rootUser = { ...adminUser, id: 99, code: "root", name: "Root", role: "root" as const };
const generalUser = { ...adminUser, id: 2, code: "bob", name: "Bob", role: "general" as const };

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

describe("/users — sidebar gating", () => {
  it("shows the Management entry for an admin user", async () => {
    mockCommands({ is_logged_in: () => true, current_user: () => adminUser, list_users: () => [] });
    await renderRoot(["/users"]);
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("shows the Management entry for a root user", async () => {
    mockCommands({ is_logged_in: () => true, current_user: () => rootUser, list_users: () => [] });
    await renderRoot(["/users"]);
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("hides the Management entry for a general user", async () => {
    mockCommands({ is_logged_in: () => true, current_user: () => generalUser });
    await renderRoot(["/users"]);
    await screen.findByTestId("sidebar");
    expect(screen.queryByText("Management")).not.toBeInTheDocument();
  });

  it("expands the Users submenu when Management is clicked", async () => {
    mockCommands({ is_logged_in: () => true, current_user: () => adminUser, list_users: () => [] });
    await renderRoot(["/"]);
    expect(screen.queryByText("Users")).not.toBeInTheDocument();
    await userEvent.click(await screen.findByText("Management"));
    expect(await screen.findByText("Users")).toBeInTheDocument();
  });
});

describe("/users — routing", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true, current_user: () => adminUser, list_users: () => [] });
  });

  it("renders the Sidebar and the Users page at /users", async () => {
    const { router } = await renderRoot(["/users"]);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/users");
  });

  it("navigates from /settings to /users when Users submenu is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);
    await userEvent.click(await screen.findByText("Management"));
    await userEvent.click(await screen.findByText("Users"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/users"),
    );
  });

  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/users"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test file to verify it fails**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/routes/users.test.tsx`
Expected: FAIL — either because `routes/_layout/users.tsx` doesn't exist (no `/users` path in `routeTree.gen.ts`) or because the Sidebar still shows Management unconditionally. The exact failure message confirms which gap remains.

- [ ] **Step 3: Modify `src/pages/Layout.tsx`**

Open `apps/desktop/aegis-desktop/src/pages/Layout.tsx`. Replace the icon imports (lines 5-9) with:

```ts
import {
  AdminPanelSettings as AdminPanelSettingsIcon,
  Home as HomeIcon,
  People as PeopleIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
```

Update the icon constant declarations (lines 13-15) to add the two new ones:

```ts
const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;
const ManagementMenuIcon = () => <AdminPanelSettingsIcon />;
const UsersMenuIcon = () => <PeopleIcon />;
```

Add the `useCurrentUser` import after the existing imports:

```ts
import { useCurrentUser } from "../data";
```

Replace the `AppLayout` function body (lines 23-56) with this version that gates the Management entry:

```tsx
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

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          transition: "margin 0.3s",
        }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}
```

- [ ] **Step 4: Create the route file**

Create `apps/desktop/aegis-desktop/src/routes/_layout/users.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { UserListPage } from "../../pages/UserList";

export const Route = createFileRoute("/_layout/users")({
  component: UserListPage,
});
```

- [ ] **Step 5: Run the test file to verify it passes**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- src/test/routes/users.test.tsx`
Expected: PASS. If the router complains about `/users` not being a known route, restart `pnpm dev` (or run `pnpm build`) so `@tanstack/router-plugin/vite` regenerates `src/routes/routeTree.gen.ts`.

- [ ] **Step 6: Run the full test suite to confirm nothing regressed**

Run: `cd apps/desktop/aegis-desktop && pnpm test`
Expected: PASS for every test file. Existing routes tests (`projects.test.tsx`, `_layout.test.tsx`) must still pass — the sidebar entry change is additive (Management appends, never replaces).

- [ ] **Step 7: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 8: Manual smoke test (optional but recommended)**

Run: `cd apps/desktop/aegis-desktop && pnpm tauri dev`
Expected: with an admin user, the sidebar shows Management → Users; clicking Users navigates to `/users`; the table renders the seeded users without the root user; toggling a Switch updates the row after the server round-trip. With a general user, neither the Management entry nor anything at `/users` is reachable.

- [ ] **Step 9: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/pages/Layout.tsx \
        apps/desktop/aegis-desktop/src/routes/_layout/users.tsx \
        apps/desktop/aegis-desktop/src/test/routes/users.test.tsx \
        apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git commit -m "feat(desktop): add Management sidebar entry and /users route"
```

---

## Self-Review

**Spec coverage:**

- i18n keys → Task 1 ✓
- `useUpdateUser` hook → Task 2 ✓
- `UserTable` component (loading/empty/error states, self-disable, mutation-loading disable, toggle, retry) → Task 3 ✓
- `UserList` orchestrator (role gate, root filter, mutation call, error surfaces) → Task 4 ✓
- `Layout.tsx` sidebar entry with role gating → Task 5 ✓
- `/users` route file → Task 5 ✓
- Tests for everything → distributed across Tasks 2-5 ✓
- Tooltip behavior distinguishing `isSelf` from `mutationLoading` → Task 3 (component code shows the `isSelf` branch) ✓
- Hook invalidates `user.list` AND `user.current` → Task 2 test asserts both ✓

**Placeholder scan:** No "TBD", "TODO", or vague steps. Every code block contains the actual file content to write. Every test code block contains the actual test cases. The optional Step 8 of Task 5 is a smoke-test instruction, not an implementation gap.

**Type consistency:**

- `UserTableProps` defined in Task 3 with the same shape used in Task 4's orchestrator and Task 4's tests.
- `useUpdateUser` shape `{ code: string; body: UpdateUserBody }` used identically in Task 2 (hook + test), Task 3 (orchestrator call), and Task 4 (test mock).
- `errorMessage` import path `../api/error` consistent across Tasks 3 and 4.
- `queryKeys.user.list()` and `queryKeys.user.current()` referenced identically in Tasks 1-2.

**Done.**