# CRF mission-assign: show user name instead of user code

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render the assignee's display `name` (resolved from `useListUsers`) instead of `userCode` on the CRF mission-assign chips — both on the form-list table and inside the mission-assign drawer.

**Architecture:** Add a small `useUserNameMap()` hook next to `useListUsers` that returns a `(userCode) => name` resolver (with `userCode` fallback). Update the two consumer components to call the resolver at the render site. Wire shape is untouched.

**Tech Stack:** React, TanStack Query (`@tanstack/react-query`), `@aegis/ui/mui` (Chip), Vitest + Testing Library.

## Global Constraints

- Frontend-only change. No backend / wire / Rust changes.
- Wire DTO `AssigneeViewResponse` keeps its current shape (`id`, `userCode`, `role`, timestamps only) — names are resolved client-side.
- Default fallback when name is unknown: render `userCode · role` (no skeleton, no spinner).
- `useListUsers` defaults to `enabled: true` — do **not** gate the new hook's underlying query off.
- Typecheck: `pnpm --filter aegis-desktop typecheck` must pass after every commit.
- Targeted tests: `pnpm --filter aegis-desktop exec vitest run <files>`.

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/features/user/data/list.ts` | modify (add hook) | Export `useUserNameMap` next to `useListUsers`. |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx` | modify | `AssigneeChip` prop rename `userCode → name`; call site resolves via hook. |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx` | modify | Existing-assignee chip uses `resolveName(a.userCode)`. |
| `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx` | modify (extend) | Mock `list_users`, add two assertions. |

No new files. No route / query-key / i18n changes.

---

### Task 1: Add `useUserNameMap` hook

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/user/data/list.ts:1-22`
- Test: `apps/desktop/aegis-desktop/src/test/features/user/useUserNameMap.test.tsx` (new)

**Interfaces:**
- Consumes: `useListUsers()` (already in same file; returns `UserView[]`).
- Produces: `export function useUserNameMap(): (userCode: string) => string`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/user/useUserNameMap.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useUserNameMap } from "../../../features/user/data/list";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function Probe({ code }: { code: string }) {
  const resolve = useUserNameMap();
  return <div data-testid="out">{resolve(code)}</div>;
}

beforeEach(() => mockInvoke.mockReset());
afterEach(() => cleanup());

describe("useUserNameMap", () => {
  it("returns the user's name when the userCode is in the list", async () => {
    mockCommands({
      list_users: () => [
        { id: 1, code: "alice", name: "Alice Wong", role: "admin", active: true,
          createdAt: "", updatedAt: "" },
      ],
    });
    const { findByTestId } = render(
      <TestQueryProvider><Probe code="alice" /></TestQueryProvider>,
    );
    expect((await findByTestId("out")).textContent).toBe("Alice Wong");
  });

  it("falls back to the userCode when the list is empty", async () => {
    mockCommands({ list_users: () => [] });
    const { findByTestId } = render(
      <TestQueryProvider><Probe code="alice" /></TestQueryProvider>,
    );
    expect((await findByTestId("out")).textContent).toBe("alice");
  });

  it("falls back to the userCode when the userCode is not in the list", async () => {
    mockCommands({ list_users: () => [] });
    const { findByTestId } = render(
      <TestQueryProvider><Probe code="ghost" /></TestQueryProvider>,
    );
    expect((await findByTestId("out")).textContent).toBe("ghost");
  });
});
```

- [ ] **Step 2: Run the test and confirm it fails**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/user/useUserNameMap.test.tsx
```

Expected: FAIL — `useUserNameMap` is not exported yet.

- [ ] **Step 3: Implement the hook**

In `apps/desktop/aegis-desktop/src/features/user/data/list.ts`, add to the
imports at the top:

```ts
import { useCallback, useMemo } from "react";
```

(keep the existing `useMutation`, `useQuery`, `useQueryClient` imports).

Then append after the existing `useListUsers` function (after the closing `}` on line 22):

```ts
/**
 * Resolve a `userCode` to its display `name` using the cached
 * `useListUsers` query. Falls back to the userCode itself when the
 * list has not loaded yet, or the user is not in the list (e.g. the
 * user was deactivated after the mission was created).
 *
 * The lookup is intentional: assignees carry `userCode` on the wire
 * (`AssigneeViewResponse` has no `name`), and the UI needs a
 * human-readable label without changing the API.
 */
export function useUserNameMap() {
  const usersQuery = useListUsers();
  const map = useMemo(
    () => new Map(usersQuery.data?.map((u) => [u.code, u.name] as const)),
    [usersQuery.data],
  );
  return useCallback(
    (userCode: string) => map.get(userCode) ?? userCode,
    [map],
  );
}
```

- [ ] **Step 4: Run the test and confirm it passes**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/user/useUserNameMap.test.tsx
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/user/data/list.ts \
        apps/desktop/aegis-desktop/src/test/features/user/useUserNameMap.test.tsx
