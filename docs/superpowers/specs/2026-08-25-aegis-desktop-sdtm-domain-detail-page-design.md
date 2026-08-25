# `SdtmDomainDetail` Page — Design

Date: 2026-08-25
Status: Approved (brainstorming complete)
Feature: `domain-model` (aegis-desktop)
Predecessor: `2026-08-25-aegis-desktop-domain-model-sdtm-domain-list-design.md`

## Summary

Add the second page of the `domain-model` feature: an authenticated route at
`/domain-model/sdtm/{domain_id}` that shows one SDTM domain's metadata plus
its variables. The variable table supports drag-and-drop reordering
(`@dnd-kit/react@0.5.0`), inline create / edit / delete of variables, and a
parallel edit drawer for the domain itself. Conventions carry over from
SdtmDomainList and from `terminology/codelists/$codelistId`.

## Non-goals

- Server-side filter or pagination of variables.
- Bulk-reorder endpoint on the server. Reorder is per-variable PUT only
  (decided during brainstorming).
- A `SortableRow` wrapper component in `@aegis/ui/dnd`. Re-export only
  (decided during brainstorming).
- Editing `variable_sequence` from a form field — drag-and-drop owns it.
- zh-CN translations of the new i18n keys. English placeholders only.
- Cross-version navigation helpers (the page assumes `domainId` resolves
  to a single fixed `versionId` via the loaded domain).

## Data source

All required endpoints already exist on the server side (`handlers.rs`):

- `GET /api/domain-model/domains/{id}` → `SdtmDomainView`
- `PUT /api/domain-model/domains/{id}` → `SdtmDomainView`
- `GET /api/domain-model/domains/{id}/variables` → `SdtmVariableView[]`
- `POST /api/domain-model/variables` → `SdtmVariableView`
- `PUT /api/domain-model/variables/{id}` → `SdtmVariableView`
- `DELETE /api/domain-model/variables/{id}` → 204

The desktop side already has the domain wire types (`SdtmDomainView`,
`SdtmDomainDescription{Detail}`, `DomainCategory`). This feature adds
the variable wire types (`SdtmVariableView`, `SdtmVariableType`,
`SdtmVariableCore`, `SdtmRole`, `SdtmVariableDescription{Detail}`),
plus `CreateSdtmVariableInput` and `UpdateSdtmVariableInput` (mirror
the server `CreateSdtmVariableRequest` / `UpdateSdtmVariableRequest`).

### Three-state semantics for nullable updates

The server `UpdateSdtmVariableRequest` exposes nullable fields as
`Option<Option<T>>`. The TS mirror uses `field?: T | null`:

- absent → don't change
- `null` → clear the field
- value → replace

Mirrors `SdtmVariableUpdate` in the domain layer.

## URL contract

Route file: `routes/_authed/_layout/domain-model/sdtm/$domainId.tsx`,
which mounts at `/domain-model/sdtm/:domainId` per TanStack Router's
camelCase path-param convention (matches existing
`terminology/$kind/codelists/$codelistId`). The user's
`{domain_id}` shorthand maps to `domainId` at the URL level.

Search params (parsed by `validateSearch`, mirrors `sdtm.tsx`):

- `lang?: string` — selected language code for Description / Structure /
  Label cells.

Path param:

- `domainId: number` (parsed via `Number(...)` from `params.domainId`).
  0/NaN renders an inline error alert with a back button (mirrors
  `CodeListDetailPage`).

**Back navigation:** the headless header's back icon calls
`navigate({ to: "/domain-model/sdtm", search: { versionId, lang } })`.
`versionId` resolves from the loaded domain's `versionId`; if unavailable,
the list page's existing fallback swaps it to the first real version.

## Page behavior

### State (local to `SdtmDomainDetail`)

- `searchFragment` — `useState<string>`, debounced via
  `useDebouncedValue` (300ms / 1000ms).
- `editDomainDrawerOpen` — boolean.
- `variableDrawer` — discriminated union:
  `{ mode: "create" } | { mode: "edit"; row: SdtmVariableView } | null`.
  `null` = closed.
