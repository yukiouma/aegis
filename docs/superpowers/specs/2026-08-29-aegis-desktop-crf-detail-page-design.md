# aegis-desktop CRF detail page — design

Date: 2026-08-29
Scope: build the body of `CrfDetailPage` so the user can read and mutate
`(domain_)annotations` for a CRF form, using the existing
`/api/crf/.../details` endpoint and CRUD endpoints on
`apps/server/aegis-server/src/transport/http/crf/handlers.rs`.

## 1. Background

`CrfDetailPage` is the placeholder body that lives at
`/project/$projectCode/crf/$formId`. The previous PR
(`2026-08-28-aegis-desktop-crf-feature-design.md`) shipped only the
header skeleton and a "Form detail view coming soon" alert. The server
already exposes:

- `GET /api/crf/forms/{id}/details` → composed fetch
  (`CrfFormDetailResponse`).
- `POST /api/crf/forms/{id}/domain-annotations` /
  `PATCH /api/crf/domain-annotations/{id}` /
  `DELETE /api/crf/domain-annotations/{id}`.
- `POST /api/crf/annotations` /
  `PATCH /api/crf/annotations/{id}` /
  `DELETE /api/crf/annotations/{id}`.

The detail page wires these up so the user can see and mutate
annotations at every layer (form, item, option, unit).

The user clarified: the domain-annotation chip format is
`name(description)` (not `name(code)`), and the edit dialog mutates
`name` + `description`. The Rust aggregate already carries those two
fields, so **no schema migration is required**.

## 2. Goals

1. Replace the `CrfDetailPage` placeholder with the spec'd layout:
   - Header (existing back / code chip / name / tools menu) +
     hover-popup menu on the form name with **New Domain** /
     **New Annotation** entries.
   - Row of **domain-annotation chips** (`name(description)`),
     closable, each opens an edit dialog; closing one confirms before
     deleting (cascades to its annotations server-side).
   - **Form annotation chips** for form-level annotations, each
     closable; click opens edit dialog, close opens a confirm dialog.
   - **Item rows**: code chip, clickable item name (opens *new*
     item-annotation dialog), annotation chips (closable, click to
     edit), unit on the right (with unit-annotation chips to the
     left of the unit), options listed under the item name for
     `kind === "Selection"` with their annotation chips on the right.
2. Annotation chip color cycles `info → warning → success → error`
   based on the *index* of the owning `domain_annotation` in the
   form's `domain_annotations` list, modulo 4. Annotations without
   a known owning domain annotation fall back to `default`.
3. Mutation success invalidates the form-detail query so chips
   reflect server state immediately.
4. Mutations surface server errors in the dialog footer (same
   `errorMessage(...)` helper the other dialogs use).

## 3. Out of scope

- Item / Option / Unit CRUD — server has the endpoints but the
  detail page only *renders* them and *annotates* them. The form
  CRUD path stays on the form-list page.
- Form code / name edit — header reads them but does not mutate.
- Bulk create / search endpoints.
- Server-side changes (the DTOs in `transport::http::dto` already
  match the spec).
