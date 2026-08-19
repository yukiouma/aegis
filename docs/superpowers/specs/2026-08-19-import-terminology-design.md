# Aegis Desktop — Import Terminology Page — Design

**Date:** 2026-08-19
**Status:** Approved (pending spec review)
**Scope:** Wire up the placeholder `ImportButton` on the SDTM / ADaM terminology pages to a new `ImportTerminology` page that lets an admin/root user pick or drop a CDISC terminology `.xls` / `.xlsx` workbook, runs the existing per-resource create handlers against the server (one call per code list, one batch call per code list's items), and refreshes the cached terminology versions on success. Adds one new Tauri command, one new HTTP wrapper, three new shared DTOs in Rust and TypeScript, one new file route, and the matching i18n strings. Builds on the terminology page work in the [2026-08-19 terminology pages design](2026-08-19-terminology-page-design.md).

---

## 1. Goals

1. Replace the placeholder `ImportButton` Snackbar ("coming soon") on `TerminologyPage` with navigation to a real `ImportTerminology` page.
2. Add one file route — `/_authed/_layout/terminology/import` — that reads `?kind=sdtm|adam` from the query string and pre-selects the ButtonGroup, falling back to `null` (no selection) when the query is missing or unrecognised.
3. Render three UI sections on the page: a back arrow + page title, a `<ToggleButtonGroup>` for kind, and a click-or-drop file upload area. Submit is disabled until both `kind` and `filepath` are set.
4. Add one Tauri command `import_terminology(kind: TerminologyKind, filepath: String) -> Result<TerminologyVersionViewResponse, ApiError>` that parses the workbook in `spawn_blocking`, then orchestrates the existing per-resource HTTP calls in sequence: version → for each code list { code list → batch of code items }.
5. Use the existing `POST /api/terminology/code-items/batch` endpoint for code items, so a 1000-item codelist is one HTTP round-trip instead of 1000.
6. On success: invalidate `queryKeys.terminology.versions()` so the existing SDTM / ADaM list pages show the new version immediately.
7. Show a `<Snackbar>` on success or failure. During the import the form is hidden and replaced by a centered spinner; the back arrow stays enabled so the user can leave the page.
8. Out of scope: a cancel button, a custom server-side batch endpoint, a versioning name UI (the parsed sheet date is used as the version name).

---

## 2. URL map

| Path                          | Route file                                                              | Component                       |
| ----------------------------- | ----------------------------------------------------------------------- | ------------------------------- |
| `/terminology/import?kind=sdtm` | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/import.tsx` | `ImportTerminologyPage` |
| `/terminology/import?kind=adam` | same                                                                    | same                            |
| `/terminology/import`         | same                                                                    | same                            |

The `kind` query param is optional and validated as `z.enum(["sdtm", "adam"]).optional()`. Missing or unrecognised values leave the ButtonGroup unselected; the user must choose before submit.

---

## 3. Files added / changed / removed

### 3.1 Added

#### Frontend

| Path                                                                                  | Responsibility |
| ------------------------------------------------------------------------------------- | -------------- |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/ImportTerminologyPage.tsx` | The new page. |
| `apps/desktop/aegis-desktop/src/test/features/terminology/import-terminology-page.test.tsx` | Vitest + RTL tests for the page. |
| `apps/desktop/aegis-desktop/src/test/features/terminology/import-button.test.tsx` | Vitest + RTL test for the modified `ImportButton`. |

#### Tauri (Rust)

| Path                                                                                  | Responsibility |
| ------------------------------------------------------------------------------------- | -------------- |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs`              | New `import_terminology` command. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs`                    | Module declaration: add `pub mod import;`. |

The new types and HTTP wrapper live inside existing files:

- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` — add `BatchCodeItemEntry`, `BatchCreateCodeItemsRequest`, `BatchCreateCodeItemsResponse`, and `batch_create()`.
- `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs` — add the `Parse` variant to `ApiError`.

### 3.2 Modified

| Path                                                                                       | Change |
| ------------------------------------------------------------------------------------------ | ------ |
| `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx`          | Replace Snackbar with `useNavigate({ to: "/terminology/import", search: { kind } })`. Take `kind: TerminologyKind` prop. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`            | Pass `kind` into `<ImportButton kind={kind} />`. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts`                        | Re-export `ImportTerminologyPage`. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/import.tsx`              | New route file (see Section 5). |
| `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts`                                   | Regenerated by `@tanstack/router-plugin` on next `vite dev` / `vite build`. |
| `apps/desktop/aegis-desktop/src/shared/api/types.ts`                                       | + `BatchCodeItemEntry`, `BatchCreateCodeItemsInput`, `BatchCreateCodeItemsResponse`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts`                                       | + `importTerminology(kind, filepath)` wrapper. |
| `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`                                          | + `commands::terminology::import::import_terminology` in `invoke_handler!`. |
| `lib/packages/ui/src/i18n/locales/en.ts`                                                    | + 7 keys under `terminology.import.*` and + `common.submit` (which is currently missing from the locale). |
| `lib/packages/ui/src/i18n/locales/zhCN.ts`                                                  | + matching zh-CN translations. |

### 3.3 Removed

None.

---

## 4. Final directory layout

```
apps/desktop/aegis-desktop/
├── src/
│   ├── features/terminology/                       (modified)
│   │   ├── components/ImportButton.tsx             (modified — now a nav button)
│   │   ├── pages/
│   │   │   ├── TerminologyPage.tsx                 (modified — passes kind to ImportButton)
│   │   │   └── ImportTerminologyPage.tsx           NEW
│   │   └── pages/index.ts                          (modified — re-exports)
│   ├── test/features/terminology/                   NEW test directory
│   │   ├── import-terminology-page.test.tsx        NEW
│   │   └── import-button.test.tsx                  NEW
│   └── routes/_authed/_layout/terminology/
│       ├── sdtm.tsx                                (unchanged)
│       ├── adam.tsx                                (unchanged)
│       └── import.tsx                              NEW
└── src-tauri/src/
    ├── commands/
    │   ├── terminology.rs                          (modified — +pub mod import)
    │   └── terminology/
    │       ├── import.rs                           NEW
    │       ├── version.rs                          (unchanged)
    │       ├── code_list.rs                        (unchanged)
    │       └── code_item.rs                        (unchanged — receives new wrapper)
    └── http/
        ├── dto.rs                                   (modified — +Parse variant on ApiError)
        └── terminology/
            ├── version.rs                           (unchanged)
            ├── code_list.rs                         (unchanged)
            └── code_item.rs                         (modified — +batch_create + 3 DTOs)
```

---

## 5. Routing

One new file route under the existing `/_authed/_layout/` pathful layout:

```tsx
// routes/_authed/_layout/terminology/import.tsx
import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ImportTerminologyPage } from "../../../../../features/terminology";