- `confirmDelete` — `SdtmVariableView | null`.
- **Drag state** — held by `@dnd-kit/react` only; we don't keep a parallel
  optimistic copy. On drop the page computes the new dense `1..N`
  ordering and fires one `useUpdateSdtmVariable().mutate({ id, body: { variableSequence } })`
  per variable whose sequence changed. Failed PUTs trigger an
  `invalidateQueries` to revert to truth; no manual rollback.

### Role gating

`canMutate = role === "admin" || role === "root"`. All three mutation
controls (edit domain, create/edit variable, delete variable) render
only when `canMutate`.

### Derived data

- `availableLanguages` — `useMemo` over `domainQuery.data?.descriptions`
  AND `variablesQuery.data ?? []`, dedupe via `Set`, sort
  alphabetically. Empty disables the dropdown.
- `filteredVariables` — `useMemo` over `variablesQuery.data ?? []` with
  `searchFragment.trim()` applied. Case-insensitive substring match
  against `name` OR `descriptions[selectedLang].details.label`. Missing
  description for the selected language falls through (label match
  silently absent).
- `initialSequence` (create only) —
  `useMemo(() => variables.length === 0 ? 1 : Math.max(...variables.map(v => v.variableSequence)) + 1, [variables])`.

### Empty / loading / error states

- `params.domainId` 0/NaN → inline error alert with back button.
- Domain fetch error → full-row alert with back button (no variables
  table rendered).
- Variables fetch error → inline `Alert severity="error"` inside
  `VariableTable` with Retry button.
- Empty variables list → `t("domainModel.sdtm.detail.empty")`.
- Filter zero matches → `t("domainModel.sdtm.detail.noMatches")`.
- Drawer mutation error → inline `Alert severity="error"` at bottom of
  drawer with `errorMessage(...)`.
- Delete confirm error → `DialogContentText` styled `error.main` inside
  the dialog.
- Reorder PUT failures → page-level `Alert severity="warning"` banner
  shown once until dismissed; list auto-refetches on any failure.

### Render

```
[ DomainHeaderTable (headless) — back | name | desc | struct | category | edit ]
[ DomainFilterBar | LanguageDropdown ]
[ Reorder-failure Alert (only when reorderFailed != null) ]
[ VariableTable ]
[ DomainEditDrawer ]
[ VariableEditDrawer (mount only when variableDrawer != null) ]
[ DeleteVariableDialog (open when confirmDelete != null) ]
```

`reorderFailed` is local `useState<string | null>`. Each per-variable
PUT that fires during a drop wraps its `onError` to set
`reorderFailed = errorMessage(...)` and the page also fires
`invalidateQueries(["domainModel", "sdtmVariables", domainId])` on
the same error. The Alert dismisses via an `onClose` that clears
state.

## Components

All new files live under
`apps/desktop/aegis-desktop/src/features/domain-model/`.

### `pages/SdtmDomainDetail.tsx`

Page-level component described above. Exports `SdtmDomainDetail`.

### `pages/index.ts`

Append `export * from "./SdtmDomainDetail";`.

### `components/DomainHeaderTable.tsx`

Single-row headless table (no `TableHead`, just one `TableBody` row).
Mirrors `CodeListDetailPage`'s codelist header.

Props:

```ts
{
  domain: SdtmDomainView | undefined;
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  onEdit: () => void;
  onBack: () => void;
}
```

Cell layout (6 cells, left → right):

1. Back icon — `ArrowBack`, tooltip `t("common.back")`, calls `onBack`.
   Always rendered (the page resolves `onBack` even when the domain is
   unknown — back is always possible).
2. Name — `domain.name` (subtitle1, weight 600).
3. Description —
   `descriptions.find(d => d.lang === selectedLang)?.details.description ?? ""`,
   `cellEllipsis` style with `title=` attribute.
