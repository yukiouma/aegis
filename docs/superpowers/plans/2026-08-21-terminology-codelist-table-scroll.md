# Terminology CodeListTable scrollable + sticky header Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the code-list table on `TerminologyPage` scroll within itself (instead of growing the page), pin the table header to the top of that scroll container, and ensure the `InfiniteScrollSentinel` only fires when the user scrolls *inside* the table.

**Architecture:** Two additive prop additions — `root` on `InfiniteScrollSentinel`, `bottomSlot` on `CodeListTable` — and a small page-level wire-up that moves the sentinel into the table via `bottomSlot`. The sentinel's `IntersectionObserver` now uses the table's scroll element as its `root`, so it only fires on table scroll.

**Tech Stack:** React 18, TypeScript, MUI v5 (`@aegis/ui/mui` re-exports), TanStack Query `useInfiniteQuery`, vitest + `@testing-library/react`, jsdom with an `IntersectionObserver` shim.

## Global Constraints

- `bottomSlot` and `root` are **optional** props — backward-compatible with all existing callers.
- `InfiniteScrollSentinel` must continue to default to viewport observation when `root` is `undefined`.
- `maxHeight: calc(100vh - 240px)` is the fixed value for the `TableContainer` (per user direction: viewport-relative).
- Sticky header via `<Table stickyHeader>` (MUI v5 built-in).
- Files touched:
  - `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` (modify)
  - `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx` (modify)
  - `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` (modify)
  - `apps/desktop/aegis-desktop/src/test/shared/components/InfiniteScrollSentinel.test.tsx` (modify — add cases)
  - `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-table.test.tsx` (modify — add case)
- Test command for fast feedback: `pnpm --filter aegis-desktop test -- code-list-table InfiniteScrollSentinel terminology-page-pagination`
- Final verification: `pnpm --filter aegis-desktop typecheck` and the test command above.

---

### Task 1: Extend `InfiniteScrollSentinel` with the optional `root` prop

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` (entire file)
- Modify: `apps/desktop/aegis-desktop/src/test/shared/components/InfiniteScrollSentinel.test.tsx` (add two cases; extend fake observer to capture options)

**Interfaces:**
- Consumes: nothing (this is a leaf shared component).
- Produces: `InfiniteScrollSentinelProps.root?: Element | null`. When the prop is an `Element`, it is forwarded as the `root` option of `new IntersectionObserver(...)`. When it is `null`, no observer is created.

#### Sub-task 1.1: Extend the fake `IntersectionObserver` in the test to capture options

**Step 1: Read the existing test setup**

The existing `beforeEach` in `apps/desktop/aegis-desktop/src/test/shared/components/InfiniteScrollSentinel.test.tsx` installs a fake `IntersectionObserver` whose constructor only captures the callback. We need to extend the fake so the new test can assert the `root` option that the component passed.

**Step 2: Replace the `beforeEach` block with a capturing fake**

Replace the existing `beforeEach` block in `InfiniteScrollSentinel.test.tsx` with this version that captures the second argument (the `IntersectionObserverInit` options):

```ts
let observers: Array<{
  cb: IntersectionObserverCallback;
  options: IntersectionObserverInit | undefined;
  observe: ReturnType<typeof vi.fn>;
  unobserve: ReturnType<typeof vi.fn>;
  disconnect: ReturnType<typeof vi.fn>;
}> = [];

