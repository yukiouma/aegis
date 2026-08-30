# aegis-desktop CRF `CreateCrfVersion` page — design

Date: 2026-08-30
Scope: add a Tauri-side `import_als` command plus a TS-side
`CreateCrfVersionPage` so a user can create a new CRF version under a
project by uploading an ALS file. The page accepts a version name, an
EDC type (Rave / eCollect V6 / eCollect Legacy), and an ALS file (xls /
xlsx / xml). The Rust side parses the file with `als-resolver`, maps the
result to the wire, runs a pre-validation pass, then creates the version
and one `bulk_create_form` per form, mirroring the proven
`terminology::import` + `ImportTerminologyPage` shape.

Sibling specs that anchor this one:
- [2026-08-28-aegis-desktop-crf-feature-design.md](2026-08-28-aegis-desktop-crf-feature-design.md)
  — the crf feature module layout, query-key factory, and route shape.
- [2026-08-29-aegis-desktop-crf-detail-page-design.md](2026-08-29-aegis-desktop-crf-detail-page-design.md)
  — the most recent crf-feature precedent for a new page (route file,
  barrel wiring, mutation hook).

## 1. Decisions log

| Question | Decision | Rationale |
| --- | --- | --- |
| Failure policy when one form's bulk insert fails mid-loop | **Pre-validate, then insert.** | Fast-fail with a precise message before any DB writes. Leaves no orphan data on the failure path. |
| Import `CRFForm.domains` → `domain_annotations` too? | **No, skip in v1.** | Spec listed only forms / items / options / units; keep scope tight. A future feature can add a per-form `create_domain_annotation` loop. |
| Route path | **`/_authed/project/$projectCode/crf/versions/new`** | Matches REST resource hierarchy (versions is a sub-resource of crf). Foreshadows a future `/crf/versions/$versionId` detail route. Project-scoped, same as the other crf pages. |
| Where to do ALS parsing | **Rust command, off-thread via `spawn_blocking`** | `als-resolver` is sync / CPU-bound (calamine + quick-xml). Matches the `terminology::from_path` precedent. |
| Where to do per-form `bulk_create_form` orchestration | **Same Rust command** | Keeps the TS page a thin form; one IPC call covers everything; mirrors `commands/terminology/import.rs`. |

## 2. Goals

1. The user opens `/project/$projectCode/crf/versions/new`, types a
   version name, picks an EDC type, and either clicks the drop zone
   (native file dialog with `xls` / `xlsx` / `xml` filter) or drags an
   ALS file onto it. The submit button stays disabled until all three
   are valid (name non-empty, EDC type set, file selected, name not
   duplicating an existing version under the same project).
2. Submitting sends `{ name, filepath, edcType }` to a new Tauri
   command `import_als`. The command returns the created
   `CrfVersionViewResponse`.
3. On success the page invalidates
   `queryKeys.crf.versionsByProject(projectCode)` and
   `queryKeys.crf.formsByVersion(view.id)`, navigates to
   `/project/$projectCode/crf?versionId=<view.id>`, and surfaces a
   success Snackbar.
4. On failure the page shows an error Snackbar with the server's
   stable code (`duplicate_crf_version`, `kind_shape_violation`, …) or
   the parser / pre-validate message. The page does not auto-roll back
   the version on partial-failure; the user can delete the partial
   version from the form-list chrome.
5. The duplicate-name check runs in-place via `useListCrfVersions`
   with a 300 ms debounce, mirroring the standard filter-input pattern
   in §11 of `aegis-desktop-development.md`. No extra IPC.

## 3. Architecture

Two new files in the Rust backend, one new feature page + supporting
hook on the TS side, one new route file. All other changes are
additive (one line per modified file).

### 3.1 File map

