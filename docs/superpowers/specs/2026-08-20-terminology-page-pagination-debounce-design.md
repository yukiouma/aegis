# Aegis Desktop — Terminology page pagination + debounced search — Design

**Date:** 2026-08-20
**Status:** Approved (pending spec review)
**Scope:** Add server-side pagination (20 rows per page) and a debounced search input to `TerminologyPage` and `CodeListDetailPage`. Updates the Tauri shim layer (Rust) so the unified list+search endpoint with `fragment`, `offset`, and `limit` is reachable from the React layer. Replaces the client-side `useMemo` substring filter with server-side FTS via the new `fragment` parameter.

---

## 1. Goals

1. Both terminology tables render rows in pages of **20** at a time, with rows loading as the user scrolls (infinite scroll).
2. The search input in `TermFilterBar` is debounced: at most one request per second, with a 300 ms trailing-debounce window so the first result appears quickly after the user stops typing.
3. Replace the client-side `useMemo` substring filter with server-side full-text search via the existing `fragment` query parameter.
4. Update the Tauri shim layer (`http/terminology/code_list.rs`, `http/terminology/code_item.rs`, `commands/terminology/*.rs`) to forward `fragment`, `offset`, `limit` and return the paged response shape `{ codelists, nextOffset? }` / `{ items, nextOffset? }`.
5. Drop the dead `search_code_lists` / `search_code_items` Tauri commands that hit the now-removed server `/search` endpoints.
6. Out of scope: cursor-only pagination (page numbers in URL), total row count, fuzzy search, configurable sort order, persisting scroll position across navigation, virtual scrolling.

---

## 2. Pagination UX

The user scrolls inside the table card; when the IntersectionObserver sees the sentinel within 200 px of the viewport bottom and `nextOffset !== null`, the page advances `offset` by 20 and a new page is fetched. The previously-fetched rows stay in the DOM — pagination is append-only. When `nextOffset === null`, the sentinel renders nothing. There is no "Prev" / page-number footer and no "Load more" button.

The two pieces of pagination state (`offset`) live in component state, not URL state. The URL still holds `versionId` (existing) so the back-and-forth round trip works.

---

## 3. Debounce behavior

The `TermFilterBar` continues to be a controlled input — the page owns the raw `query` state. The page feeds the raw value through a `useDebouncedValue` hook with `delayMs: 300` and `maxWaitMs: 1000`:

- The hook returns the **settled value** — it lags behind the input until the trailing-debounce window (300 ms) or the max-wait window (1000 ms) elapses, whichever comes first.
- If the user keeps typing continuously, the hook emits the latest value at most once every 1000 ms.
- When typing pauses, the final value lands 300 ms after the last keystroke.

The debounced value is what gets passed to `useListCodeLists` as `fragment`. The raw value (`query`) stays in the input so the user never sees a "lagged" cursor — only the network request is gated.

---

## 4. Wire / Rust layer

### 4.1 `http/terminology/code_list.rs`

