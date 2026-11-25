# CRF — Global search page — Design

**Date:** 2026-08-31
**Scope:** Frontend + Tauri command surface. Add a `CrfGlobalSearchPage` in the `crf` feature that searches forms / items / units / options / domain annotations / annotations across a whole CRF version using a single fragment. Touches the desktop app end-to-end (route, page, components, hooks, api wrappers, query keys, i18n keys, `CrfToolsMenu` + `CrfDetailPage` edit, search-focus plumbing) **and** the Tauri command + HTTP layer for the six `search_*_by_version` endpoints. The aegis-server already exposes all six search endpoints — no backend changes are required.

---

## 1. Goal

From `CrfFormListPage` (or `CrfDetailPage`), let the user click `Global Search` in the `CrfToolsMenu` to open a new dedicated search page (`CrfGlobalSearchPage`) bound to the currently selected CRF version. The page contains:

- a single search field (back-arrow on the left)
- a tab toggle across six result kinds (`Forms | Items | Units | Options | Domain annotations | Annotations`)
- one table per tab whose rows can be launched into the corresponding `CrfDetailPage`, automatically scrolled to the matching anchor

The page must preserve `versionId` across navigation (forward from the list / detail page, back via arrow, into `CrfDetailPage`).

---

## 2. Non-goals

- No aegis-server changes (all six `search_*_by_version` handlers already exist in `apps/server/aegis-server/src/transport/http/crf/handlers.rs`).
- No new server endpoint.
- No new TS DTOs.
- No full-text highlighting / snippets / relevance ranking.
- No cross-version search. The search is scoped to the version selected in the source page, carried via `?versionId=`.
- No infinite-scroll list rendering on first page-load when search is empty.
- No per-row mutations (edit / delete) from the search page — read-only by design.
- No sortable columns / persistent column widths.
- No modification to the existing per-form filter drawer (`CrfFormFilterDrawer`).

---

## 3. UX

### 3.1 `CrfFormListPage` toolbar — `Global Search` menu item

Already present in `CrfToolsMenu`, but it currently navigates without `?versionId=`. Fix: pass the selected `versionId` so the search page opens on the same version the user was browsing.

```ts
<CrfToolsMenu projectCode={projectCode} versionId={selectedVersionId} />
```

The menu's "Global Search" item navigates to:

```ts
navigate({
  to: "/project/$projectCode/crf/search",
  params: { projectCode },
  search: versionId != null ? { versionId } : undefined,
});
```

`CrfDetailPage` already preserves `versionId` via `search: (prev: Record<string, unknown>) => prev`, so the same menu works there unchanged — the detail page passes the carried `versionId` through.