4. Structure — same pattern.
5. Category — `domain.category` rendered as the raw enum string.
6. Edit icon — `Edit`, tooltip `t("domainModel.sdtm.detail.editTooltip")`,
   visible iff `canMutate`. Calls `onEdit`.

If `error && !domain`, the row collapses to a single full-width cell
showing the error alert + back button (mirrors
`CodeListDetailPage`).

### `components/VariableTable.tsx`

MUI `Table` with drag-and-drop reorder via `@dnd-kit/react`. Column layout:

| (headless, drag handle) | Name | Label | Role | (headless, ops) |
|---|---|---|---|---|
| `DragIndicator` icon, `cursor: grab` | `variable.name` + `<Chip size="small">{type-short}</Chip>` + `<Chip size="small">{core}</Chip>` | `descriptions.find(d => d.lang === selectedLang)?.details.label ?? ""` | `variableRole ?? "—"` (rendered via `SdtmRole.as_str()`) | IconButton(Edit) + IconButton(Delete), gated by `canMutate` |

- Variable type short form: `Numeric → "N"`, `Character → "C"` (per
  spec, no i18n key needed).
- Variable core chip uses the localized label from
  `domainModel.sdtm.variable.core.{Req|Exp|Perm|Supp}` with fallback to
  the raw enum string.

Props:

```ts
{
  rows: SdtmVariableView[];
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  emptyMessage: string;
  onRetry: () => void;
  onCreate: () => void;            // opens drawer in create mode
  onEdit: (row: SdtmVariableView) => void;
  onDelete: (row: SdtmVariableView) => void;
  onReorder: (orderedIds: number[]) => void;
}
```

The component wraps the `TableContainer` with `<DragDropProvider onDragEnd={...}>`
and renders each row as `<DraggableRow>` that composes `useDraggable` +
`useDroppable`. On drag end, the provider fires `onDragEnd(event)`; the
component computes the new id order, then calls `onReorder(newOrder)`.
The page turns `newOrder` into a list of `{ id, variableSequence }` and
fires the per-variable PUTs.

The header row's ops cell renders an `Add` icon button (visible iff
`canMutate`) with tooltip `t("domainModel.sdtm.variable.create.tooltip")`
calling `onCreate`.

### `components/DomainEditDrawer.tsx`

Drawer for editing the domain. Same shape as `CodeListDrawer` minus
the version select (locked to the row's version). Fields:

- `name` — TextField, required.
- `category` — Select from `DomainCategory` enum.
- `descriptions` — list of `{ lang, description, structure }` rows with
  add/remove buttons (mirrors the description list UX from
  `CodeListDrawer`'s definition/synonym block; we don't have a sibling
  to copy, so we build the inline `add`/`remove` row directly).

Props:

```ts
{
  open: boolean;
  row: SdtmDomainView;
  onClose: () => void;
  onUpdate: (id: number, body: UpdateSdtmDomainInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}
```

Title = `t("domainModel.sdtm.detail.editTitle")`. Submit label =
`t("common.save")`.

### `components/VariableEditDrawer.tsx`

Drawer for create **or** edit of a variable. Same dual-mode shape as
`CodeListDrawer`. `variable_sequence` is **never** rendered in the form
(the page owns sequence assignment: drag-and-drop for reorder,
`max+1` for create).

Props:

```ts
{
  open: boolean;
  mode: "create" | "edit";
  row?: SdtmVariableView;            // edit only
  domainId: number;                  // create only
  initialSequence?: number;          // create only: page passes max+1
  onClose: () => void;
  onCreate: (input: CreateSdtmVariableInput) => void;
  onUpdate: (id: number, body: UpdateSdtmVariableInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}
```

Fields (same in both modes except defaults):

- `name` — TextField, required.
- `variableControlled` — TextField, optional. Three-state semantics:
  empty input on submit sends `null` (clear).
- `variableType` — Select: Numeric / Character.
  Create default: `Character`.
- `variableCore` — Select: Req / Exp / Perm / Supp.
  Create default: `Req`.
- `variableRole` — Select: 8 roles + "—". "—" sends `null` on submit.
- `descriptions` — list of `{ lang, label }` rows with add/remove.

Title: `create → t("domainModel.sdtm.variable.create.title")`,
`edit → t("domainModel.sdtm.variable.editTitle")`. Submit label:
`create → t("common.create")`, `edit → t("common.save")`.

### `components/DeleteVariableDialog.tsx`

Same shape as `DeleteDomainDialog` but typed against `SdtmVariableView`.
Title = `t("domainModel.sdtm.variable.delete.confirmTitle")`. Body =
`t("domainModel.sdtm.variable.delete.confirmMessage")` + `error.main` for
the error (if any).

### Reused components (targeted edits only)

- `DomainFilterBar` — accept an optional `placeholder?: string` prop.
  Default unchanged (`t("domainModel.sdtm.filter.placeholder")`). The
  detail page passes `t("domainModel.sdtm.detail.filter.placeholder")`.
- `LanguageDropdown` — used unchanged.
- `DomainTable` — add `onNavigate?: (row) => void` prop. When provided,
  the OpenInNew icon is enabled and wired to it. When absent, falls back
  to today's disabled behavior (preserves existing SdtmDomainList if
  the page is later refactored to pass nothing).