const kindSchema = z.object({
  kind: z.enum(["sdtm", "adam"]).optional(),
});

export const Route = createFileRoute("/_authed/_layout/terminology/import")({
  validateSearch: kindSchema,
  component: () => <ImportTerminologyPage />,
});
```

`validateSearch` parses the optional `?kind=` value and surfaces a typed `kind` to the page via `Route.useSearch()`. The page reads it on first render and seeds local `kind` state; the user can change the selection in the ButtonGroup at any time before submitting.

The route file is registered automatically by `@tanstack/router-plugin`; `routeTree.gen.ts` is regenerated on the next `vite dev` / `vite build`.

---

## 6. Page

### 6.1 `ImportTerminologyPage`

Single page component with local state: `kind: TerminologyKind | null`, `filepath: string | null`, `snackbar: { open: boolean; severity: 'success' | 'error'; message: string }`.

Layout:

```tsx
<Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 3 }}>
  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
    <BackArrow onClick={() => navigate({ to: backLink })} />
    <Typography variant="h5">{t('terminology.import.title')}</Typography>
  </Box>

  {importMutation.isPending ? (
    <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2, py: 8 }}>
      <CircularProgress />
      <Typography>{t('terminology.import.importing')}</Typography>
    </Box>
  ) : (
    <>
      <ToggleButtonGroup
        exclusive
        value={kind}
        onChange={(_, v) => setKind(v)}
        disabled={importMutation.isPending}
      >
        <ToggleButton value="sdtm">SDTM</ToggleButton>
        <ToggleButton value="adam">ADaM</ToggleButton>
      </ToggleButtonGroup>

      <DropZone
        filepath={filepath}
        onPickFile={async () => {
          const path = await open({
            multiple: false,
            filters: [{ name: 'Excel', extensions: ['xls', 'xlsx'] }],
          });
          if (typeof path === 'string') setFilepath(path);
        }}
        onDropFile={(path) => setFilepath(path)}
        onClear={() => setFilepath(null)}
        disabled={importMutation.isPending}
      />

      <Button
        variant="contained"
        disabled={kind === null || filepath === null || importMutation.isPending}
        onClick={() => importMutation.mutate({ kind: kind!, filepath: filepath! })}
      >
        {t('common.submit')}
      </Button>
    </>
  )}

  <Snackbar
    open={snackbar.open}
    autoHideDuration={4000}
    onClose={() => setSnackbar((s) => ({ ...s, open: false }))}
  >
    <Alert severity={snackbar.severity} onClose={() => setSnackbar((s) => ({ ...s, open: false }))}>
      {snackbar.message}
    </Alert>
  </Snackbar>