- RBAC gating (consistent with the rest of the feature).
- Drag-and-drop reordering of items / annotations.
- Pagination / infinite scroll (the detail payload is bounded by
  the form's tree).

## 4. Backend plumbing (desktop Rust shim)

Three new HTTP modules plus additions to the existing `form.rs`,
under `apps/desktop/aegis-desktop/src-tauri/src/http/crf/`:

```
http/crf/
├── form.rs                  (existing; add `details` helper + types)
├── item.rs                  (NEW)
├── annotation.rs            (NEW)
├── domain_annotation.rs     (NEW)
└── version.rs               (existing)
```

Each module follows the existing `form.rs` pattern: DTO structs with
`#[serde(rename_all = "camelCase")]`, `pub async fn` wrappers that
call `c.request(...)` or `c.request_bytes(...)`, and wire-level
`#[cfg(test)] mod tests` with `wiremock`.

`form.rs` gains:

```rust
pub struct CrfItemViewResponse { /* id, form_id, code, name, kind, order, ... */ }
pub struct CrfOptionViewResponse { /* id, item_id, value, ... */ }
pub struct CrfUnitViewResponse { /* id, item_id, value, ... */ }
pub struct CrfOptionDetailResponse { pub option: CrfOptionViewResponse, pub annotations: Vec<AnnotationViewResponse> }
pub struct CrfUnitDetailResponse { pub unit: CrfUnitViewResponse, pub annotations: Vec<AnnotationViewResponse> }
pub struct CrfItemDetailResponse { pub item: CrfItemViewResponse, pub options: Vec<CrfOptionDetailResponse>, pub units: Vec<CrfUnitDetailResponse>, pub annotations: Vec<AnnotationViewResponse> }
pub struct DomainAnnotationViewResponse { /* id, form_id, name, description, ... */ }
pub struct AnnotationOwner /* serde tag = "kind", rename = "option" for the Option variant */;
pub struct AnnotationViewResponse { /* id, domain_annotation_id, content, assign, owner */ }
pub struct CrfFormDetailResponse { pub form: CrfFormViewResponse, pub form_annotations: Vec<AnnotationViewResponse>, pub items: Vec<CrfItemDetailResponse>, pub domain_annotations: Vec<DomainAnnotationViewResponse> }
```

Plus CRUD request shapes and async helpers (`create / get_by_id /
list / update / delete` per resource).

New command shims under
`apps/desktop/aegis-desktop/src-tauri/src/commands/crf/`:

```
commands/crf/
├── annotation.rs            (NEW: create/update/delete + lists)
├── domain_annotation.rs     (NEW: create/get_by_id/list/update/delete + search)
├── form.rs                  (existing; add `get_crf_form_details`)
├── item.rs                  (NEW: get_by_id/list + search)
└── version.rs               (existing)
```

Each shim is a `#[tauri::command] async fn` that delegates to the
matching `http::crf::*` helper. All commands are registered in the
`generate_handler!` invocation in
`apps/desktop/aegis-desktop/src-tauri/src/lib.rs`.

## 5. Wire-DTO mirror (TS)

`apps/desktop/aegis-desktop/src/shared/api/types.ts` gains the
following shapes, hand-maintained to mirror the server DTOs:

```ts
export type CrfItemKind = "text" | "selection" | "checkbox" | "datetime" | "label";

export interface CrfItem  { id; formId; code; name; kind; order; notSubmitted; createdAt; updatedAt; }
export interface CrfOption { id; itemId; value; notSubmitted; createdAt; updatedAt; }
export interface CrfUnit   { id; itemId; value; notSubmitted; createdAt; updatedAt; }
export interface DomainAnnotation { id; formId; name; description; createdAt; updatedAt; }

export type AnnotationOwner =
  | { kind: "form"; id: number }
  | { kind: "item"; id: number }
  | { kind: "option"; id: number }
  | { kind: "unit"; id: number };

export interface Annotation { id; domainAnnotationId; content; assign; owner: AnnotationOwner; createdAt; updatedAt; }
export interface CrfFormDetail {
  form: CrfForm;
  formAnnotations: Annotation[];
  items: CrfItemDetail[];
  domainAnnotations: DomainAnnotation[];
}
export interface CrfItemDetail  { item: CrfItem;  options: CrfOptionDetail[]; units: CrfUnitDetail[]; annotations: Annotation[]; }
export interface CrfOptionDetail { option: CrfOption; annotations: Annotation[]; }
export interface CrfUnitDetail   { unit: CrfUnit;   annotations: Annotation[]; }

export interface CreateDomainAnnotationInput { name: string; description: string; }
export interface UpdateDomainAnnotationInput { name?: string; description?: string; }
export interface CreateAnnotationInput { domainAnnotationId: number; content: string; assign: boolean; owner: AnnotationOwner; }
export interface UpdateAnnotationInput { content?: string; assign?: boolean; }
```

`shared/api/index.ts` exposes wrappers that match the Tauri command
names: `getCrfFormDetails(id)`, `createDomainAnnotation(formId, body)`,
`updateDomainAnnotation(id, body)`, `deleteDomainAnnotation(id)`,
`createAnnotation(body)`, `updateAnnotation(id, body)`,
`deleteAnnotation(id)`. Each forwards `{ id, body: { ...body } }` or
the equivalent per the request shape (see `createCrfForm` for the
`{ versionId, body }` precedent).

## 6. Query layer

New key factory entry in `shared/query/keys.ts`:

```ts
crf: {
  ...existing,
  formDetail: (id: number) => ["crf", "formDetail", id] as const,
}
```

New file `apps/desktop/aegis-desktop/src/features/crf/data/detail.ts`
hosts:

- `useCrfFormDetail(id: number | null)` — `useQuery<CrfFormDetail>`.
- `useCreateDomainAnnotation()` / `useUpdateDomainAnnotation()` /
  `useDeleteDomainAnnotation()` — each invalidates
  `queryKeys.crf.formDetail(formId)`. `deleteDomainAnnotation` only
  needs the domain annotation's `formId` to scope the invalidation,
  so the mutation caller passes `{ id, formId }`.
- `useCreateAnnotation()` / `useUpdateAnnotation()` /
  `useDeleteAnnotation()` — same invalidation pattern. Delete
  callers pass `{ id, formId }` so the hook can invalidate.

The existing `data/list.ts` keeps its current shape; nothing moves
out of it.

## 7. Components

New files under `apps/desktop/aegis-desktop/src/features/crf/components/`:

### `DomainAnnotationDialog.tsx`
Right-anchored `Drawer` (matches `CrfFormDrawer` style). Props:
`{ open, mode: "create" | "edit", row?, onClose, onSubmit(body),
mutationError, mutationPending }`. Renders two `TextField`s for
`name` (required) and `description` (multiline). Body shape:
`{ name: string; description: string }`.

### `AnnotationDialog.tsx`
Right-anchored `Drawer`. Props:
`{ open, mode: "create" | "edit", owner: AnnotationOwner, row?,
availableDomainAnnotations: DomainAnnotation[], onClose, onSubmit(body),
mutationError, mutationPending }`. Fields:

- `domain_annotation` — `Select` of `availableDomainAnnotations`,
  required, disabled in edit mode (the spec says the owner is fixed
  at create time; same applies to the owning domain annotation —
  mutations only edit content + assign).
- `content` — multiline `TextField`, required.
- `assign` — `Checkbox` (`Assign` label).

Body shape:
`{ domainAnnotationId: number; content: string; assign: boolean }`.

The dialog **does not** include `owner`; the calling chip / page
composes the final `CreateAnnotationInput` by merging
`{ ...dialogBody, owner }` at the call site.

### `DeleteDomainAnnotationDialog.tsx`
`Dialog` (matches `DeleteCrfFormDialog`). Confirm copy
`"Delete domain annotation \"{name}\" and all annotations using it?
This cannot be undone."`.

### `DeleteAnnotationDialog.tsx`
`Dialog`. Confirm copy
`"Delete this annotation? This cannot be undone."`. Preview the
truncated `content`.

### `AnnotationChip.tsx`
`{ annotation: Annotation, colorIndex: number, onEdit(),
onDelete() }`. Uses `<Chip label={annotation.content} ... />` with
`color` from a small `domainAnnotationColor(index)` helper that
maps `index % 4` to `"info" | "warning" | "success" | "error"`.
Closable (calls `onDelete` on the X). Clickable body (calls
`onEdit`).

### `CrfItemRow.tsx`
`{ itemDetail: CrfItemDetail, colorByDomainAnnotationId: Map<number, number>,
onCreateItemAnnotation(itemId), onEditAnnotation(a),
onDeleteAnnotation(a), onCreateOptionAnnotation(optionId),
onCreateUnitAnnotation(unitId), ... }`. Renders:

```
[code-chip]  itemName                [annotation-chips…]   [unit-annotation-chips…] unit
             ├ option1   [option-annotation-chips]
             ├ option2   [option-annotation-chips]
             ...
```

Item name is a `Link`/`Typography` with `onClick` opening the
*new-annotation* dialog (owner = `Item`). Unit (when present) is on
the right; unit annotations live on the left of the unit. Options
(when `kind === "Selection"`) render under the item name, each with
its annotation chips to the right.

### `CrfAnnotationArea.tsx`
Renders the form-level annotation chips in a `Stack`-with-flex-wrap.
Same edit/close pattern as the chips inside `CrfItemRow`.

`components/index.ts` re-exports the new files.

## 8. Page composition

`CrfDetailPage` becomes:

```tsx
const id = … // existing parse
const { data, isFetching, isError, error } = useCrfFormDetail(id);
// six mutations

const [domainDialog, setDomainDialog] = useState<DomainDialogState>(null);
const [annotationDialog, setAnnotationDialog] = useState<AnnotationDialogState>(null);
const [confirmDeleteDomain, setConfirmDeleteDomain] = useState<DomainAnnotation | null>(null);
const [confirmDeleteAnnotation, setConfirmDeleteAnnotation] = useState<{ annotation: Annotation; formId: number } | null>(null);
const [formNameMenuAnchor, setFormNameMenuAnchor] = useState<HTMLElement | null>(null);

const colorByDomainAnnotationId = useMemo(() => {
  const map = new Map<number, number>();
  data?.domainAnnotations.forEach((d, i) => map.set(d.id, i));
  return map;
}, [data]);
```

Layout:

```
<Box>
  <Header>
    <IconButton back />
    {data?.form.code && <Chip label={data.form.code} variant="outlined" />}
    <Typography
      variant="h5"
      onMouseEnter={(e) => setFormNameMenuAnchor(e.currentTarget)}
      onMouseLeave={() => setFormNameMenuAnchor(null)}
    >
      {data?.form.name ?? t("crf.detail.title")}
    </Typography>
    <Menu open={Boolean(anchor)} anchorEl={anchor} onClose={…}> // swapped to Popover at impl time, see §12
      <MenuItem onClick={() => { setFormNameMenuAnchor(null); setDomainDialog({ mode: "create" }); }}>
        {t("crf.detail.menu.newDomain")}
      </MenuItem>
      <MenuItem onClick={() => { setFormNameMenuAnchor(null); setAnnotationDialog({ mode: "create", owner: { kind: "form", id } }); }}>
        {t("crf.detail.menu.newAnnotation")}
      </MenuItem>
    </Menu>
    <Box flexGrow />
    <CrfToolsMenu />
  </Header>

  <DomainAnnotationChipsRow>…</DomainAnnotationChipsRow>     // name(description), closable, click → edit
  <FormAnnotationArea>…</FormAnnotationArea>                  // closable annotation chips
  <ItemList>…</ItemList>                                      // CrfItemRow per item

  <DomainAnnotationDialog … />
  <AnnotationDialog … />
  <DeleteDomainAnnotationDialog … />
  <DeleteAnnotationDialog … />
</Box>
```

Loading / error states keep their current shape (spinner /
`errorMessage(error)`).

## 9. i18n

Add to `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`:

```
crf.detail.menu.newDomain
crf.detail.menu.newAnnotation
crf.detail.domainChip.label           // template: "{name} ({description})"
crf.detail.empty                      // "No items yet"
crf.detail.itemKind.selection         // "Selection" — falls through to enum
crf.detail.itemKind.text
crf.detail.itemKind.checkbox
crf.detail.itemKind.datetime
crf.detail.itemKind.label
crf.detail.optionLabel                // "Option {value}"
crf.detail.unitLabel                  // "Unit: {value}"
crf.detail.optionsHeading             // "Options"
crf.domainDialog.create.title
crf.domainDialog.edit.title
crf.domainDialog.field.name
crf.domainDialog.field.description
crf.domainDialog.submit.create
crf.domainDialog.submit.save
crf.annotationDialog.create.title
crf.annotationDialog.edit.title
crf.annotationDialog.field.domainAnnotation
crf.annotationDialog.field.content
crf.annotationDialog.field.assign
crf.annotationDialog.submit.create
crf.annotationDialog.submit.save
crf.annotationDialog.domainAnnotation.none
crf.deleteDomain.title
crf.deleteDomain.message              // template: "Delete domain annotation \"{name}\" and all annotations using it? This cannot be undone."
crf.deleteDomain.submit
crf.deleteAnnotation.title
crf.deleteAnnotation.message
crf.deleteAnnotation.submit
crf.detail.loadFailed                 // template: "Failed to load form detail: {message}"
```

Chinese translations are added in the same commit; both files stay
in sync.

## 10. Tests

### Rust
- New `http::crf::item::tests`, `http::crf::annotation::tests`,
  `http::crf::domain_annotation::tests` mirroring the wiremock-style
  unit tests already in `http::crf::form::tests`. Each module gets
  one happy-path test per CRUD verb plus the `update_*` skip-None
  serialization assertion that the existing form tests use.
- New `get_by_id` test on `http::crf::form` for
  `GET /api/crf/forms/{id}/details`.

### TS (Vitest)
- `src/test/features/crf/crf-detail-page.test.tsx` — happy-path
  render with `mockCommands({ get_crf_form_details, … })`.
  Asserts the header reads code + name, the domain annotation chip
  renders as `name(description)`, and clicking the form name opens
  the menu.
- `crf-annotation-chip.test.tsx` — color cycle (`index % 4`),
  click / delete callbacks.
- `crf-annotation-dialog.test.tsx` — `domainAnnotation` Select is
  disabled in edit mode; submit disabled until `content` non-empty.
- `crf-domain-annotation-dialog.test.tsx` — submit disabled until
  `name` non-empty.
- Extend `src/test/shared/api.test.ts` with the new wrappers,
  asserting `invoke` is called with the right command name and
  payload shape.

## 11. Files touched

Server-side shim / plumbing (Rust):
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs` (NEW)
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

Frontend (TS):
- `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- `apps/desktop/aegis-desktop/src/features/crf/data/detail.ts` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/DeleteDomainAnnotationDialog.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/DeleteAnnotationDialog.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/features/crf/components/index.ts`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`

i18n:
- `lib/packages/ui/src/i18n/locales/en.ts`
- `lib/packages/ui/src/i18n/locales/zhCN.ts`

Tests:
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-domain-annotation-dialog.test.tsx` (NEW)
- `apps/desktop/aegis-desktop/src/test/shared/api.test.ts` (extend)

## 12. Risks / non-goals

- **Hover-popup on the form name**: MUI's `Menu` opens on click by
  default; wiring it to `onMouseEnter` + `onMouseLeave` is finicky
  (the menu unmounts the moment the cursor leaves the trigger).
  Implementation detail: use MUI's `Popover` with `onMouseLeave`
  on the popover content, or a small delay. I'll pick the
  implementation that survives a fast cursor move during
  implementation and document it in the plan.
- **Annotation chip when no domain annotations exist**: the
  `Select` in `AnnotationDialog` is empty. The dialog shows a
  disabled option *"No domain annotations on this form"* and the
  submit button stays disabled. Add a `Create one first` prompt
  linking back to the hover menu.
- **AnnotationOwner serde rename**: the wire uses `option` (not
  `Option`) as the variant tag. The TS mirror uses the same.
- **No backfill**: existing DB rows already have annotations; the
  page renders them as-is. No migration step required.
