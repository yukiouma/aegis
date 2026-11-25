# aegis-desktop CRF feature — design

Date: 2026-08-28
Scope: add a `crf` feature to the Tauri desktop app under the project workspace window.

## 1. Goals

Add three pages under the project workspace, anchored on the existing
CRF server endpoints (`apps/server/aegis-server/src/transport/http/crf/`):

- **CRF Form List** — pick a CRF version, list forms, create / edit /
  delete a form, filter rows. Per-row assign-takers opens an empty
  drawer for now.
- **CRF Detail** — header (back / code chip / name / global-search
  button) + placeholder body.
- **CRF Global Search** — search input + empty result table.

All routes live under `/project/$projectCode/crf`.

The "related API is not ready" notes in the brief apply only to:

- The **status** column (rendered as a literal "Pending" chip with a
  `PendingActions` icon — no fetch).
- The **taker** column (cell left empty — no fetch).
- The **taker assignment** drawer (title + empty body + close).
- The **involved** filter checkbox (`disabled`).
- The **global search** result table (no fetch — page renders empty).

Everything else uses real data: form list, version dropdown, create
drawer, edit drawer, delete dialog, detail page form fetch.

## 2. Out of scope

- Server-side CRF endpoints (already implemented).
- Item / Option / Unit / Annotation pages (server exists, no UI yet).
- `get_form_details` (composed fetch) — detail page uses the simpler
  `get_form_by_id`.
- `bulk_create_form`, `search_forms_by_version`, version create / update
  / delete — page only consumes versions, never creates them.
- Pagination on the form list (server returns plain `Vec`; switch to
  infinite scroll later if needed).
- Client-side RBAC gating (every authenticated user can mutate this
  PR; server remains the source of truth).

## 3. Routes

File-based routes under `_authed/project/$projectCode/`:

```
_authed/project/$projectCode/
├── route.tsx                  (unchanged)
├── index.tsx                  (unchanged)
├── dashboard.tsx              (unchanged)
├── configuration.tsx          (unchanged)
└── crf/
    ├── index.tsx              → CrfFormListPage     (URL /project/$projectCode/crf)
    ├── $formId.tsx            → CrfDetailPage       (URL /project/$projectCode/crf/$formId)
    └── search.tsx             → CrfGlobalSearchPage (URL /project/$projectCode/crf/search)
```

The sidebar entry in `ProjectWorkspaceLayout.tsx` adds:

```ts
{
  link: `/project/${projectCode}/crf`,
  title: t("workspace.menu.crf"),
  icon: CrfMenuIcon,
}
```

Where `CrfMenuIcon = () => <AssignmentIcon />`.

## 4. Frontend feature module

Location: `apps/desktop/aegis-desktop/src/features/crf/`

```
features/crf/
├── index.ts                          (barrel: pages + data)
├── data/
│   ├── index.ts                      (`export * from "./list";`)
│   └── list.ts                       (React Query hooks)
├── components/
│   ├── index.ts
│   ├── CrfFormDrawer.tsx             (mode: "create" | "edit")
│   ├── DeleteCrfFormDialog.tsx       (confirmation)
│   ├── CrfFormFilterDrawer.tsx       (search + status + involved)
│   ├── CrfFormTable.tsx              (header actions + row actions)
│   ├── CrfVersionDropdown.tsx        (parent-scope selector)
│   ├── CrfAssignTakersDrawer.tsx     (title + empty body + close)
│   ├── CrfStatusChip.tsx             (literal "Pending" chip)
│   └── CrfGlobalSearchButton.tsx     (Button with Search icon + label)
└── pages/
    ├── index.ts
    ├── CrfFormListPage.tsx
    ├── CrfDetailPage.tsx
    └── CrfGlobalSearchPage.tsx
```

### 4.1 Data hooks (`data/list.ts`)

```ts
export function useListCrfVersions(projectCode: string | null);
export function useListCrfForms(versionId: number | null);
export function useGetCrfForm(id: number | null);
export function useCreateCrfForm();
export function useUpdateCrfForm();
export function useDeleteCrfForm();
```