</Box>
```

The back arrow always renders. When `kind === null`, it goes to `/terminology/sdtm` (the same fallback the ButtonGroup-disabled submit uses). When `kind === 'sdtm'` or `'adam'`, it goes to `/terminology/{kind}`.

### 6.2 `DropZone`

Inlined in the same file (no separate component yet — under 60 lines). Renders a dashed-border `<Box>` styled as a drop target. The click handler invokes `@tauri-apps/plugin-dialog`'s `open()`; the drag handlers `onDragOver` (preventDefault to allow drop) and `onDrop` (read `e.dataTransfer.files[0]`, check the extension, accept or flash the error state). When `filepath` is set, the drop area collapses and a `<Chip>` shows the basename with a delete `x` to clear it.

### 6.3 Hook wiring

```ts
const importMutation = useMutation<
  TerminologyVersionView,
  ApiError,
  { kind: TerminologyKind; filepath: string }
>({
  mutationFn: ({ kind, filepath }) => api.importTerminology(kind, filepath),
  onSuccess: (version) => {
    qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    setSnackbar({
      open: true,
      severity: 'success',
      message: t('terminology.import.success', { name: version.name }),
    });
  },
  onError: (err) => {
    setSnackbar({
      open: true,
      severity: 'error',
      message: t('terminology.import.failure', { message: errorMessage(err) }),
    });
  },
});
```

The mutation runs against the Tauri command's promise. React Query does not auto-cancel it; if the user navigates away mid-import, the command continues server-side and the `onSuccess` / `onError` callbacks fire on an unmounted component (React tolerates this; the Snackbar state update is silently dropped, which is acceptable since the user has left the page).

---

## 7. Data flow & Tauri commands

### 7.1 New HTTP wrapper — `batch_create`

`src-tauri/src/http/terminology/code_item.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}

