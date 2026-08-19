# Aegis Desktop — Terminology Management Pages — Design

**Date:** 2026-08-19
**Status:** Approved (pending spec review)
**Scope:** Add a `Terminology` feature to `aegis-desktop` for browsing and editing the SDTM / ADaM terminology versions, code lists, and code items that already exist in the `terminology` server module. New sidebar entry above `Management` (admin/root gated), two pages (`Terminology` and `CodeListDetail`), per-resource Tauri command shims, React Query wiring, and i18n keys.

---

## 1. Goals

1. Add two top-level routes — `/terminology/sdtm` and `/terminology/adam` — listing the code lists for the selected terminology version of that kind. Each version is a published CDISC terminology release identified by `(kind, name)` and is fetched from the existing `/api/terminology/versions` endpoint.
2. Add a detail route per kind — `/terminology/sdtm/codelists/$codelistId` and `/terminology/adam/codelists/$codelistId` — listing the code items for the selected codelist.
3. Reuse the existing `/_authed/_layout/` pathful layout so every new page picks up `AppLayout` (sidebar + footer) automatically. The pages themselves are reachable by every authenticated user; mutation controls are gated to `admin` and `root`.
4. Wire 13 Tauri command shims covering every server endpoint under `/api/terminology/{versions,code-lists,code-items}` that already exists in `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`.
5. Follow the existing `user` and `project` feature conventions: `data/` directory for React Query hooks, `pages/` for route components, `components/` for presentational components, single barrel `index.ts`.
6. Out of scope: the `ImportTerminology` page (placeholder only), single-resource GET endpoints on the server (we list + filter client-side), pagination (the list endpoint returns all rows).

---

## 2. URL map

| Path                                          | Route file                                                                              | Component                  |
| --------------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------- |
| `/terminology/sdtm`                           | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm.tsx`            | `TerminologyPage kind="sdtm"` |
| `/terminology/adam`                           | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam.tsx`            | `TerminologyPage kind="adam"` |
| `/terminology/sdtm/codelists/$codelistId`     | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm/codelists/$codelistId.tsx` | `CodeListDetailPage` |
| `/terminology/adam/codelists/$codelistId`     | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam/codelists/$codelistId.tsx` | `CodeListDetailPage` |

`CodeListDetailPage` does **not** take `kind` as a prop. The kind is derived inside the component by looking up the codelist's `version_id` in the cached versions list.

---

## 3. Files added / changed / removed

### 3.1 Added

#### Frontend

| Path                                                                                              | Responsibility |
| ------------------------------------------------------------------------------------------------- | -------------- |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`                   | Route component for SDTM/ADaM list page. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx`                | Route component for the per-codelist detail. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/VersionDropdown.tsx`              | `<Select>` of versions filtered by `kind`. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/TermFilterBar.tsx`                | Search `<TextField>` for codelists and code items. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx`                | Presentational table; `mode: "list" \| "single"`. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx`                | Presentational table for code items. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListDrawer.tsx`               | Right-side MUI `Drawer` form for create / edit codelist. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemDrawer.tsx`               | Right-side MUI `Drawer` form for create / edit code item. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/DescriptionsCell.tsx`             | Renders the 3-row SYN / DEF / NCI description block, skipping empties. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/ExtensibleIcon.tsx`              | Small ↗ icon shown after a code when `extensible === true`. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx`                 | Placeholder `IconButton` showing a "coming soon" Snackbar. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`                                | React Query hooks for all 13 endpoints. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/index.ts`                               | Barrel re-exporting hooks + types. |
| `apps/desktop/aegis-desktop/src/features/terminology/index.ts`                                    | Feature barrel. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm.tsx`                     | Route file. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam.tsx`                     | Route file. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm/codelists/$codelistId.tsx`| Route file. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam/codelists/$codelistId.tsx`| Route file. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/*.test.tsx`                      | Vitest + RTL tests per presentational component. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/list.test.ts`                           | Hook tests with mocked `api`. |

#### Tauri (Rust)

| Path                                                              | Responsibility |
| ----------------------------------------------------------------- | -------------- |
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology.rs`     | Module declaration: `pub mod version; pub mod code_list; pub mod code_item;` |
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/version.rs`     | DTOs + 5 HTTP functions for versions. |
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs`   | DTOs + 5 HTTP functions for code lists. |
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`   | DTOs + 5 HTTP functions for code items. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs`        | Module declaration: `pub mod version; pub mod code_list; pub mod code_item;` |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/version.rs` | Tauri command shims for versions. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs`| Tauri command shims for code lists. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`| Tauri command shims for code items. |

#### i18n

| Path                                                            | Responsibility |
| --------------------------------------------------------------- | -------------- |
| `lib/packages/ui/src/i18n/locales/en.ts`                        | + ~25 keys under `nav.*`, `terminology.*`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts`                      | + matching zh-CN translations. |