### 3.2 `CrfGlobalSearchPage`

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [←]  [Search forms, items, units, options, annotations…           🔍]        │
│                                                                              │
│ [Forms | Items | Units | Options | Domain annotations | Annotations]   Tabs  │
│                                                                              │
│ ┌───────────────────────────────────────────────────────────────────────┐    │
│ │ Code │ Name │ Kind │      ⤴                                           │    │  Items tab
│ ├───────────────────────────────────────────────────────────────────────┤    │
│ │ AET  │ Term │ text │      ⤴                                           │    │
│ │ ...                                                                     │    │
│ └───────────────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────────────┘
```

| Concern | Behaviour |
|---|---|
| Back arrow | Navigate to `/project/$projectCode/crf?versionId=<versionId>`. |
| Search input | Reused `TermFilterBar` (controlled, debounced 300 ms). Placeholder `crf.globalSearch.searchPlaceholder`. |
| Tabs | MUI `<ToggleButtonGroup exclusive>` (size small). Default `forms`. Local state, not URL state. |
| Empty input | Hint (`crf.globalSearch.emptyInput`) replaces the table. Tabs remain usable (queries disabled until fragment is non-empty). |
| Loading | Per-tab: when `loading === true` and no rows yet → centered `<CircularProgress />`. When rows exist, keep them visible during refetch. |
| Error | Per-tab: inline `<Alert severity="error">` + Retry `<Button>` calling `refetch()`. i18n key `crf.globalSearch.loadFailed.<kind>`. |
| Empty results | Per-tab: `crf.globalSearch.noMatches.<kind>`. |
| Operation column | `<IconButton size="small">` with `<LaunchIcon>`, wrapped in `<Tooltip title={crf.globalSearch.row.openTooltip}>`. Click handler navigates per tab (§3.3). |

### 3.3 Per-tab row → navigation

| Tab | Row shape | Click action |
|---|---|---|
| Forms | `CrfForm { id, versionId, code, name, … }` | `navigate(/crf/$formId, { search: { versionId, focus: "form-<id>" } })` |
| Items | `CrfItem { id, formId, code, name, kind, … }` | `navigate(/crf/$formId, { search: { versionId, focus: "item-<id>" } })` |
| Units | `CrfUnit { id, itemId, value, … }` | Need `item.formId` — `<UnitRow>` reads item via `useGetCrfItem(itemId)`; click reads `item.formId` and navigates with `focus: "unit-<id>"`. If `item` is missing in cache the click is a no-op (form no longer exists). |
| Options | `CrfOption { id, itemId, value, … }` | Symmetric to Units: `focus: "option-<id>"`. |
| Domain annotations | `DomainAnnotation { id, formId, name, description, … }` | `focus: "domain-<id>"` (maps to existing `data-testid="domain-annotation-chip-<id>"`). |
| Annotations | `Annotation { id, domainAnnotationId, owner: {kind, id}, … }` | Derive formId: `owner.kind === "form"` → formId = owner.id; `owner.kind === "item"` / `"option"` / `"unit"` → look up the item via `useGetCrfItem`, then `formId = item.formId`. `focus: "annotation-<id>"` (maps to a new `data-testid="crf-annotation-<id>"` on the annotation chip). |

### 3.4 `CrfDetailPage` — auto-scroll on mount

Reads `useSearch({ strict: false })` for `focus`. When `focus` is set AND `detail` is loaded, parse `focus = "<kind>-<id>"` and `scrollIntoView`:

```ts
const focus = search.focus;       // "item-21" | "unit-31" | "option-41" | "domain-50" | "annotation-71"
useEffect(() => {
  if (!focus || !detail) return;
  const [kind, idStr] = focus.split("-");
  if (!kind || !idStr) return;
  const el = document.querySelector(`[data-testid="crf-${kind}-${idStr}"]`)
          ?? document.querySelector(`[data-testid="domain-annotation-chip-${idStr}"]`);
  el?.scrollIntoView({ block: "start", behavior: "smooth" });
}, [focus, detail]);
```

Two testid prefixes are tried because the existing `CrfDetailPage` uses `data-testid="domain-annotation-chip-<id>"` for domain annotation chips (line 354 of `CrfDetailPage.tsx`). Every other anchor uses the `crf-<kind>-<id>` convention.

The scroll container in `CrfDetailPage` (line 400) is the `Box` wrapping `detail.items.map(…)` — `scrollIntoView` walks up to find a scrollable ancestor, so no container ref is needed.

`focus="form-<id>"` is a no-op for scroll (no specific anchor), but useful as a marker that we came from a form-row click so we can decide whether to also open the form-name popover. We won't do that — keep it simple.

### 3.5 Per-tab tables (column choices)

| Tab | Column | Source | i18n key |
|---|---|---|---|
| Forms | Code | `row.code` | `crf.globalSearch.col.code` |
| Forms | Name | `row.name` | `crf.globalSearch.col.name` |
| Forms | (action) | `<IconButton>` + `<LaunchIcon>` | — |
| Items | Code | `row.code` | `crf.globalSearch.col.code` |
| Items | Name | `row.name` | `crf.globalSearch.col.name` |
| Items | Kind | `row.kind` | `crf.globalSearch.col.kind` |
| Items | (action) | `<IconButton>` + `<LaunchIcon>` | — |
| Units | Form code | via `<FormCodeCell formId={row.itemFormId} />` (uses `useGetCrfItem` then `useGetCrfForm`) | `crf.globalSearch.col.code` (reused — header reads "Code (form)" if ambiguous; we'll alias via two separate columns: "Form" + "Unit" — see below) |
| Units | Value | `row.value` | `crf.globalSearch.col.value` |
| Units | (action) | `<IconButton>` + `<LaunchIcon>` | — |
| Options | Form code | via `<FormCodeCell>` | `crf.globalSearch.col.code` |
| Options | Value | `row.value` | `crf.globalSearch.col.value` |
| Options | (action) | `<IconButton>` + `<LaunchIcon>` | — |
| Domain annotations | Name | `row.name` | `crf.globalSearch.col.name` |
| Domain annotations | Description | `row.description` | `crf.globalSearch.col.description` |
| Domain annotations | (action) | `<IconButton>` + `<LaunchIcon>` | — |
| Annotations | Content | `row.content` | `crf.globalSearch.col.content` |
| Annotations | Assigned | `row.assign ? "✓" : ""` | `crf.globalSearch.col.assign` |
| Annotations | Owner | `row.owner.kind + ":" + row.owner.id` | `crf.globalSearch.col.owner` |
| Annotations | (action) | `<IconButton>` + `<LaunchIcon>` | — |

Units and Options use **two columns** to disambiguate "form code" from "item code" / "unit value":

- **Units tab** columns: `Form code` | `Item code` | `Unit value` | (action)
  - `Form code` via `useGetCrfItem(itemId)` (cached) → `item.formId` → `useGetCrfForm(formId)` (cached) → `form.code`.
  - `Item code` via `useGetCrfItem(itemId)` (cached) → `item.code`.
  - `Unit value` is `row.value`.
  - React Query dedupes repeated `itemId` lookups across rows, so 50 units under the same item only fire two HTTP calls.
- **Options tab** columns: `Form code` | `Item code` | `Option value` | (action) — symmetric.

This avoids forcing a single overloaded "Code" column to mean different things per tab.

`<FormCodeCell>` / `<ItemCodeCell>` are private sub-components that call `useGetCrfItem` / `useGetCrfForm` and render `data?.code ?? #${id}`. The `#${id}` fallback keeps the table usable while the parent is loading or has errored.

