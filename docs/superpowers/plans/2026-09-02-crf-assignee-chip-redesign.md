# CRF assignee chip redesign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Switch the CRF assignee chips and the mission-assign drawer's assignee row to a name-as-primary-label, role-as-border-style shape; switch the drawer's per-row remove button to a `DeleteIcon` with `color="error"`; drop the user code from the drawer's user-dropdown options.

**Architecture:** Three mechanical edits in two component files plus two assertion updates in one test file. No new hooks, no new i18n keys, no new components. The `useUserNameMap` hook from the previous feature is reused unchanged.

**Tech Stack:** React, `@aegis/ui/mui` (`Chip`, `Typography`, `IconButton`), `@aegis/ui/icons` (`DeleteIcon`), Vitest + Testing Library.

## Global Constraints

- Frontend-only change. No backend / wire / Rust changes.
- `useUserNameMap` hook — **do not modify**. Same fallback semantics.
- i18n keys `crf.missionAssign.roleQc` / `crf.missionAssign.roleDev` — unchanged.
- The chip's outline + border style is the role signal for both the table chip and the drawer's role chip: dev = solid border (default), qc = dashed border (`sx={{ borderStyle: "dashed" }}`). Both chips use `variant="outlined"` and `color="primary"`.
- Typecheck: `pnpm --filter aegis-desktop typecheck` must pass after every commit.
- Targeted tests: `pnpm --filter aegis-desktop exec vitest run <files>`.

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx` | modify | `AssigneeChip` — drop role text, outlined for both, dashed border for qc. |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx` | modify | Drawer dropdown label; assignees row shape (Typography + Chip + IconButton with DeleteIcon + color="error"). |
| `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx` | modify (loosen two assertions) | Two existing regexes match `${name} · ${role}`; loosen to `name` only. |

No new files. No new tests. No new i18n entries.

---

### Task 1: Update `AssigneeChip` in the CRF form table

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx:116-140`

**Interfaces:**
- Consumes: `AssigneeChip({ role, name })` — props unchanged.
- Produces: chip renders `name` only; `variant="outlined"`; qc chip keeps `sx={{ borderStyle: "dashed" }}`.

- [ ] **Step 1: Edit `AssigneeChip`**

Replace the `AssigneeChip` function body in
`apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
(lines 116–140) with:

```tsx
function AssigneeChip({
  role,
  name,
}: {
  role: "dev" | "qc";
  name: string;
}) {
  const { t } = useI18n();
  // The chip carries the assignee's display name; the role is
  // communicated by the chip's border style. Both chips are
  // outlined. QC uses a dashed border; DEV keeps the default
  // (solid).
  const isQc = role === "qc";
  return (
    <Chip
      size="small"
      label={name}
      variant="outlined"
      color="primary"
      sx={isQc ? { borderStyle: "dashed" } : undefined}
    />
  );
}
```

The `role` and `name` prop signatures are unchanged. The call site at
line 178 already passes the resolved name and the role.

- [ ] **Step 2: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: clean exit.

- [ ] **Step 3: Run the existing CRF list-page tests to confirm what
        now breaks**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/crf/crf-form-list-page.test.tsx
```

Expected: the two assertions added in the previous feature (`renders
the assignee's display name (resolved via useListUsers) on the chip`
and `falls back to userCode on the chip when list_users returns an
empty list`) **fail** with "Unable to find element with text /^Carol
Lin · /" or `/^carol · /`. The `computeNewFullOrder` block still
passes. The pre-existing "renders the heading" failure is unchanged.

This is the expected state — Task 3 loosens the assertions.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx
git commit -m "refactor(desktop): make assignee name-only with role-driven chip border"
```

---

### Task 2: Update the mission-assign drawer

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx:14-15, 198-237, 240-256`

**Interfaces:**
- Consumes: `useUserNameMap` (already imported from `../../user/data/list`).
- Produces: (a) `getOptionLabel` returns `name` only; (b) assignees row contains `<Typography>{name}</Typography>`, role `Chip`, `IconButton` with `DeleteIcon` + `color="error"`; (c) `CloseIcon` import stays (header still uses it), `DeleteIcon` is added.