beforeEach(() => {
  observers = [];
  const fakeObserver = class {
    cb: IntersectionObserverCallback;
    options: IntersectionObserverInit | undefined;
    observe = vi.fn();
    unobserve = vi.fn();
    disconnect = vi.fn();
    constructor(cb: IntersectionObserverCallback, options?: IntersectionObserverInit) {
      this.cb = cb;
      this.options = options;
      observers.push(this);
    }
  };
  (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver =
    fakeObserver;
});
```

The existing `afterEach` (which clears `observers`) and `fireIntersect` helper do not need to change.

#### Sub-task 1.2: Add the failing test — `forwards root to IntersectionObserver`

**Step 1: Add this case inside the existing `describe("InfiniteScrollSentinel", …)` block, after the `disconnects the observer when hasMore flips to false` case**

```ts
it("forwards root to IntersectionObserver when root is provided", () => {
  const root = document.createElement("div");
  const onIntersect = vi.fn();
  render(
    <InfiniteScrollSentinel
      onIntersect={onIntersect}
      hasMore
      loading={false}
      root={root}
    />,
  );
  expect(observers).toHaveLength(1);
  expect(observers[0].options?.root).toBe(root);
});

it("does not create an observer when root is null even if hasMore is true", () => {
  const onIntersect = vi.fn();
  render(
    <InfiniteScrollSentinel
      onIntersect={onIntersect}
      hasMore
      loading={false}
      root={null}
    />,
  );
  expect(observers).toHaveLength(0);
  expect(onIntersect).not.toHaveBeenCalled();
});
```

**Step 2: Run the test file and confirm the two new cases FAIL**

Run: `pnpm --filter aegis-desktop test -- InfiniteScrollSentinel`

Expected:
- The existing 5 cases still pass (the `root` prop is optional; the default is `undefined`, which means the observer is still created).
- The new `forwards root…` case FAILS — current code never passes `options.root` to the constructor.
- The new `does not create an observer when root is null` case FAILS — current code creates an observer even when `root` is `null`.

#### Sub-task 1.3: Implement the `root` prop in `InfiniteScrollSentinel.tsx`

**Step 1: Replace the entire file with this content**

```tsx
import { Box, CircularProgress } from "@aegis/ui/mui";
import { useEffect, useRef } from "react";

export interface InfiniteScrollSentinelProps {
  /** Called when the sentinel scrolls into view and `hasMore && !loading`. */
  onIntersect: () => void;
  /** Stop firing `onIntersect` when false. */
  hasMore: boolean;
  /** Suppress `onIntersect` while a page fetch is in flight. */
  loading: boolean;
  /** Pixel margin before the viewport edge at which the observer fires. */
  rootMargin?: string;
  /** IntersectionObserver root. When set, the observer fires based on this
   *  element's visibility instead of the viewport. Use when the sentinel
   *  lives inside a scroll container that scrolls independently of the
   *  page (e.g. a scrollable MUI TableContainer). */
  root?: Element | null;
}

/**
 * Single-pixel-high sentinel that calls `onIntersect` when it scrolls into
 * view. The parent owns `offset` and `hasMore`; this component is pure.
 *
 * When `root` is omitted, the observer uses the viewport. When `root` is an
 * `Element`, the observer fires based on visibility inside that element's
 * scroll box — pair with the sentinel being rendered *inside* the scroll
 * container. When `root` is `null`, no observer is created (handles the
 * first render before a parent's callback ref resolves).
 */
export function InfiniteScrollSentinel({
  onIntersect,
  hasMore,
  loading,
  rootMargin = "0px 0px 200px 0px",
  root,
}: InfiniteScrollSentinelProps) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasMore) return;
    if (root == null) return;
    const el = ref.current;
    if (el == null) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && !loading) {
            onIntersect();
            break;
          }
        }
      },
      { root, rootMargin },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loading, onIntersect, rootMargin, root]);

  if (!hasMore) return null;

  return (
    <Box
      ref={ref}
      sx={{
        display: "flex",
        justifyContent: "center",
        py: 1,
        minHeight: 8,
      }}
      data-testid="infinite-scroll-sentinel"
    >
      {loading ? (
        <Box data-testid="sentinel-spinner">
          <CircularProgress size={20} />
        </Box>
      ) : null}
    </Box>
  );
}
```

**Step 2: Run the test file and confirm all 7 cases PASS**

Run: `pnpm --filter aegis-desktop test -- InfiniteScrollSentinel`

Expected: all 5 existing cases + the 2 new cases pass.

**Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx \
        apps/desktop/aegis-desktop/src/test/shared/components/InfiniteScrollSentinel.test.tsx
git commit -m "feat(desktop): add optional root prop to InfiniteScrollSentinel

Lets the IntersectionObserver use a caller-provided scroll container
as its root, so the sentinel only fires on scroll inside that container.
When root is null, no observer is created; when undefined, behavior
matches today (viewport)."
```