### 3.2 Modified

| Path                                                                                       | Change |
| ------------------------------------------------------------------------------------------ | ------ |
| `apps/desktop/aegis-desktop/src/shared/api/types.ts`                                       | + `TerminologyKind`, `TerminologyVersionView`, `TerminologyVersionListResponse`, `CodeListView`, `CodeListListResponse`, `CodeItemView`, `CodeItemListResponse`, `CreateCodeListInput`, `UpdateCodeListInput`, `CreateCodeItemInput`, `UpdateCodeItemInput`, `CreateTerminologyVersionInput`, `UpdateTerminologyVersionInput`, `SearchTerminologyQuery`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts`                                       | + 13 wrapper functions on the `api` object, + matching type re-exports. |
| `apps/desktop/aegis-desktop/src/shared/query/keys.ts`                                      | + `terminology` family. |
| `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`                     | + `terminologyEntry` (`MenuBook` icon) inserted between Projects and Management, gated on `canManage`. Sub-entries `SDTM` (`Storage` icon) and `ADaM` (`Analytics` icon). |
| `apps/desktop/aegis-desktop/src-tauri/src/http.rs`                                         | + `pub mod terminology;` |
| `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`                                     | + `TerminologyKind` enum (`#[serde(rename_all = "lowercase")] { Sdtm, Adam }`), shared by every resource file under `http/terminology/`. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`                                     | + `pub mod terminology;` |
| `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`                                          | + 13 entries in the `invoke_handler!` macro. |

### 3.3 Removed

None.

---

## 4. Final directory layout

```
apps/desktop/aegis-desktop/
├── src/
│   ├── features/
│   │   ├── app/components/AppLayout.tsx          (modified — +terminologyEntry)
│   │   └── terminology/                          (new)
│   │       ├── index.ts
│   │       ├── pages/
│   │       │   ├── TerminologyPage.tsx
│   │       │   └── CodeListDetailPage.tsx
│   │       ├── components/
│   │       │   ├── VersionDropdown.tsx
│   │       │   ├── TermFilterBar.tsx
│   │       │   ├── CodeListTable.tsx
│   │       │   ├── CodeItemTable.tsx
│   │       │   ├── CodeListDrawer.tsx
│   │       │   ├── CodeItemDrawer.tsx
│   │       │   ├── DescriptionsCell.tsx
│   │       │   ├── ExtensibleIcon.tsx
│   │       │   └── ImportButton.tsx
│   │       └── data/
│   │           ├── list.ts
│   │           └── index.ts
│   ├── routes/_authed/_layout/terminology/        (new)
│   │   ├── sdtm.tsx
│   │   ├── adam.tsx
│   │   ├── sdtm/codelists/$codelistId.tsx
│   │   └── adam/codelists/$codelistId.tsx
│   ├── shared/
│   │   ├── api/
│   │   │   ├── types.ts                           (modified)
│   │   │   └── index.ts                           (modified)
│   │   └── query/
│   │       └── keys.ts                            (modified)
└── src-tauri/src/
    ├── http.rs                                    (modified)
    ├── http/
    │   ├── dto.rs                                 (modified)
    │   └── terminology/
    │       ├── version.rs
    │       ├── code_list.rs
    │       └── code_item.rs
    ├── commands.rs                                (modified)
    └── commands/
        ├── terminology/
        │   ├── version.rs
        │   ├── code_list.rs
        │   └── code_item.rs
        └── (existing modules unchanged)
```

---

## 5. Routing

Four new file routes under the existing `/_authed/_layout/` pathful layout:

```tsx
// routes/_authed/_layout/terminology/sdtm.tsx
export const Route = createFileRoute("/_authed/_layout/terminology/sdtm")({
  component: TerminologyPageSdtm,
});

function TerminologyPageSdtm() {
  return <TerminologyPage kind="sdtm" />;
}
```