pub async fn batch_create(
    c: &HttpClient,
    body: BatchCreateCodeItemsRequest,
) -> Result<BatchCreateCodeItemsResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-items/batch",
        Some(&body),
    )
    .await
}
```

This mirrors the existing per-item `create()` function in the same file. No change to `HttpClient::request` or to `NO_AUTH_PATHS` — the new path requires the Bearer token, which is attached automatically.

### 7.2 New error variant

`src-tauri/src/http/dto.rs`, on the `ApiError` enum:

```rust
#[error("workbook parse error: {message}")]
Parse { message: String },
```

This gives a typed home for xls parsing failures distinct from the existing `Store { message }` (which is for the Tauri store plugin) and `Network` / `Http` (which are for outbound HTTP). On the wire this becomes `{"kind": "parse", "message": "..."}` and is rendered through the existing `errorMessage(...)` helper in `src/shared/api/error.ts`.

### 7.3 New Tauri command — `import_terminology`

`src-tauri/src/commands/terminology/import.rs`:

```rust
use tauri::State;
use terminology::{from_path, TerminologyError};

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::code_item::{
    self, BatchCodeItemEntry, BatchCreateCodeItemsRequest,
};
use crate::http::terminology::code_list::{self, CreateCodeListRequest};
use crate::http::terminology::version::{self, CreateTerminologyVersionRequest};
use crate::http::terminology::TerminologyVersionViewResponse;