Replace the existing `CodeListListResponse` (single-array envelope) with a paged envelope and add a `list_paged` function. The `search` function is removed.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeListPagedResponse {
    pub codelists: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

pub async fn list_paged(
    c: &HttpClient,
    q: CodeListListQuery,
) -> Result<CodeListPagedResponse, ApiError> {
    let mut path = format!(
        "/api/terminology/code-lists?versionId={}&offset={}&limit={}",
        q.version_id, q.offset, q.limit
    );
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}
```

`percent_encode_fragment` is moved to a small shared helper or kept duplicated; either way it remains identical to the existing implementation.

### 4.2 `http/terminology/code_item.rs`

Mirror of section 4.1:

- `CodeItemPagedResponse { items, next_offset? }`
- `CodeItemListQuery { codelist_id, fragment?, offset, limit }`
- `list_paged(c, q) -> Result<CodeItemPagedResponse, ApiError>`
- Remove `search`, `CodeItemSearchHitsResponse`, `CodeItemSearchHitResponse`, `CodeItemSearchQuery`.

### 4.3 `commands/terminology/code_list.rs`

Replace `list_code_lists` with a shim that forwards the new parameters and returns the paged response:

```rust
#[tauri::command]
pub async fn list_code_lists(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeListPagedResponse, ApiError> {
    code_list::list_paged(
        &client,
        CodeListListQuery { version_id, fragment, offset, limit },
    )
    .await
}
```

Remove `search_code_lists`.

### 4.4 `commands/terminology/code_item.rs`

Same shape: `list_code_items` takes `(codelist_id, fragment?, offset, limit)` returning `CodeItemPagedResponse`. Remove `search_code_items`.

### 4.5 `lib.rs`

Drop the two `search_code_*` entries from the `invoke_handler!` macro list.

### 4.6 Tests (Rust)

- `list_returns_first_page_with_next_offset_when_more_pages_exist` — mock responds with `{ codelists: [a, b, … 21 rows], nextOffset: 20 }`. Assert the typed round-trip.
- `list_returns_no_next_offset_on_last_page` — mock `{ codelists: […3 rows], nextOffset: undefined }`.
- `list_with_fragment_includes_fragment_query_param` — mock `WireMock::query_param("fragment", "AE")`.
- `list_with_no_fragment_omits_fragment_query_param` — same mock without the matcher must reject the request.
- `list_passes_offset_and_limit` — mock `query_param("offset", "40")` and `query_param("limit", "20")`.
- `list_round_trips_snake_case_next_offset` — mock `{ "codelists": [], "next_offset": 100 }` deserializes into `{ codelists, nextOffset: Some(100) }` (camelCase rename verified).

Mirror the five for `list_paged` on `code_item.rs`.

---

## 5. Shared API (TS)

### 5.1 `shared/api/types.ts`

Add:

```ts
export interface PagedCodeListListResponse {
  codelists: CodeListView[];
  nextOffset?: number;
}

export interface PagedCodeItemListResponse {
  items: CodeItemView[];
  nextOffset?: number;
}

export interface CodeListListQuery {
  versionId: number;
  fragment?: string;
  offset?: number;
  limit?: number;
}

export interface CodeItemListQuery {
  codelistId: number;
  fragment?: string;
  offset?: number;
  limit?: number;
}
```

The wire shapes use resource-specific field names (`codelists` for code lists, `items` for code items) to match the Rust `serde(rename_all = "camelCase")` output — see section 4. A generic `Page<T>` does not fit because the two resources use different field names, so we keep them explicit.

The existing `CodeListListResponse` / `CodeItemListResponse` (flat single-field envelopes) are removed — nothing else references them.

### 5.2 `shared/api/index.ts`

Replace:

```ts
listCodeLists: (versionId: number): Promise<CodeListView[]> =>
  call<CodeListView[]>("list_code_lists", { versionId }),
```

with:

```ts
listCodeLists: (
  versionId: number,
  options: CodeListListQuery = {},
): Promise<PagedCodeListListResponse> =>
  call<PagedCodeListListResponse>("list_code_lists", {
    versionId,
    fragment: options.fragment,
    offset: options.offset,
    limit: options.limit,
  }),
```

Mirror for `listCodeItems`. The `call<T>` helper already accepts `undefined`-valued keys and serializes them as `undefined`, which Tauri serializes as `null` then axum parses as `None`; the empty-fragment case is filtered out client-side before being sent (see section 6).

The `SearchTerminologyQuery` type is removed (it was for the dead `/search` endpoint).

### 5.3 `shared/api/index.ts` exports

Add `PagedCodeListListResponse`, `PagedCodeItemListResponse`, `CodeListListQuery`, `CodeItemListQuery` to the `export type { … }` block. Remove the `SearchTerminologyQuery` export.

---

## 6. Query layer

### 6.1 `shared/query/keys.ts`

Replace the `codeLists` / `codeItems` key factories with versions that include `fragment` and `offset`:

```ts
terminology: {
  versions: () => ["terminology", "versions"] as const,
  codeLists: (versionId: number, fragment: string, offset: number) =>
    ["terminology", "codeLists", versionId, fragment, offset] as const,
  codeItems: (codelistId: number, fragment: string, offset: number) =>
    ["terminology", "codeItems", codelistId, fragment, offset] as const,
  codeList: (id: number) => ["terminology", "codeList", id] as const,
},
```

Drop `searchCodeLists` / `searchCodeItems` (unused).

Mutation `invalidateQueries` calls now target `["terminology", "codeLists", versionId]` — the prefix matches every `(fragment, offset)` pair and React Query clears them all.

### 6.2 `features/terminology/data/list.ts`

Replace `useListCodeLists(versionId)` with:

```ts
export interface ListCodeListsOptions {
  fragment?: string;
  offset?: number;
}

export function useListCodeLists(
  versionId: number | null,
  options: ListCodeListsOptions = {},
) {
  const fragment = options.fragment ?? "";
  const offset = options.offset ?? 0;
  return useQuery<PagedCodeListListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0, fragment, offset),
    queryFn: () =>
      api.listCodeLists(versionId!, {
        fragment: fragment || undefined,
        offset,
        limit: PAGE_SIZE,
      }),
    enabled: versionId != null && versionId > 0,
  });
}
```

`PAGE_SIZE = 20` lives in `data/list.ts` as a module-level constant so the same value feeds both the API call and the offset increment.

`useListCodeItems(codelistId, options?)` mirrors the same shape.

Mutation hooks keep their invalidation strategy; the prefix-based invalidation above already covers every offset and fragment.

---

## 7. New shared hook: `useDebouncedValue`

### 7.1 File

`apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts` — new.

### 7.2 Contract

```ts
export interface UseDebouncedValueOptions {
  /** Trailing-edge debounce window after the last change. */
  delayMs: number;
  /** Maximum time to wait between fires while the value is still changing. */
  maxWaitMs: number;
}