Same shape for `adam.tsx`. The detail routes:

```tsx
// routes/_authed/_layout/terminology/sdtm/codelists/$codelistId.tsx
export const Route = createFileRoute(
  "/_authed/_layout/terminology/sdtm/codelists/$codelistId",
)({
  parseParams: (raw) => ({ codelistId: Number(raw.codelistId) }),
  stringifyParams: ({ codelistId }) => ({ codelistId: String(codelistId) }),
  component: CodeListDetailPage,
});
```

`parseParams` / `stringifyParams` keep the URL stable while coercing to `number` for the hook inputs.

The new routes register themselves automatically via the TanStack Router Vite plugin; `routeTree.gen.ts` is regenerated on `vite dev` / `vite build`.

---

## 6. Pages

### 6.1 `TerminologyPage`

Owns local state: `selectedVersionId: number | null`, `search: string`, `codelistDrawer: { mode: 'create' | 'edit' | null; row?: CodeListView }`. Layout:

```tsx
<Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
  <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
    <TermFilterBar query={search} onQueryChange={setSearch} sx={{ flexGrow: 1 }} />
    <VersionDropdown kind={kind} value={selectedVersionId} onChange={setSelectedVersionId} disabled={versions.length === 0} />
    <ImportButton />
  </Box>
  <CodeListTable
    mode="list"
    rows={filteredCodeLists}
    loading={codeListsQuery.isLoading}
    mutationLoading={anyMutationPending}
    error={codeListsQuery.error}
    onRetry={codeListsQuery.refetch}
    onCreate={openCreateDrawer}
    onEdit={openEditDrawer}
    onDelete={onDeleteCodelist}
    onOpen={onOpenCodelist}
    canMutate={canMutate}
  />
  <CodeListDrawer
    open={drawerState.mode !== null}
    mode={drawerState.mode ?? 'create'}
    row={drawerState.row}
    versions={versions}
    versionId={selectedVersionId ?? 0}
    onClose={closeDrawer}
    onCreate={onCreateCodelist}
    onUpdate={onUpdateCodelist}
    canMutate={canMutate}
  />
</Box>
```

`filteredCodeLists` is a `useMemo` that:
1. Filters by `search.trim().toLowerCase()` substring against `code`, `name`, `submissionValue`, `synonym`, `definition`, `nciPreferredTerm`.
2. Returns the unchanged list when `search` is empty.

`canMutate` is `currentUser.data?.role === 'admin' || currentUser.data?.role === 'root'`. The check is at the page level so it threads through both the table and the drawer.

### 6.2 `CodeListDetailPage`

Reads `codelistId` from `Route.useParams()`. Owns: `search: string`, `itemDrawer`, `editCodelistDrawer`. Layout:

```tsx
<Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
    <BackButton onClick={() => navigate({ to: backLink })} disabled={!backLink} />
    <Breadcrumb items={breadcrumbItems} />
  </Box>
  <CodeListTable
    mode="single"
    rows={[codelist]}
    loading={codelistQuery.isLoading}
    mutationLoading={anyMutationPending}
    error={codelistQuery.error ?? itemsQuery.error}
    onRetry={() => { codelistQuery.refetch(); itemsQuery.refetch(); }}
    onEdit={openEditCodelistDrawer}
    canMutate={canMutate}
  />
  <TermFilterBar query={search} onQueryChange={setSearch} />
  <CodeItemTable
    rows={filteredItems}
    loading={itemsQuery.isLoading}
    mutationLoading={anyMutationPending}
    error={itemsQuery.error}
    onRetry={itemsQuery.refetch}
    onCreate={openCreateItemDrawer}
    onEdit={openEditItemDrawer}
    onDelete={onDeleteItem}
    canMutate={canMutate}
  />
  <CodeListDrawer mode="edit" row={codelist} ... />
  <CodeItemDrawer mode={itemDrawer.mode} row={itemDrawer.row} codelistId={codelistId} versionId={codelist.versionId} ... />
</Box>
```

`backLink` and `breadcrumbItems` are derived by looking up `codelist.versionId` in the cached versions list. The detail page reads its parent route via `useMatchRoute()` (or the simpler approach: `window.location.pathname.startsWith('/terminology/adam')`) to fall back to the kind encoded in the URL when the versions cache is empty; the breadcrumb shows only the codelist code until the lookup resolves.

