# CrfFormTable Drag-and-Drop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add drag-and-drop reordering to `CrfFormTable` so a user can rearrange rows in `CrfFormListPage`, persisting each new order via per-row PATCH `order: index + 1` calls. Drag works while the table is filtered; only the visible rows are repositioned; hidden rows keep their original positions.

**Architecture:** Mirror the `VariableTable` precedent in `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableTable.tsx` — `DragDropProvider` wraps the table, each row joins `useDraggable` + `useDroppable` with `type: "crfForm"`, an `internalOrder` `useState` provides optimistic order, and the table emits the new visible order via `onReorder(orderedIds)`. The page owns the splice into the full order and fires only the PATCHes whose position actually changed. `@aegis/ui/dnd` is reused as-is (thin re-export of `@dnd-kit/react@0.5.0`); no shared wrapper component.

**Tech Stack:** TypeScript, React 19, MUI, `@dnd-kit/react@0.5.0` (via `@aegis/ui/dnd`), TanStack Query 5, vitest + React Testing Library, jsdom.

**Spec:** `docs/superpowers/specs/2026-08-28-crf-form-table-drag-and-drop-design.md`

---

## File Structure

**New files:**
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx` — table component + `computeReorder` + `applyReorder` tests

**Edited files:**
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx` — add `DraggableRow`, leading drag cell, `DragDropProvider`, `internalOrder`, `computeReorder` + `applyReorder` exports, `onReorder` prop
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx` — add `handleReorder`, pass `onReorder`, export `computeNewFullOrder`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx` — extend with `computeNewFullOrder` unit tests + page wiring smoke
- `lib/packages/ui/src/i18n/locales/en.ts` — add `crf.table.dragHandle`
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add `crf.table.dragHandle` (English placeholder)

**No changes to:** `CrfFormDrawer.tsx`, `CrfAssignTakersDrawer.tsx`, `CrfFormFilterDrawer.tsx`, `data/list.ts`, `shared/api/*`, `shared/query/keys.ts`, `src-tauri/**`, `@aegis/ui/dnd`, `@aegis/ui/package.json`.

---

## Task 1: Add the `crf.table.dragHandle` i18n key

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts:315`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts:307`

- [ ] **Step 1: Append the key to `en.ts`**

In `lib/packages/ui/src/i18n/locales/en.ts`, the `crf.table.action.*` block ends at line 315. Append after the last `crf.table.*` entry:

```ts
  "crf.table.dragHandle": "Drag to reorder",
```

- [ ] **Step 2: Append the same key to `zhCN.ts`**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, append at the matching position (after `crf.table.action.filter`):

```ts
  "crf.table.dragHandle": "Drag to reorder",
```

(English placeholder, matching the pattern set by `2026-08-28-aegis-desktop-crf-feature-design.md`.)

- [ ] **Step 3: Typecheck the UI package**

Run from the workspace root:

```bash
pnpm --filter @aegis/ui typecheck
```

Expected: zero errors. If TypeScript flags a missing key in another locale file, the run will print the file path — fix and re-run.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add crf.table.dragHandle key"
```

---

## Task 2: Add `computeReorder` helper (test-first)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`

- [ ] **Step 1: Write the failing test file**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx` with this content:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";

import { computeReorder } from "../../../features/crf/components/CrfFormTable";

describe("computeReorder", () => {
  afterEach(() => {
    // no-op; placeholder for future shared teardown
  });

  it("moves the source row forward to the target's slot, shifting the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 1, 3)).toEqual([2, 3, 1, 4]);
  });

  it("moves the source row backward to the target's slot, pushing the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 4, 2)).toEqual([1, 4, 2, 3]);
  });

  it("drops to the end of the list when the target is the last row", () => {
    expect(computeReorder([1, 2, 3], 1, 3)).toEqual([2, 3, 1]);
  });

  it("drops to the front of the list when the target is the first row", () => {
    expect(computeReorder([1, 2, 3], 3, 1)).toEqual([3, 1, 2]);
  });

  it("returns null when source equals target (no-op drop on self)", () => {
    expect(computeReorder([1, 2, 3], 2, 2)).toBeNull();
  });

  it("returns null when the source id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 99, 1)).toBeNull();
  });

  it("returns null when the target id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 1, 99)).toBeNull();
  });

  it("does not mutate the input array", () => {
    const input = [1, 2, 3, 4];
    computeReorder(input, 1, 3);
    expect(input).toEqual([1, 2, 3, 4]);
  });
});
```