```
apps/desktop/aegis-desktop/src-tauri/src/
  commands/crf/version.rs              MODIFIED: add `import_als` shim
  http/crf/version.rs                   MODIFIED: add ImportAls* DTOs + parse + map + loop
  lib.rs                                MODIFIED: register `commands::crf::version::import_als`
                                          in the `tauri::generate_handler![ … ]` list

apps/desktop/aegis-desktop/src/
  routes/_authed/project/$projectCode/crf/versions/
    new.tsx                             NEW route file
  features/crf/
    index.ts                            MODIFIED: export CreateCrfVersionPage
    pages/
      CreateCrfVersionPage.tsx         NEW page
      index.ts                          MODIFIED: export it
    data/
      import.ts                         NEW mutation hook (single useImportAls)
      index.ts                          MODIFIED: re-export it
  shared/api/
    types.ts                            MODIFIED: add CrfEdcType union + ImportAlsRequest +
                                          CrfVersionViewResponse TS mirror (the latter only
                                          if it isn't already mirrored)
    index.ts                            MODIFIED: add api.importAls(name, filepath, edcType)

lib/packages/ui/src/i18n/<lang>/
  <resource>.json                       MODIFIED: add `crf.import.*` keys
```

### 3.2 Boundary discipline

- `commands/crf/version.rs` stays a 1-line shim. The new `import_als`
  command looks exactly like its neighbours (signature: `(client,
  name, filepath, edc_type) -> Result<CrfVersionViewResponse, ApiError>`).
- `http/crf/version.rs` is the **only** file that imports
  `als_resolver` and the only file that holds a `als_resolver::Project`
  or maps `ControlType → CrfItemKind`. The Tauri command module must
  not import `als-resolver` directly — it stays a pass-through.
- The TS feature never sees a parsed `Project`; it sends three
  primitives (`name`, `filepath`, `edcType`) and gets back a
  `CrfVersionViewResponse` shape that mirrors the existing wire DTO in
  `http/crf/version.rs`.

### 3.3 Field mapping table (`als_resolver::Project` → wire DTO)

`http/crf/version.rs::import_als` uses this table as the source of
truth. Tests pin every row.

| als-resolver field | wire field | Notes |
| --- | --- | --- |
| `CRFForm.name` | `form.code` | als-resolver's `name` is the OID. |
| `CRFForm.description` | `form.name` | human-facing name. |
| `CRFForm.order` | `form.order` | set by als-resolver's visit traversal. |
| `CRFForm.items[i].name` | `item.code` | |
| `CRFForm.items[i].label` | `item.name` | |
| `index in items` | `item.order` | als-resolver does not assign item order; use 0-based index. |
| `ControlType::TEXT` | `item.kind = "text"` | |
| `ControlType::DATETIME` | `item.kind = "datetime"` | |
| `ControlType::SELECTION` | `item.kind = "selection"` | pre-validate: options must be non-empty. |
| `ControlType::CHECKBOX` | `item.kind = "checkbox"` | pre-validate: options must be non-empty. |
| `CRFItem.item_option: None` | `options: []` | |
| `CRFItem.item_option: Some(vec)` | each → `{ value: option.option_display }` | |
| `CRFItem.item_unit: None` | `units: []` | |
| `CRFItem.item_unit: Some(u)` | `[{ value: u.value }]` | |
| `CRFForm.domains` | _dropped_ | per §1: no domain annotations in v1. |
| Every `not_submitted` field | `false` | new version = fresh, expected to be submitted. |

## 4. Data flow

The Rust command does three isolated steps; no early return can leave
half-mutated state. Errors propagate as `ApiError`; the TS-side
mutation hook renders them through the existing `errorMessage(e)`
helper.