- [ ] **Step 1: Add `DeleteIcon` to the icon import**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`,
replace the existing icon import (line 15):

```ts
import { Close as CloseIcon } from "@aegis/ui/icons";
```

with:

```ts
import { Close as CloseIcon, Delete as DeleteIcon } from "@aegis/ui/icons";
```

`Delete` is exported by `@aegis/ui/icons` (verified by grep on
similar imports — `Delete as DeleteIcon` is the convention used in
`CrfFormTable.tsx`).

- [ ] **Step 2: Replace the assignees list row shape**

Replace the entire `matched && matched.assignees.length > 0 ? (
<Stack …> … </Stack> ) : ( <Typography … /> )` block (lines 198–237)
with:

```tsx
<Box>
  {matched && matched.assignees.length > 0 ? (
    <Stack spacing={1}>
      {matched.assignees.map((a) => (
        <Box
          key={a.id}
          sx={{ display: "flex", alignItems: "center", gap: 1 }}
        >
          <Typography sx={{ flexGrow: 1 }}>
            {resolveName(a.userCode)}
          </Typography>
          <Chip
            size="small"
            label={
              a.role === "qc"
                ? t("crf.missionAssign.roleQc")
                : t("crf.missionAssign.roleDev")
            }
            variant="outlined"
            color="primary"
            sx={
              a.role === "qc" ? { borderStyle: "dashed" } : undefined
            }
          />
          <IconButton
            size="small"
            color="error"
            aria-label={t("crf.missionAssign.remove")}
            onClick={() => handleRemove(a.id)}
          >
            <DeleteIcon fontSize="small" />
          </IconButton>
        </Box>
      ))}
    </Stack>
  ) : (
    <Typography color="text.secondary" variant="body2">
      {t("crf.missionAssign.empty")}
    </Typography>
  )}
</Box>
```

Notes:

- The outer `<Box>` wrapping the matched/empty branch is kept so the
  empty-state rendering stays aligned with the current layout.
- The row's flex container drops `justifyContent: "space-between"`
  — `flexGrow: 1` on the `Typography` pushes the role chip and the
  delete button to the right edge.

- [ ] **Step 3: Drop the user code from the dropdown label**

Inside the `Autocomplete` JSX (around lines 240–256), change:

```tsx
getOptionLabel={(opt) => `${opt.code} — ${opt.name}`}
```

to:

```tsx
getOptionLabel={(opt) => opt.name}
```

`isOptionEqualToValue={(opt, val) => opt.code === val.code}` stays
unchanged — the value/match key is still the user `code`.

- [ ] **Step 4: Typecheck and run targeted tests**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop exec vitest run \
  src/test/features/mission/missions.test.tsx \
  src/test/features/crf/crf-form-list-page.test.tsx \
  src/test/features/user/useUserNameMap.test.tsx
```

Expected:

- typecheck clean.
- `useUserNameMap.test.tsx` — all pass (resolver unchanged).
- `missions.test.tsx` — same as before (no new failures, no new
  passes).
- `crf-form-list-page.test.tsx` — the two assertions added in the
  previous feature still fail with the same `· role` regex error;
  the rest of the file is unaffected.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx
git commit -m "refactor(desktop): split drawer assignee row into name + role chip + delete"
```

---

### Task 3: Loosen the two chip-shape assertions

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx:165, 240`

**Interfaces:**
- Consumes: the two test cases added in the previous feature.
- Produces: assertions match `name` only, since the chip no longer
  carries the `· role` suffix.

- [ ] **Step 1: Loosen the resolved-name assertion**

In `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`,
inside the test `renders the assignee's display name (resolved via
useListUsers) on the chip`, replace:

```tsx
await waitFor(() =>
  expect(screen.getByText(/^Carol Lin · /)).toBeInTheDocument(),
);
```

with:

```tsx
await waitFor(() =>
  expect(screen.getByText("Carol Lin")).toBeInTheDocument(),
);
```

- [ ] **Step 2: Loosen the fallback assertion**

Inside the test `falls back to userCode on the chip when list_users
returns an empty list`, replace:

```tsx
expect(await screen.findByText(/^carol · /)).toBeInTheDocument();
```

with:

```tsx
expect(await screen.findByText("carol")).toBeInTheDocument();
```

- [ ] **Step 3: Run the test file**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/crf/crf-form-list-page.test.tsx
```

Expected:

- The two updated tests now pass.
- `computeNewFullOrder` block still passes (no regression).
- The pre-existing `renders the heading + one form row from the
  mocked backend` failure is unchanged (unrelated to this work).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx
git commit -m "test(desktop): update chip assertions for name-only assignee chip"
```

---

### Task 4: Final verification

**Files:**
- No file changes; this task runs the verification commands.

- [ ] **Step 1: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: clean exit.

- [ ] **Step 2: Targeted test run across the 3 affected files**

```bash
pnpm --filter aegis-desktop exec vitest run \
  src/test/features/mission/missions.test.tsx \
  src/test/features/crf/crf-form-list-page.test.tsx \
  src/test/features/user/useUserNameMap.test.tsx
```

Expected: all previously-passing tests still pass. The only
remaining failure is the pre-existing
`renders the heading + one form row` test — unchanged, unrelated.

- [ ] **Step 3: Report**

Tell the user the work is complete and ready to resume the
finishing-a-development-branch flow (merge / push / keep as-is).