### 6.3 Tables

`CodeListTable` and `CodeItemTable` share a common shape:

| Prop                | Type                                              | Purpose                                                                 |
| ------------------- | ------------------------------------------------- | ----------------------------------------------------------------------- |
| `rows`              | `T[]`                                             | Visible rows.                                                           |
| `loading`           | `boolean`                                         | First-load spinner.                                                     |
| `mutationLoading`   | `boolean`                                         | Disables all action buttons while a mutation is in flight.             |
| `error`             | `ApiError \| null`                                | Renders error Alert + Retry.                                            |
| `onRetry`           | `() => void`                                      | Manual refetch.                                                         |
| `canMutate`         | `boolean`                                         | Hides every mutation affordance when `false`.                           |
| `onCreate` (list only) | `() => void`                                  | Header `+` button.                                                      |
| `onEdit`            | `(row: T) => void`                                | Edit button.                                                            |
| `onDelete` (list only) | `(row: T) => void`                            | Delete button.                                                          |
| `onOpen` (codelist list only) | `(row: CodeListView) => void`       | Navigate to detail page.                                                |

Columns:

- **code** — `row.code` followed by `<ExtensibleIcon visible={row.extensible} />`.
- **name** — `row.name`.
- **submission value** — `row.submissionValue`.
- **descriptions** — `<DescriptionsCell synonym={row.synonym} definition={row.definition} nciPreferredTerm={row.nciPreferredTerm} />`.
- **operation** — `<IconButton>`s per the prop matrix above; disabled when `mutationLoading`.

### 6.4 `DescriptionsCell`

```tsx
const rows: Array<[string, string]> = [
  ['SYN', synonym],
  ['DEF', definition],
  ['NCI', nciPreferredTerm],
].filter(([, v]) => v.trim() !== '');

return (
  <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
    {rows.map(([label, value]) => (
      <Box key={label} sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
        <Chip label={label} size="small" />
        <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>{value}</Typography>
      </Box>
    ))}
  </Box>
);
```

Empty / whitespace fields are filtered out entirely; the cell collapses to zero rows when every field is empty.

### 6.5 `VersionDropdown`

`<Select<TerminologyVersionView>` filtered by `kind` (e.g. only SDTM versions on the SDTM page). Disabled when the filtered list is empty; renders helper text `"No versions yet"` below the field. The page's `selectedVersionId` is initialized to the first version's id whenever the list of matching versions changes from empty to non-empty.

### 6.6 `ImportButton`

`IconButton` with `<AddIcon />`. `onClick` opens a `<Snackbar autoHideDuration={3000}>` with `message="terminology.importComingSoon"`. The button always renders (even on the SDTM/ADaAM list page) so the spec's UI is satisfied.

---

## 7. Data flow & Tauri commands

### 7.1 Rust HTTP layer

`src-tauri/src/http/terminology/version.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionViewResponse {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionListResponse {
    pub versions: Vec<TerminologyVersionViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminologyVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TerminologyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

pub async fn create(c: &HttpClient, body: CreateTerminologyVersionRequest) -> Result<TerminologyVersionViewResponse, ApiError> { /* POST /api/terminology/versions */ }
pub async fn list(c: &HttpClient) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> { /* GET /api/terminology/versions */ }
pub async fn get_by_id(c: &HttpClient, id: i64) -> Result<TerminologyVersionViewResponse, ApiError> { /* GET /api/terminology/versions/{id} */ }
pub async fn update(c: &HttpClient, id: i64, body: UpdateTerminologyVersionRequest) -> Result<TerminologyVersionViewResponse, ApiError> { /* PATCH /api/terminology/versions/{id} */ }
pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> { /* DELETE /api/terminology/versions/{id} */ }
```

The DTOs use `rename_all = "camelCase"` so the JSON wire matches the existing `UserViewResponse` convention used throughout `http/`. Wiremock tests pin the round-trip for each function.

The other two resource files follow the same shape:

