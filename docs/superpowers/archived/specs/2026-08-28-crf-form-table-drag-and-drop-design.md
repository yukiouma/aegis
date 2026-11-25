# `CrfFormTable` — drag-and-drop reordering

Date: 2026-08-28
Status: Approved (brainstorming complete)
Feature: `crf` (aegis-desktop)
Predecessor: `2026-08-28-aegis-desktop-crf-feature-design.md`

## Summary

Add drag-and-drop reordering to the existing `CrfFormTable` on
`CrfFormListPage`. The pattern follows the only existing precedent in
the codebase — `VariableTable` on `SdtmDomainDetail`
(`2026-08-25-aegis-desktop-sdtm-domain-detail-page-design.md`).
`@aegis/ui/dnd` (a thin re-export of `@dnd-kit/react@0.5.0`) already
ships; the desktop consumes it directly. Reorder is allowed even while
the table is filtered, but only the rows currently visible are
repositioned — hidden rows keep their original positions.

## Non-goals

- Backend reorder endpoint. `PATCH /api/crf/forms/{id}` already accepts
  an `order` field, and the desktop's existing `useUpdateCrfForm` hook
  already exposes it.
- Permission gating (`canMutate` etc.). The CRF feature does not yet
  have a role concept on the page; the actions cell is visible to every
  authenticated user, matching today's behavior. Drag is too.
- Rollback on PATCH failure. Matches `VariableTable` exactly — failed
  reorders silently fall back to the next query refetch.
- An `order` number column. The visual order is the order.
- Server-side filter or pagination of forms.
- Editing `order` from a form field. Drag-and-drop owns it. The
  `CrfFormDrawer` create/edit forms do not render an `order` input
  (already true today — `order` is set client-side on create only).
- zh-CN translations of the new i18n keys. English placeholders only.

## Data source

No backend changes. Already wired:

- `GET /api/crf/versions/{version_id}/forms` → `CrfForm[]`
- `PATCH /api/crf/forms/{id}` body `{ order?, code?, name?, notSubmitted? }`
  → `CrfForm`

`CrfForm.order` is already a first-class field on the wire (both Rust
DTO `order: i32` and TS `CrfForm.order: number`). `UpdateCrfFormInput`
already types it as `order?: number`. Nothing new on the Tauri or
shared-API layer.

## Page behavior

### Filter interaction

The page already exposes a `CrfFormFilterDrawer` with `searchInput`
(debounced 300ms) and `statusSelected[]`. The filter is client-side
only — `filteredRows` is the page's filtered view of `allRows`.

**Drag is allowed while filtered.** Dropping a row produces a new
visible-order; the page splices it into the full order (see
`computeNewFullOrder` below) and PATCHes only the rows whose position
actually changed. Hidden rows keep their original orders.

Worked example: full `[A(1), B(2), C(3), D(4), E(5)]`, filter shows
`[A, C, E]`, user drops E above A → new visible order `[E, A, C]`.
Resulting full order `[E, B, A, D, C]`. PATCHes fire for A, C, E only
(positions changed); B, D untouched.

### Render order

The table renders the same six columns as today, with a new leading
column prepended:

| (drag handle) | code | name | taker | status | (actions) |
|---|---|---|---|---|---|

The leading column is **always rendered** (an empty cell with no
icon when the row count is `< 2`, since there is nothing to drop into).
The actions cell is unchanged.

### Drag source

The whole `<TableRow>` is both `useDraggable` and `useDroppable`,
matching `VariableTable`'s `DraggableRow`. The leading column shows
`DragIndicator` from `@aegis/ui/icons` with `cursor: "grab"` and
`opacity: 0.6` as a visual affordance — the row itself is the drag
source, not the icon.

Drag type / accept: `"crfForm"` (mirrors `"variable"` on
`VariableTable`). CrfFormTable's drags never interact with the variable
table's drops.

### Optimistic order

Mirror `VariableTable`'s pattern:

```ts
const [internalOrder, setInternalOrder] = useState<number[] | null>(null);

const orderedIds = useMemo(() => {
  if (internalOrder) return internalOrder;
  return rows.map((r) => r.id);
}, [rows, internalOrder]);
```

On `onDragEnd`, the table computes `applyReorder(orderedIds, event)`,
sets `internalOrder` to the result, and fires `onReorder(newVisibleIds)`
to the page. The page then computes the new full order, diffs against
the current order, and fires one PATCH per row whose position changed.