## Data hooks

Append to `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts`:

```ts
export function useGetSdtmDomain(id: number | null) {
  return useQuery<SdtmDomainView, ApiError>({
    queryKey: queryKeys.domainModel.sdtmDomain(id ?? 0),
    queryFn: () => api.getSdtmDomainById(id!),
    enabled: id != null && id > 0,
  });
}

export function useListSdtmVariables(domainId: number | null) {
  return useQuery<SdtmVariableView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmVariables(domainId ?? 0),
    queryFn: () => api.listSdtmVariablesByDomain(domainId!),
    enabled: domainId != null && domainId > 0,
  });
}

export function useCreateSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<SdtmVariableView, ApiError, CreateSdtmVariableInput>({
    mutationFn: api.createSdtmVariable,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmVariables", created.domainId],
      });
    },
  });
}

export function useUpdateSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<SdtmVariableView, ApiError, { id: number; body: UpdateSdtmVariableInput }>({
    mutationFn: ({ id, body }) => api.updateSdtmVariable(id, body),
    onSuccess: () => {
      // Page-level invalidation keeps us honest about the unknown
      // domainId; the page knows which list to refetch.
    },
  });
}

export function useDeleteSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: (id) => api.deleteSdtmVariable(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["domainModel", "sdtmVariables"] });
    },
  });
}
```

`data/index.ts` re-exports the existing list hooks plus all new hooks.

## Query key factory

Append to `apps/desktop/aegis-desktop/src/shared/query/keys.ts`:

```ts
domainModel: {
  sdtmVersions: () => ["domainModel", "sdtmVersions"] as const,
  sdtmDomains: (versionId: number) =>
    ["domainModel", "sdtmDomains", versionId] as const,
  sdtmDomain: (id: number) =>
    ["domainModel", "sdtmDomain", id] as const,
  sdtmVariables: (domainId: number) =>
    ["domainModel", "sdtmVariables", domainId] as const,
},
```

## Shared API additions

`apps/desktop/aegis-desktop/src/shared/api/types.ts` — append:

```ts
export type SdtmVariableType = "Numeric" | "Character";
export type SdtmVariableCore = "Req" | "Exp" | "Perm" | "Supp";
export type SdtmRole =
  | "Identifier"
  | "Topic"
  | "Timing"
  | "Record Qualifier"
  | "Synonym Qualifier"
  | "Variable Qualifier"
  | "Grouping Qualifier"
  | "Rule";

export interface SdtmVariableDescriptionDetail {
  label: string;
}
export interface SdtmVariableDescription {
  lang: string;
  details: SdtmVariableDescriptionDetail;
}
export interface SdtmVariableView {
  id: number;
  domainId: number;
  name: string;
  variableControlled?: string;
  variableType: SdtmVariableType;
  variableCore: SdtmVariableCore;
  variableRole?: SdtmRole;
  variableSequence: number;
  descriptions: SdtmVariableDescription[];
  createdAt: string;
  updatedAt: string;
}
export interface SdtmVariableListResponse {
  variables: SdtmVariableView[];
}
export interface CreateSdtmVariableInput {
  domainId: number;
  name: string;
  variableControlled?: string;
  variableType: SdtmVariableType;
  variableCore: SdtmVariableCore;
  variableRole?: SdtmRole;
  variableSequence: number;
  descriptions: SdtmVariableDescription[];
}
// Three-state semantics for nullable updates: absent = no change,
// null = clear, value = replace.
export interface UpdateSdtmVariableInput {
  name?: string;
  variableControlled?: string | null;
  variableType?: SdtmVariableType;
  variableCore?: SdtmVariableCore;
  variableRole?: SdtmRole | null;
  variableSequence?: number;
  descriptions?: SdtmVariableDescription[];
}
export interface UpdateSdtmDomainInput {
  name?: string;
  category?: DomainCategory;
  descriptions?: SdtmDomainDescription[];
}
```

`apps/desktop/aegis-desktop/src/shared/api/index.ts` — append to `api`:

```ts
getSdtmDomainById: (id: number): Promise<SdtmDomainView> =>
  call<SdtmDomainView>("get_sdtm_domain_by_id", { id }),

updateSdtmDomain: (id: number, body: UpdateSdtmDomainInput): Promise<SdtmDomainView> =>
  call<SdtmDomainView>("update_sdtm_domain", { id, body: { ...body } }),

listSdtmVariablesByDomain: async (domainId: number): Promise<SdtmVariableView[]> => {
  const resp = await call<SdtmVariableListResponse>(
    "list_sdtm_variables_by_domain",
    { domainId },
  );
  return resp.variables;
},

createSdtmVariable: (input: CreateSdtmVariableInput): Promise<SdtmVariableView> =>
  call<SdtmVariableView>("create_sdtm_variable", { ...input }),

updateSdtmVariable: (id: number, body: UpdateSdtmVariableInput): Promise<SdtmVariableView> =>
  call<SdtmVariableView>("update_sdtm_variable", { id, body: { ...body } }),

deleteSdtmVariable: (id: number): Promise<void> =>
  call<void>("delete_sdtm_variable", { id }),
```

And add the new types to the barrel `export type { ... }` block.

## Tauri additions

All under `apps/desktop/aegis-desktop/src-tauri/src/`.

### `http/domain_model/variable.rs` (new)

Mirror `http/domain_model/version.rs`:

- Wire DTOs (camelCase) — `SdtmVariableType` / `SdtmVariableCore` (both
  `#[serde(rename_all = "PascalCase")]`) and `SdtmRole` with explicit
  `#[serde(rename = "...")]` per the server's wire form.
- `SdtmVariableViewResponse`, `SdtmVariableListResponse`,
  `SdtmVariableDescription{,Detail}` (camelCase).
- `CreateSdtmVariableRequest` (all required), `UpdateSdtmVariableRequest`
  with `Option<Option<T>>` for `variable_controlled` / `variable_role`
  and `#[serde(skip_serializing_if = "Option::is_none")]` on each.
- Functions: `create(c, body)`, `list_by_domain(c, domainId)`,
  `get_by_id(c, id)`, `update(c, id, body)`, `delete(c, id)`. Page
  uses `create`, `list_by_domain`, `update`, `delete`; `get_by_id`
  lands for parity.

### Wiremock tests in `variable.rs::tests`

- `list_by_domain_returns_variables`
- `create_returns_view`
- `update_returns_view`
- `delete_succeeds`
- `update_request_skips_none_fields` (parallel to
  `domain.rs::update_request_skips_none_fields`)

### `http/domain_model.rs` (existing)

Append `pub mod variable;`. `http.rs` already exposes `domain_model`;
no change.