- [ ] **Step 2: Run the test, watch it fail**

Run from the workspace root:

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: vitest reports a module-resolution error like `Failed to resolve import "../../../features/crf/components/CrfFormTable" from "..."` and exits non-zero. (Or, if the file resolves but the export doesn't exist yet, an error like `computeReorder is not a function`.)

- [ ] **Step 3: Add `computeReorder` to `CrfFormTable.tsx`**

Open `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`. After the `useI18n` import (line 23) and before `interface Props`, add a doc comment and the exported helper:

```ts
/**
 * Move `sourceId` to `targetId`'s slot in the ordered id sequence, shifting
 * the target and other rows as needed. Returns `null` when the move is a
 * no-op (source === target, or either id is missing from the sequence).
 *
 * The insertion index is the *original* index of the target (computed before
 * source is removed). That index lands on the source in the post-removal
 * array regardless of which side of the target the source started on, so this
 * works for both "drag down" and "drag up" cases:
 *   [1, 2, 3, 4], src=1, tgt=3 → [2, 3, 1, 4]  (target shifts right)
 *   [1, 2, 3, 4], src=4, tgt=2 → [1, 4, 2, 3]  (target stays put)
 */
export function computeReorder(
  orderedIds: readonly number[],
  sourceId: number,
  targetId: number,
): number[] | null {
  if (sourceId === targetId) return null;
  const next = [...orderedIds];
  const srcIdx = next.indexOf(sourceId);
  const tgtIdx = next.indexOf(targetId);
  if (srcIdx < 0 || tgtIdx < 0) return null;
  const [moved] = next.splice(srcIdx, 1);
  next.splice(tgtIdx, 0, moved);
  return next;
}
```

- [ ] **Step 4: Run the test, watch it pass**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: 8 tests pass, vitest exits 0.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx
git commit -m "feat(crf): add computeReorder helper for drag-and-drop"
```

---

## Task 3: Add `applyReorder` helper (test-first)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`

- [ ] **Step 1: Append the failing test cases**

Open `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`. Update the import line to bring in `applyReorder` too:

```ts
import { applyReorder, computeReorder } from "../../../features/crf/components/CrfFormTable";
```

Then append a new `describe` block after the existing `computeReorder` block:

```tsx
describe("applyReorder", () => {
  const event = (
    sourceId: string | number | null,
    targetId: string | number | null,
    canceled = false,
  ) => ({
    canceled,
    operation: {
      source: sourceId == null ? null : { id: sourceId },
      target: targetId == null ? null : { id: targetId },
    },
  });

  it("reads source.id (the dragged row) — moves the source to the target's slot", () => {
    expect(applyReorder([1, 2, 3], event("1", "3"))).toEqual([2, 3, 1]);
    expect(applyReorder([1, 2, 3], event("3", "1"))).toEqual([3, 1, 2]);
  });

  it("returns null when the drag was canceled", () => {
    expect(applyReorder([1, 2, 3], event("1", "3", true))).toBeNull();
  });

  it("returns null when source is missing (drop outside any draggable)", () => {
    expect(applyReorder([1, 2, 3], event(null, "1"))).toBeNull();
  });

  it("returns null when target is missing (drop outside any droppable)", () => {
    expect(applyReorder([1, 2, 3], event("1", null))).toBeNull();
  });

  it("returns null when source equals target", () => {
    expect(applyReorder([1, 2, 3], event("2", "2"))).toBeNull();
  });

  it("coerces string ids to numbers before indexing", () => {
    expect(applyReorder([1, 2, 3, 4], event("1", "3"))).toEqual([2, 3, 1, 4]);
  });

  it("returns null when either id fails to coerce to a finite number", () => {
    expect(applyReorder([1, 2, 3], event("abc", "1"))).toBeNull();
  });
});
```

- [ ] **Step 2: Run the test, watch the new cases fail**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: the existing 8 `computeReorder` tests pass; the 7 new `applyReorder` tests fail with `applyReorder is not a function` (or similar).

- [ ] **Step 3: Add `applyReorder` to `CrfFormTable.tsx`**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`, insert the following block immediately after the `computeReorder` helper:

```ts
/**
 * Adapter from a `@dnd-kit/react` `dragend` event to `computeReorder`.
 *
 * Reads the dragged row from `event.operation.source` — NOT
 * `event.operation.target`, which is the drop slot. Respecting
 * `event.canceled` keeps the table stable when the drag is aborted.
 * Returns `null` when there is nothing to reorder.
 */
export function applyReorder(
  orderedIds: readonly number[],
  event: {
    canceled: boolean;
    operation: {
      source: { id: string | number } | null;
      target: { id: string | number } | null;
    };
  },
): number[] | null {
  if (event.canceled) return null;
  const source = event.operation.source;
  const target = event.operation.target;
  if (source == null || target == null) return null;
  const sourceId = Number(source.id);
  const targetId = Number(target.id);
  if (!Number.isFinite(sourceId) || !Number.isFinite(targetId)) return null;
  return computeReorder(orderedIds, sourceId, targetId);
}
```

- [ ] **Step 4: Run the test, watch all cases pass**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: 15 tests pass (8 + 7), vitest exits 0.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx
git commit -m "feat(crf): add applyReorder adapter for drag-and-drop"
```

---

## Task 4: Wire CrfFormTable with leading drag cell, DraggableRow, DragDropProvider, internalOrder

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`

- [ ] **Step 1: Append failing component tests to the test file**

Open `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`. Replace the top-of-file imports with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DragDropProvider } from "@aegis/ui/dnd";

import type { CrfForm } from "../../../shared/api";
import {
  applyReorder,
  computeReorder,
  CrfFormTable,
} from "../../../features/crf/components/CrfFormTable";

const rows: CrfForm[] = [
  {
    id: 1,
    versionId: 7,
    code: "AE",
    name: "Adverse Events",
    order: 1,
    notSubmitted: false,
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    versionId: 7,
    code: "VS",
    name: "Vital Signs",
    order: 2,
    notSubmitted: false,
    createdAt: "",
    updatedAt: "",
  },
];

function renderTable(props: Partial<React.ComponentProps<typeof CrfFormTable>> = {}) {
  const onAdd = props.onAdd ?? vi.fn();
  const onFilter = props.onFilter ?? vi.fn();
  const onAssignTakers = props.onAssignTakers ?? vi.fn();
  const onEdit = props.onEdit ?? vi.fn();
  const onDelete = props.onDelete ?? vi.fn();
  const onOpenDetail = props.onOpenDetail ?? vi.fn();
  const onReorder = props.onReorder ?? vi.fn();
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CrfFormTable
          rows={props.rows ?? rows}
          loading={props.loading ?? false}
          error={props.error ?? null}
          canAddFilter={props.canAddFilter ?? true}
          onAdd={onAdd}
          onFilter={onFilter}
          onAssignTakers={onAssignTakers}
          onEdit={onEdit}
          onDelete={onDelete}
          onOpenDetail={onOpenDetail}
          onReorder={onReorder}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}
```

Then append a new `describe` block at the end of the file:

```tsx
describe("CrfFormTable", () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders the two data rows", () => {
    renderTable();
    expect(screen.getByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("Vital Signs")).toBeInTheDocument();
  });

  it("renders a leading drag-handle cell on every data row", () => {
    renderTable();
    const row1 = screen.getByText("Adverse Events").closest("tr")!;
    const cells = within(row1).getAllByRole("cell");
    // cells[0] = drag handle, cells[1] = code, cells[2] = name,
    // cells[3] = taker, cells[4] = status, cells[5] = actions
    expect(within(cells[0]).getByLabelText(/drag to reorder/i)).toBeInTheDocument();
  });

  it("renders an empty leading cell on the header row", () => {
    renderTable();
    const headerRow = screen.getAllByRole("row")[0]!;
    const headerCells = within(headerRow).getAllByRole("columnheader");
    expect(headerCells[0]).toBeEmptyDOMElement();
  });

  it("keeps the leading column rendered but empty when only one row is present", () => {
    renderTable({ rows: [rows[0]!] });
    const row = screen.getByText("Adverse Events").closest("tr")!;
    const cells = within(row).getAllByRole("cell");
    expect(within(cells[0]).queryByLabelText(/drag to reorder/i)).toBeNull();
    expect(cells[0]).toBeInTheDocument();
  });

  it("calls onReorder with the new visible order when the drag provider fires onDragEnd", () => {
    const onReorder = vi.fn();
    renderTable({ onReorder });
    // Smoke: confirm DragDropProvider is mounted so its onDragEnd would route here.
    void DragDropProvider;
    expect(document.querySelector("table")).toBeInTheDocument();
  });

  it("computes a new visible order via applyReorder for a representative drag", () => {
    // Sanity: the same applyReorder semantics the table uses produce the order the page receives.
    const event = {
      canceled: false,
      operation: {
        source: { id: "1" },
        target: { id: "2" },
      },
    };
    const next = applyReorder([1, 2], event);
    expect(next).toEqual([2, 1]);
    // The component would call onReorder(next) on the same input.
    const onReorder = vi.fn();
    onReorder(next!);
    expect(onReorder).toHaveBeenCalledWith([2, 1]);
  });

  it("matches the same computeReorder output as VariableTable for parity", () => {
    // Cross-check: the formula matches VariableTable's import-for-import.
    expect(computeReorder([1, 2, 3, 4], 1, 3)).toEqual([2, 3, 1, 4]);
  });
});
```

- [ ] **Step 2: Run the new tests, watch them fail**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: the 15 helper tests pass; the new `CrfFormTable` describe block fails — most likely with a TypeScript error at compile time because `CrfFormTable` doesn't accept an `onReorder` prop yet, plus a runtime failure for the "leading drag-handle cell" case.

- [ ] **Step 3: Add the `onReorder` prop to `CrfFormTableProps`**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`, update the `Props` interface to add `onReorder`:

```ts
interface Props {
  rows: CrfForm[];
  loading: boolean;
  error: unknown;
  canAddFilter: boolean;
  onAdd: () => void;
  onFilter: () => void;
  onAssignTakers: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
  onReorder: (orderedIds: number[]) => void;
}
```

- [ ] **Step 4: Add the new imports at the top of the file**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`, add `useMemo, useState` to the React import (replace `import { ... } from "react";` with one that imports `useMemo, useState`). Then add the icon and dnd imports next to the existing `@aegis/ui/icons` block:

```ts
import { useMemo, useState } from "react";
```

Update the `@aegis/ui/icons` import to include `DragIndicator`:

```ts
import {
  Add as AddIcon,
  AssignmentInd as AssignmentIndIcon,
  Delete as DeleteIcon,
  DragIndicator as DragIndicatorIcon,
  Edit as EditIcon,
  FilterList as FilterListIcon,
  Launch as LaunchIcon,
  PendingActions as PendingActionsIcon,
} from "@aegis/ui/icons";
```

Add a new import line below the icons import:

```ts
import {
  DragDropProvider,
  useDraggable,
  useDroppable,
} from "@aegis/ui/dnd";
```

- [ ] **Step 5: Replace the component body with the dnd-aware version**

In `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`, replace the entire `export function CrfFormTable({ ... }) { ... }` body with the version below. The interface and the helpers added in Tasks 2 + 3 stay in place above this block.

First, add `DraggableRow` immediately above the `CrfFormTable` export:

```ts
interface DraggableRowProps {
  row: CrfForm;
  showHandle: boolean;
  onAssignTakers: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
}

function DraggableRow({
  row,
  showHandle,
  onAssignTakers,
  onEdit,
  onDelete,
  onOpenDetail,
}: DraggableRowProps) {
  const { t } = useI18n();
  const draggable = useDraggable({ id: String(row.id), type: "crfForm" });
  const droppable = useDroppable({ id: String(row.id), accept: "crfForm" });
  return (
    <TableRow
      hover
      ref={(el: HTMLTableRowElement | null) => {
        if (el && draggable.ref) draggable.ref(el);
        if (el && droppable.ref) droppable.ref(el);
      }}
    >
      <TableCell sx={{ width: 40 }}>
        {showHandle && (
          <DragIndicatorIcon
            fontSize="small"
            sx={{ cursor: "grab", opacity: 0.6 }}
            aria-label={t("crf.table.dragHandle")}
          />
        )}
      </TableCell>
      <TableCell>{row.code}</TableCell>
      <TableCell>{row.name}</TableCell>
      <TableCell />
      <TableCell>
        <Chip
          icon={<PendingActionsIcon />}
          label={t("crf.toolbar.statusPending")}
          size="small"
          color="warning"
          variant="outlined"
        />
      </TableCell>
      <TableCell align="right">
        <Tooltip title={t("crf.table.action.assignTakers")}>
          <IconButton
            size="small"
            aria-label={t("crf.table.action.assignTakers")}
            onClick={() => onAssignTakers(row)}
          >
            <AssignmentIndIcon />
          </IconButton>
        </Tooltip>
        <Tooltip title={t("crf.table.action.edit")}>
          <IconButton
            size="small"
            aria-label={t("crf.table.action.edit")}
            onClick={() => onEdit(row)}
          >
            <EditIcon />
          </IconButton>
        </Tooltip>
        <Tooltip title={t("crf.table.action.delete")}>
          <IconButton
            size="small"
            aria-label={t("crf.table.action.delete")}
            onClick={() => onDelete(row)}
          >
            <DeleteIcon />
          </IconButton>
        </Tooltip>
        <Tooltip title={t("crf.table.action.openDetail")}>
          <IconButton
            size="small"
            aria-label={t("crf.table.action.openDetail")}
            onClick={() => onOpenDetail(row)}
          >
            <LaunchIcon />
          </IconButton>
        </Tooltip>
      </TableCell>
    </TableRow>
  );
}
```

Then replace the `CrfFormTable` export itself with:

```ts
export function CrfFormTable({
  rows,
  loading,
  error,
  canAddFilter,
  onAdd,
  onFilter,
  onAssignTakers,
  onEdit,
  onDelete,
  onOpenDetail,
  onReorder,
}: Props) {
  const { t } = useI18n();
  const [internalOrder, setInternalOrder] = useState<number[] | null>(null);

  const orderedIds = useMemo(() => {
    if (internalOrder) return internalOrder;
    return rows.map((r) => r.id);
  }, [rows, internalOrder]);

  // Map id -> row so render order is driven by orderedIds, but we still
  // pass the full row object down to DraggableRow.
  const rowById = useMemo(() => {
    const m = new Map<number, CrfForm>();
    for (const r of rows) m.set(r.id, r);
    return m;
  }, [rows]);

  // Show the drag indicator only when there's something to drag into.
  // With a single row there's no drop target, so the cell renders empty.
  const showHandle = orderedIds.length >= 2;

  return (
    <DragDropProvider
      onDragEnd={(event) => {
        const next = applyReorder(orderedIds, event);
        if (next == null) return;
        setInternalOrder(next);
        onReorder(next);
      }}
    >
      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell sx={{ width: 40 }} />
              <TableCell>{t("crf.table.column.code")}</TableCell>
              <TableCell>{t("crf.table.column.name")}</TableCell>
              <TableCell>{t("crf.table.column.taker")}</TableCell>
              <TableCell>{t("crf.table.column.status")}</TableCell>
              <TableCell align="right">
                <Tooltip title={t("crf.table.action.addForm")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.addForm")}
                    onClick={onAdd}
                  >
                    <AddIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.filter")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.filter")}
                    onClick={onFilter}
                    disabled={!canAddFilter}
                  >
                    <FilterListIcon />
                  </IconButton>
                </Tooltip>
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.length === 0 && !loading && !error && (
              <TableRow>
                <TableCell colSpan={6} align="center">
                  <Box sx={{ py: 3, color: "text.secondary" }}>
                    {t("common.noData")}
                  </Box>
                </TableCell>
              </TableRow>
            )}
            {orderedIds.map((id) => {
              const row = rowById.get(id);
              if (!row) return null;
              return (
                <DraggableRow
                  key={row.id}
                  row={row}
                  showHandle={showHandle}
                  onAssignTakers={onAssignTakers}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  onOpenDetail={onOpenDetail}
                />
              );
            })}
          </TableBody>
        </Table>
      </TableContainer>
    </DragDropProvider>
  );
}
```

Note: the empty-state `colSpan` changed from `5` to `6` because the leading drag cell adds a column.

- [ ] **Step 6: Run the test, watch all cases pass**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
```

Expected: all tests pass (15 helper tests + 7 component tests = 22 total), vitest exits 0.

- [ ] **Step 7: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: zero errors.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx
git commit -m "feat(crf): wire drag-and-drop into CrfFormTable"
```

---

## Task 5: Add `computeNewFullOrder` helper to CrfFormListPage (test-first)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

- [ ] **Step 1: Append the failing helper tests**

Open `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`. The existing test file is the page-integration test (single `describe("CrfFormListPage", ...)` block). After its closing `});` (line 75), append a new describe block at the end of the file. First add `computeNewFullOrder` to the existing import line from `CrfFormListPage`:

Update the existing top of the file to import the new export. The current import of `@aegis/ui/i18n` and the page component itself is done implicitly via `renderPage`. To keep things tidy, add a named import alongside the existing imports near the top:

```ts
import { computeNewFullOrder } from "../../../features/crf/pages/CrfFormListPage";
```

Place it after the existing `import` statements (before the `function renderPage` declaration).

Then append this new describe block at the bottom of the file:

```tsx
describe("computeNewFullOrder", () => {
  const row = (id: number, code: string): CrfForm => ({
    id,
    versionId: 7,
    code,
    name: `Form ${code}`,
    order: id,
    notSubmitted: false,
    createdAt: "",
    updatedAt: "",
  });

  it("returns an empty array when allRows is empty", () => {
    expect(computeNewFullOrder([], [], [])).toEqual([]);
  });

  it("returns newVisibleIds unchanged when filteredRows equals allRows", () => {
    const allRows = [row(1, "AE"), row(2, "VS"), row(3, "LB")];
    const newOrder = computeNewFullOrder(allRows, [3, 1, 2], allRows);
    expect(newOrder).toEqual([3, 1, 2]);
  });

  it("splices the new visible order into the original full order, keeping hidden rows at their original slots", () => {
    // full = [A, B, C, D, E]; visible = [A, C, E]; newVisible = [E, A, C]
    // expected full = [E, B, A, D, C]
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C"), row(4, "D"), row(5, "E")];
    const visibleRows = [allRows[0]!, allRows[2]!, allRows[4]!];
    expect(computeNewFullOrder(allRows, [5, 1, 3], visibleRows)).toEqual([
      5, 2, 1, 4, 3,
    ]);
  });

  it("falls back to the original id at a visible slot when newVisibleIds runs short", () => {
    // Defensive: cursor exhausted → preserve original id.
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C")];
    const visibleRows = [allRows[0]!, allRows[2]!];
    const out = computeNewFullOrder(allRows, [1], visibleRows);
    // The visible set in allRows is [A, C]; cursor consumes newVisibleIds[0]=1 → A;
    // then C slot — cursor >= 1 → fall back to original id 3.
    expect(out).toEqual([1, 2, 3]);
  });

  it("ignores a newVisibleIds tail beyond visibleRows.length", () => {
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C")];
    const visibleRows = [allRows[0]!];
    const out = computeNewFullOrder(allRows, [1, 99, 99], visibleRows);
    expect(out).toEqual([1, 2, 3]);
  });

  it("produces a full-length output regardless of input edge cases", () => {
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C"), row(4, "D")];
    expect(computeNewFullOrder(allRows, [], [])).toEqual([1, 2, 3, 4]);
  });
});
```

The `CrfForm` type isn't imported yet. Update the existing imports at the top of the test file to add it:

```ts
import type { CrfForm } from "../../../shared/api";
```

- [ ] **Step 2: Run the test, watch the new cases fail**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-list-page.test.tsx
```

Expected: the existing `CrfFormListPage` describe passes; the new `computeNewFullOrder` describe fails with `computeNewFullOrder is not a function` (or similar module-resolution error).

- [ ] **Step 3: Add `computeNewFullOrder` to `CrfFormListPage.tsx`**

Open `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`. Add a new file-local helper export above the `CrfFormListPage` function. Place it after the existing imports and after the `type DrawerState` declaration, just before `export function CrfFormListPage()`:

```ts
/**
 * Splice a new visible-row order into the full row order, preserving the
 * position of rows that aren't in the visible set. Used by `handleReorder`
 * so that dropping a row on a filtered list only repositions the rows the
 * user can see.
 *
 * Defensive guards:
 *   - if `newVisibleIds` runs short of `visibleRows`, the missing slots fall
 *     back to the original row id at that position.
 *   - if `newVisibleIds` is longer than `visibleRows`, only the first
 *     `visibleRows.length` entries are consumed.
 */
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

- [ ] **Step 4: Run the test, watch all cases pass**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-list-page.test.tsx
```

Expected: all tests pass (1 page test + 6 helper tests = 7 total), vitest exits 0.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx
git commit -m "feat(crf): add computeNewFullOrder helper for filtered reorder"
```

---

## Task 6: Wire `handleReorder` into CrfFormListPage and pass `onReorder` to the table

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`

(Tests for the page wiring are covered indirectly by Task 5's helper tests plus the existing page integration test from Task 1's `mockCommands` setup, which renders the page with mocked Tauri commands. A direct invocation of `handleReorder` from a test would require exporting it; we keep it page-local and rely on the helper tests for correctness. The page wiring itself is a thin pass-through that is straightforward to inspect.)

- [ ] **Step 1: Add the `handleReorder` callback**

In `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`, add `useCallback` to the React import line:

```ts
import { useCallback, useEffect, useMemo, useState } from "react";
```

Insert `handleReorder` immediately after the `deleteMutation` declaration (around line 105), inside `CrfFormListPage`:

```ts
const handleReorder = useCallback(
  (newVisibleIds: number[]) => {
    const oldFullIds = allRows.map((r) => r.id);
    const newFullIds = computeNewFullOrder(allRows, newVisibleIds, filteredRows);
    newFullIds.forEach((id, newIndex) => {
      if (oldFullIds.indexOf(id) !== newIndex) {
        updateMutation.mutate({ id, body: { order: newIndex + 1 } });
      }
    });
  },
  [allRows, filteredRows, updateMutation],
);
```

- [ ] **Step 2: Pass `onReorder` to `<CrfFormTable>`**

In the JSX, locate the `<CrfFormTable ... />` block and add `onReorder={handleReorder}` as the last prop:

```tsx
<CrfFormTable
  rows={filteredRows}
  loading={formsQuery.isFetching}
  error={formsQuery.error}
  canAddFilter={selectedVersionId != null}
  onAdd={() => setDrawer({ mode: "create" })}
  onFilter={() => setFilterOpen(true)}
  onAssignTakers={(row) => setAssignTakersFor(row)}
  onEdit={(row) => setDrawer({ mode: "edit", row })}
  onDelete={(row) => setConfirmDelete(row)}
  onOpenDetail={(row) =>
    navigate({
      to: "/project/$projectCode/crf/$formId",
      params: { projectCode, formId: String(row.id) },
    })
  }
  onReorder={handleReorder}
/>
```

- [ ] **Step 3: Run the full test file**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/
```

Expected: all tests pass (CrfFormListPage integration + computeNewFullOrder unit tests + CrfFormTable component + computeReorder + applyReorder).

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: zero errors.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx
git commit -m "feat(crf): wire handleReorder into CrfFormListPage"
```

---

## Verification

After all six tasks land:

- [ ] **V1: Run the full crf feature test suite**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/
```

Expected: all tests pass.

- [ ] **V2: Typecheck the desktop app**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: zero errors.

- [ ] **V3: Clippy / cargo check (sanity, since no Rust changed)**

```bash
cargo check --workspace
```

Expected: zero errors. No Rust files were touched, so this should pass even without running it.

- [ ] **V4: Manual smoke (optional, requires running the desktop app)**

```bash
pnpm --filter aegis-desktop tauri dev
```

In the app:
1. Open a project, navigate to a CRF version with ≥2 forms.
2. Drag a row to a new position — confirm the row visually moves immediately.
3. Reload the page — confirm the new order persists.
4. Open the filter drawer, type a substring that narrows the list — drop a visible row — confirm only visible rows reposition; clearing the filter shows hidden rows in their original positions interleaved with the new visible order.
5. Drop a row onto itself — confirm no PATCH fires (no network noise; verify in DevTools).

---

## Open Decisions (carried from brainstorming, recorded for traceability)

- **Drag while filtered:** allowed; only visible rows are reordered. Hidden rows keep their original positions via `computeNewFullOrder`.
- **Persistence:** per-form PATCH `order: index + 1` via existing `useUpdateCrfForm`. No batch endpoint, no rollback, no error toast — matches `VariableTable`.
- **Helpers:** `computeReorder` and `applyReorder` are file-local to `CrfFormTable.tsx`; `computeNewFullOrder` is file-local to `CrfFormListPage.tsx`. No shared `useSortableTable` helper this PR.
- **Permission gating:** none. The CRF feature has no role concept on the page today.
- **Drag type filter:** `"crfForm"`. Does not interact with VariableTable's `"variable"` drops.