```
[TS] CreateCrfVersionPage
  ├─ pickFile → open({ extensions: ["xls", "xlsx", "xml"] })
  ├─ onDragDropEvent(TAURI) → filepath (Tauri v2 intercepts OS-level;
  │   DOM `drop` is not fired inside the webview)
  ├─ useListCrfVersions(projectCode) + useDebouncedValue(name, 300)
  │      → duplicate-name warning chip in-place
  └─ submit (when name valid + no dup + edcType set + filepath set)
        │
        ▼ invoke("import_als", { name, filepath, edcType })
[Rust] commands::crf::version::import_als
        │ (delegates without change)
        ▼
[Rust] http::crf::version::import_als(name, filepath, edcType)
  1. tokio::task::spawn_blocking(move || match edc_type {
        Rave             => als_resolver::parse_rave_als(BufReader::new(File::open(filepath)?)),
        EcollectV6       => als_resolver::parse_ecollect_v6_als(...),
        EcollectLegacy   => als_resolver::parse_ecollect_legacy_als(...),
     })  → Result<Project, AlsParseError>
       AlsParseError → ApiError::Parse { message: e.to_string() }
  2. PRE-VALIDATE every form/item/option/unit
       - form.code, form.name non-empty (trim)
       - item.code, item.name non-empty (trim)
       - option.value, unit.value non-empty (trim)
       - kind-shape: Selection|Checkbox ⇒ options non-empty;
                    Text|Datetime|Label ⇒ options empty
       Each violation → ApiError::Parse { message: "<form #N item 'XYZ': …>" }
       (Parse (not Http) because the page renders all import-time
       errors via the same Snackbar path. Status 400-equivalent in
       shared/api/error.ts via the kind="parse" branch.)
       Pre-validation mirrors the server-side validator in
       `lib/crates/crf/src/domain/crf_bulk_form.rs` — server wins on
       any future divergence.
  3. POST /api/crf/projects/{project_code}/versions { name }
       → CrfVersionViewResponse { id, project_code, name, … }
       Failure (incl. 409 duplicate_crf_version) propagates.
  4. for each CRFForm f in project.forms:
       POST /api/crf/versions/{id}/forms/bulk
         body = BulkCreateCrfFormRequest mapped from f
       Failure (incl. 400 kind_shape_violation, 409 duplicate_crf_form)
       propagates. The version is left with N-1 forms; user deletes
       it manually. See §5 Recovery story.
  5. return CrfVersionViewResponse { id, project_code, name, … }
        │
        ▼
[TS] mutation onSuccess(view)
  - qc.invalidateQueries({ queryKey: queryKeys.crf.versionsByProject(projectCode) })
  - qc.invalidateQueries({ queryKey: queryKeys.crf.formsByVersion(view.id) })
  - navigate({ to: "/_authed/project/$projectCode/crf",
               search: (prev) => ({ ...prev, versionId: view.id }),
               replace: true })
  - Snackbar: success / error
```

## 5. Error handling

Three buckets, distinguished by source. Each resolves to a single
render path on the page (inline field warning, or Snackbar).

### 5.1 Bucket A — Frontend-side, no IPC

Caught at submit time; the submit button stays disabled so no mutation
fires.

| Symptom | Where shown | i18n key |
| --- | --- | --- |
| Empty / whitespace-only `name` | under name field | `crf.import.errors.nameRequired` |
| Name collides with an existing version under the same project (`useListCrfVersions`) | under name field as a red chip | `crf.import.errors.nameDuplicate` |
| No `edcType` selected | under EDC picker | `crf.import.errors.edcTypeRequired` |
| No `filepath` selected | drop zone label | `crf.import.errors.fileRequired` |
| Drop with extension not in `.xls` / `.xlsx` / `.xml` | drop-zone border flashes red for 1.5 s | `crf.import.errors.fileTypeHint` |

### 5.2 Bucket B — Rust-command errors

All map to `ApiError` and surface in the mutation's `onError`
Snackbar via `errorMessage(e)`. The user sees the server's stable
`code` verbatim:

| Source | `ApiError` variant | Snackbar message |
| --- | --- | --- |
| ALS parse failure | `Parse { message }` | parse error text |
| Pre-validation mismatch in Rust | `Parse { message: "<form #N …>" }` | parse error text |
| 409 on `/projects/.../versions` | `Http { 409, "duplicate_crf_version", … }` | `duplicate_crf_version: …` |
| 400 on `/versions/.../forms/bulk` | `Http { 400, "kind_shape_violation", … }` | `kind_shape_violation: …` |
| 409 on `/versions/.../forms/bulk` | `Http { 409, "duplicate_crf_form"\|"duplicate_crf_item", … }` | exact code |
| Other 401 / 403 / 404 / 5xx | `Http { … }` | exact code + message |
| Network drop / refresh failure | `Network { … }` / `RefreshFailed` | network message |

The command does NOT auto-roll-back on partial-failure (a version
exists with N-1 forms). See Recovery story below.

### 5.3 Recovery story (user-owned, not silent)

When the partial-failure pattern surfaces (Bucket B, Data Flow step
4), the version was inserted in step 3 and some forms were inserted
in step 4 before the failing form. We do **not** call `delete_version`
behind the user's back. Instead:

- The error Snackbar shows the exact server code + message, so the
  user knows which form / kind-shape / duplicate code triggered it.
- The user navigates to the form list (the navigation in step 5
  includes `versionId: <view.id>`, so they land on their new version)
  and deletes the partial version via the existing delete chrome in
  `CrfFormListPage`.