---

## 4. Architecture

```
CrfFormListPage / CrfDetailPage → click "Global Search" in CrfToolsMenu
  → navigate(/project/$projectCode/crf/search, { versionId })
       → CrfGlobalSearchPage
           ├─ Back IconButton → navigate(/project/$projectCode/crf, { versionId })
           ├─ TermFilterBar (controlled, debounced 300 ms)
           ├─ ToggleButtonGroup (forms|items|units|options|domains|annotations) — local state
           ├─ if trimmedFragment === "" → hint
           └─ else render active tab's table
                ├─ SearchFormsTable        → useSearchCrfForms(v, fragment, { enabled: tab==="forms" && fragment!="" })
                ├─ SearchItemsTable        → useSearchCrfItems(v, fragment, …)
                ├─ SearchUnitsTable        → useSearchCrfUnits(v, fragment, …) — per-row useGetCrfItem
                ├─ SearchOptionsTable      → useSearchCrfOptions(v, fragment, …) — per-row useGetCrfItem
                ├─ SearchDomainAnnotationsTable → useSearchCrfDomainAnnotations(v, fragment, …)
                └─ SearchAnnotationsTable  → useSearchCrfAnnotations(v, fragment, …) — per-row owner → formId chain
                       → click row → navigate(/crf/$formId, { versionId, focus: "<kind>-<id>" })
                             → CrfDetailPage reads ?focus=…, scrolls on detail load
```

The page never writes to the URL when toggling tabs or typing — only navigation events (back / launch row / forward from `CrfFormListPage`) carry params.

---

## 5. Routing

### 5.1 `search.tsx` — new `validateSearch`

`apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx`:

```ts
export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/search",
)({
  validateSearch: (raw): { versionId?: number } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === "" ? undefined : Number(raw.versionId)
        : typeof raw.versionId === "number" ? raw.versionId : undefined,
  }),
  component: CrfGlobalSearchPage,
});
```

Public path: `/project/$projectCode/crf/search`. Path params: `projectCode`. Search params: `{ versionId?: number }`. The page also receives `focus` from forward navigation (see §5.2), but it's not a search param of this route — only the detail route declares it. `routeTree.gen.ts` is auto-regenerated.

### 5.2 `$formId.tsx` — extended `validateSearch`

`apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx`:

The detail route already has a `validateSearch` for `versionId` (the `CrfDetailPage` reads it via `useSearch({ strict: false })`). Extend it to also accept `focus`:

```ts
validateSearch: (raw): { versionId?: number; focus?: string } => ({
  versionId: ...same as today...,
  focus: typeof raw.focus === "string" && raw.focus !== "" ? raw.focus : undefined,
}),
```