git commit -m "feat(desktop): add useUserNameMap hook for assignee chip labels"
```

---

### Task 2: Wire `useUserNameMap` into `CrfFormTable`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx:1-32, 116-140, 175-179, 254-282`

**Interfaces:**
- Consumes: `useUserNameMap` from `../../../features/user/data/list` (added in Task 1).
- Produces: chip label changes from `${userCode} · ${role}` to `${name} · ${role}`.

- [ ] **Step 1: Update the imports**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`:

1. Change the React import (line 1) from `import { useMemo, useState } from "react";` to
   `import { useMemo, useState } from "react";` — already correct, no change needed.

2. Add `useUserNameMap` to the existing import from `user/data/list`. Replace the
   `import type { CrfForm, MissionViewResponse } from "../../../shared/api";` line
   with:

   ```ts
   import type { CrfForm, MissionViewResponse } from "../../../shared/api";
   import { useUserNameMap } from "../../user/data/list";
   ```

- [ ] **Step 2: Update `AssigneeChip` prop signature and label**

Replace the `AssigneeChip` component (lines 116–140) with:

```tsx
function AssigneeChip({
  role,
  name,
}: {
  role: "dev" | "qc";
  name: string;
}) {
  const { t } = useI18n();
  // QC role uses a dashed outlined chip per design; DEV uses a solid
  // filled chip. `name` is the assignee's display name resolved via
  // `useUserNameMap` in the parent — falls back to the userCode when
  // the user list isn't loaded yet, so the chip stays legible.
  const isQc = role === "qc";
  return (
    <Chip
      size="small"
      label={`${name} · ${t(
        isQc ? "crf.missionAssign.roleQc" : "crf.missionAssign.roleDev",
      )}`}
      variant={isQc ? "outlined" : "filled"}
      color="primary"
      sx={isQc ? { borderStyle: "dashed" } : undefined}
    />
  );
}
```

- [ ] **Step 3: Resolve the name in `CrfFormTable` and pass it down**

Inside `CrfFormTable` (the function starting at line 240), immediately after the
`const isLeader = useIsProjectLeader(projectCode);` line, add:

```tsx
const resolveName = useUserNameMap();
```

`DraggableRow` is its own component, so it cannot read `resolveName` from
`CrfFormTable`'s closure. Add a `resolveName` field to the `DraggableRowProps`
interface (around line 105):

```tsx
interface DraggableRowProps {
  row: CrfForm;
  mission: MissionViewResponse | undefined;
  showHandle: boolean;
  canAssign: boolean;
  resolveName: (userCode: string) => string;
  onAssignTakers: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
}
```

Then update the `DraggableRow` destructuring (around line 142) to include
`resolveName`, and update the `AssigneeChip` call site (lines 175–179):

```tsx
{mission?.assignees.length ? (
  mission.assignees.map((a) => (
    <AssigneeChip key={a.id} role={a.role} name={resolveName(a.userCode)} />
  ))
) : (
  <Box sx={{ color: "text.secondary", fontSize: 12 }}>
    {t("crf.missionAssign.empty")}
  </Box>
)}
```

Finally, update the `<DraggableRow ... />` JSX call site inside the table
body (around line 355) to pass `resolveName={resolveName}`.

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: clean exit, no errors.

- [ ] **Step 5: Run the existing CRF list-page tests**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/crf/crf-form-list-page.test.tsx
```

Expected: same as before the change for the `computeNewFullOrder` block (all
pass). The `renders the heading + one form row` test was already failing for an
unrelated reason (no `heading` element on the page) — that pre-existing
failure stays. No new failures.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx
git commit -m "feat(desktop): show assignee name on CRF form-list chip"
```

---

### Task 3: Wire `useUserNameMap` into `CrfMissionAssignDrawer`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx:1-32, 59-104, 211-221`

**Interfaces:**
- Consumes: `useUserNameMap` from `../../../features/user/data/list`.
- Produces: existing-assignee chip label changes from `${a.userCode} · ${role}` to `${resolveName(a.userCode)} · ${role}`. The autocomplete picker's `code — name` label stays untouched.

- [ ] **Step 1: Add the import**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`,
add a new import line (alongside the existing `import { useAddAssignee, ... }` block):

```ts
import { useUserNameMap } from "../../user/data/list";
```

- [ ] **Step 2: Resolve the name in the drawer body**

Inside `CrfMissionAssignDrawer` (the function starting at line 59), immediately
after the `const createMission = useCreateMission(projectCode);` line (line 71),
add:

```tsx
const resolveName = useUserNameMap();
```

- [ ] **Step 3: Update the chip label**

In the existing-assignee block (lines 211–221), replace the `label` prop:

```tsx
<Chip
  size="small"
  label={`${resolveName(a.userCode)} · ${
    a.role === "qc"
      ? t("crf.missionAssign.roleQc")
      : t("crf.missionAssign.roleDev")
  }`}
  variant={a.role === "qc" ? "outlined" : "filled"}
  color="primary"
  sx={a.role === "qc" ? { borderStyle: "dashed" } : undefined}