- `useListCrfVersions` and `useListCrfForms` are `useQuery` (not
  infinite); the server returns a flat `Vec`.
- `useListCrfForms` is `enabled: versionId != null && versionId > 0`.
- `useGetCrfForm` uses `enabled: Number.isFinite(id) && id > 0`.
- `useCreateCrfForm` invalidates
  `["crf", "formsByVersion", created.versionId]` on success.
- `useUpdateCrfForm` invalidates the same list key plus
  `queryKeys.crf.form(updated.id)`.
- `useDeleteCrfForm` invalidates the list key on success.
- The search/filter is applied **client-side** in the page
  (`useDebouncedValue` + `useMemo`); no server-side search this PR.

### 4.2 Page anatomy

#### `CrfFormListPage.tsx`

URL is the source of truth for `?versionId=`:
- `useSearch({ strict: false }) as { versionId?: number }`.
- `useListCrfVersions(projectCode)` drives the dropdown.
- If `versionId` missing or invalid, `useEffect` navigates with
  `replace: true` to the first version's id.

Page-owned filter state (passed into `CrfFormFilterDrawer`):
- `searchInput: string` (raw, controlled by drawer text field).
- `statusSelected: ("approved" | "pending")[]` (multi-select).
- `involvedChecked: boolean` (held but ignored — checkbox is disabled).
- `debouncedSearch = useDebouncedValue(searchInput, { delayMs: 300,
  maxWaitMs: 1000 })`.
- `filteredRows = useMemo(() => rows.filter(r =>
  (debouncedSearch === "" || r.code.toLowerCase().includes(debouncedSearch.toLowerCase())
    || r.name.toLowerCase().includes(debouncedSearch.toLowerCase()))
  && (statusSelected.length === 0
    || (statusSelected.includes("approved") /* placeholder: not implemented */)
    || (statusSelected.includes("pending") /* placeholder */))
  && (!involvedChecked /* placeholder */)
), [rows, debouncedSearch, statusSelected, involvedChecked])`.
- Status + involved filtering is intentionally a no-op this PR; the
  placeholders document the future wiring so the spec is honest about
  what is and isn't enforced today.

Layout (top-to-bottom):

1. `<Typography variant="h4">` — i18n key `crf.formList.heading`.
2. **Toolbar row:** `CrfVersionDropdown` · `CrfStatusChip` ·
   `CrfGlobalSearchButton`.
3. `CrfFormTable`:
   - Header "Operations" cell: `IconButton<AddIcon/>` (open create
     drawer) + `IconButton<FilterListIcon/>` (open filter drawer).
   - Per-row "Operations" cell:
     `IconButton<AssignmentIndIcon/>` (assign-takers) ·
     `IconButton<EditIcon/>` (edit) ·
     `IconButton<DeleteIcon/>` (delete) ·
     `IconButton<LaunchIcon/>` (open detail).
4. `CrfFormDrawer` (right-anchored, 480px) — controlled by
   `drawerState: { mode: "create" } | { mode: "edit"; row } | null`.
5. `CrfFormFilterDrawer` (right-anchored, 480px).
6. `DeleteCrfFormDialog` — controlled by
   `confirmDelete: CrfForm | null`.
7. `CrfAssignTakersDrawer` — controlled by
   `assignTakersFor: CrfForm | null`.

Mutations plumb `error` + `isPending` to their drawer/dialog.

#### `CrfDetailPage.tsx`

`formId = Number(useParams({ strict: false }).formId)`. Renders only if
`Number.isFinite(formId) && formId > 0`. Fetches
`useGetCrfForm(formId)`.

Header row: `IconButton<ArrowBackIcon/>` · `Chip label={form.code}` ·
`Typography variant="h5">{form.name}</Typography>` ·
`CrfGlobalSearchButton`.

Body: `<Alert severity="info">{t("crf.detail.placeholder")}</Alert>`.

On load failure (404 / network): `<Alert severity="error">…</Alert>`
with the back button still functional.

#### `CrfGlobalSearchPage.tsx`