export function useDebouncedValue<T>(
  value: T,
  options: UseDebouncedValueOptions,
): T;
```

The hook returns the **settled value** — i.e., the most recent value that the timer has already emitted. While the input keeps changing, the returned value **lags** behind until the trailing-debounce window (`delayMs`) or the max-wait window (`maxWaitMs`) elapses, at which point the hook emits the latest value and the caller sees it on the next render.

Concrete semantics:

- On the first render and on every input change, a `delayMs` timer is (re)started.
- Independently, a `maxWaitMs` timer is (re)started on each input change.
- The hook emits the latest input value when **either** timer fires (whichever comes first).
- After an emit, both timers reset.
- If the input never changes, no timer fires and the returned value never changes.

This is the standard lodash `_.debounce(fn, wait, { maxWait })` semantics adapted for React.

Implementation uses `useRef` for the pending value and the timer handles, and `useEffect` (re)schedules both timers when the input or the options change.

### 7.3 Tests

`shared/hooks/useDebouncedValue.test.ts`:

- `returns_initial_value_on_first_render` — on mount, the hook returns the current input value immediately.
- `emits_trailing_value_after_delay_when_input_stops` — type fast, then idle; after `delayMs` the hook returns the latest value.
- `throttles_continuous_changes_to_max_wait` — input keeps changing for 2 s; the hook emits at most once per `maxWaitMs`.
- `does_not_emit_when_value_unchanged` — re-render with the same value is a no-op (no timers fire).
- `cancels_pending_timer_on_unmount` — unmount during a pending window: no setState-after-unmount warning.

Tests use `vitest.useFakeTimers()` + `renderHook` from `@testing-library/react`.

---

## 8. New shared component: `InfiniteScrollSentinel`

### 8.1 File

`apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` — new.

### 8.2 Contract

```ts
export interface InfiniteScrollSentinelProps {
  /** Called when the sentinel scrolls into view and `hasMore` is true. */
  onIntersect: () => void;
  /** Stop firing `onIntersect` when false. */
  hasMore: boolean;
  /** Suppress `onIntersect` while a page fetch is in flight. */
  loading: boolean;
  /** Pixel margin before the viewport edge at which the observer fires. */
  rootMargin?: string;
}
```

Implementation:

- Renders a single `<div ref={sentinelRef} />`.
- Uses `IntersectionObserver` with the provided `rootMargin` (default `"0px 0px 200px 0px"`).
- Calls `onIntersect` when the observer fires AND `hasMore && !loading`.
- Disconnects the observer when `hasMore === false` (avoids useless re-renders).
- Renders a small `<CircularProgress size={20} />` inside the div while `loading === true`, so the user sees feedback when scrolling quickly.

### 8.3 Tests

`shared/components/infinite-scroll-sentinel.test.tsx`:

- Fires `onIntersect` once on intersection when `hasMore=true, loading=false`.
- Does **not** fire when `hasMore=false` (observer disconnects).
- Does **not** fire when `loading=true` (subsequent intersection while a fetch is in flight is a no-op).
- Shows the spinner when `loading=true`.

IntersectionObserver is polyfilled via `intersection-observer` (already in dev deps if needed; otherwise we add it).

---

## 9. Pages

### 9.1 `TerminologyPage.tsx`

- Drop the local `useMemo` substring filter entirely.
- Add `const [offset, setOffset] = useState(0)`.
- Add `const debouncedFragment = useDebouncedValue(search, { delayMs: 300, maxWaitMs: 1000 })`.
- Add a `useEffect(() => setOffset(0), [versionId, debouncedFragment])` so any change to either resets the page cursor.
- Replace `useListCodeLists(selectedVersionId)` with `useListCodeLists(selectedVersionId, { fragment: debouncedFragment, offset })`.
- `rows` now comes from `codeListsQuery.data?.items ?? []`.
- Render `<InfiniteScrollSentinel onIntersect={() => setOffset(o => o + PAGE_SIZE)} hasMore={codeListsQuery.data?.nextOffset != null} loading={codeListsQuery.isFetching} />` immediately after `<CodeListTable />`.
- The empty-state message: `t('terminology.codelist.noMatches')` when `debouncedFragment.trim() !== ""`; `t('terminology.codelist.empty')` otherwise. (Same logic as today; we now key off `debouncedFragment` instead of `search` so the empty state doesn't flash on every keystroke.)
- `onRetry={codeListsQuery.refetch}` continues to work — `refetch()` re-runs the current page query.

### 9.2 `CodeListDetailPage.tsx`

Same shape: drop the local filter, add `offset` + `debouncedFragment`, reset on `(codelistId, debouncedFragment)` change, render the sentinel below `<CodeItemTable />`.

### 9.3 Why reset on `versionId` change

A new version means a fresh result set; leaving offset at 40 would show an empty list briefly. The reset is one line and removes a class of confusing UX.

---

## 10. Files changed

**Rust (Tauri shim)**

- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs` — drop `CodeListListResponse`, `CodeListSearchHitsResponse`, `CodeListSearchHitResponse`, `CodeListSearchQuery`, `search()`. Add `CodeListPagedResponse`, `CodeListListQuery`, `list_paged()`. Update `tests`.
- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` — drop the `*Search*` types / `search()`. Add `CodeItemPagedResponse`, `CodeItemListQuery`, `list_paged()`. Update `tests`.
- `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs` — drop `search_code_lists`. Replace `list_code_lists` signature.
- `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` — drop `search_code_items`. Replace `list_code_items` signature.
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — drop the two `search_code_*` entries from `invoke_handler!`.

**Shared**

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — add `Page<T>`, `PagedCodeListListResponse`, `PagedCodeItemListResponse`, `CodeListListQuery`, `CodeItemListQuery`. Remove `SearchTerminologyQuery`.
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — replace `listCodeLists` / `listCodeItems` wrappers.
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — extend `codeLists` / `codeItems` keys; drop `searchCodeLists` / `searchCodeItems`.
- `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts` — **new**.
- `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.test.ts` — **new**.
- `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` — **new**.
- `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.test.tsx` — **new**.

**Features**

- `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` — replace `useListCodeLists` / `useListCodeItems` signatures; export `PAGE_SIZE = 20`.
- `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` — drop client filter; add offset + debounce; add sentinel.
- `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx` — same shape.

**Tests**

- `apps/desktop/aegis-desktop/src/test/features/terminology/version-dropdown-persistence.test.tsx` — update `list_code_lists` mock to return `{ codelists: [...], nextOffset: undefined }` (and same for `list_code_items`).
- Any other test file that mocks `list_code_lists` or `list_code_items` — same one-line fix.

---

## 11. Error handling

| Input | Behavior | Result |
| --- | --- | --- |
| `fragment = ""` or whitespace-only | treated as no fragment | plain list path |
| `fragment` contains reserved tsquery char (`& \| ! ( ) :`) | server returns 400 | existing `errorMessage` + Retry button |
| `offset` past the last page | server returns `{ items: [], nextOffset: undefined }` | empty-state row, sentinel hides |
| Version change | page resets offset, refetches from 0 | (no error) |
| `codelistId` change | page resets offset, refetches from 0 | (no error) |
| Network failure on page N | existing `<Alert severity="error">` + Retry on that page | keeps prior pages visible |

No new error states.

---

## 12. Testing

### 12.1 Rust

- Section 4.6 covers the new wiremock tests.
- Drop the old `search_returns_hits` and `search_rejects_empty_fragment` tests.

### 12.2 Hook

- `features/terminology/data/list.test.ts`:
  - `useListCodeLists` returns a `Page<CodeListView>`; the query key changes when `fragment` or `offset` change.
  - `useCreateCodeList`'s invalidation prefix-matches every fragment/offset pair.
  - Same shape for items.

### 12.3 Hook (shared)

- `shared/hooks/useDebouncedValue.test.ts` — section 7.3.

### 12.4 Component (shared)

- `shared/components/InfiniteScrollSentinel.test.tsx` — section 8.3.

### 12.5 Page integration

- New `test/features/terminology/terminology-page-pagination.test.tsx`:
  - Mock `list_code_lists` to return `{ codelists: [a, b, c], nextOffset: 20 }` on the first call and `{ codelists: [d, e, f], nextOffset: 40 }` on the second.
  - Scroll the sentinel into view, assert `list_code_lists` was called a second time with `offset = 20`.
  - Set `nextOffset = undefined` on the second response; scroll again; assert no third call.
  - Type into the search field; assert `list_code_lists` is called with `fragment = "AE"` and `offset = 0` (reset).
  - Wait through `vi.useFakeTimers()`; assert continuous typing fires at most once per 1000 ms (advance timers in 200 ms steps for 3 s, count invocations).
  - Mutation (create) invalidates every page; refetch of any fragment fires with fresh data.

- Mirror as `code-list-detail-pagination.test.tsx` for the detail page.

### 12.6 Existing tests

- `version-dropdown-persistence.test.tsx` — fix the mock to the new paged envelope shape; assertions stay green.

---

## 13. Verification

- `cargo test -p aegis-desktop --lib` — wiremock round-trips.
- `pnpm --filter aegis-desktop test` — Vitest suite (existing tests stay green; new tests pass).
- `pnpm --filter aegis-desktop typecheck` (or `tsc --noEmit` via the project's command) — type-level: every site that imported `CodeListView[]` from the old `useListCodeLists` must be updated.

---

## 14. Out of scope

- Cursor-based / keyset pagination
- Total row count
- Page numbers in the URL (only `versionId` stays URL-bound)
- "Prev" / "Next" buttons or page-number footer
- Virtual scrolling
- Persisting scroll position across navigation
- Fuzzy / ILIKE search (server-side is FTS only, unchanged)
- Sort order other than the server default
- Server-side changes (already shipped in the 2026-08-20 refactor)

---

## 15. Risks

| Risk | Mitigation |
| --- | --- |
| IntersectionObserver fires before the in-flight page returns | `loading` guard suppresses duplicate `onIntersect` calls |
| User scrolls past the sentinel faster than the fetch | Sentinel re-fires when `loading` flips back to `false` (observer is reconnected on every render) |
| Cache bloat from 5–10 pages of 20 rows each | React Query default `gcTime` of 5 min; max ~200 rows per version, negligible |
| TypeScript drift — `useListCodeLists` now returns `Page<T>` | Page-level refactor uses `data.items`; tests catch any miss |
| Stale tests mock the old flat-array response | Section 10 lists every test that needs a one-line fix; we update them in the same PR |