### `commands/domain_model/variable.rs` (new)

One `#[tauri::command]` per HTTP function:

```rust
create_sdtm_variable(client, input) -> Result<SdtmVariableViewResponse, ApiError>
list_sdtm_variables_by_domain(client, domain_id) -> Result<SdtmVariableListResponse, ApiError>
get_sdtm_variable_by_id(client, id) -> Result<SdtmVariableViewResponse, ApiError>
update_sdtm_variable(client, id, body) -> Result<SdtmVariableViewResponse, ApiError>
delete_sdtm_variable(client, id) -> Result<(), ApiError>
```

### `commands/domain_model.rs` (existing)

Append `pub mod variable;`. `commands.rs` already exposes `domain_model`;
no change.

### `lib.rs`

Append to `tauri::generate_handler!`:

```rust
commands::domain_model::variable::create_sdtm_variable,
commands::domain_model::variable::list_sdtm_variables_by_domain,
commands::domain_model::variable::get_sdtm_variable_by_id,
commands::domain_model::variable::update_sdtm_variable,
commands::domain_model::variable::delete_sdtm_variable,
```

## `@aegis/ui/dnd` — `@dnd-kit/react` adapter

### Install

Add `"@dnd-kit/react": "0.5.0"` to `lib/packages/ui/dependencies`
(it's imported by code, not just used as a peer). React 19 satisfies
the peer constraint.

### New subpath `lib/packages/ui/src/dnd/index.ts`

```ts
export * from '@dnd-kit/react';
```

### `lib/packages/ui/package.json` — add `./dnd` to `exports`

```json
"./dnd": "./src/dnd/index.ts"
```

### Usage in `aegis-desktop`

`VariableTable.tsx` imports:

```ts
import {
  DragDropProvider,
  useDraggable,
  useDroppable,
} from "@aegis/ui/dnd";
```

No custom wrapper component lives in `@aegis/ui` (per brainstorming
decision). The desktop composes the primitives directly inside
`DraggableRow`.

## Routing

### New route file
`apps/desktop/aegis-desktop/src/routes/_authed/_layout/domain-model/sdtm/$domainId.tsx`:

```ts
export const Route = createFileRoute(
  "/_authed/_layout/domain-model/sdtm/$domainId",
)({
  validateSearch: (raw): { lang?: string } => ({
    lang: typeof raw.lang === "string" && raw.lang !== "" ? raw.lang : undefined,
  }),
  parseParams: (raw) => ({
    domainId: Number((raw as { domainId: string }).domainId),
  }),
  component: () => <SdtmDomainDetail />,
});
```

`routeTree.gen.ts` regenerates on build; do not hand-edit.

### `SdtmDomainList` → detail navigation

The previously-disabled navigate icon is wired:

```ts
onNavigate={(row) =>
  navigate({
    to: "/domain-model/sdtm/$domainId",
    params: { domainId: String(row.id) },
    search: { lang: selectedLang ?? undefined },
  })
}
```

`DomainTable` accepts the new optional `onNavigate` prop. When provided,
the OpenInNew icon becomes enabled and clickable; when absent, the icon
falls back to disabled (preserves today's SdtmDomainList behavior).

## i18n additions

Append to `lib/packages/ui/src/i18n/locales/en.ts` (and `zhCN.ts` with
English placeholders, per the existing pattern):

```ts
"domainModel.sdtm.detail.backTooltip": "Back to domains",
"domainModel.sdtm.detail.editTooltip": "Edit domain",
"domainModel.sdtm.detail.editTitle": "Edit domain",
"domainModel.sdtm.detail.filter.placeholder": "Filter by name or label",
"domainModel.sdtm.detail.col.name": "Name",
"domainModel.sdtm.detail.col.label": "Label",
"domainModel.sdtm.detail.col.role": "Role",
"domainModel.sdtm.detail.empty": "No variables in this domain.",
"domainModel.sdtm.detail.noMatches": "No variables match the current filter.",
"domainModel.sdtm.detail.loadFailed": "Failed to load domain: {message}",
"domainModel.sdtm.detail.variablesLoadFailed": "Failed to load variables: {message}",
"domainModel.sdtm.detail.reorderFailed": "Reorder failed: {message}",
"domainModel.sdtm.variable.create.title": "Create variable",
"domainModel.sdtm.variable.create.tooltip": "Create variable",
"domainModel.sdtm.variable.editTitle": "Edit variable",
"domainModel.sdtm.variable.field.name": "Name",
"domainModel.sdtm.variable.field.variableControlled": "Controlled vocabulary (CCDD)",
"domainModel.sdtm.variable.field.variableType": "Type",
"domainModel.sdtm.variable.field.variableCore": "Core",
"domainModel.sdtm.variable.field.variableRole": "Role",
"domainModel.sdtm.variable.field.descriptions": "Labels",
"domainModel.sdtm.variable.field.descriptions.lang": "Language",
"domainModel.sdtm.variable.field.descriptions.label": "Label",
"domainModel.sdtm.variable.type.Numeric": "Numeric",
"domainModel.sdtm.variable.type.Character": "Character",
"domainModel.sdtm.variable.core.Req": "Required",
"domainModel.sdtm.variable.core.Exp": "Expected",
"domainModel.sdtm.variable.core.Perm": "Permissible",
"domainModel.sdtm.variable.core.Supp": "Supplemental",
"domainModel.sdtm.variable.delete.confirmTitle": "Delete variable?",
"domainModel.sdtm.variable.delete.confirmMessage": "This cannot be undone.",
"common.create": "Create",
```

## Testing

Tests live under `apps/desktop/aegis-desktop/src/test/features/domain-model/`,
flat (no `pages/` subdirectory), mirroring the existing
`test/features/domain-model/` layout (e.g. `domain-table.test.tsx`,
`sdtm-domain-list.test.tsx`, `version-dropdown.test.tsx`). The test
file naming convention is `{component-name}.test.tsx`. Data-hook tests
live in a `data/` subfolder (mirroring
`test/features/terminology/data/list.test.tsx`).

### Component tests (one file per component, `src/test/features/domain-model/`)

- `domain-header-table.test.tsx` — renders the headless row, hides the
  edit icon for general-role users, falls back to the error alert with
  back button when the domain fetch errors.
- `variable-table.test.tsx` — renders the five columns, applies the
  type/core chips correctly, swaps label when `selectedLang` changes,
  hides the add/edit/delete icons for general-role users, and exercises
  the drag-and-drop hook: simulate `onReorder([2,1,3,4])` and assert
  the page receives exactly that ordering.
- `domain-edit-drawer.test.tsx` — submit fires `onUpdate` with the
  expected body; mutation error renders the inline `Alert`.
- `variable-edit-drawer.test.tsx` — both modes: create uses the
  provided `initialSequence` and `domainId`, edit uses the row's id
  and never sends `variableSequence`. Three-state semantics for
  `variableControlled` / `variableRole`.
- `delete-variable-dialog.test.tsx` — confirm fires `onConfirm(row)`;
  pending disables buttons; error renders in `error.main`.

### Page integration test (`src/test/features/domain-model/sdtm-domain-detail.test.tsx`)

Renders the page with `QueryClientProvider`, TanStack Router test
adapter, and a mocked `useCurrentUser`. Cover:

- `params.domainId = 0` → inline error alert + back button.
- Domain not found (404) → error alert + back button.
- Domain + variables load → header row + variable rows render.
- Switch `lang` → variable Label cells update.
- Type into filter → variables narrow by name OR selected-lang label
  substring (case-insensitive).
- General-role user does not see edit/delete/add icons.
- Admin-role user: open edit drawer, save → query invalidated, drawer
  closes.
- Admin-role user: click the add icon in the table header → drawer
  opens in create mode. Submit → `useCreateSdtmVariable` mock called
  with `{ domainId, variableSequence: max+1, ... }`. Drawer does not
  render a `variableSequence` field.
- Admin-role user: open delete dialog, confirm → row disappears.
- Drag-and-drop: `onReorder([2,1,3,4])` from the test hook → only the
  affected variables get PUTs; `useUpdateSdtmVariable` mock receives
  `{id:1, body:{variableSequence:2}}` and
  `{id:2, body:{variableSequence:1}}` (NOT ids 3 and 4, since their
  positions didn't change).

### Data hook tests (`src/test/features/domain-model/data/list.test.tsx`)

Cover the new hooks in isolation:
- `useGetSdtmDomain` is disabled for `id = 0`/`null`.
- `useListSdtmVariables` is disabled for `domainId = 0`/`null`.
- `useCreateSdtmVariable` on success invalidates
  `["domainModel", "sdtmVariables", created.domainId]`.
- `useDeleteSdtmVariable` on success invalidates
  `["domainModel", "sdtmVariables"]` (broad — no domainId in the
  mutation result).

### Tauri HTTP shim tests
`apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/variable.rs::tests`:

- `list_by_domain_returns_variables`
- `create_returns_view`
- `update_returns_view`
- `delete_succeeds`
- `update_request_skips_none_fields`

## File-by-file change list

New files:

- `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainHeaderTable.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableTable.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DeleteVariableDialog.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-detail.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-header-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-edit-drawer.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/delete-variable-dialog.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/domain-model/data/list.test.tsx`
- `apps/desktop/aegis-desktop/src/routes/_authed/_layout/domain-model/sdtm/$domainId.tsx`
- `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/variable.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/variable.rs`
- `lib/packages/ui/src/dnd/index.ts`

Edited files:

- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx`
  — add `onNavigate?: (row) => void` prop
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx`
  — pass `onNavigate` to `DomainTable`
- `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainFilterBar.tsx`
  — accept optional `placeholder` override
- `apps/desktop/aegis-desktop/src/features/domain-model/components/index.ts`
  — re-export new components
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/index.ts`
  — re-export `SdtmDomainDetail`
- `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts`
  — append new hooks
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — append
  `sdtmDomain` and `sdtmVariables`
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — append variable
  types + update input types
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — append 6 new api
  methods + type re-exports
- `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model.rs` — add
  `pub mod variable;`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model.rs` —
  add `pub mod variable;`
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — register new
  commands
- `lib/packages/ui/package.json` — add `@dnd-kit/react@0.5.0` to
  `dependencies`; add `./dnd` to `exports`
- `lib/packages/ui/src/i18n/locales/en.ts` — append keys
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — append keys with English
  placeholders
- `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` — regenerated
  by `vite build` / `vite dev` — do not hand-edit

## Open decisions resolved during brainstorming

- Drag-and-drop persistence: per-variable PUT, dense 1..N renumber on
  every drop (no batch endpoint, no fractional indirection).
- `@aegis/ui/dnd` is a thin re-export of `@dnd-kit/react` only — no
  custom wrapper component.
- Variable edit drawer fields: name + variable_controlled + variable_type
  + variable_core + variable_role + descriptions. `variable_sequence` is
  intentionally excluded (drag-and-drop owns it).
- Create-variable: add icon button in the table header's ops cell; the
  drawer supports both `mode: "create"` and `mode: "edit"`.
- New variable sequence = `max(existing.sequence) + 1` (or `1` when
  empty). Defaults for create: `variableType = Character`,
  `variableCore = Req`, `variableRole = null` (clear).
- Reorder PUT failures: page-level `Alert severity="warning"` banner
  above the variable table + automatic
  `invalidateQueries(["domainModel", "sdtmVariables", domainId])` to
  revert to truth.
- `lang` is preserved across navigation (back to list, forward to
  detail).
- `versionId` is preserved across navigation back to the list page
  (resolved from the loaded domain).
- URL uses TanStack Router's camelCase path-param convention
  (`$domainId`); the user's `{domain_id}` shorthand maps to `domainId`
  at the wire level.