`internalOrder` resets to `null` automatically when the underlying
`rows` array changes reference (e.g. version change, mutation
success invalidates the list query) — `useMemo` deps make this fall
through to the server's `rows.map((r) => r.id)` truth.

## Components

### `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`

Existing presentational table, extended in place.

**Props delta:**

```ts
interface Props {
  // ...existing 9 props (rows, loading, error, canAddFilter, onAdd,
  //                       onFilter, onAssignTakers, onEdit, onDelete,
  //                       onOpenDetail)
  onReorder: (orderedIds: number[]) => void;  // NEW
}
```

**New imports:**

```ts
import { DragIndicator as DragIndicatorIcon } from "@aegis/ui/icons";
import {
  DragDropProvider,
  useDraggable,
  useDroppable,
} from "@aegis/ui/dnd";
```

**New internal component `DraggableRow`:**

Identical structure to `VariableTable`'s `DraggableRow`, swapped for
`CrfForm` and `type: "crfForm"`. Composes both refs onto the
`<TableRow>` element (single ref callback).

**Top-level table:**

Wrap the existing `<TableContainer>` with
`<DragDropProvider onDragEnd={...}>` and render `orderedIds.map(id =>
...)` instead of `rows.map(row => ...)`. The header gains a leading
empty `<TableCell sx={{ width: 40 }} />` so the column count stays
consistent with the rows.

**File-local exported helpers (for tests):**

```ts
export function computeReorder(
  orderedIds: number[],
  sourceId: number,
  targetId: number,
): number[] | null

export function applyReorder(
  orderedIds: number[],
  event: { operation: { source: { id: string | number } }; canceled: boolean },
): number[] | null
```

Direct port of `VariableTable`'s implementations. `applyReorder`
coerces `event.operation.source.id` to a number via `Number(...)` so
`String(row.id)` on the drag side matches the `number[]` typed
`orderedIds` on the compute side.

### `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`

**New file-local exported helper (for tests):**

```ts
export function computeNewFullOrder(
  allRows: CrfForm[],
  newVisibleIds: number[],
  visibleRows: CrfForm[],
): number[] {
  const visibleIds = new Set(visibleRows.map((r) => r.id));
  const out: number[] = [];
  let cursor = 0;
  for (const row of allRows) {
    if (visibleIds.has(row.id)) {
      // Cursor exhausted → fall back to the original id at this slot.
      // Defensive: the page should always pass a same-length array.
      const id =
        cursor < newVisibleIds.length
          ? newVisibleIds[cursor++]
          : row.id;
      out.push(id);
    } else {
      out.push(row.id);
    }
  }
  return out;
}
```

Defensive guards: if `newVisibleIds.length < visibleRows.length`,
the missing slots fall back to the row's original id (output length
matches `allRows.length` regardless). If
`newVisibleIds.length > visibleRows.length`, only the first
`visibleRows.length` entries are consumed; the tail is ignored.

**New page handler:**

```ts
const handleReorder = useCallback((newVisibleIds: number[]) => {
  const oldFullIds = allRows.map((r) => r.id);
  const newFullIds = computeNewFullOrder(allRows, newVisibleIds, filteredRows);
  newFullIds.forEach((id, newIndex) => {
    if (oldFullIds.indexOf(id) !== newIndex) {
      updateMutation.mutate({ id, body: { order: newIndex + 1 } });
    }
  });
}, [allRows, filteredRows, updateMutation]);
```

PATCHes fire **only for rows whose position actually changed**. Hidden
rows that stay in their original slots are not touched. PATCHes are
fired in parallel — TanStack Query invalidates
`queryKeys.crf.formsByVersion(selectedVersionId)` on each
`useUpdateCrfForm` success, so the table re-renders to truth.

The existing `input.order = allRows.length + 1` on create stays
unchanged — new rows still append to the end. No change to the create
drawer.

### Reused components

None. `CrfFormTable` only grows; no drawer / dialog / chip changes.

## i18n additions

Append to `lib/packages/ui/src/i18n/locales/en.ts` (and `zhCN.ts`
with English placeholders, matching the existing pattern):

```ts
"crf.table.dragHandle": "Drag to reorder",
```

Used as the `aria-label` on the leading drag cell. No other new keys
— the actions cell uses existing `crf.table.action.*` keys unchanged.

## Testing

Tests live under
`apps/desktop/aegis-desktop/src/test/features/crf/`. The test file
naming convention is `{component-name}.test.tsx`, matching
`test/features/domain-model/`.

### `crf-form-table.test.tsx` (new — helpers + table integration)

Pure-function cases for `computeReorder`:

