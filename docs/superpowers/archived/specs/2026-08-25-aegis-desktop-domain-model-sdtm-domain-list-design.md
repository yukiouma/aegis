# `SdtmDomainList` Page — Design

Date: 2026-08-25
Status: Approved (brainstorming complete)
Feature: `domain-model` (aegis-desktop)

## Summary

Add the first page of the `domain-model` feature: an authenticated route at
`/domain-model/sdtm` that lists SDTM domains of a selected version, supports
client-side filtering, and lets the user switch the language used for the
Description / Structure columns. The page mirrors the existing `terminology`
feature's structure end-to-end (page → components → data hooks → shared API
client → Tauri HTTP shim → Tauri command) so conventions carry over for the
rest of `domain-model` (variables, detail page, ADaM domains, etc.).

## Non-goals

- `SdtmDomainDetail` page. The "navigate to detail" icon is rendered but
  always `disabled`. No detail route is created.
- Create / update / get-by-id for SDTM domains or versions. Only the three
  endpoints needed by this page are wired (`list_versions`,
  `list_domains_by_version`, `delete_domain`).
- SDTM variables, ADaM domains.
- Server-side filtering. The filter runs client-side on the loaded list.
- zh-CN translations of the new i18n keys. The English tree is added only.
- Persistence of the selected language across sessions. URL is the source of
  truth; if the URL has no `lang`, the page derives one from the data.

## Data source

The Rust server already exposes the necessary wire DTOs in
`apps/server/aegis-server/src/transport/http/dto.rs` (SdtmDomainViewResponse,
SdtmDomainDescription{Detail}, SdtmVersionViewResponse, SdtmDomainListResponse,
SdtmVersionListResponse, DomainCategory). The handlers in
`apps/server/aegis-server/src/transport/http/domain_model/handlers.rs` already
publish these endpoints under `/api/domain-model/*`. The desktop side has
zero `domain-model` files yet — this feature adds them all.

### Wire shapes this feature consumes

| Rust struct (server)        | TS mirror (desktop)               |
|----------------------------|-----------------------------------|
| `SdtmVersionViewResponse`  | `SdtmVersionView`                 |
| `SdtmDomainViewResponse`   | `SdtmDomainView`                  |
| `SdtmDomainDescription`    | `SdtmDomainDescription`           |
| `SdtmDomainDescriptionDetail` | `SdtmDomainDescriptionDetail` |
| `DomainCategory` (enum)    | `DomainCategory` (string union)   |
| `SdtmVersionListResponse`  | `SdtmVersionListResponse`         |
| `SdtmDomainListResponse`   | `SdtmDomainListResponse`          |

`SdtmDomainListResponse` is `{ domains: SdtmDomainView[] }` — the server
returns the entire list, **not** paginated.

## URL contract

Route: `/domain-model/sdtm`
Search params (both round-tripped as number-or-string by `validateSearch`,
matching the `terminology/sdtm.tsx` pattern):

- `versionId?: number` — selected SDTM version id
- `lang?: string` — selected language code

The search text is **not** in the URL — it is local component state that
clears on version change.

## Page behavior

### State

- `versionId` — from URL `versionId`. If absent or invalid (not in
  `versionsQuery.data`), fall back to the first SDTM version and write the
  URL back via `navigate({ replace: true })`.
- `lang` — from URL `lang`. If absent or not present in the current
  `availableLanguages`, fall back to the first entry (alphabetically) once
  data loads.
- `searchFragment` — local `useState<string>`, debounced through
  `useDebouncedValue` (300ms / 1000ms).
- `confirmDelete` — local `useState<SdtmDomainView | null>`.

### Role gating

`canMutate = role === "admin" || role === "root"` read from `useCurrentUser()`.
Only `canMutate` rows render the delete icon. The disabled navigate icon
always renders.

### Derived data