- `code_list.rs` — DTOs `CodeListViewResponse`, `CodeListListResponse`, `CreateCodeListRequest`, `UpdateCodeListRequest`; 5 functions for `/api/terminology/code-lists[/id]`. `extensible: bool` field on the view.
- `code_item.rs` — DTOs `CodeItemViewResponse`, `CodeItemListResponse`, `CreateCodeItemRequest`, `UpdateCodeItemRequest`; 5 functions for `/api/terminology/code-items[/id]`. The create / update path takes `codelist_id` and `version_id` per the server's wire DTO.

### 7.2 Tauri command shims

`commands/terminology/version.rs`:

```rust
#[tauri::command]
pub async fn create_terminology_version(
    client: State<'_, HttpClient>,
    kind: TerminologyKindWire,
    name: String,
) -> Result<TermTerminologyVersionViewResponse, ApiError> {
    version::create(&client, CreateTerminologyVersionRequest { kind, name }).await
}
// ... list_terminology_versions, get_terminology_version_by_id, update_terminology_version, delete_terminology_version
```

`commands/terminology/code_list.rs` and `code_item.rs` follow the same one-`#[tauri::command]`-per-HTTP-function shape. 13 shims total.

### 7.3 lib.rs

All 13 commands registered in the existing `invoke_handler!` macro list:

```rust
commands::terminology::version::create_terminology_version,
commands::terminology::version::list_terminology_versions,
commands::terminology::version::get_terminology_version_by_id,
commands::terminology::version::update_terminology_version,
commands::terminology::version::delete_terminology_version,
commands::terminology::code_list::create_code_list,
commands::terminology::code_list::list_code_lists,
commands::terminology::code_list::update_code_list,
commands::terminology::code_list::delete_code_list,
commands::terminology::code_list::search_code_lists,
commands::terminology::code_item::create_code_item,
commands::terminology::code_item::list_code_items,
commands::terminology::code_item::update_code_item,
commands::terminology::code_item::delete_code_item,
commands::terminology::code_item::search_code_items,
```

### 7.4 Frontend types

`shared/api/types.ts`:

```ts
export type TerminologyKind = 'sdtm' | 'adam';

export interface TerminologyVersionView {
  id: number;
  kind: TerminologyKind;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface TerminologyVersionListResponse {
  versions: TerminologyVersionView[];
}

export interface CreateTerminologyVersionInput { kind: TerminologyKind; name: string }
export interface UpdateTerminologyVersionInput { kind?: TerminologyKind; name?: string }

export interface CodeListView {
  id: number;
  versionId: number;
  code: string;
  extensible: boolean;
  name: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
  createdAt: string;
  updatedAt: string;
}
export interface CodeListListResponse { codelists: CodeListView[] }
export interface CreateCodeListInput {
  versionId: number; code: string; extensible: boolean; name: string;
  submissionValue: string; synonym: string; definition: string; nciPreferredTerm: string;
}
export interface UpdateCodeListInput {
  code?: string; extensible?: boolean; name?: string; submissionValue?: string;
  synonym?: string; definition?: string; nciPreferredTerm?: string;
}

export interface CodeItemView {
  id: number; codelistId: number; versionId: number; code: string;
  submissionValue: string; synonym: string; definition: string; nciPreferredTerm: string;
  createdAt: string; updatedAt: string;
}
export interface CodeItemListResponse { items: CodeItemView[] }
export interface CreateCodeItemInput {
  codelistId: number; versionId: number; code: string; submissionValue: string;
  synonym: string; definition: string; nciPreferredTerm: string;
}
export interface UpdateCodeItemInput {
  code?: string; submissionValue?: string; synonym?: string;
  definition?: string; nciPreferredTerm?: string;
}

export interface SearchTerminologyQuery {
  versionId: number;
  fragment: string;
  limit?: number;
}
```

The single-resource GET endpoints for code lists and code items are NOT exposed in this PR — the detail page uses `list_code_lists` + `list_code_items` plus client-side filtering. See Section 11 for the rationale.

### 7.5 React Query keys

`shared/query/keys.ts`:

```ts
terminology: {
  versions: () => ['terminology', 'versions'] as const,
  version: (id: number) => ['terminology', 'version', id] as const,
  codeLists: (versionId: number) => ['terminology', 'codeLists', versionId] as const,
  codeItems: (codelistId: number) => ['terminology', 'codeItems', codelistId] as const,
  searchCodeLists: (versionId: number, fragment: string) =>
    ['terminology', 'searchCodeLists', versionId, fragment] as const,
  searchCodeItems: (versionId: number, fragment: string) =>
    ['terminology', 'searchCodeItems', versionId, fragment] as const,
}
```