This is an honest, observable contract: the user sees what happened
and decides whether to keep, fix, or remove the partial work.

### 5.4 Silent / invariant errors

Caught at write-time and asserted, not propagated:

- `version_id <= 0` after `create_version` call → `ApiError::Parse`
  ("invalid version_id from create_version response"). Defensive only.
- Empty `items: Vec` in a form is fine — pass `items: []` to the server.

## 6. Testing

18 tests, no live-DB fixtures, no `wiremock` on the TS side. The
`wiremock` use is limited to two Rust integration tests (#10 and
#11).

### 6.1 Rust unit tests (`http/crf/version.rs`)

1. `map_control_type_to_kind_covers_all_variants` — exhaustive:
   `TEXT→text, DATETIME→datetime, SELECTION→selection, CHECKBOX→checkbox`.
   Any new variant must break this test.
2. `map_project_to_bulk_requests_preserves_order` — hand-rolled
   `Project` with 3 forms × 4 items × 2 options. Asserts the resulting
   `Vec<BulkCreateCrfFormRequest>` matches the expected
   form-code → item-code → option-value tree with `order = 0..n`.
3. `pre_validate_rejects_selection_with_empty_options` — Selection
   item with `options: []` returns
   `Err(AlsImportError::KindShapeViolation { form_index, item_code, kind, field: "options" })`.
4. `pre_validate_rejects_text_with_options` — Text item with one
   option returns the same shape.
5. `pre_validate_rejects_empty_form_code` — form.code = `"   "`
   returns `Err(AlsImportError::EmptyCode { target: "form", index })`.
6. `pre_validate_accepts_zero_items` — form with `items: []`
   returns `Ok`.
7. `pre_validate_isolates_per_form_errors` — pass a Project with 3
   forms where form #2 fails; the error must name `form_index: 1` (zero-based).
8. `parse_filename_dispatches_correctly` — `edc_type=rave` on a
   `.xml` → `parse_rave_als`; `edc_type=ecollectV6` on a `.xlsx` →
   `parse_ecollect_v6_als`; `edc_type=ecollectLegacy` on a `.xls` →
   `parse_ecollect_legacy_als`. The dispatch is by `edc_type`, not
   extension — pins that contract.
9. `als_import_error_wraps_to_api_error_parse` — `From<AlsImportError>
   for ApiError` produces `ApiError::Parse { message }` carrying the
   inner message.

### 6.2 Rust integration tests (`http/crf/version.rs`, `#[tokio::test]`)

10. `import_als_returns_version_id_from_create_response` — `wiremock`
    + `MemoryStore`. Stubs `POST /projects/P1/versions → 201 { id: 7 }`
    and a 2-form `Project`. Asserts `import_als(...)` returns the
    version view with `id = 7` and that exactly one `forms/bulk` call
    per form landed, both with `version_id: 7` patched in.
11. `import_als_aborts_on_first_form_409` — form #1 → 201, form #2 →
    409 `duplicate_crf_form`. Asserts `Err(ApiError::Http { 409,
    "duplicate_crf_form", … })`; only one `forms/bulk` call hit the
    mock; `versions` was called exactly once (no auto-rollback).

### 6.3 Frontend component tests (`src/test/features/crf/create-crf-version.test.tsx`)

12. `renders three required fields and submit-disabled by default` —
    name input, EDC picker, drop zone, submit disabled. Uses
    `renderWithQueryClient` with empty `listCrfVersions`.
13. `shows duplicate warning when name collides with an existing version`
    — `mockCommands({ listCrfVersions: () => [{ id: 1, name: "v1", … }]
    })`, type `v1`, wait debounce, assert the warning chip appears.
14. `enables submit only when all three fields are valid` — four
    parameterized cases. Asserts `disabled` flips on/off correctly.
15. `opens native file dialog when drop zone is clicked` —
    `vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn()
    .mockResolvedValue("/abs/path.xls") }))`, click the zone, assert
    `open` was called with `extensions: ["xls", "xlsx", "xml"]`, assert
    the chip renders the basename.
16. `drag-drop of unsupported extension flashes red border and rejects`
    — uses the same `vi.mock("@tauri-apps/api/webview", …) +
    dragDropHandler` capture pattern as
    `src/test/features/terminology/import-terminology-page.test.tsx`
    (set up by importing the page's `onDragDropEvent` callback into a
    local `dragDropHandler`, then calling it with `{ type: "drop",
    paths: ["/x/photo.png"] }`). Assert the file chip does NOT appear
    and the drop zone's red-border style is applied (locate the drop
    zone via a `data-testid`).