- `versionsForSdtm = versions.filter(v => v.kind === "sdtm")` (mirrors the
  terminology page's filter; future code can reuse this when ADaM is added).
- `availableLanguages` — `useMemo` over `domainsQuery.data?.domains ?? []`:
  flatten `descriptions[].lang`, deduplicate via `Set`, sort
  alphabetically. Empty when the list is empty.
- `filteredRows` — `useMemo` over `domainsQuery.data?.domains ?? []` with
  `searchFragment.trim()` applied. The filter is case-insensitive substring
  match against:
  1. `row.name`
  2. every `description.details.description` across all languages
  3. every `description.details.structure` across all languages

### Empty / loading / error states

- No SDTM versions at all → centered placeholder
  (`t("domainModel.sdtm.noVersions")`).
- Version has zero domains → table empty message
  (`t("domainModel.sdtm.empty")`).
- Filter returned zero matches → table empty message
  (`t("domainModel.sdtm.noMatches")`).
- `domainsQuery.error` → inline error banner with Retry
  (mirrors `CodeListTable`).
- `deleteDomain.error` → rendered inside the confirm dialog as a
  `DialogContentText` styled `error.main`.

### Render

```
[ DomainFilterBar | VersionDropdown | LanguageDropdown ]   ← top filter row
[ DomainTable (or empty/error state) ]
[ DeleteDomainDialog (open when confirmDelete != null) ]
```

## Components

All new files live under `apps/desktop/aegis-desktop/src/features/domain-model/`.

### `pages/SdtmDomainList.tsx`

Page-level component described above. Exports `SdtmDomainList` and
re-exports from `pages/index.ts`.

### `components/DomainFilterBar.tsx`

Single MUI `TextField` mirroring `TermFilterBar`. Props:
`query: string`, `onQueryChange: (s: string) => void`. Label comes from
`t("domainModel.sdtm.filter.placeholder")`.

### `components/VersionDropdown.tsx`

Same shape as `features/terminology/components/VersionDropdown.tsx` minus
the `kind` filter (SdtmDomainList only shows SDTM versions). Filters
`versions` where `kind === "sdtm"`. Empty disables the control.

### `components/LanguageDropdown.tsx`

MUI `FormControl` + `Select<string>`. Props: `options: string[]`,
`value: string | null`, `onChange: (lang: string | null) => void`,
`disabled?: boolean`. Disables itself when `options.length === 0`. Label
comes from `t("domainModel.sdtm.lang.label")`.

### `components/DomainTable.tsx`

Presentational MUI `Table`. Props:

```ts
{
  rows: SdtmDomainView[];          // already filtered
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  onRetry: () => void;
  onDelete: (row: SdtmDomainView) => void;
  emptyMessage: string;
}
```

Columns:

| Name | Description | Structure | Category | Operations |
|---|---|---|---|---|
| `row.name` | `row.descriptions.find(d => d.lang === selectedLang)?.details.description ?? ""` | `…structure ?? ""` | `row.category` (rendered as the raw enum string: `Special Purpose`, `Interventions`, `Events`, `Findings`, `Trial Design`, `Relationships`, `Study Reference`) | Disabled `OpenInNew` icon (tooltip = `t("domainModel.sdtm.action.navigate.tooltip")`); if `canMutate`, `DeleteOutline` icon (tooltip = `t("domainModel.sdtm.action.delete.tooltip")`) calling `onDelete(row)` |

Long description / structure cells truncate via
`white-space: nowrap; overflow: hidden; text-overflow: ellipsis` with the
full text in `title=` for hover.

### `components/DeleteDomainDialog.tsx`

`Dialog` extracted from the inline pattern in `TerminologyPage`. Props:

```ts
{
  open: boolean;
  row: SdtmDomainView | null;
  onClose: () => void;
  onConfirm: (row: SdtmDomainView) => void;
  pending: boolean;
  error: unknown;
}
```

Title = `t("domainModel.sdtm.delete.confirmTitle")`. Body =
`t("domainModel.sdtm.delete.confirmMessage")` plus the error (if any) in
`error.main`. Cancel + Confirm buttons; Confirm disabled while `pending`.

## Data hooks

All new files in `apps/desktop/aegis-desktop/src/features/domain-model/data/`.

### `list.ts`

```ts
export const PAGE_SIZE = 0; // unused for now; export kept for parity
                          // with terminology's data/list.ts

export function useListSdtmVersions(): useQuery<SdtmVersionView[], ApiError>
  queryKey: queryKeys.domainModel.sdtmVersions()
  queryFn:  () => api.listSdtmVersions()

export function useListSdtmDomains(
  versionId: number | null,
): useQuery<SdtmDomainView[], ApiError>
  queryKey: queryKeys.domainModel.sdtmDomains(versionId ?? 0)
  queryFn:  () => api.listSdtmDomainsByVersion(versionId!)
  enabled:  versionId != null && versionId > 0

export function useDeleteSdtmDomain(): useMutation<void, ApiError, number>
  mutationFn: (id) => api.deleteSdtmDomain(id)
  onSuccess: invalidateQueries(["domainModel", "sdtmDomains"]) // broad — the
                                                              // 204 response
                                                              // does not echo
                                                              // the versionId.
```

`index.ts` exports the three hooks plus `PAGE_SIZE`.

## Query key factory

Append to `apps/desktop/aegis-desktop/src/shared/query/keys.ts`:

```ts
domainModel: {
  sdtmVersions: () => ["domainModel", "sdtmVersions"] as const,
  sdtmDomains:  (versionId: number) =>
    ["domainModel", "sdtmDomains", versionId] as const,
},
```

## Shared API additions

`apps/desktop/aegis-desktop/src/shared/api/types.ts` — append:

```ts
export type DomainCategory =
  | "Special Purpose"
  | "Interventions"
  | "Events"
  | "Findings"
  | "Trial Design"
  | "Relationships"
  | "Study Reference";

export interface SdtmDomainDescriptionDetail {
  description: string;
  structure: string;
}
export interface SdtmDomainDescription {
  lang: string;
  details: SdtmDomainDescriptionDetail;
}
export interface SdtmDomainView {
  id: number;
  versionId: number;
  name: string;
  category: DomainCategory;
  descriptions: SdtmDomainDescription[];
  createdAt: string;
  updatedAt: string;
}
export interface SdtmVersionView {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
}
export interface SdtmDomainListResponse {
  domains: SdtmDomainView[];
}
export interface SdtmVersionListResponse {
  versions: SdtmVersionView[];
}
```

`apps/desktop/aegis-desktop/src/shared/api/client.tsx` — append to the `api`
object:

```ts
listSdtmVersions: (): Promise<SdtmVersionView[]> =>
  call<SdtmVersionView[]>("list_sdtm_versions"),

listSdtmDomainsByVersion: async (
  versionId: number,
): Promise<SdtmDomainView[]> => {
  const resp = await call<SdtmDomainListResponse>(
    "list_sdtm_domains_by_version",
    { versionId },
  );
  return resp.domains;
},

deleteSdtmDomain: (id: number): Promise<void> =>
  call<void>("delete_sdtm_domain", { id }),
```

And re-export the new types from the barrel.

## Tauri additions

All under `apps/desktop/aegis-desktop/src-tauri/src/`.

### `http/domain_model.rs` and `http/domain_model/{version,domain}.rs`

Mirror the layout of `http/terminology.rs` + `http/terminology/{version,
code_list,code_item}.rs`:

- `http/dto.rs` — append the `DomainCategory` enum (wire form
  `#[serde(rename = "...")]` per variant — `SpecialPurpose`,
  `Interventions`, `Events`, `Findings`, `TrialDesign`, `Relationships`,
  `StudyReference`), parallel to the existing `TerminologyKind` and
  `Role`. Add unit tests asserting the wire strings round-trip.
- `http/domain_model/version.rs` — wire DTOs (`SdtmVersionViewResponse`,
  `SdtmVersionListResponse`) and async functions `list(c)`, plus
  `create` / `get_by_id` / `update` / `delete` even though the page only
  uses `list`. Rationale: parity with the existing terminology files keeps
  the module consistent and unblocks future features without a second
  refactor pass. The page only calls `list`. The unused functions have
  wiremock tests like their terminology counterparts.
- `http/domain_model/domain.rs` — wire DTOs (`SdtmDomainViewResponse`,
  `SdtmDomainDescription{Detail}`, `SdtmDomainListResponse`) and async
  functions `list_by_version(c, versionId)`, `get_by_id(c, id)`,
  `create(c, body)`, `update(c, id, body)`, `delete(c, id)`. Only
  `list_by_version` and `delete` are used by this page; the rest land for
  parity. `DomainCategory` is imported from `crate::http::dto`.
- `http/domain_model.rs` — module root with `pub mod version; pub mod domain;`.
- Update `http.rs` to add `pub mod domain_model;`.

### `commands/domain_model.rs` and `commands/domain_model/{version,domain}.rs`

Mirror `commands/terminology.rs` + `commands/terminology/{version,code_list,
code_item,import}.rs`:

- `commands/domain_model/version.rs` exposes `list_sdtm_versions`,
  `create_sdtm_version`, `get_sdtm_version_by_id`, `update_sdtm_version`,
  `delete_sdtm_version`. Only `list_sdtm_versions` is used by this page.
- `commands/domain_model/domain.rs` exposes
  `list_sdtm_domains_by_version`, `get_sdtm_domain_by_id`,
  `create_sdtm_domain`, `update_sdtm_domain`, `delete_sdtm_domain`. Only
  `list_sdtm_domains_by_version` and `delete_sdtm_domain` are used by this
  page.
- `commands/domain_model.rs` — module root with `pub mod version;
  pub mod domain;`.
- Update `commands.rs` to add `pub mod domain_model;`.

### `lib.rs`

Add to `tauri::generate_handler!`:

```rust
commands::domain_model::version::list_sdtm_versions,
// create_sdtm_version, get_sdtm_version_by_id, update_sdtm_version,
   delete_sdtm_version   (added for parity but unused by this page)
commands::domain_model::domain::list_sdtm_domains_by_version,
commands::domain_model::domain::delete_sdtm_domain,
// create_sdtm_domain, get_sdtm_domain_by_id, update_sdtm_domain
   (added for parity but unused by this page)
```

The non-used commands are still registered so the parity surface is
complete. Each has its own wiremock test.

## Routing

`apps/desktop/aegis-desktop/src/routes/_authed/_layout/domain-model/sdtm.tsx`
mirrors `terminology/sdtm.tsx`:

```ts
export const Route = createFileRoute("/_authed/_layout/domain-model/sdtm")({
  validateSearch: (raw): { versionId?: number; lang?: string } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === "" ? undefined : Number(raw.versionId)
        : typeof raw.versionId === "number" ? raw.versionId : undefined,
    lang:
      typeof raw.lang === "string" && raw.lang !== "" ? raw.lang : undefined,
  }),
  component: () => <SdtmDomainList />,
});
```

The TanStack Router file-based router will pick this up on regeneration of
`routeTree.gen.ts` (handled by the build).

## Sidebar entry (out of scope for this spec)

Adding a sidebar entry for `/domain-model/sdtm` is **not** part of this
feature; the page is reachable via direct URL only until the next sidebar
refactor PR. The AppLayout sidebar code (`features/app/components/AppLayout.tsx`)
is not touched here.

## i18n additions

`lib/packages/ui/src/i18n/locales/en.ts` — append:

```ts
"domainModel.title": "SDTM Domain Model",
"domainModel.sdtm.heading": "SDTM Domains",
"domainModel.sdtm.filter.placeholder": "Filter by name or description",
"domainModel.sdtm.version.label": "Version",
"domainModel.sdtm.version.placeholder": "No versions",
"domainModel.sdtm.lang.label": "Language",
"domainModel.sdtm.empty": "No domains in this version.",
"domainModel.sdtm.noMatches": "No domains match the current filter.",
"domainModel.sdtm.col.name": "Name",
"domainModel.sdtm.col.description": "Description",
"domainModel.sdtm.col.structure": "Structure",
"domainModel.sdtm.col.category": "Category",
"domainModel.sdtm.action.navigate.tooltip": "Open detail (coming soon)",
"domainModel.sdtm.action.delete.tooltip": "Delete domain",
"domainModel.sdtm.delete.confirmTitle": "Delete domain?",
"domainModel.sdtm.delete.confirmMessage": "This cannot be undone.",
"domainModel.sdtm.noVersions": "No SDTM versions exist yet.",
```

`zh-CN.ts` — append the same keys with English values (no Chinese
translations shipped; tree left in place for follow-up).

## Testing

### Tauri HTTP shim wiremock tests

`apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/version.rs::tests`
and `domain.rs::tests` cover, at minimum:

- `list_sdtm_versions_returns_versions`
- `list_sdtm_domains_by_version_returns_domains`
- `delete_sdtm_domain_succeeds`

Each mirrors the style of
`apps/desktop/aegis-desktop/src-tauri/src/http/terminology/version.rs::tests`.

### Feature integration test

`apps/desktop/aegis-desktop/src/features/domain-model/SdtmDomainList.test.tsx`
renders the page with `QueryClientProvider`, `MemoryRouter` (TanStack
Router test adapter), and stubs `useCurrentUser`. Cover:

- Initial render: no versions → placeholder shown.
- One version, no domains: empty message shown.
- Two domains, English descriptions present: rows render Description in
  English.
- Switch `lang` to a code with no descriptions: Description and Structure
  cells are empty (not "—" — per the clarifying Q&A).
- Type into filter: rows narrow to those whose name OR any description OR
  any structure contains the substring (case-insensitive).
- Switch version: filter resets; `lang` resets to the new version's first
  available language.
- General-role user does not see the delete icon.
- Admin-role user: click delete, confirm, row disappears from the list.

Existing router-level openapi tests in
`apps/server/aegis-server/src/transport/http/router.rs::tests` already
assert `/api/domain-model/versions` and
`/api/domain-model/versions/{version_id}/domains` are registered. No
server change → no new assertion needed.

## File-by-file change list

New files:

- `apps/desktop/aegis-desktop/src/features/domain-model/index.ts`
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/index.ts`
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.test.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/index.ts`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainFilterBar.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/VersionDropdown.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/LanguageDropdown.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DeleteDomainDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/data/index.ts`
- `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts`
- `apps/desktop/aegis-desktop/src/routes/_authed/_layout/domain-model/sdtm.tsx`
- `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/version.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/domain.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/version.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/domain.rs`

Edited files:

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (append domain-model
  types + re-exports)
- `apps/desktop/aegis-desktop/src/shared/api/client.tsx` (append domain-model
  methods + barrel re-exports)
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` (re-export new types)
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (append
  `domainModel.*`)
- `apps/desktop/aegis-desktop/src-tauri/src/http.rs` (add `pub mod
  domain_model;`)
- `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` (add `pub mod
  domain_model;`)
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (register new commands)
- `lib/packages/ui/src/i18n/locales/en.ts` (append keys)
- `lib/packages/ui/src/i18n/locales/zhCN.ts` (append keys with English
  placeholders)
- `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (regenerated by
  `vite build` / `vite dev` — do not hand-edit)

## Open decisions resolved during brainstorming

- Filter scope: client-side filter on the loaded list (no server change).
- Language dropdown options: derived from the loaded data
  (`descriptions[].lang`).
- Detail nav button: shown but `disabled` always. No detail route.
- Missing description/structure for selected lang: render empty string (no
  em-dash, no fallback).
- Category column: render raw enum value as-is; no i18n lookup.