---

### Task 2: Add `bottomSlot` + scrollable container + sticky header to `CodeListTable`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx` (whole file)
- Modify: `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-table.test.tsx` (add one case)

**Interfaces:**
- Consumes: nothing.
- Produces: `CodeListTableProps.bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode`. When provided, the table renders `bottomSlot(scrollEl)` *inside* the `TableContainer`, after the existing empty-state `Box`. The `scrollEl` is the table's scroll container element (a `<div>` rendered by `TableContainer component={Paper}`).

#### Sub-task 2.1: Add the failing test — `renders bottomSlot inside the scroll container`

**Step 1: Extend `renderTable` in `code-list-table.test.tsx` so callers can pass a `bottomSlot`**

Replace the `renderTable` helper in `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-table.test.tsx` with this version (added optional `bottomSlot`):

```ts
function renderTable(props: {
  rows: CodeListView[];
  canMutate: boolean;
  onOpen?: ReturnType<typeof vi.fn>;
  onDelete?: ReturnType<typeof vi.fn>;
  onCreate?: ReturnType<typeof vi.fn>;
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CodeListTable
          mode="list"
          rows={props.rows}
          loading={false}
          mutationLoading={false}
          error={null}
          canMutate={props.canMutate}
          onRetry={() => {}}
          onCreate={props.onCreate ?? (() => {})}
          onDelete={props.onDelete ?? (() => {})}
          onOpen={props.onOpen ?? (() => {})}
          bottomSlot={props.bottomSlot}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}
```

`ReactNode` needs to be added to the import line at the top of the test file. Replace the existing imports block:

```ts
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { CodeListTable } from
  "../../../features/terminology/components/CodeListTable";
import type { CodeListView } from "../../../shared/api";
```

**Step 2: Add a new `describe` block at the end of the file**

Append this block after the existing `describe("CodeListTable — action gating", …)`:

```ts
describe("CodeListTable — bottomSlot", () => {
  it("renders bottomSlot's output inside the scroll container", () => {
    renderTable({
      rows: [],
      canMutate: false,
      bottomSlot: () => <div data-testid="codelist-slot">sentinel here</div>,
    });

    const slot = screen.getByTestId("codelist-slot");
    // The Paper component is what TableContainer renders as; the slot must be a descendant.
    const paper = slot.closest(".MuiPaper-root");
    expect(paper).not.toBeNull();
    expect(paper).toContainElement(slot);
  });

  it("passes the scroll container element to bottomSlot", () => {
    const captured: Array<HTMLElement | null> = [];
    renderTable({
      rows: [],
      canMutate: false,
      bottomSlot: (el) => {
        captured.push(el);
        return <div data-testid="codelist-slot" />;
      },
    });

    // At least one call must receive a non-null element (post-mount).
    const nonNull = captured.find((el) => el !== null);
    expect(nonNull).toBeDefined();
    expect(nonNull!.classList.contains("MuiPaper-root")).toBe(true);
  });
});
```

**Step 3: Run the test file and confirm the two new cases FAIL**

Run: `pnpm --filter aegis-desktop test -- code-list-table`

Expected:
- Existing 5 cases still pass (no API change to required props).
- New `renders bottomSlot's output…` case FAILS — current `CodeListTable` does not render any `bottomSlot`.
- New `passes the scroll container element to bottomSlot` case FAILS — same reason.

#### Sub-task 2.2: Implement the changes in `CodeListTable.tsx`

**Step 1: Replace the entire file with this content**