### 7.6 Hooks

`features/terminology/data/list.ts`:

```ts
export function useListTerminologyVersions() {
  return useQuery<TerminologyVersionView[], ApiError>({
    queryKey: queryKeys.terminology.versions(),
    queryFn: () => api.listTerminologyVersions(),
  });
}

export function useCreateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<TerminologyVersionView, ApiError, CreateTerminologyVersionInput>({
    mutationFn: api.createTerminologyVersion,
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() }),
  });
}

export function useUpdateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<TerminologyVersionView, ApiError, { id: number; body: UpdateTerminologyVersionInput }>({
    mutationFn: ({ id, body }) => api.updateTerminologyVersion(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useDeleteTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: api.deleteTerminologyVersion,
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() }),
  });
}

export function useListCodeLists(versionId: number | null) {
  return useQuery<CodeListView[], ApiError>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0),
    queryFn: () => api.listCodeLists(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

export function useCreateCodeList() {
  const qc = useQueryClient();
  return useMutation<CodeListView, ApiError, CreateCodeListInput>({
    mutationFn: api.createCodeList,
    onSuccess: (created) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeLists(created.versionId) });
    },
  });
}

export function useUpdateCodeList() {
  const qc = useQueryClient();
  return useMutation<CodeListView, ApiError, { id: number; body: UpdateCodeListInput }>({
    mutationFn: ({ id, body }) => api.updateCodeList(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeLists(updated.versionId) });
    },
  });
}

export function useDeleteCodeList() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; versionId: number }>({
    mutationFn: ({ id }) => api.deleteCodeList(id),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeLists(vars.versionId) });
    },
  });
}

export function useListCodeItems(codelistId: number | null) {
  return useQuery<CodeItemView[], ApiError>({
    queryKey: queryKeys.terminology.codeItems(codelistId ?? 0),
    queryFn: () => api.listCodeItems(codelistId!),
    enabled: codelistId != null && codelistId > 0,
  });
}

export function useCreateCodeItem() {
  const qc = useQueryClient();
  return useMutation<CodeItemView, ApiError, CreateCodeItemInput>({
    mutationFn: api.createCodeItem,
    onSuccess: (created) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeItems(created.codelistId) });
    },
  });
}

export function useUpdateCodeItem() {
  const qc = useQueryClient();
  return useMutation<CodeItemView, ApiError, { id: number; body: UpdateCodeItemInput }>({
    mutationFn: ({ id, body }) => api.updateCodeItem(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeItems(updated.codelistId) });
    },
  });
}

export function useDeleteCodeItem() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; codelistId: number }>({
    mutationFn: ({ id }) => api.deleteCodeItem(id),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.codeItems(vars.codelistId) });
    },
  });
}
```

The hook layer does **not** gate on `role` — every hook is callable from every page. Authorization is enforced at the UI layer (button visibility) and at the server (the existing `require_admin_or_root` middleware).

### 7.7 Authorization UI

Every page reads `currentUser.data?.role` and computes `canMutate = role === 'admin' || role === 'root'`. The `canMutate` boolean threads through `CodeListTable`, `CodeItemTable`, `CodeListDrawer`, and `CodeItemDrawer`:

- Tables hide the `+` header button, the edit icon, the delete icon, and the open icon when `canMutate === false`.
- Drawers render a "Read-only" label and disable every input + the submit button when `canMutate === false`.

The server still validates every mutation through `require_admin_or_root`; the UI gating is purely cosmetic.

---

## 8. Sidebar

`features/app/components/AppLayout.tsx`:

```tsx
const terminologyEntry: MenuItem = {
  link: '#',
  title: t('nav.terminology'),
  icon: TerminologyMenuIcon,
  subMenu: [
    { link: '/terminology/sdtm', title: t('nav.terminology.sdtm'), icon: SdtmMenuIcon },
    { link: '/terminology/adam', title: t('nav.terminology.adam'), icon: AdamMenuIcon },
  ],
};

const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 2),  // Home, Projects
      terminologyEntry,          // Terminology (submenu: SDTM, ADaM)
      managementEntry,           // Management (submenu: Users)
      ...baseMenu.slice(2),     // Settings
    ]
  : baseMenu;
```

Icons:

