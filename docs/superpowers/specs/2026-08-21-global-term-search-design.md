# Terminology — Global term search page — Design

**Date:** 2026-08-21
**Scope:** Frontend + Tauri command surface. Add a new `GlobalTermSearch` page in the `terminology` feature that searches code lists and code items across a whole terminology version using a single fragment. Touches the desktop app end-to-end (route, page, components, hooks, api wrapper, query keys, i18n keys, TerminologyPage edit) **and** the Tauri command + HTTP layer for `list_code_items` (because the command currently requires `codelist_id`). The aegis-server already supports cross-codelist code-item queries — no backend changes are required.

---

## 1. Goal

From `TerminologyPage`, let the user click a `ManageSearch` icon to open a new dedicated search page (`GlobalTermSearchPage`) bound to the currently selected terminology version. The page contains:

- a single search field (back-arrow on the left)
- a toggle between "Code Lists" and "Code Items" results
- a results table whose rows can be launched into the corresponding `CodeListDetailPage`

The page must preserve `versionId` across navigation (forward from `TerminologyPage`, back via arrow, and into `CodeListDetailPage`).

## 2. Non-goals

- No aegis-server changes (the server's `list_code_items` already accepts an optional `codelistId`).
- No new server endpoint.
- No new TS DTOs.
- No full-text highlighting / snippets in the result table.
- No cross-kind search. The search is scoped to the version's `kind` (`sdtm` or `adam`) carried in the route path.
- No infinite-scroll list rendering on first page-load when search is empty.

## 3. UX

### 3.1 `TerminologyPage` top action row

```
┌────────────────────────────────────────────────────────────────────────┐
│  [Search by code, name, …]   [SDTM ▾]   [📥 Import]   [🔍 ManageSearch]│
└────────────────────────────────────────────────────────────────────────┘
```

A new `IconButton` (`<ManageSearchIcon />`) is placed **immediately before** `<ImportButton>` in the top action row of `TerminologyPage`. Disabled when no version is selected. Wrapped in a `<Tooltip>` reading `terminology.search.open`.

Click handler navigates to `/terminology/{kind}/search?versionId=<selected>`.

### 3.2 `GlobalTermSearchPage`

```
┌────────────────────────────────────────────────────────────────────────┐
│  [←]  [Search code lists and code items ............................]  │
│                                                                        │
│  [ Code Lists | Code Items ]   ← ToggleButtonGroup                     │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Code     │ Name │ Submission value │ Descriptions │      ⤴        │  │  (Code Lists)
│  ├──────────────────────────────────────────────────────────────────┤  │
│  │ ABC      │ Foo  │ FOO              │ …            │      ⤴        │  │
│  │ …                                                                 │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│  ── or ──                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Code │ Codelist │ Submission value │ Descriptions │       ⤴      │  │  (Code Items)
│  ├──────────────────────────────────────────────────────────────────┤  │
│  │ 001  │ Foo      │ 001              │ …            │       ⤴      │  │
│  │ …                                                                 │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

| Concern | Behaviour |
|---|---|
| Back arrow | Navigate to `/terminology/{kind}?versionId=<versionId>`. Preserves the version. |
| Search field | Reused `TermFilterBar` (controlled, debounced 300 ms). Placeholder: `terminology.search.placeholder`. |
| Toggle | MUI `<ToggleButtonGroup exclusive>` (size small). Default value: `codelists`. State is local to the page (not URL state). |
| Empty input | Hint message (`terminology.search.emptyInput`) replaces the table. The toggle remains usable. |
| Loading | When `loading === true` and no rows yet → centered `<CircularProgress />`. When rows exist, just keep them visible and let the InfiniteScrollSentinel trigger the next page. |
| Error | Inline `<Alert severity="error">` + Retry `<Button>` calling `refetch()`. i18n key `terminology.codelist.loadFailed` (for codelists) or `terminology.codeitem.loadFailed.search` (for items). Pattern matches the existing `CodeListTable`. |
| Empty results | Existing `terminology.codelist.noMatches` / `terminology.codeitem.noMatches` messages. |
| Operation column | `IconButton size="small"` with `<LaunchIcon fontSize="small" />`, wrapped in `<Tooltip title={common.open}>`. Code Lists row navigates to `/terminology/{kind}/codelists/{row.id}?versionId=<v>`. Code Items row navigates to `/terminology/{kind}/codelists/{row.codelistId}?versionId=<v>`. |
| Infinite scroll | Reuse `<InfiniteScrollSentinel>` as in `TerminologyPage`. |

### 3.3 Result tables

#### `SearchCodeListTable` (private)

| Column | Source | i18n key |
|---|---|---|
| Code | `row.code` (+ optional "EXT" chip if `row.extensible`) | `terminology.codelist.field.code` |
| Name | `row.name` | `terminology.codelist.field.name` |
| Submission value | `row.submissionValue` | `terminology.codelist.field.submissionValue` |
| Descriptions | `<DescriptionsCell>` (synonym / definition / nciPreferredTerm) | `terminology.codelist.field.descriptions` |
| Operation | `<IconButton>` + `<LaunchIcon>` | — |

#### `SearchCodeItemTable` (private)

| Column | Source | i18n key |
|---|---|---|
| Code | `row.code` | `terminology.codeitem.field.code` |
| Codelist | `<CodelistNameCell codelistId={row.codelistId} />` | `terminology.codeitem.field.codelist` |
| Submission value | `row.submissionValue` | `terminology.codeitem.field.submissionValue` |
| Descriptions | `<DescriptionsCell>` | `terminology.codeitem.field.descriptions` |
| Operation | `<IconButton>` + `<LaunchIcon>` | — |

`<CodelistNameCell>` is a private sub-component that calls `useGetCodeList(codelistId)` and renders `data?.name ?? #${codelistId}`. React Query dedupes repeated `codelistId` lookups across rows.

## 4. Architecture

```
TerminologyPage → click ManageSearch icon
  → navigate(/terminology/$kind/search, { versionId })
      → GlobalTermSearchPage
           ├─ Back IconButton → navigate(/terminology/{kind}, { versionId })
           ├─ TermFilterBar (controlled, debounced)
           ├─ ToggleButtonGroup (codelists | codeitems) — local state
           ├─ if trimmed fragment is empty → hint
           └─ else table for selected tab
                ├─ SearchCodeListTable  → useSearchCodeLists(versionId, { fragment })
                │     → api.listCodeLists(versionId, { fragment, offset, limit })
                │           → invoke('list_code_lists')     [Tauri command unchanged]
                │                 → GET /api/terminology/code-lists
                └─ SearchCodeItemTable  → useSearchCodeItems(versionId, { fragment })
                      → api.listCodeItems(null, { fragment, offset, limit })
                            → invoke('list_code_items', { codelistId: undefined, … })
                                  [Tauri command updated: codelistId now Option<i64>]
                                  → GET /api/terminology/code-items
                                        ↑ codelistId query param is omitted when None
```

The page never writes to the URL when toggling tabs or typing — only navigation events (back / launch row / forward from `TerminologyPage`) carry params.

## 5. Routing

### 5.1 New route file

`apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/$kind/search.tsx`

```ts
import { createFileRoute } from "@tanstack/react-router";
import type { TerminologyKind } from "../../../../../../shared/api";
import { GlobalTermSearchPage } from "../../../../../../features/terminology";

const KIND_VALUES: readonly TerminologyKind[] = ["sdtm", "adam"];

export const Route = createFileRoute(
  "/_authed/_layout/terminology/$kind/search",
)({
  parseParams: (raw) => ({
    kind: KIND_VALUES.includes(raw.kind as TerminologyKind)
      ? (raw.kind as TerminologyKind)
      : "sdtm",
  }),
  stringifyParams: ({ kind }) => ({ kind }),
  validateSearch: (raw): { versionId?: number } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === "" ? undefined : Number(raw.versionId)
        : typeof raw.versionId === "number" ? raw.versionId : undefined,
  }),
  component: GlobalTermSearchPage,
});
```

Public path: `/terminology/{sdtm|adam}/search`. Path params: `kind`. Search params: `{ versionId?: number }`. `routeTree.gen.ts` is auto-regenerated — no manual edits.

### 5.2 Navigation recipes

```ts
// TerminologyPage → GlobalTermSearchPage (forward)
navigate({
  to: "/terminology/$kind/search",
  params: { kind },
  search: selectedVersionId != null ? { versionId: selectedVersionId } : undefined,
});

// GlobalTermSearchPage → TerminologyPage (back)
navigate({
  to: kind === "sdtm" ? "/terminology/sdtm" : "/terminology/adam",
  search: versionId != null ? { versionId } : undefined,
});

// Row LaunchIcon → CodeListDetailPage
navigate({
  to: "/terminology/$kind/codelists/$codelistId",
  params: { kind, codelistId: targetCodelistId },
  search: versionId != null ? { versionId } : undefined,
});
```

`targetCodelistId` is `row.id` for Code Lists rows and `row.codelistId` for Code Items rows.

## 6. Public contracts (TS)

### 6.1 `api.listCodeItems` (modified)

`apps/desktop/aegis-desktop/src/shared/api/index.ts`:

```ts
listCodeItems: (
  codelistId: number | null,
  options: CodeItemListQuery = {},
): Promise<PagedCodeItemListResponse> =>
  call<PagedCodeItemListResponse>("list_code_items", {
    codelistId: codelistId ?? undefined,
    fragment: options.fragment,
    offset: options.offset,
    limit: options.limit,
  }),
```

`codelistId: null` ⇒ key omitted from the Tauri args object. Existing per-codelist call sites pass a non-null `codelistId`, so their wire shape is unchanged.

### 6.2 Query key factory (add)

`apps/desktop/aegis-desktop/src/shared/query/keys.ts`:

```ts
codeItemsGlobal: (versionId: number, fragment: string) =>
  ["terminology", "codeItemsGlobal", versionId, fragment] as const,
```

### 6.3 New hooks

`apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`:

```ts
export const useSearchCodeLists = (
  versionId: number | null,
  options: ListPagedOptions,
): UseInfiniteQueryResult<PagedCodeListListResponse, ApiError>;

export const useSearchCodeItems = (
  versionId: number | null,
  options: ListPagedOptions,
): UseInfiniteQueryResult<PagedCodeItemListResponse, ApiError>;
```

Both use the existing `PAGE_SIZE` constant, `queryKeys.terminology.codeLists` / new `codeItemsGlobal` key, and `api.listCodeLists` / `api.listCodeItems(null, …)`. Both are disabled (`enabled: false`) when `versionId == null || versionId <= 0`. The page additionally chooses not to render the table at all when the trimmed fragment is empty — the hook itself does not gate on fragment (so that subsequent typing triggers a fresh query immediately, the same pattern `useListCodeLists` already uses).

### 6.4 Reused hooks

- `useGetCodeList(id)` — already exists; used inside `CodelistNameCell`.
- `useDebouncedValue` — already exists; used for the search input (300 ms / 1000 ms max wait, matches `TerminologyPage`).

### 6.5 Tauri command + HTTP layer (modified)

The current `list_code_items` Tauri command requires a non-null `codelist_id: i64`. We need to support the cross-codelist case where `codelist_id` is absent.

`apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`:

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: Option<i64>,            // was i64
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

pub async fn list_paged(
    c: &HttpClient,
    q: CodeItemListQuery,
) -> Result<CodeItemPagedResponse, ApiError> {
    let mut path = String::from("/api/terminology/code-items?offset=");
    path.push_str(&q.offset.to_string());
    path.push_str("&limit=");
    path.push_str(&q.limit.to_string());
    if let Some(id) = q.codelist_id {
        path.push_str("&codelistId=");
        path.push_str(&id.to_string());
    }
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}
```

`apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`:

```rust
#[tauri::command]
pub async fn list_code_items(
    client: State<'_, HttpClient>,
    codelist_id: Option<i64>,                  // was i64
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    code_item::list_paged(
        &client,
        CodeItemListQuery { codelist_id, fragment, offset, limit },
    )
    .await
}
```

Existing call sites (e.g. `useListCodeItems`) pass a non-null number; the new behaviour is strictly additive at the wire level (an explicit `codelistId=null` from JS is deserialized to `None` and the query param is omitted from the URL).

## 7. i18n keys

Additions to `lib/packages/ui/src/i18n/locales/en.ts` and `…/zhCN.ts` (TS type parity enforced):

| Key | en | zhCN |
|---|---|---|
| `terminology.search.open` | `Search across this version` | `跨版本搜索` |
| `terminology.search.backTooltip` | `Back to terminology` | `返回术语` |
| `terminology.search.placeholder` | `Search code lists and code items` | `搜索代码列表与代码项` |
| `terminology.search.tab.codelists` | `Code Lists` | `代码列表` |
| `terminology.search.tab.codeitems` | `Code Items` | `代码项` |
| `terminology.search.emptyInput` | `Type a search term to find code lists or code items` | `请输入搜索关键字以查找代码列表或代码项` |
| `terminology.codeitem.field.codelist` | `Codelist` | `代码列表` |
| `terminology.codeitem.loadFailed.search` | `Failed to load code items: {message}` | `加载代码项失败：{message}` |

Existing keys reused: `terminology.codelist.field.*`, `terminology.codeitem.field.{code,submissionValue,descriptions}`, `terminology.codelist.noMatches`, `terminology.codeitem.noMatches`, `terminology.codelist.loadFailed`, `common.open`, `common.cancel`, `common.back`.

## 8. Error handling

| Scenario | Behaviour |
|---|---|
| Network failure | Inline alert with `terminology.*.loadFailed` + Retry button → `query.refetch()`. |
| `useGetCodeList(id)` failure inside a row | Cell falls back to `#${codelistId}` — table does not break. |
| Empty fragment after debounce | Hook disabled; page hides the table and shows `terminology.search.emptyInput`. |
| Version not selected on entry | Page shows the empty hint. ManageSearch button in `TerminologyPage` is disabled when this is the case. |
| Invalid `kind` URL param | Defaults to `"sdtm"` (parity with the existing `$codelistId` route). |
| Token expired / 401 | Existing `ApiError` handling; auth interceptor redirects to login. |

## 9. Files touched

New:

- `apps/desktop/aegis-desktop/src/features/terminology/pages/GlobalTermSearchPage.tsx`
- `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/$kind/search.tsx`

Modified:

- `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts` — re-export new page
- `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` — add ManageSearch icon button
- `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` — add `useSearchCodeLists`, `useSearchCodeItems`
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — make `api.listCodeItems` accept `null` codelistId
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — add `codeItemsGlobal` factory
- `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` — `list_code_items` command: `codelist_id: i64` → `Option<i64>`
- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` — `CodeItemListQuery.codelist_id: i64` → `Option<i64>`; `list_paged` skips the query param when `None`; add wiremock tests
- `lib/packages/ui/src/i18n/locales/en.ts` and `lib/packages/ui/src/i18n/locales/zhCN.ts` — new keys

Untouched (verified unchanged):

- aegis-server (no Rust edits)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (no new DTOs)
- `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx` (not reused for the search page)

## 10. Testing

### 10.1 Rust (Tauri HTTP layer, wiremock)

Add to `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs::tests`:

- `list_paged_with_none_codelist_id_omits_query_param` — sends `CodeItemListQuery { codelist_id: None, fragment: Some("AE".into()), offset: 0, limit: 20 }`, asserts the mock server's path has no `codelistId` query param but does have `fragment=AE`. (Existing test for the fragment path stays green.)
- `list_paged_with_some_codelist_id_includes_query_param` — `codelist_id: Some(11)`, asserts the path contains `codelistId=11`. Re-asserts the original wire shape.

Run with `cargo test -p aegis-desktop --lib http::terminology::code_item`.

### 10.2 TS / UI

The project has no automated UI test suite for the desktop app, and the spec does not request new tests. Verification is manual:

1. From `/terminology/sdtm` with version `X`, click ManageSearch → lands on `/terminology/sdtm/search?versionId=X`.
2. Type `xyz` in Code Lists tab → matches render with correct columns; arrow-back returns to `/terminology/sdtm?versionId=X`.
3. Switch to Code Items tab → same fragment searches across codelists in version `X`. Each row shows its parent codelist's name.
4. Click LaunchIcon on a Code Items row → opens `/terminology/sdtm/codelists/{codelistId}?versionId=X`.
5. Empty input → only toggle + hint are visible.
6. Clear input after typing → results clear; toggle remains usable.
7. Repeat 1-6 from `/terminology/adam`.

### 10.3 Verification commands

- `pnpm --filter aegis-desktop tsc --noEmit` — TS type check.
- `pnpm --filter aegis-desktop build` — Vite build.
- `cargo test -p aegis-desktop --lib http::terminology::code_item` — new Tauri wiremock tests.
- `cargo check -p aegis-desktop` — Rust type check (Tauri command signature change must compile).

## 11. Rollback

All changes are additive except:

- `api.listCodeItems(codelistId: number, …)` → `api.listCodeItems(codelistId: number | null, …)`. Reverting to `number` would only fail compilation in the new `useSearchCodeItems` hook (which is also being reverted). All other call sites pass a non-null number, so the runtime wire shape is identical.
- Tauri command `list_code_items(codelist_id: i64, …)` → `codelist_id: Option<i64>`. Reverting to `i64` would only fail compilation in the new `useSearchCodeItems` call site (which is also being reverted). The wire shape is unchanged for all existing call sites (they pass a number, which serializes to `Some(11)`).

No migrations, no schema changes, no server changes — rollback is purely a desktop-app revert (frontend + Tauri command surface).

## 12. Out of scope

- Server-side highlighting / snippets
- Cross-kind search (searching both SDTM and ADaM in one view)
- Replacing the existing `useMemo` filter pattern on `TerminologyPage` / `CodeListDetailPage`
- Reusing `CodeListTable` for the search page
- Per-row mutations (edit / delete) from the search page — read-only by design
- Sortable columns / persistent column widths