#[tauri::command]
pub async fn import_terminology(
    client: State<'_, HttpClient>,
    kind: TerminologyKind,
    filepath: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    // 1. Parse the workbook off-thread (calamine is sync / CPU-bound).
    let parsed = tokio::task::spawn_blocking(move || from_path(&filepath))
        .await
        .map_err(|e| ApiError::Parse {
            message: format!("join error: {e}"),
        })?
        .map_err(|e| ApiError::Parse {
            message: e.to_string(),
        })?;

    // 2. Create the version.
    let version_view = version::create(
        &client,
        CreateTerminologyVersionRequest {
            kind,
            name: parsed.name,
        },
    )
    .await?;

    // 3. For each code list, create the list and batch-create its items.
    for cl in parsed.codelist {
        let cl_view = code_list::create(
            &client,
            CreateCodeListRequest {
                version_id: version_view.id,
                code: cl.code,
                extensible: cl.extensible,
                name: cl.name,
                submission_value: cl.submission_value,
                synonym: cl.synonym,
                definition: cl.definition,
                nci_preferred_term: cl.nci_preferred_term,
            },
        )
        .await?;

        if cl.code_list.is_empty() {
            continue;
        }

        code_item::batch_create(
            &client,
            BatchCreateCodeItemsRequest {
                codelist_id: cl_view.id,
                version_id: version_view.id,
                items: cl
                    .code_list
                    .into_iter()
                    .map(|i| BatchCodeItemEntry {
                        code: i.code,
                        submission_value: i.submission_value,
                        synonym: i.synonym,
                        definition: i.definition,
                        nci_preferred_term: i.nci_preferred_term,
                    })
                    .collect(),
            },
        )
        .await?;
    }

    Ok(version_view)
}
```

`terminology::from_path` already probes the file with `std::fs::File::open` first, so a missing-file error surfaces as `TerminologyError::Io { path, source }` rather than as a `calamine::Error`. We map both kinds to `ApiError::Parse` via `Display`. The call runs in `spawn_blocking` because calamine is a synchronous, CPU-bound parser.

The command returns the `TerminologyVersionViewResponse` from step 2 so the frontend can show the resulting version's `name` in the success Snackbar. On any HTTP failure (e.g. duplicate version 409), the loop has not yet started iterating codelists, so no partial state is left on the server. (A failure at step 3 leaves the version with no code lists — this is acceptable since the user can delete the empty version manually.)

### 7.4 lib.rs registration

`src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::terminology::import::import_terminology,
])
```

`Cargo.toml` requires no change — `terminology = { git = "...", tag = "v0.1.0-alpha.1", package = "terminology" }` is already declared at line 33.

### 7.5 Frontend types & wrapper

`src/shared/api/types.ts`, after `UpdateCodeItemInput`:

```ts
export interface BatchCodeItemEntry {
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface BatchCreateCodeItemsInput {
  codelistId: number;
  versionId: number;
  items: BatchCodeItemEntry[];
}

export interface BatchCreateCodeItemsResponse {
  count: number;
  codelistId: number;
  versionId: number;
}
```

`src/shared/api/index.ts`:

```ts
importTerminology: (
  kind: TerminologyKind,
  filepath: string,
): Promise<TerminologyVersionView> =>
  call<TerminologyVersionView>('import_terminology', { kind, filepath }),
```

### 7.6 Updated `ImportButton`

```tsx
// components/ImportButton.tsx
import { useNavigate } from '@tanstack/react-router';
import { IconButton, Tooltip } from '@aegis/ui/mui';
import { Add as AddIcon } from '@aegis/ui/icons';
import { useI18n } from '@aegis/ui/i18n';
import type { TerminologyKind } from '../../../shared/api';

export function ImportButton({ kind }: { kind: TerminologyKind }) {
  const navigate = useNavigate();
  const { t } = useI18n();
  return (
    <Tooltip title={t('terminology.import.title')}>
      <IconButton
        aria-label={t('terminology.import.title')}
        onClick={() => navigate({ to: '/terminology/import', search: { kind } })}
      >
        <AddIcon />
      </IconButton>
    </Tooltip>
  );
}
```

`TerminologyPage.tsx` passes `kind` to `<ImportButton kind={kind} />`. The "coming soon" Snackbar key (`terminology.importComingSoon`) is removed from `en.ts` / `zhCN.ts`.

---

## 8. i18n

`lib/packages/ui/src/i18n/locales/en.ts`:

```ts
'terminology.import.title': 'Import terminology',
'terminology.import.subtitle': 'Upload a CDISC terminology workbook',
'terminology.import.dropZone': 'Drop an .xls or .xlsx file here, or click to choose',
'terminology.import.fileTypeHint': 'Only .xls or .xlsx files are supported',
'terminology.import.importing': 'Importing terminology…',
'terminology.import.success': 'Imported terminology version {name}',
'terminology.import.failure': 'Import failed: {message}',

'common.submit': 'Submit',
```

`zhCN.ts` receives matching zh-CN translations (`'提交'` for `common.submit`, etc.). The existing `terminology.importComingSoon` key is removed. The `common.submit` key is **new** — it does not currently exist in either locale, so this PR adds it.

---

## 9. Error handling

- **Workbook parse failure** — `terminology::TerminologyError` is mapped to `ApiError::Parse { message: e.to_string() }` in the Rust command. The frontend renders it through `errorMessage(err)` (which already handles the unknown variant gracefully) and shows the failure Snackbar.
- **HTTP 409 duplicate version** — `ApiError::Http { status: 409, code: "duplicate", message }` from `version::create`. The codelist loop has not started, so the server is consistent. The Snackbar surfaces the conflict message and the user can pick a different file.
- **HTTP 4xx/5xx mid-loop** — already-created codelists and items remain in the database. The user can either delete the partial version manually (existing PATCH / DELETE endpoints) or retry with a corrected file. We do not attempt automatic rollback (would require either a server-side transaction endpoint or a separate DELETE call per entity, both out of scope).
- **Drop-zone invalid file** — the drop handler rejects non-`.xls` / `.xlsx` extensions; the drop zone briefly flashes `error.main` border and helper text, then resets. No state mutation, no toast.
- **Click-picker invalid selection** — the dialog plugin's `filters` option restricts the OS picker to `.xls` / `.xlsx`; the user cannot select anything else from the picker.
- **Empty codelist (parsed but no items)** — the loop skips `code_item::batch_create` entirely; no round-trip is wasted.

---

## 10. Out-of-scope / decisions deferred

1. **Cancel button** — out of scope. The user can leave the page via the back arrow while the import is running; the Tauri command continues server-side. Adding a cancellation primitive requires a future PR that registers the in-flight task on `app.manage(...)`.
2. **Custom server batch endpoint** — out of scope. The existing `batch_create_code_items` covers the per-codelist case efficiently. If a future workload needs sub-second imports for thousands of codelists, a new `POST /api/terminology/import` endpoint can replace the loop entirely.
3. **Custom version name UI** — out of scope. The parsed sheet date is used as `version.name`. Two imports of the same `(kind, date)` sheet fail with HTTP 409.
4. **Progress events** — out of scope. The user sees only "Importing terminology…". Granular progress (e.g. "Imported 230 / 1012 codelists") would require Tauri events; defer.
5. **Drag-drop visual polish** — the drop zone is functional but not styled beyond a dashed border. Final styling is implementation-level detail.

---

## 11. Testing

### 11.1 Vitest + RTL — `src/test/features/terminology/import-terminology-page.test.tsx`

Mock `@tauri-apps/api/core` and `@tauri-apps/plugin-dialog`. Cases:

1. Renders the empty form: back arrow, title, ButtonGroup with both options unselected, drop zone, disabled submit.
2. Pre-selects kind when `?kind=sdtm` is in the URL (`renderInRouter(<ImportTerminologyPage />, { initialEntries: ['/terminology/import?kind=sdtm'] })`).
3. Clicking the drop zone calls `open()` from the dialog plugin with the right filter; the resolved path is stored in component state.
4. Dropping a file whose name ends in `.pdf` flashes the error hint, leaves `filepath` empty, leaves the submit disabled.
5. Dropping a file whose name ends in `.xlsx` stores its `path` in state and shows the chip.
6. Submitting invokes `import_terminology` with `{ kind, filepath }`, hides the form, renders the spinner, then on success shows the success Snackbar and `queryKeys.terminology.versions()` is invalidated.
7. Submitting with `?kind=sdtm` in the URL pre-selects SDTM, but the user can switch to ADaM in the ButtonGroup before submitting; the submitted `kind` reflects the user's final choice.
8. Clicking the back arrow navigates to `/terminology/sdtm` when no kind is selected, or `/terminology/{kind}` otherwise.
9. On API failure (mock `import_terminology` to reject with `ApiError::Http { 409, ... }`), the spinner disappears, the form re-enables, and the error Snackbar is shown with `errorMessage(err)`.

### 11.2 Vitest + RTL — `src/test/features/terminology/import-button.test.tsx`

1. Clicking the button navigates to `/terminology/import?kind=sdtm` when `kind="sdtm"` is passed.
2. Same for `adam`.

### 11.3 Rust

No new unit tests in scope. The orchestration in `import.rs` is a thin wrapper over the existing `version::create`, `code_list::create`, and `code_item::batch_create` functions; each of those has wiremock coverage in the existing terminology HTTP wrapper tests. Adding an end-to-end Rust test for `import_terminology` requires a real `terminology::from_path` fixture and a mocked server; deferred.

---

## 12. Risks

| Risk | Mitigation |
| --- | --- |
| A large workbook (1000+ codelists) hits the reqwest 15-second timeout | The per-codelist HTTP round-trips are sequential; a worst-case SDTM workbook takes many minutes. The spinner is sufficient UX, but the timeout is a hard ceiling. Mitigation deferred to a future server batch endpoint. |
| Mid-import HTTP failure leaves orphaned codelists and items | Documented behavior — the user can delete the partial version manually. No automatic rollback. |
| User drops a non-xls file | Drop zone flashes the error hint and resets silently. Submit stays disabled. |
| User navigates back mid-import | Back arrow stays enabled. The mutation completes in the background; the success/failure Snackbar on the import page is not seen. The terminology versions list is still invalidated, so the new version appears on the destination page once the user returns. |
| Two imports of the same `(kind, date)` sheet | Server returns 409 on the version create; nothing else is created. The user sees a clear conflict message and can rename or skip. |
| The user is logged in as `general` | The page is reachable by every authenticated user, but the server's `require_admin_or_root` returns 403 on the version create. The Snackbar surfaces the 403. |