```tsx
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useCallback, useState, type ReactNode } from "react";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, CodeListView } from "../../../shared/api";
import { DescriptionsCell } from "./DescriptionsCell";

/**
 * `mode: "list"` — used by the Terminology list page. Renders the
 * `+` header button and per-row launch + delete icons (each gated
 * on `canMutate`). The edit affordance lives on the detail page's
 * codelist header, not on the table.
 */
export type CodeListTableProps = {
  mode: "list";
  rows: CodeListView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  canMutate: boolean;
  onRetry: () => void;
  onCreate: () => void;
  onDelete: (row: CodeListView) => void;
  onOpen: (row: CodeListView) => void;
  emptyMessage?: string;
  /** Rendered inside the scroll container, after the Table. Receives the
   *  scroll container element so callers can wire IntersectionObserver
   *  roots that observe scroll-within-table. */
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
};

export function CodeListTable(props: CodeListTableProps) {
  const { t } = useI18n();
  const showSpinner = props.loading && props.rows.length === 0;
  const emptyMessage = props.emptyMessage ?? t("terminology.codelist.empty");

  // Capture the TableContainer's DOM element via a callback ref so we can
  // hand it to `bottomSlot`. The state update forces a re-render when the
  // element is committed, after which `bottomSlot` receives a non-null value.
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const containerRefCallback = useCallback((node: HTMLDivElement | null) => {
    setScrollEl(node);
  }, []);

  if (props.error && props.rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codelist.loadFailed", {
            message: errorMessage(props.error),
          })}
        </Alert>
        <Box>
          <Button onClick={props.onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      <TableContainer
        component={Paper}
        ref={containerRefCallback}
        sx={{ maxHeight: "calc(100vh - 240px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codelist.field.code")}</TableCell>
              <TableCell>{t("terminology.codelist.field.name")}</TableCell>
              <TableCell>
                {t("terminology.codelist.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codelist.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 110 }} align="right">
                {props.canMutate ? (
                  <Tooltip title={t("terminology.codelist.create.title")}>
                    <IconButton
                      size="small"
                      aria-label={t("terminology.codelist.create.title")}
                      onClick={props.onCreate}
                      disabled={props.mutationLoading}
                    >
                      <AddIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                ) : null}
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {props.rows.map((row) => {
              const disabled = props.mutationLoading;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <span>{row.code}</span>
                      {row.extensible && (
                        <Tooltip title={t("terminology.extensible")}>
                          <Chip label="EXT" size="small" />
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>{row.name}</TableCell>
                  <TableCell>{row.submissionValue}</TableCell>
                  <TableCell>
                    <DescriptionsCell
                      synonym={row.synonym}
                      definition={row.definition}
                      nciPreferredTerm={row.nciPreferredTerm}
                    />
                  </TableCell>
                  <TableCell align="right">
                    <Box
                      sx={{
                        display: "flex",
                        gap: 0.5,
                        justifyContent: "flex-end",
                      }}
                    >
                      <Tooltip title={t("terminology.codelist.field.code")}>
                        <IconButton
                          size="small"
                          aria-label={`open ${row.code}`}
                          onClick={() => props.onOpen(row)}
                          disabled={disabled}
                        >
                          <LaunchIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                      {props.canMutate && (
                        <Tooltip
                          title={t("terminology.action.delete.confirmTitle")}
                        >
                          <IconButton
                            size="small"
                            color="error"
                            aria-label={`delete ${row.code}`}
                            onClick={() => props.onDelete(row)}
                            disabled={disabled}
                          >
                            <DeleteIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && props.rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{emptyMessage}</Typography>
          </Box>
        )}
        {props.bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}
```

**Step 2: Run the test file and confirm all 7 cases PASS**

Run: `pnpm --filter aegis-desktop test -- code-list-table`

Expected: all 5 existing cases + the 2 new cases pass.

**Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx \
        apps/desktop/aegis-desktop/src/test/features/terminology/code-list-table.test.tsx
git commit -m "feat(desktop): make CodeListTable scrollable with sticky header + bottomSlot

- TableContainer gets maxHeight calc(100vh - 240px) so the table
  scrolls within itself; the page no longer grows with each page fetch.
- TableHead is pinned to the top of the scroll container via
  Table's stickyHeader prop.
- New optional bottomSlot render-prop receives the scroll container
  element so callers can wire an IntersectionObserver root that
  observes scroll-within-table only."
