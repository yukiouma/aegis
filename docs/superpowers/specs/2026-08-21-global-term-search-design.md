# Terminology — Global term search page — Design

**Date:** 2026-08-21
**Scope:** Frontend-only. Add a new `GlobalTermSearch` page in the `terminology` feature that searches code lists and code items across a whole terminology version using a single fragment. Touches the desktop app end-to-end (route, page, components, hooks, api wrapper, query keys, i18n keys, TerminologyPage edit). The aegis-server already supports cross-codelist code-item queries — no backend changes are required.

---

## 1. Goal

From `TerminologyPage`, let the user click a `ManageSearch` icon to open a new dedicated search page (`GlobalTermSearchPage`) bound to the currently selected terminology version. The page contains:

- a single search field (back-arrow on the left)
- a toggle between "Code Lists" and "Code Items" results
- a results table whose rows can be launched into the corresponding `CodeListDetailPage`

The page must preserve `versionId` across navigation (forward from `TerminologyPage`, back via arrow, and into `CodeListDetailPage`).

## 2. Non-goals

- No backend / Tauri changes (the server's `list_code_items` already accepts an optional `codelistId`; the Tauri command `list_code_items` already takes positional args that flow through to the server).
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
| Loading | Reuse the existing inline loading pattern (top progress bar / disabled refresh button on the table header). |
| Error | Inline alert + Retry button calling `refetch()`. i18n key `terminology.codelist.loadFailed` (for codelists) or `terminology.codeitem.loadFailed.search` (for items). |
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
                            → invoke('list_code_items')     [Tauri command unchanged]
                                  → GET /api/terminology/code-items
                                        ↑ codelistId param is omitted (null)
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

Both use the existing `PAGE_SIZE` constant, `queryKeys.terminology.codeLists` / new `codeItemsGlobal` key, and `api.listCodeLists` / `api.listCodeItems(null, …)`. Both are disabled when `versionId == null || versionId <= 0`. Both `enabled` short-circuit when `fragment` trimmed is empty (gated by the page).

### 6.4 Reused hooks

- `useGetCodeList(id)` — already exists; used inside `CodelistNameCell`.
- `useDebouncedValue` — already exists; used for the search input (300 ms / 1000 ms max wait, matches `TerminologyPage`).

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
- `lib/packages/ui/src/i18n/locales/en.ts` and `lib/packages/ui/src/i18n/locales/zhCN.ts` — new keys

Untouched (verified unchanged):

- aegis-server (no Rust edits)
- `apps/desktop/aegis-desktop/src-tauri/**` (Tauri command signatures already accept `null`/omitted codelistId because the args are positional)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (no new DTOs)
- `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx` (not reused for the search page)

## 10. Testing

The project has no automated UI test suite for the desktop app, and the spec does not request new tests. Verification is manual:

1. From `/terminology/sdtm` with version `X`, click ManageSearch → lands on `/terminology/sdtm/search?versionId=X`.
2. Type `xyz` in Code Lists tab → matches render with correct columns; arrow-back returns to `/terminology/sdtm?versionId=X`.
3. Switch to Code Items tab → same fragment searches across codelists in version `X`. Each row shows its parent codelist's name.
4. Click LaunchIcon on a Code Items row → opens `/terminology/sdtm/codelists/{codelistId}?versionId=X`.
5. Empty input → only toggle + hint are visible.
6. Clear input after typing → results clear; toggle remains usable.
7. Repeat 1-6 from `/terminology/adam`.
8. `pnpm --filter aegis-desktop tsc --noEmit` — type check passes.
9. `pnpm --filter aegis-desktop build` (or `cargo check` in `src-tauri`) — build passes (no Rust changes expected).

## 11. Rollback

All changes are additive except:

- `api.listCodeItems(codelistId: number, …)` → `api.listCodeItems(codelistId: number | null, …)`. Reverting to `number` would only fail compilation in the new `useSearchCodeItems` hook (which is also being reverted). All other call sites pass a non-null number, so the runtime wire shape is identical.

No migrations, no schema changes, no server changes — rollback is purely a frontend revert.

## 12. Out of scope

- Server-side highlighting / snippets
- Cross-kind search (searching both SDTM and ADaM in one view)
- Replacing the existing `useMemo` filter pattern on `TerminologyPage` / `CodeListDetailPage`
- Reusing `CodeListTable` for the search page
- Per-row mutations (edit / delete) from the search page — read-only by design
- Sortable columns / persistent column widths