17. `submit calls api.importAls and invalidates versionsByProject on success`
    — `vi.spyOn(api, "importAls").mockResolvedValue({ id: 99, … })`,
    submit, await, assert `invalidateQueries` was called with
    `queryKeys.crf.versionsByProject("P1")` and with
    `queryKeys.crf.formsByVersion(99)`, and `navigate` was called with
    `to: "/_authed/project/$projectCode/crf"` plus
    `search: { versionId: 99 }`.
18. `mutation error renders snackbar with server code` — mock
    `importAls` to throw `{ kind: "http", status: 409, code:
    "duplicate_crf_version", message: "…" }`, submit, await, assert
    the Snackbar contains the error code/message.

(Test #9 is the `AlsImportError → ApiError::Parse` round-trip; the
others count toward the 18.)

### 6.4 i18n key tests

For every new key in `crf.import.*` (§7): assert it exists in every
`@aegis/ui` translation resource (`en`, etc.). Existing translator
helper is re-used.

### 6.5 What is intentionally NOT tested

- Live-DB integration end-to-end. The desktop crate is transport-only
  in this codebase; the live-DB coverage already lives in
  `lib/crates/crf`'s own `bulk_create_form` tests, exercised on real
  Postgres when the project's e2e suite runs.
- Server-side kind-shape validation drift — by design. The pre-validate
  pass is a fast-fail; the server remains the source of truth.
- Pre-validation re-runs on every selection rule — one test per rule is
  sufficient for the assertion surface.

## 7. i18n keys (new under `crf.import.*`)

```
crf.import.title
crf.import.nameLabel
crf.import.namePlaceholder
crf.import.errors.nameRequired
crf.import.errors.nameDuplicate
crf.import.edcTypeLabel
crf.import.errors.edcTypeRequired
crf.import.edcTypeRave
crf.import.edcTypeEcollectV6
crf.import.edcTypeEcollectLegacy
crf.import.dropZone
crf.import.selectedFile (with chip onDelete tooltip)
crf.import.errors.fileRequired
crf.import.errors.fileTypeHint
crf.import.submit
crf.import.importing
crf.import.success (with { name })
crf.import.failure (with { message })
```

All keys go through `useI18n()` → `t(...)` — no inline strings.

## 8. Out of scope (deliberate)

1. **Domain annotations.** `CRFForm.domains: Vec<Domain>` is dropped.
   A future feature would add a per-form `create_domain_annotation`
   loop after each successful `bulk_create_form`.
2. **Progress streaming.** The command returns only when every form is
   inserted; large ALS files can sit on the spinner for minutes. A v2
   could `app.emit("als-import:progress", { done, total })` and render
   a `LinearProgress` bar.
3. **Cancel / abort.** No mid-import Cancel button. Adding one requires
   a `tokio_util::sync::CancellationToken` plumbed into the command and
   a matching `api.cancelAlsImport()` endpoint.
4. **Auto-detect EDC type from extension / contents.** The user must
   pick. A heuristic (XML→Rave, XLSX-with-FormOID→eCollect V6, …
   otherwise→Legacy) plus a trust-the-user fallback would be a separate
   UX surface.
5. **Re-import / update-in-place.** This is a *create* flow. Editing an
   existing CRF version from a new ALS would be a separate feature.
6. **Annotation metadata on items / options / units.**
   `CRFItem.annotations: Vec<Annotation>` and friends are dropped; the
   bulk DTOs have no place for them today. Expanding the bulk wire
   shape is a server-side change.
7. **Visit / schedule import.** `Project.visit: Vec<Visit>` is dropped.
   The CRF schema doesn't have a visits concept today; that would be a
   new entity.
8. **Per-row `not_submitted` UI during import.** New versions always
   arrive with `not_submitted=false`. Per-row toggling is via the
   existing per-row editing chrome.

## 9. Verification gate

The standard gate from `aegis-desktop-development.md` §13 plus the
new tests above. One PR, two commits (`docs(spec):`, then
`feat(crf):`):

```bash
# Frontend
pnpm typecheck
pnpm test
pnpm build

# Backend
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```
