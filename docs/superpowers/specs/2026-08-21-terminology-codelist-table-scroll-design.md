# Aegis Desktop — Terminology CodeListTable scrollable + sticky header — Design

**Date:** 2026-08-21
**Status:** Approved (pending spec review)
**Scope:** Make the code-list table on `TerminologyPage` scroll within itself (so the page doesn't grow unboundedly with each page fetch), pin the table header to the top of that scroll container, and ensure the `InfiniteScrollSentinel` only fires when the user scrolls the table — not the page.

Out of scope (per user direction): applying the same change to `CodeItemTable` / `CodeListDetailPage`.

---

## 1. Goals

1. The code-list table on `TerminologyPage` lives inside a scrollable container with `maxHeight: calc(100vh - 240px)` and `overflow: auto` — the table itself scrolls, the page does not.
2. The `TableHead` is sticky: it stays visible at the top of the scroll container while the user scrolls through rows.
3. The `InfiniteScrollSentinel` triggers `fetchNextPage` only when the user scrolls *within* the table, not when the page scrolls.
4. No regressions in the existing pagination test (`terminology-page-pagination.test.tsx`) or in `InfiniteScrollSentinel.test.tsx` / `code-list-table.test.tsx`.
5. The new behavior is additive — `CodeItemTable` and any future caller that doesn't supply a sentinel slot still works.

---

## 2. Architecture

```
TerminologyPage
  └─ useInfiniteQuery(codeLists)              ← unchanged
  └─ CodeListTable
        ├─ TableContainer (maxHeight, overflow auto) ← new: scroll root
        │ ├─ Table (stickyHeader)                    ← new: sticky thead
        │    ├─ TableHead                            │ TableBody (rows)
        │ ├─ Empty-state Box (when rows.length === 0)
        │ └─ InfiniteScrollSentinel (root=container) ← moved here, INSIDE
        └─ (sentinel no longer rendered outside)     ← removed
```

The page still owns the `useInfiniteQuery` and calls `fetchNextPage`. The table is now the scroll container, and the sentinel becomes its last child, observing the container rather than the viewport.

---

## 3. `InfiniteScrollSentinel` — additive `root` prop

### 3.1 New prop

```ts
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
```

### 3.2 Behavior

- `root` is **optional and backwards-compatible**. When `undefined`, the observer uses the viewport (current behavior — `CodeListDetailPage` and any future caller without a root still works).
- When `root` is an `Element`, it is forwarded as the `root` option of `new IntersectionObserver(cb, { root, rootMargin })`.
- When `root` is `null`, the effect short-circuits and creates no observer. This handles the first render before the parent callback ref resolves.
- Default `rootMargin` stays `"0px 0px 200px 0px"`.
- `useEffect` dependency array grows from `[hasMore, loading, onIntersect, rootMargin]` to `[hasMore, loading, onIntersect, rootMargin, root]`. The effect body adds an `if (root == null) return;` guard next to the existing `if (!hasMore) return;`.

### 3.3 Render output — unchanged

The component still returns `null` when `!hasMore`, and otherwise renders the same `Box` with `data-testid="infinite-scroll-sentinel"` plus the optional spinner. No markup change.

---

## 4. `CodeListTable` — `bottomSlot` slot + scrollable container + sticky header

### 4.1 New prop

```ts
export type CodeListTableProps = {
  /* …existing props… */
  /** Rendered inside the scroll container, after the Table. Receives the
   *  scroll container element so callers can wire IntersectionObserver
   *  roots that observe scroll-within-table. */
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
};
```

### 4.2 Behavior

1. The component creates a callback ref for the `TableContainer`'s DOM element and stores the element in `useState<HTMLDivElement | null>(null)`. The state update forces a re-render once the element is committed, so `bottomSlot` receives a non-null value on the second render.
2. `TableContainer` receives `sx={{ maxHeight: "calc(100vh - 240px)" }}`. `overflow` is auto by default for `TableContainer` — no explicit prop needed.
3. `<Table size="small" stickyHeader>` — adds `stickyHeader` so the `TableHead` pins to the top of the scroll container.
4. After the existing empty-state `Box`, `{bottomSlot?.(scrollEl)}` is rendered inside the `TableContainer`. The empty-state box and the slot are siblings of the `Table` — both inside the scroll container, so they participate in scrolling.

`bottomSlot` is optional. Other callers (currently none outside the test suite) keep working without it.

### 4.3 Render output — diff sketch

```diff
   return (
     <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
       {showSpinner && ( /* unchanged */ )}

       <TableContainer
         component={Paper}
+        ref={containerRefCallback}
+        sx={{ maxHeight: "calc(100vh - 240px)" }}
       >
-        <Table size="small">
+        <Table size="small" stickyHeader>
           <TableHead>…</TableHead>
           <TableBody>…</TableBody>
         </Table>
         {!showSpinner && props.rows.length === 0 && (
           <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
             <Typography color="textSecondary">{emptyMessage}</Typography>
           </Box>
         )}
+        {bottomSlot?.(scrollEl)}
       </TableContainer>
     </Box>
   );
```

The new state + callback ref:

```ts
const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
const containerRefCallback = useCallback((node: HTMLDivElement | null) => {
  setScrollEl(node);
}, []);
```

`ReactNode` is added to the import list (from `"react"`).

---

## 5. `TerminologyPage` — wire the slot, remove the external sentinel

### 5.1 Change

The page currently renders `<InfiniteScrollSentinel ... />` as a sibling of `<CodeListTable />`. Replace it with a `bottomSlot` passed to `CodeListTable`:

```diff
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
     onOpen={(row) => { /* unchanged */ }}
     emptyMessage={
       trimmedQuery
         ? t("terminology.codelist.noMatches")
         : t("terminology.codelist.empty")
     }
+    bottomSlot={(scrollEl) => (
+      <InfiniteScrollSentinel
+        root={scrollEl}
+        onIntersect={() => void codeListsQuery.fetchNextPage()}
+        hasMore={hasMore}
+        loading={codeListsQuery.isFetchingNextPage}
+      />
+    )}
   />

-  <InfiniteScrollSentinel
-    onIntersect={() => void codeListsQuery.fetchNextPage()}
-    hasMore={hasMore}
-    loading={codeListsQuery.isFetchingNextPage}
-  />
```

The `InfiniteScrollSentinel` import in `TerminologyPage.tsx` is **kept** — the page still owns the component, just hands it to the table.

### 5.2 Why the sentinel can still see the scroll

With `root={scrollEl}` and the sentinel living *inside* the `TableContainer`, the `IntersectionObserver` watches the sentinel's intersection with the table's scroll box. Scrolling the page does not move the sentinel relative to the scroll box, so `isIntersecting` does not flip and `onIntersect` is not called.

---

## 6. Files changed

| File | Change |
| --- | --- |
| `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` | Add optional `root?: Element \| null` prop; pass it to `IntersectionObserver`; add it to the `useEffect` deps; add `if (root == null) return;` guard. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx` | Add optional `bottomSlot?: (scrollEl: HTMLElement \| null) => ReactNode` prop; add callback-ref + `useState` for the scroll element; add `maxHeight: "calc(100vh - 240px)"` to `TableContainer`; add `stickyHeader` to `Table`; render `{bottomSlot?.(scrollEl)}` inside `TableContainer`. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` | Move the `<InfiniteScrollSentinel>` into `<CodeListTable bottomSlot={…} />`; pass `root={scrollEl}`. |
| `apps/desktop/aegis-desktop/src/test/shared/components/InfiniteScrollSentinel.test.tsx` | Add `forwards root to IntersectionObserver` and `does not create an observer when root is null` cases. |
| `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-table.test.tsx` | Add `renders bottomSlot inside the scroll container` case. |

No changes to `CodeItemTable.tsx`, `CodeListDetailPage.tsx`, or any backend/wire code.

---

## 7. Testing

### 7.1 Existing tests — must stay green

- `code-list-table.test.tsx` — checks action gating (open / delete / create header); existing assertions keep passing. The new `bottomSlot` prop is optional and not supplied in these tests.
- `infinite-scroll-sentinel.test.tsx` — existing cases ignore the second observer arg, so the new `root` option (default `undefined`) is transparent.
- `terminology-page-pagination.test.tsx` — the fake observer captures the callback regardless of DOM position; firing `observers[0].cb(...)` still drives `fetchNextPage`. The number of observers created (1) is unchanged.

### 7.2 New tests

`infinite-scroll-sentinel.test.tsx`:

- `forwards root to IntersectionObserver` — pass an actual `HTMLDivElement` as `root`, assert the fake observer was constructed with `{ root, rootMargin }` matching what the component passed. Requires the existing test setup to capture the options argument (extend the fake `IntersectionObserver` class to record `options`).
- `does not create an observer when root is null` — render with `root={null}` and `hasMore`, assert `observers.length === 0` and `onIntersect` is not called.

`code-list-table.test.tsx`:

- `renders bottomSlot inside the scroll container` — pass `bottomSlot={() => <div data-testid="codelist-slot" />}`; assert the `data-testid="codelist-slot"` element is a descendant of the `Paper` (the `TableContainer` component).

### 7.3 Verification commands

- `pnpm --filter aegis-desktop test -- code-list-table InfiniteScrollSentinel terminology-page-pagination` — all green.
- `pnpm --filter aegis-desktop typecheck` — no new TS errors.

---

## 8. Risks

| Risk | Mitigation |
| --- | --- |
| Sticky header doesn't pin in jsdom tests (no real scroll) | Sticky header is a pure CSS behaviour via `stickyHeader`; the unit tests don't assert layout, only DOM presence. No risk to existing assertions. |
| `root` is `null` on first render, observer never sets up | The callback ref fires during commit, triggering a state update; the next render commits a non-null `root`; the sentinel's `useEffect` re-runs and creates the observer. The first-render `null` only happens once per mount. |
| Other future callers of `CodeListTable` break if they expect an external sentinel | `bottomSlot` is optional; the existing external-sentinel pattern still works if a caller doesn't supply the slot. We only migrate `TerminologyPage`. |
| `maxHeight: calc(100vh - 240px)` is too small on tall filter bars / extra page chrome | The page chrome today is `p: 4` (32 px top + bottom) + a filter row (~64 px) + `gap: 2` (16 px) ≈ 144 px; 240 px leaves ~96 px of buffer. If the actual chrome grows, the value can be tweaked in one place. |

---

## 9. Out of scope

- `CodeItemTable` / `CodeListDetailPage` — same pattern; deferred per user direction.
- Virtual scrolling — at 20 rows/page the table never grows past a few hundred rows; not justified.
- Persisting scroll position across navigation — not requested.
- Changing the default `rootMargin` of `InfiniteScrollSentinel`.
- Server-side changes.