- `TerminologyMenuIcon` — `MenuBook` (MUI).
- `SdtmMenuIcon` — `Storage`.
- `AdamMenuIcon` — `Analytics`.

The entry is gated on `canManage` (admin/root), matching the existing `Management` entry. Non-admin users see no Terminology entry, matching the current authorization policy.

---

## 9. Error handling

- **Tauri** — every HTTP function uses `c.request(...)` which already maps server non-2xx to `ApiError::Http { status, code, message }`. No new error-mapping code is needed.
- **Tables** — render `<Alert severity="error">` + `<Button onClick={onRetry}>Retry</Button>` when `error != null`, mirroring `UserTable`.
- **Drawers** — render an inline `<Alert severity="error">` above the submit button when the mutation's `error` is set. The submit button re-enables on error so the user can correct the field.
- **Empty lists** — when `rows.length === 0 && !loading && !error`, render a centered `<Typography>`:
  - `t('terminology.codelist.empty')` for the unfiltered list
  - `t('terminology.codelist.noMatches')` when a search query is active
- **Loading state** — `loading && rows.length === 0` shows `<CircularProgress />`. Mutation loading disables every action button (`disabled={mutationLoading}`) and re-enables on completion.
- **Server permission failures (403)** — the server's `require_admin_or_root` returns 403 with the project's stable token. The desktop side surfaces it through `errorMessage(...)`. No client-side guard prevents a `general` user from attempting a mutation (e.g. via the JS console); the server is the final word.
- **404 on detail page** — if a stale URL points to a deleted codelist, the detail page renders the standard `<Alert>` + Retry + a "Back to terminology" link.

---

## 10. i18n

`lib/packages/ui/src/i18n/locales/en.ts`:

```ts
'nav.terminology': 'Terminology',
'nav.terminology.sdtm': 'SDTM',
'nav.terminology.adam': 'ADaM',

'terminology.heading': 'Terminology — {kind}',
'terminology.detail.heading': 'Terminology — {kind} › {code}',
'terminology.version.placeholder': 'No versions yet',
'terminology.version.helper': 'Select a terminology version',
'terminology.extensible': 'Extensible',
'terminology.importComingSoon': 'Terminology import is coming soon',

'terminology.codelist.search.placeholder': 'Search by code, name, submission value, or description',
'terminology.codelist.field.code': 'Code',
'terminology.codelist.field.name': 'Name',
'terminology.codelist.field.submissionValue': 'Submission value',
'terminology.codelist.field.descriptions': 'Descriptions',
'terminology.codelist.field.extensible': 'Extensible',
'terminology.codelist.field.synonym': 'Synonym',
'terminology.codelist.field.definition': 'Definition',
'terminology.codelist.field.nciPreferredTerm': 'NCI preferred term',
'terminology.codelist.empty': 'No code lists in this version',
'terminology.codelist.noMatches': 'No matching code lists',
'terminology.codelist.loadFailed': 'Failed to load code lists: {message}',
'terminology.codelist.create.title': 'Create code list',
'terminology.codelist.edit.title': 'Edit code list',
'terminology.codelist.action.create': 'Create',
'terminology.codelist.action.save': 'Save',
'terminology.codelist.readOnly': 'Read-only',

'terminology.codeitem.search.placeholder': 'Search by code, name, submission value, or description',
'terminology.codeitem.field.code': 'Code',
'terminology.codeitem.field.name': 'Name',
'terminology.codeitem.field.submissionValue': 'Submission value',
'terminology.codeitem.field.descriptions': 'Descriptions',
'terminology.codeitem.field.synonym': 'Synonym',
'terminology.codeitem.field.definition': 'Definition',
'terminology.codeitem.field.nciPreferredTerm': 'NCI preferred term',
'terminology.codeitem.empty': 'No code items in this code list',
'terminology.codeitem.noMatches': 'No matching code items',
'terminology.codeitem.loadFailed': 'Failed to load code items: {message}',
'terminology.codeitem.create.title': 'Create code item',
'terminology.codeitem.edit.title': 'Edit code item',
'terminology.codeitem.action.create': 'Create',
'terminology.codeitem.action.save': 'Save',
'terminology.codeitem.readOnly': 'Read-only',

'terminology.action.delete.confirmTitle': 'Delete code list',
'terminology.action.delete.confirmMessage': 'Delete this code list and all of its items? This cannot be undone.',
'terminology.codeitem.action.delete.confirmTitle': 'Delete code item',
'terminology.codeitem.action.delete.confirmMessage': 'Delete this code item? This cannot be undone.',
'common.confirm': 'Confirm',
'common.cancel': 'Cancel',
'common.retry': 'Retry',
'common.back': 'Back',
```