- Empty `orderedIds` → returns `null`.
- Source === target → returns `null`.
- Unknown source id → returns `null`.
- Unknown target id → returns `null`.
- Move first to end / end to first / middle swap — verify splice.

Pure-function cases for `applyReorder`:

- `event.canceled === true` → returns `null`.
- `event.operation.source.id` is a number string `"3"` → dispatches to
  `computeReorder(..., 3, target)`.
- `event.operation.source.id` is a number `3` (some dnd-kit versions
  pass native ids) → coerce via `Number(...)`, dispatches the same way.

Table integration cases:

- Renders the six columns including the new leading drag cell.
- `DragIndicatorIcon` is present in every row's leading cell.
- Calling the drag-end callback with `[2,1,3]` from the test harness
  fires `onReorder([2,1,3])` exactly.
- `onReorder` is not called when `applyReorder` returns `null` (self
  drop, canceled).

### `crf-form-list-page.test.tsx` (new — page integration)

Renders the page with `QueryClientProvider`, TanStack Router test
adapter, and mocked `useListCrfForms` / `useUpdateCrfForm` /
`useCreateCrfForm`. Cover:

- The leading drag cell is rendered when rows are present.
- Drag with no filter → `useUpdateCrfForm` mock receives N PATCHes,
  one per row, with sequential `order` values 1..N.
- Drag with a filter active → only rows whose position changed get
  PATCHes; hidden rows are not PATCHed; PATCHes use sequential
  positions in the full list (e.g. `[E,A,C]` with hidden `[B,D]` →
  PATCH `{id: E, order: 1}`, `{id: A, order: 3}`, `{id: C, order: 5}`).
  The drag-handle cell remains rendered and `onReorder` still fires.
- `internalOrder` resets to server truth after the list query is
  invalidated (drop, mutate succeeds, table re-renders in the new
  order without flicker).

### `crf-form-list-page.test.tsx` — helper unit cases

Pure-function cases for `computeNewFullOrder`:

- Empty `allRows` → empty result.
- `filteredRows === allRows` → result equals `newVisibleIds` exactly.
- Visible subset reordered, hidden set interleaved — verify hidden ids
  appear at their original slot indices.
- `newVisibleIds.length < visibleRows.length` — cursor walk truncates
  cleanly, no `undefined` reads.
- `newVisibleIds.length > visibleRows.length` — cursor walk consumes
  the first `visibleRows.length` entries only.

### dnd-kit + jsdom shim

Reused as-is from `apps/desktop/aegis-desktop/src/test/helpers/setup.ts`
(provides `ResizeObserver` and `PointerEvent` shims). The same shim
is already exercised by `test/features/domain-model/variable-table.test.tsx`
— copy-paste the import, no changes needed.

## File-by-file change list

New files:

- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

Edited files:

- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
  — add `onReorder` prop, `DraggableRow` internal component, leading
    drag cell, `DragDropProvider` wrapper, `internalOrder` state,
    `computeReorder` / `applyReorder` file-local exports
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`
  — add `handleReorder`, pass `onReorder` to `CrfFormTable`,
    `computeNewFullOrder` file-local export
- `lib/packages/ui/src/i18n/locales/en.ts` — append `crf.table.dragHandle`
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — append `crf.table.dragHandle`
  with English placeholder

No changes to:

- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormFilterDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/data/list.ts`
- `apps/desktop/aegis-desktop/src/shared/api/*`
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- `apps/desktop/aegis-desktop/src-tauri/**`
- `lib/packages/ui/src/dnd/index.ts`
- `lib/packages/ui/package.json`

## Open decisions resolved during brainstorming

- Drag-and-drop persistence: per-form PATCH `order: index + 1`, one
  PATCH per row whose position actually changed. No batch endpoint, no
  rollback, no error toast — matches `VariableTable`.
- `@aegis/ui/dnd` is reused as-is. No new wrapper component.
- `computeReorder` / `applyReorder` are copied into
  `CrfFormTable.tsx` (file-local, exported for tests). No shared
  `useSortableTable` helper — that's a future refactor, not this PR.
- Filter behavior: drag stays enabled while filtered; only visible
  rows are reordered; hidden rows keep their original positions. The
  page does the splice via `computeNewFullOrder`.
- Drag handle is a leading column with `DragIndicatorIcon` and
  `cursor: "grab"` (matches `VariableTable`).
- No new i18n keys beyond one: `crf.table.dragHandle` for the
  `aria-label`.
- No `canMutate` gating. The CRF feature has no role concept on the
  page today; adding one is out of scope.