```

---

### Task 3: Wire `bottomSlot` in `TerminologyPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` (single JSX change — see diff below)

**Interfaces:**
- Consumes: `CodeListTableProps.bottomSlot` (Task 2) and `InfiniteScrollSentinelProps.root` (Task 1).
- Produces: a `CodeListTable` whose sentinel lives inside its scroll container, plus the removal of the standalone `<InfiniteScrollSentinel>` rendered below the table.

#### Sub-task 3.1: Move the sentinel into `CodeListTable` via `bottomSlot`

**Step 1: In `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`, make this single edit**

Replace this existing JSX block:

```tsx
      <CodeListTable
        mode="list"
        rows={rows}
        loading={codeListsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={error}
        canMutate={canMutate}
        onRetry={codeListsQuery.refetch}
        onCreate={() => setDrawer({ mode: "create" })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpen={(row) => {
          void navigate({
            to: "/terminology/$kind/codelists/$codelistId",
            params: { kind, codelistId: row.id },
            search:
              selectedVersionId != null
                ? { versionId: selectedVersionId }
                : undefined,
          });
        }}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codelist.noMatches")
            : t("terminology.codelist.empty")
        }
      />

      <InfiniteScrollSentinel
        onIntersect={() => void codeListsQuery.fetchNextPage()}
        hasMore={hasMore}
        loading={codeListsQuery.isFetchingNextPage}
      />
```

with:

```tsx
      <CodeListTable
        mode="list"
        rows={rows}
        loading={codeListsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={error}
        canMutate={canMutate}
        onRetry={codeListsQuery.refetch}
        onCreate={() => setDrawer({ mode: "create" })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpen={(row) => {
          void navigate({
            to: "/terminology/$kind/codelists/$codelistId",
            params: { kind, codelistId: row.id },
            search:
              selectedVersionId != null
                ? { versionId: selectedVersionId }
                : undefined,
          });
        }}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codelist.noMatches")
            : t("terminology.codelist.empty")
        }
        bottomSlot={(scrollEl) => (
          <InfiniteScrollSentinel
            root={scrollEl}
            onIntersect={() => void codeListsQuery.fetchNextPage()}
            hasMore={hasMore}
            loading={codeListsQuery.isFetchingNextPage}
          />
        )}
      />
```

The `InfiniteScrollSentinel` import at the top of the file is **kept** — the page still owns the component, just hands it to the table.

**Step 2: Verify the existing pagination test still passes**

Run: `pnpm --filter aegis-desktop test -- terminology-page-pagination`

Expected: all 4 cases pass (the existing test fakes `IntersectionObserver`, captures the callback, and manually fires it — that flow is unchanged; the sentinel is now mounted inside the table rather than below it, but the observer count is still 1 and `observers[0]` still corresponds to the sentinel).

#### Sub-task 3.2: Final commit

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx
git commit -m "feat(desktop): move InfiniteScrollSentinel into CodeListTable's scroll container

The sentinel now lives inside TableContainer (via the new bottomSlot
slot) and uses the table's scroll element as its IntersectionObserver
root. Page-level scroll no longer triggers a fetchNextPage; only
scrolling inside the table does."
```

---

### Task 4: Final verification

**Step 1: Run the full fast-feedback test set**

```bash
pnpm --filter aegis-desktop test -- code-list-table InfiniteScrollSentinel terminology-page-pagination
```

Expected: all cases pass (5 + 7 + 4 = 16 minimum across the three suites, plus any other terminology suites that don't depend on these files).

**Step 2: Run TypeScript typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: no new errors. The only TS surface changes are the optional `root` and `bottomSlot` props plus the new `useState`/`useCallback`/`ReactNode` imports in `CodeListTable.tsx` and the new `useState` import in `InfiniteScrollSentinel.test.tsx` (wait — that file uses no React imports; only the test imports `ReactNode`, see Task 2.1).

**Step 3: Commit any small follow-ups if needed**

If `typecheck` flagged anything the previous tasks missed, fix and commit it now (no separate commit message — amend or follow-up commit with a clear message).