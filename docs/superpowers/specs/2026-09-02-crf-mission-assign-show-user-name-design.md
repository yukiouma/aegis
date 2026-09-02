# 2026-09-02 — CRF mission-assign: show user name instead of user code

> Status: design approved, pending implementation.
>
> Scope: in the CRF mission-assign flow, render the assignee's display
> `name` (sourced from `useListUsers`) instead of their `userCode`.
> Applies to both the assignee chip on the CRF form-list table and the
> existing-assignee chip inside the mission-assign drawer.

## 1. Goal and non-goals

**Goal.** Render a user's display name on the CRF mission-assign chips
so leaders can recognize assignees without memorizing codes. The wire
DTO `AssigneeViewResponse` carries only `userCode` — names are
resolved client-side by looking up `userCode` against the result of
`useListUsers`.

**Non-goals.**

- Changing the wire shape (`AssigneeViewResponse`). The lookup is
  client-side; the backend is untouched.
- Showing names for non-assignee identifiers (project members list,
  the existing autocomplete picker in the drawer already shows
  `code — name` and stays as-is).
- Cross-project name resolution. `useListUsers` returns all users; the
  lookup is global, not scoped to the active project. A user that
  leaves a project remains resolvable by `userCode` so historical
  chips keep showing their name.
- A new `useUserDirectory` abstraction beyond what's needed for this
  one feature.

## 2. Where the chips live today

The `userCode` rendering sits in exactly two places (full
`userCode → name` mapping list):

| File | Line(s) | Current label |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx` | 116–140 (`AssigneeChip`), 177–179 (call site) | `${userCode} · ${role}` |
| `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx` | 211–221 (chip), 199–231 (block) | `${a.userCode} · ${role}` |

The drawer's autocomplete picker (lines 240–256) already renders
`code — name` and is **not** in scope.

## 3. Lookup hook

Add a small companion to `useListUsers` in
`apps/desktop/aegis-desktop/src/features/user/data/list.ts`:

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
    () => new Map(usersQuery.data?.map((u) => [u.code, u.name])),
    [usersQuery.data],
  );
  return useCallback(
    (userCode: string) => map.get(userCode) ?? userCode,
    [map],
  );
}
```

Notes:

- Reuses the existing `queryKeys.user.list()` cache key, so if the
  user list is already loaded by another authed route (e.g. user
  management), this is a free lookup.
- `useListUsers` defaults to `enabled: true`; the lookup fires on
  first mount. If the call fails, `data` is `undefined`, the map is
  empty, and every chip falls back to `userCode` — the same behavior
  as "not yet loaded."
- The hook is co-located with `useListUsers` because it is its
  one-purpose consumer. A more general `useUserDirectory` would be
  YAGNI today.

## 4. Consumer edits

### 4.1 `CrfFormTable.tsx`

- `AssigneeChip` (line 116) changes its prop from `userCode: string`
  to `name: string`. The chip keeps rendering `${name} · ${role}` —
  the only difference is the prop name and that the caller resolves
  the name first.
- The `DraggableRow` call site (lines 177–179) gains one line:
  resolve the name via `useUserNameMap()` (called inside
  `CrfFormTable`, not inside `DraggableRow`, to keep the hook count
  per render stable) and pass the resolved name to `AssigneeChip`.

### 4.2 `CrfMissionAssignDrawer.tsx`

- The chip at lines 211–221 swaps `${a.userCode}` for
  `${resolveName(a.userCode)}`, using the same `useUserNameMap()`
  hook.
- The autocomplete picker's label stays `code — name` (unchanged).

### 4.3 `CrfFormListPage.tsx`

No change. The page already wires `missions` and the drawer props
through; it does not render assignees itself.

## 5. Fallback semantics

| State | Rendered label |
| --- | --- |
| `useListUsers` resolved and `userCode` is in the map | `name · role` |
| `useListUsers` loading (data undefined) | `userCode · role` |
| `useListUsers` resolved but `userCode` not in the list | `userCode · role` |
| `useListUsers` errored | `userCode · role` |

The chip must remain legible in every state. The fallback is silent
(no skeleton, no spinner) because (a) the list typically resolves on
the first paint and (b) drawing attention to "name not yet known" is
worse than briefly showing the code.

## 6. Testing

Add to `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`:

1. Mock `list_users` to return two `UserView` records (one matching
   the assignee `userCode`, one not) and assert the assignee chip
   shows the resolved `name`.
2. Mock `list_users` to return `[]` and assert the chip still shows
   the original `userCode` (fallback path).

The existing `crf-form-list-page.test.tsx` already mocks
`list_missions_by_project`, `list_crf_versions`, `list_crf_forms_by_version`,
and `get_project_by_code` — the new mock slots in alongside.

The `useListUsers` test surface lives behind the same
`mockCommands` helper used by the existing mission/CRF tests; no new
test infrastructure is needed.

## 7. Migration / rollout

Single PR, no schema migration, no feature flag. The change is
self-contained: the hook is added, both consumer files are edited,
tests are extended. The `userCode` fallback keeps the change safe to
ship without coordination with the user-management surface.

## 8. Out-of-scope follow-ups (not in this PR)

- A `name: string` field on `AssigneeViewResponse` to remove the
  client-side join. Would require server-side changes in the mission
  crate and is deferred until a second surface needs it.
- Showing the user's name in the assignee chip's tooltip / hover
  state, separate from the label.
- i18n of user names (currently shown as stored).