If today's detail route does **not** already declare `validateSearch`, add both `versionId` and `focus` so the detail page can read either. (Plan step will verify which is the case.)

### 5.3 Navigation recipes

```ts
// CrfFormListPage / CrfDetailPage → CrfGlobalSearchPage (forward)
navigate({
  to: "/project/$projectCode/crf/search",
  params: { projectCode },
  search: versionId != null ? { versionId } : undefined,
});

// CrfGlobalSearchPage → CrfFormListPage (back)
navigate({
  to: "/project/$projectCode/crf",
  params: { projectCode },
  search: versionId != null ? { versionId } : undefined,
});

// Search row → CrfDetailPage (per tab)
navigate({
  to: "/project/$projectCode/crf/$formId",
  params: { projectCode, formId: targetFormId },
  search: {
    versionId: versionId ?? undefined,
    focus: "<kind>-<id>",
  },
});
```

`targetFormId` is `row.id` for Forms rows, `row.formId` for Items and Domain annotations rows, derived from `item.formId` for Units / Options / Annotations rows (per §3.3).

---

## 6. Public contracts (TS)

### 6.1 `api.*` wrappers (modified)

`apps/desktop/aegis-desktop/src/shared/api/index.ts` — six new wrappers, all returning the existing envelope arrays:

```ts
searchCrfFormsByVersion: async (versionId: number, fragment: string): Promise<CrfForm[]> => {
  const resp = await call<CrfFormListResponse>("search_crf_forms_by_version", { versionId, fragment });
  return resp.forms;
},
searchCrfItemsByVersion: async (versionId: number, fragment: string): Promise<CrfItem[]> => {
  const resp = await call<{ items: CrfItem[] }>("search_crf_items_by_version", { versionId, fragment });
  return resp.items;
},
searchCrfOptionsByVersion: async (versionId: number, fragment: string): Promise<CrfOption[]> => {
  const resp = await call<CrfOptionListResponse>("search_crf_options_by_version", { versionId, fragment });
  return resp.options;
},
searchCrfUnitsByVersion: async (versionId: number, fragment: string): Promise<CrfUnit[]> => {
  const resp = await call<CrfUnitListResponse>("search_crf_units_by_version", { versionId, fragment });
  return resp.units;
},
searchCrfDomainAnnotationsByVersion: async (versionId: number, fragment: string): Promise<DomainAnnotation[]> => {
  const resp = await call<DomainAnnotationListResponse>("search_crf_domain_annotations_by_version", { versionId, fragment });
  return resp.domainAnnotations;
},
searchCrfAnnotationsByVersion: async (versionId: number, fragment: string): Promise<Annotation[]> => {
  const resp = await call<AnnotationListResponse>("search_crf_annotations_by_version", { versionId, fragment });
  return resp.annotations;
},
```