Header row: `IconButton<ArrowBackIcon/>` ·
`Typography variant="h4">CRF Global Search — {projectCode}</Typography>`
· `CrfGlobalSearchButton` (always visible for navigation parity).

Body: `TextField` (controlled, `onChange` no-op) + `Table` with columns
Form / Item / Option / Annotation and a single placeholder row
"No results".

### 4.3 Component details

- `CrfFormDrawer`: `mode: "create" | "edit"`, fields `code`
  (`required`, `maxLength=64`) + `name` (`required`, `multiline
  minRows=2`). Per-field `useState` with `EMPTY_FIELDS = { code: "",
  name: "" }`. `useEffect` on `[open, mode, row]` seeds/clears state.
  Mutation error rendered as `<Alert severity="error">`. Footer:
  Cancel + Save.
- `DeleteCrfFormDialog`: `Dialog` with body
  `Delete form "{code} — {name}"?`. Footer: Cancel + Delete (red
  `contained`). Disabled while `mutationPending`.
- `CrfFormFilterDrawer`: receives `searchInput`, `onSearchInputChange`,
  `statusSelected`, `onStatusSelectedChange`, `onClear`, `onApply`
  from the page. Body has `TextField` (search code+name), `Select`
  with `multiple` for status (options `Approved` / `Pending` rendered
  as static strings), `Checkbox` for "Involved" (`disabled`). Footer:
  Clear + Apply. The page owns the raw string; the drawer is purely
  controlled. The page debounces `searchInput` via `useDebouncedValue`
  before applying the in-memory filter (status + involved are tracked
  but unused this PR).
- `CrfAssignTakersDrawer`: title + `<Typography>Coming soon</Typography>`
  + Close button. No body content.
- `CrfStatusChip`: yellow `<Chip icon={<PendingActionsIcon/>} label="Pending" color="warning" variant="outlined" />`.
- `CrfGlobalSearchButton`:
  ```tsx
  <Button
    startIcon={<SearchIcon />}
    onClick={() => navigate({ to: "/project/$projectCode/crf/search", params: { projectCode } })}
  >
    {t("crf.toolbar.globalSearch")}
  </Button>
  ```

## 5. Shared types + query keys + api namespace

### `shared/api/types.ts`

Append a new section:

```ts
export interface CrfVersion {
  id: number;
  projectCode: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}
export interface CrfVersionListResponse { versions: CrfVersion[]; }

export interface CrfForm {
  id: number;
  versionId: number;
  code: string;
  name: string;
  order: number;
  notSubmitted: boolean;
  createdAt: string;
  updatedAt: string;
}
export interface CrfFormListResponse { forms: CrfForm[]; }
export interface CreateCrfFormInput {
  code: string;
  name: string;
  order: number;
  notSubmitted: boolean;
}
export interface UpdateCrfFormInput {
  code?: string;
  name?: string;
  order?: number;
  notSubmitted?: boolean;
}
```

### `shared/query/keys.ts` — new branch

```ts
crf: {
  versionsByProject: (projectCode: string) =>
    ["crf", "versionsByProject", projectCode] as const,
  formsByVersion: (versionId: number) =>
    ["crf", "formsByVersion", versionId] as const,
  form: (id: number) =>
    ["crf", "form", id] as const,
},
```

### `shared/api/index.ts` — new namespace

```ts
crf: {
  listVersions:       (projectCode: string) =>
    call<CrfVersion[]>("list_crf_versions", { projectCode }),
  listFormsByVersion: (versionId: number) =>
    call<CrfForm[]>("list_crf_forms_by_version", { versionId }),
  getFormById:        (id: number) =>
    call<CrfForm>("get_crf_form_by_id", { id }),
  createForm:         (versionId: number, body: CreateCrfFormInput) =>
    call<CrfForm>("create_crf_form", { versionId, body }),
  updateForm:         (id: number, body: UpdateCrfFormInput) =>
    call<CrfForm>("update_crf_form", { id, body }),
  deleteForm:         (id: number) =>
    call<void>("delete_crf_form", { id }),
},
```

## 6. Backend Tauri shims

Modules follow the project's sibling-file pattern (no `mod.rs`):