/>
```

Do not touch the `Autocomplete`'s `getOptionLabel` (lines 240–256) — it
already shows `code — name`.

- [ ] **Step 4: Typecheck and run tests**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop exec vitest run \
  src/test/features/mission/missions.test.tsx \
  src/test/features/crf/crf-form-list-page.test.tsx \
  src/test/features/user/useUserNameMap.test.tsx
```

Expected: typecheck clean, tests pass (the same pre-existing
`crf-form-list-page` heading failure remains; everything else green).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx
git commit -m "feat(desktop): show assignee name on mission-assign drawer chip"
```

---

### Task 4: Extend integration test for resolved name + fallback

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx:1-89`

**Interfaces:**
- Consumes: existing test harness (`mockCommands`, `renderWithFullRouter`, `TestQueryProvider`).
- Produces: two new test cases — one asserting the resolved name appears, one asserting the userCode fallback when the user list is empty.

- [ ] **Step 1: Read the current test file**

```bash
cat apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx
```

Confirm the existing structure: `describe("CrfFormListPage", ...)` with one
test ("renders the heading + one form row from the mocked backend"), followed
by `describe("computeNewFullOrder", ...)`.

- [ ] **Step 2: Add the resolved-name test**

Insert a new test inside the `describe("CrfFormListPage", ...)` block, after
the existing test (line 88). Add this block:

```tsx
it("renders the assignee's display name (resolved via useListUsers) on the chip", async () => {
  mockCommands({
    is_logged_in: () => true,
    current_user: () => ({
      id: 1, code: "u", name: "U", role: "admin", active: true,
      createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
    }),
    list_crf_versions: () => ({
      versions: [{ id: 7, projectCode: "abc", name: "v1" }],
    }),
    list_crf_forms_by_version: () => ({
      forms: [
        {
          id: 11, versionId: 7, code: "AE", name: "Adverse Events",
          order: 0, notSubmitted: false,
          createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
        },
      ],
    }),
    list_missions_by_project: () => [
      {
        id: 1, projectCode: "abc", missionKind: "crf", missionCode: "AE",
        assignees: [{ id: 99, userCode: "carol", role: "qc",
          createdAt: "", updatedAt: "" }],
        createdAt: "", updatedAt: "",
      },
    ],
    list_users: () => [
      { id: 2, code: "carol", name: "Carol Lin", role: "worker", active: true,
        createdAt: "", updatedAt: "" },
    ],
    get_project_by_code: () => ({
      id: 1, code: "abc", description: "", members: { leaders: [], workers: [] },
      unblindMembers: { leaders: [], workers: [] }, tags: [], active: true,
      createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
    }),
  });

  renderPage(["/project/abc/crf?versionId=7"]);

  expect(await screen.findByText("Carol Lin")).toBeInTheDocument();
});
```

Note: `AssigneeViewResponse` requires `createdAt` and `updatedAt` per the wire
DTO — supply empty strings if you don't have meaningful values.

- [ ] **Step 3: Add the fallback test**

Append a second new test after the one added in Step 2:

```tsx
it("falls back to userCode on the chip when list_users returns an empty list", async () => {
  mockCommands({
    is_logged_in: () => true,
    current_user: () => ({
      id: 1, code: "u", name: "U", role: "admin", active: true,
      createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
    }),
    list_crf_versions: () => ({
      versions: [{ id: 7, projectCode: "abc", name: "v1" }],
    }),
    list_crf_forms_by_version: () => ({
      forms: [
        {
          id: 11, versionId: 7, code: "AE", name: "Adverse Events",
          order: 0, notSubmitted: false,
          createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
        },
      ],
    }),
    list_missions_by_project: () => [
      {
        id: 1, projectCode: "abc", missionKind: "crf", missionCode: "AE",
        assignees: [{ id: 99, userCode: "carol", role: "qc",
          createdAt: "", updatedAt: "" }],
        createdAt: "", updatedAt: "",
      },
    ],
    list_users: () => [],
    get_project_by_code: () => ({
      id: 1, code: "abc", description: "", members: { leaders: [], workers: [] },
      unblindMembers: { leaders: [], workers: [] }, tags: [], active: true,
      createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
    }),
  });

  renderPage(["/project/abc/crf?versionId=7"]);

  // The chip falls back to the userCode when `list_users` hasn't
  // resolved a name.
  expect(await screen.findByText(/^carol/)).toBeInTheDocument();
});
```

- [ ] **Step 4: Run the test file**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/crf/crf-form-list-page.test.tsx
```

Expected: the original `computeNewFullOrder` tests pass; the new
"renders the assignee's display name" test passes; the new
"falls back to userCode" test passes. The pre-existing
"renders the heading + one form row from the mocked backend" test
still fails for the unrelated heading reason.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx
git commit -m "test(desktop): cover assignee name + fallback on CRF form-list chip"
```