(Use the existing `CrfItemListResponse` shape if it's already defined; if not, declare the inline `{ items: CrfItem[] }` literal that mirrors the server wire shape — confirmed in plan step.)

### 6.2 Query key factory (add)

`apps/desktop/aegis-desktop/src/shared/query/keys.ts` — six new entries under `queryKeys.crf`:

```ts
searchFormsByVersion: (v: number, f: string) =>
  ["crf", "searchFormsByVersion", v, f] as const,
searchItemsByVersion: (v: number, f: string) =>
  ["crf", "searchItemsByVersion", v, f] as const,
searchUnitsByVersion: (v: number, f: string) =>
  ["crf", "searchUnitsByVersion", v, f] as const,
searchOptionsByVersion: (v: number, f: string) =>
  ["crf", "searchOptionsByVersion", v, f] as const,
searchDomainAnnotationsByVersion: (v: number, f: string) =>
  ["crf", "searchDomainAnnotationsByVersion", v, f] as const,
searchAnnotationsByVersion: (v: number, f: string) =>
  ["crf", "searchAnnotationsByVersion", v, f] as const,
```

### 6.3 New hooks

`apps/desktop/aegis-desktop/src/features/crf/data/search.ts`:

```ts
type EnabledOptions = { enabled?: boolean };

export function useSearchCrfForms(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfForm[], ApiError> { … }
// …one per entity type, same shape.

export function useGetCrfItem(id: number | null): UseQueryResult<CrfItem, ApiError> {
  return useQuery({
    queryKey: queryKeys.crf.item(id ?? 0),
    queryFn: () => api.getCrfItemById(id!),
    enabled: id != null && id > 0,
  });
}
```

Each `useSearchCrfXxx` hook:
- `enabled: options?.enabled !== false && versionId != null && versionId > 0 && fragment.trim() !== ""`.
- Uses `queryKeys.crf.searchXxxByVersion(versionId, fragment)` and `api.searchCrfXxxByVersion(versionId, fragment)`.
- Project defaults apply (`staleTime: Infinity`, `retry: false`).

`useGetCrfItem` is needed for the per-row lookups in Units / Options / Annotations tables. It reuses the existing `queryKeys.crf.item` key (the form-detail page already reads `useGetCrfForm` from that key shape — wait, `crf.item` doesn't exist yet; the spec adds it here).

Add the `crf.item` query key in §6.2's block:

```ts
item: (id: number) => ["crf", "item", id] as const,
```

### 6.4 Reused hooks

- `useGetCrfForm(id)` — already exists; used inside `<FormCodeCell>` and the Annotations row's owner → form chain.
- `useDebouncedValue` — already exists; used for the search input (300 ms / 1000 ms max wait, matches `GlobalTermSearchPage`).

### 6.5 Tauri command + HTTP layer (new)

For each of the six search endpoints, add one HTTP fn + one `#[tauri::command]`. Naming: `http::crf::<area>::search_by_version` + `commands::crf::<area>::search_crf_<area>_by_version`. Pattern matches existing `list_by_version` / `list_crf_forms_by_version`.

Example for form area (`apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`):

```rust
pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<CrfFormListResponse, ApiError> {
    let encoded = percent_encoding::utf8_percent_encode(&fragment, NON_ALPHANUMERIC).to_string();
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/forms/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

`apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`:

```rust
#[tauri::command]
pub async fn search_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfFormListResponse, ApiError> {
    form::search_by_version(&client, version_id, fragment).await
}
```

Same shape per area (`item`, `option`, `unit`, `domain_annotation`, `annotation`). Five additional `http/crf/*.rs` files get one new `search_by_version` fn each; five additional `commands/crf/*.rs` files get one new `#[tauri::command]` each; one new top-level re-export entry in `commands/crf.rs` (or wherever the area modules are aggregated today).

Each new HTTP fn gets a wiremock test that asserts the path contains `forms/search?fragment=…` (or its area-specific counterpart) and decodes the response envelope.

`NON_ALPHANUMERIC` and `percent_encoding` are already in the Tauri crate's dependency graph (used by the terminology search code) — no new deps.

---

## 7. i18n keys

The four-column skeleton's column keys (`crf.globalSearch.col.{form,item,option,annotation}`) are deleted — they don't fit the per-kind tab design. Replaced by:

| Key | en | zhCN |
|---|---|---|
| `crf.globalSearch.heading` | `CRF Global Search — {projectCode}` | (existing — verbatim) |
| `crf.globalSearch.searchPlaceholder` | `Search forms, items, units, options, annotations…` | (existing — verbatim) |
| `crf.globalSearch.emptyInput` | `Type a search term to find forms, items, units, options, or annotations` | `请输入搜索关键字以查找表单、项目、单位、选项或注解` |
| `crf.globalSearch.tab.forms` | `Forms` | `表单` |
| `crf.globalSearch.tab.items` | `Items` | `项目` |
| `crf.globalSearch.tab.units` | `Units` | `单位` |
| `crf.globalSearch.tab.options` | `Options` | `选项` |
| `crf.globalSearch.tab.domainAnnotations` | `Domain annotations` | `域注解` |
| `crf.globalSearch.tab.annotations` | `Annotations` | `注解` |
| `crf.globalSearch.col.code` | `Code` | `代码` |
| `crf.globalSearch.col.name` | `Name` | `名称` |
| `crf.globalSearch.col.kind` | `Kind` | `类型` |
| `crf.globalSearch.col.value` | `Value` | `取值` |
| `crf.globalSearch.col.description` | `Description` | `描述` |
| `crf.globalSearch.col.content` | `Content` | `内容` |
| `crf.globalSearch.col.assign` | `Assigned` | `已分配` |
| `crf.globalSearch.col.owner` | `Owner` | `所有者` |
| `crf.globalSearch.col.formCode` | `Form code` | `表单代码` |
| `crf.globalSearch.col.itemCode` | `Item code` | `项目代码` |
| `crf.globalSearch.col.unitValue` | `Unit value` | `单位取值` |
| `crf.globalSearch.col.optionValue` | `Option value` | `选项取值` |
| `crf.globalSearch.loadFailed.forms` | `Failed to load forms: {message}` | `加载表单失败：{message}` |
| `crf.globalSearch.loadFailed.items` | `Failed to load items: {message}` | `加载项目失败：{message}` |
| `crf.globalSearch.loadFailed.units` | `Failed to load units: {message}` | `加载单位失败：{message}` |
| `crf.globalSearch.loadFailed.options` | `Failed to load options: {message}` | `加载选项失败：{message}` |
| `crf.globalSearch.loadFailed.domainAnnotations` | `Failed to load domain annotations: {message}` | `加载域注解失败：{message}` |
| `crf.globalSearch.loadFailed.annotations` | `Failed to load annotations: {message}` | `加载注解失败：{message}` |
| `crf.globalSearch.noMatches.forms` | `No matching forms` | `无匹配表单` |
| `crf.globalSearch.noMatches.items` | `No matching items` | `无匹配项目` |
| `crf.globalSearch.noMatches.units` | `No matching units` | `无匹配单位` |
| `crf.globalSearch.noMatches.options` | `No matching options` | `无匹配选项` |
| `crf.globalSearch.noMatches.domainAnnotations` | `No matching domain annotations` | `无匹配域注解` |
| `crf.globalSearch.noMatches.annotations` | `No matching annotations` | `无匹配注解` |
| `crf.globalSearch.row.openTooltip` | `Open in form detail` | `在表单详情中打开` |

Existing keys reused: `crf.globalSearch.{heading, searchPlaceholder}`, `crf.toolbar.globalSearch`, `crf.detail.back`, `common.retry`, `common.open`, `common.cancel`, `common.back`.

Deleted keys (no longer fit the tab design):
- `crf.globalSearch.empty` (was a per-table empty string — replaced by `noMatches.<kind>`)
- `crf.globalSearch.col.{form,item,option,annotation}` (replaced by `<kind>-specific column keys` above)

---

## 8. Error handling

| Scenario | Behaviour |
|---|---|
| Network failure on any tab | Inline `<Alert severity="error">` + Retry `<Button>` → `query.refetch()`. Per-tab — never rolls up across tabs. |
| Empty fragment after debounce | Page hides all tables and shows `crf.globalSearch.emptyInput`. Tabs remain usable (queries disabled). |
| `versionId` invalid / missing | Page shows `crf.globalSearch.emptyInput` (no tabs render anything). CrfToolsMenu entry-point button is disabled when `selectedVersionId == null`. |
| `useGetCrfItem` failure inside a Units / Options / Annotations row | Cell falls back to `#${id}` — table does not break. |
| Annotation row click with no item in cache (deleted between search and click) | Click waits for the query and is a no-op if the item resolves to `undefined` (form no longer exists). |
| Form deleted between search and click | Navigates to `/crf/$formId`; detail page shows `crf.detail.loadFailed` alert. |
| Token expired / 401 | Existing `ApiError` handling; auth interceptor redirects to login. |

---

## 9. Files touched

**New files**

- `apps/desktop/aegis-desktop/src/features/crf/data/search.ts`

**Modified**

- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx` — replace skeleton with the tab-based design
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfToolsMenu.tsx` — add `versionId?: number | null` prop; thread to `navigate({ search: { versionId } })`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx` — pass `versionId={selectedVersionId}` to `<CrfToolsMenu>`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` — read `focus` from `useSearch`; add `useEffect` that scrolls on detail load
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx` — add `data-testid={`crf-annotation-${annotation.id}`}` if not present (verify in plan step)
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx` — add `validateSearch` for `versionId`
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx` — extend `validateSearch` to add `focus` (if `versionId` is already declared, add `focus`; otherwise add both)
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — six new wrappers
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — six new search keys + `crf.item`
- `lib/packages/ui/src/i18n/locales/en.ts` and `lib/packages/ui/src/i18n/locales/zhCN.ts` — new keys; remove the four deleted column keys
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs` — `search_by_version` fn + wiremock test
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs` — same
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/option.rs` — same
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/unit.rs` — same
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs` — same
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs` — same
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` — `search_crf_forms_by_version` command
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs` — `search_crf_items_by_version`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/option.rs` — `search_crf_options_by_version`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/unit.rs` — `search_crf_units_by_version`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs` — `search_crf_domain_annotations_by_version`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs` — `search_crf_annotations_by_version`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs` (or wherever the area modules are aggregated today) — register the six new commands
- `apps/desktop/aegis-desktop/src-tauri/src/main.rs` (or equivalent Tauri builder) — register the six new commands in the `invoke_handler` list

**Untouched (verified)**

- aegis-server (no Rust edits)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (no new DTOs)
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` (testids already present: `crf-item-row-<id>`, `crf-option-<id>`, `crf-unit-<id>`)
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormFilterDrawer.tsx` (per-form search hint)
- All other existing CRF pages, hooks, query keys, wire DTOs
- Existing Tauri commands (only new ones added)

---

## 10. Testing

### 10.1 Rust (Tauri HTTP layer, wiremock)

For each new `http::crf::*::search_by_version` fn:

- `search_by_version_with_fragment_includes_query_param` — sends the fn with a known fragment, asserts the mock server's path contains `fragment=<url-encoded>`.
- `search_by_version_with_special_chars_url_encodes_fragment` — sends a fragment with spaces / unicode / `&`, asserts the URL is properly percent-encoded (the server's behavior on `&` matters most).
- `search_by_version_with_empty_fragment_returns_400` — server returns 400; client surfaces the error as `ApiError`.

Run with `cargo test -p aegis-desktop --lib http::crf`. Total: 3 tests × 6 areas = 18 new tests.

### 10.2 TS / UI

The project has no automated UI test suite for the desktop app, and the spec does not request new tests. Verification is manual (§10.3).

### 10.3 Manual UI walkthrough

1. From `/project/<code>/crf?versionId=<v>`, click `Global Search` in the toolbar → lands on `/project/<code>/crf/search?versionId=<v>`. Selected version preserved.
2. Empty input → hint visible; tabs remain clickable but disabled.
3. Type `xyz` → Forms tab loads, shows matches. Switching to Items / Units / Options / Domain annotations / Annotations triggers each tab's fetch on first select.
4. Click a Forms row → navigates to `/project/<code>/crf/<formId>?versionId=<v>&focus=form-<id>`. Detail page loads.
5. Click an Items row → detail page loads, scroll position lands on the item row.
6. Click a Units / Options row → scroll lands on the unit / option row.
7. Click a Domain annotations row → scroll lands on the domain annotation chip.
8. Click an Annotations row → scroll lands on the annotation chip.
9. Browser back returns to the search page with the previous tab still selected.
10. Network failure on Forms tab → alert + Retry. Other tabs unaffected.
11. From `/project/<code>/crf/<formId>`, click `Global Search` → back-arrow returns to the detail page (search and detail both preserved).

### 10.4 Verification commands

- `pnpm --filter aegis-desktop tsc --noEmit` — TS type check.
- `pnpm --filter aegis-desktop build` — Vite build.
- `cargo test -p aegis-desktop --lib http::crf` — new Tauri wiremock tests.
- `cargo check -p aegis-desktop` — Rust type check.

---

## 11. Rollback

All changes are additive except:

- `crf.globalSearch.col.{form,item,option,annotation}` keys deleted from `en.ts` / `zhCN.ts`. These are unused outside the new page, so reverting the i18n edits removes any consumer. No other file references them.
- `CrfToolsMenu` gains a `versionId?` prop. If the prop is omitted the menu still works — the `navigate` call's `search` simply becomes `undefined`.

No migrations, no schema changes, no server changes — rollback is purely a desktop-app revert (frontend + Tauri command surface + i18n).

---

## 12. Out of scope

- Server-side highlighting / snippets / relevance ranking
- Cross-version search
- Editing / deleting rows from the search page
- Sortable columns / persistent column widths
- Per-tab infinite-scroll (server returns all matches today — no pagination)
- Modifying the existing per-form filter drawer (`CrfFormFilterDrawer`)
- Hover-preview of the row's matching fragment
- Keyboard shortcut to open the global search page