```
src-tauri/src/
├── http.rs                          (add `pub mod crf;`)
├── http/
│   ├── crf.rs                       (`pub mod form; pub mod version;`)
│   ├── crf/version.rs               (DTOs + list_by_project)
│   └── crf/form.rs                  (DTOs + list_by_version + create + update + delete + get_by_id)
├── commands.rs                      (add `pub mod crf;`)
├── commands/
│   ├── crf.rs                       (`pub mod form; pub mod version;`)
│   ├── crf/version.rs               (1 command shim)
│   └── crf/form.rs                  (5 command shims)
└── lib.rs                           (extend generate_handler! with // crf block)
```

### `http/crf/version.rs`

DTOs (all `#[serde(rename_all = "camelCase")]`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrfVersionListResponse { pub versions: Vec<CrfVersionViewResponse> }
```

Function:

```rust
pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
) -> Result<Vec<CrfVersionViewResponse>, ApiError>
```

Hits `GET /api/crf/projects/{project_code}/versions`.

### `http/crf/form.rs`

DTOs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrfFormListResponse { pub forms: Vec<CrfFormViewResponse> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfFormRequest {
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfFormRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}
```

Functions:

```rust
pub async fn list_by_version(c: &HttpClient, version_id: i64)
    -> Result<Vec<CrfFormViewResponse>, ApiError>;          // GET /api/crf/versions/{id}/forms

pub async fn create(c: &HttpClient, version_id: i64, body: CreateCrfFormRequest)
    -> Result<CrfFormViewResponse, ApiError>;                // POST /api/crf/versions/{id}/forms

pub async fn update(c: &HttpClient, id: i64, body: UpdateCrfFormRequest)
    -> Result<CrfFormViewResponse, ApiError>;                // PATCH /api/crf/forms/{id}

pub async fn delete(c: &HttpClient, id: i64)
    -> Result<(), ApiError>;                                 // DELETE /api/crf/forms/{id}

pub async fn get_by_id(c: &HttpClient, id: i64)
    -> Result<CrfFormViewResponse, ApiError>;                // GET /api/crf/forms/{id}
```

Each http submodule carries an inline `#[cfg(test)] mod tests` block
using `wiremock::MockServer` + `MemoryStore`, mirroring
`http/terminology/version.rs`.

### Commands

`commands/crf/version.rs`:

```rust
#[tauri::command]
pub async fn list_crf_versions(
    client: State<'_, HttpClient>,
    project_code: String,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    crate::http::crf::version::list_by_project(&client, &project_code).await
}
```

`commands/crf/form.rs` mirrors the http layer with five shims:
`list_crf_forms_by_version`, `create_crf_form`, `update_crf_form`,
`delete_crf_form`, `get_crf_form_by_id`.

### `lib.rs` registration

Append inside `tauri::generate_handler![...]`:

```rust
// crf
commands::crf::version::list_crf_versions,
commands::crf::form::list_crf_forms_by_version,
commands::crf::form::create_crf_form,
commands::crf::form::update_crf_form,
commands::crf::form::delete_crf_form,
commands::crf::form::get_crf_form_by_id,
```

## 7. i18n

Append to both `lib/packages/ui/src/i18n/locales/en.ts` and
`locales/zhCN.ts`:

```ts
"workspace.menu.crf":                       "CRF",
"crf.formList.heading":                     "CRF Form List — {projectCode}",
"crf.detail.title":                         "CRF Detail",
"crf.detail.placeholder":                   "Form detail view coming soon",
"crf.detail.back":                          "Back to form list",
"crf.globalSearch.heading":                 "CRF Global Search — {projectCode}",
"crf.globalSearch.searchPlaceholder":       "Search forms, items, options, annotations…",
"crf.globalSearch.empty":                   "No results",
"crf.globalSearch.col.form":                "Form",
"crf.globalSearch.col.item":                "Item",
"crf.globalSearch.col.option":              "Option",
"crf.globalSearch.col.annotation":          "Annotation",
"crf.toolbar.statusPending":                "Pending",
"crf.toolbar.globalSearch":                 "Global Search",
"crf.toolbar.globalSearchHint":             "Open the global CRF search",
"crf.table.column.code":                    "Form Code",
"crf.table.column.name":                    "Form Name",
"crf.table.column.taker":                   "Taker",
"crf.table.column.status":                  "Status",
"crf.table.column.actions":                 "Operations",
"crf.table.action.assignTakers":            "Assign takers",
"crf.table.action.edit":                    "Edit form",
"crf.table.action.delete":                  "Delete form",
"crf.table.action.openDetail":              "Open form detail",
"crf.table.action.addForm":                 "Add form",
"crf.table.action.filter":                  "Filter forms",
"crf.drawer.create.title":                  "Create CRF Form",
"crf.drawer.edit.title":                    "Edit CRF Form",
"crf.drawer.field.code":                    "Form Code",
"crf.drawer.field.name":                    "Form Name",
"crf.drawer.submit.create":                 "Create",
"crf.drawer.submit.save":                   "Save",
"crf.filter.title":                         "Filter CRF Forms",
"crf.filter.search":                        "Search by code or name",
"crf.filter.status":                        "Status",
"crf.filter.status.approved":               "Approved",
"crf.filter.status.pending":                "Pending",
"crf.filter.involved":                      "Involved",
"crf.delete.title":                         "Delete CRF Form",
"crf.delete.message":                       'Delete form "{code} — {name}"? This cannot be undone.',
"crf.delete.submit":                        "Delete",
"crf.assignTakers.title":                   "Assign Takers",
"crf.assignTakers.placeholder":             "Takers UI coming soon",
```

(`zhCN.ts` mirrors each string.)

## 8. Error handling

- Mutations surface their `ApiError` via the standard
  `<Alert severity="error">{errorMessage(err)}</Alert>` pattern.
- Detail page renders an `<Alert severity="error">` on `useGetCrfForm`
  failure with the back button still functional.
- RBAC is **not** enforced in the CRF UI this PR — every authenticated
  user can create / edit / delete forms. Server-side permission checks
  remain the source of truth.

## 9. Testing

Per `apps/desktop/aegis-desktop/docs/guidelines/aegis-desktop-development.md`
section 10:

- Vitest: a smoke test for `CrfFormListPage` that mounts under
  `renderInRouter(ui)`, mocks `invoke` with `mockCommands({...})`,
  returns a single row, and asserts the row text + that the table
  renders. Plus a smoke test for `CrfFormDrawer` in `mode="create"`
  that fires `create_crf_form` on submit.
- Rust: `#[cfg(test)] mod tests` in each new http submodule
  (`http/crf/version.rs`, `http/crf/form.rs`) using `wiremock` +
  `MemoryStore`, mirroring `http/terminology/version.rs`.

## 10. Verification gate

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```

## 11. Files touched (summary)

### Add

- `src/routes/_authed/project/$projectCode/crf/index.tsx`
- `src/routes/_authed/project/$projectCode/crf/$formId.tsx`
- `src/routes/_authed/project/$projectCode/crf/search.tsx`
- `src/features/crf/{index.ts, data/list.ts, components/*.tsx, pages/*.tsx}`
- `src-tauri/src/http/crf.rs`, `src-tauri/src/http/crf/version.rs`, `src-tauri/src/http/crf/form.rs`
- `src-tauri/src/commands/crf.rs`, `src-tauri/src/commands/crf/version.rs`, `src-tauri/src/commands/crf/form.rs`

### Modify

- `src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx` — add CRF menu entry + `AssignmentIcon` import.
- `src/shared/api/types.ts` — add CRF section.
- `src/shared/query/keys.ts` — add `crf` branch.
- `src/shared/api/index.ts` — add `crf` namespace.
- `src-tauri/src/http.rs` — `pub mod crf;`.
- `src-tauri/src/commands.rs` — `pub mod crf;`.
- `src-tauri/src/lib.rs` — extend `generate_handler!` with the `// crf` block.
- `lib/packages/ui/src/i18n/locales/en.ts`, `zhCN.ts` — add CRF keys.