`zhCN.ts` receives matching Chinese translations.

---

## 11. Out-of-scope / decisions deferred

1. **ImportTerminology page** — out of scope for this PR. The `+ import` button is a placeholder that shows "coming soon" via Snackbar. A follow-up feature will add the workbook-upload flow.
2. **Single-resource GET endpoints** — `GET /api/terminology/code-lists/{id}` and `GET /api/terminology/code-items/{id}` are not added. The detail page uses `list_code_lists` + `list_code_items` plus client-side filtering; `CodeListDetailPage` filters the list down to the one codelist matching `codelistId` in the parent. If the list grows past a few thousand rows, a future PR will add the GET endpoints.
3. **Server-side full-text search** — the existing `/code-lists/search` and `/code-items/search` endpoints are exposed on the Tauri shim layer (so a future consumer can use them) but the desktop pages use the list endpoint + client-side substring filter. Stemming + ranking can be added later by switching the table to call `useSearchCodeLists` with a debounced fragment.
4. **Pagination** — the list endpoints return all rows for a version. SDTM v2024 has ~100 code lists; the largest is ~50 items. Pagination is unnecessary in the first cut.
5. **Drawer field ordering, chip color, breadcrumb spacing** — implementation-level details that the writing-plans step will pin down.

---

## 12. Testing

### 12.1 Rust unit + wiremock tests

Per resource file under `src-tauri/src/http/terminology/`, mirroring the existing `http/user.rs` `#[cfg(test)] mod tests` block:

- One wiremock round-trip test per HTTP function asserting 2xx decode.
- One JSON-serialization test per `CreateXxxRequest` / `UpdateXxxRequest` confirming `None` fields are skipped.

Per shim file under `src-tauri/src/commands/terminology/`, a small test confirming the command forwards to the HTTP function and that the typed arguments still match.

### 12.2 Vitest + RTL

Per presentational component in `features/terminology/components/*.test.tsx`:

- `<DescriptionsCell />` — empty fields hidden, chips render in order, multi-line layout intact.
- `<CodeListTable mode="list" />` — renders rows, header `+` and edit/delete cells only when `canMutate = true`, shows empty-state when `rows = []`.
- `<CodeListTable mode="single" />` — never renders the `+` header; renders edit only.
- `<CodeItemTable />` — same admin-gating checks.
- `<VersionDropdown />` — disabled when `versions = []`; renders helper text; filters by `kind`.
- `<CodeListDrawer />` — submit disabled when `code` is whitespace; reads "Read-only" for `general` users; calls `onCreate` / `onUpdate` with the right payload.
- `<CodeItemDrawer />` — same shape.
- `<ImportButton />` — opens the Snackbar with the right message.

Per hook in `features/terminology/data/list.test.ts`, mocking `api`:

- `useListCodeLists` — fetches when `enabled`; disabled when `versionId` is `0`.
- `useCreateCodeList` — calls `api.createCodeList`; on success invalidates `queryKeys.terminology.codeLists(versionId)` for the right `versionId`.
- `useDeleteCodeList` — invalidates the right keys.
- Same three for code items.

No end-to-end / Playwright tests. The two pages are reachable by hand and the existing dev loop covers manual verification.

---

## 13. Risks

| Risk | Mitigation |
| --- | --- |
| Server returns snake_case but our `http/` DTOs are `camelCase` | Wiremock tests in each `http/terminology/*.rs` file pin a real round-trip so any drift fails the build. |
| A stale URL points to a deleted codelist | Detail page handles 404 with the standard error UI and a "Back" link. |
| `general` user attempts a mutation via the JS console | The server's `require_admin_or_root` rejects with 403. UI gating is cosmetic. |
| Empty versions list | `VersionDropdown` is disabled with helper text; the table renders its empty-state message. |
| `CodeListDetailPage` resolves the kind before the versions cache is populated | The page renders the codelist code as the only breadcrumb segment; the back button is disabled until the lookup resolves. No flicker. |