# CRF Form `approved` Field — Design

**Date:** 2026-09-23
**Scope:** Add a `approved: bool` (default `false`) flag to `CrfForm`, persisted end-to-end and exposed on the wire. A dedicated server endpoint toggles the flag; the **Tauri command** enforces the open-issues gate client-side (the crf usecase stays simple and does not compose mission service).

## Motivation

Today the CRF workflow has no concept of "this form has been reviewed and is approved". The table and detail page show a static `Pending` chip placeholder; the aggregate status chip is hard-coded. The QC role exists on the mission assignee side but has nothing to do at the form level. This change gives the QC a single, deliberate action — flip a form from `Pending` to `Approved` once all open issues on it are closed — and surfaces that state at three read-only places (the detail header, the table row, the version-aggregate chip).

## Shape of the change

The field is **not** exposed on `UpdateCrfFormRequest` / `UpdateCrfFormInput`. Toggling `approved` is the only operation gated on open-issues, so it gets:

1. A dedicated server endpoint `POST /api/crf/forms/{id}/approval` whose only job is to flip the flag (no gate logic on the server).
2. A dedicated Tauri command `set_crf_form_approved(id, approved, open_issue_count)` which holds the gate — it rejects `approved=true` when `open_issue_count > 0` before calling the server.

The crf usecase (`set_approved`) is intentionally simple: it loads the form and persists the new value. It does **not** compose mission / issue services, and it does **not** carry the open-issues count. That keeps the crf crate's port surface stable (no new cross-crate dependencies) and confines the gate to one Rust file in the desktop shell.

The existing `PATCH /forms/{id}` keeps its current scope (code, name, order, `not_submitted`).

## Data model & wire shape

`approved: bool` is added end-to-end, mirroring how `not_submitted` is wired today. Default is `false` at every layer that has one.

### DB

A new migration `0008_add_crf_forms_approved.sql` adds `approved BOOLEAN NOT NULL DEFAULT FALSE` to `crf_forms`. The existing `0002_create_crf_forms.sql` is **not** edited; the new migration keeps history linear. Idempotent (`ADD COLUMN IF NOT EXISTS`).

### Domain (`lib/crates/crf`)

- `CrfForm` gains `pub approved: bool`.
- `CrfForm::new` and `CrfForm::for_repository` take a new `approved: bool` arg.
- `CrfFormNew` (the create DTO) gains `pub approved: bool`.
- `CrfFormUpdate` (the partial-update DTO) **does not** gain a field — the partial-update path no longer touches `approved`.

### Usecase (`lib/crates/crf/src/usecase`)

- `CrfFormView` (the view DTO) gains `pub approved: bool`; `From<CrfForm>` populates it.
- `CreateCrfForm` (the create command) gains `pub approved: bool`.
- `CreateCrfBulkForm`'s `From` impl propagates `approved` from `cmd.form.approved` into `CrfFormNew.approved`.
- `UpdateCrfForm` (the update command) **does not** gain a field.
- New command `SetCrfApproved { id: i64, approved: bool }`.
- New method `CrfUsecase::set_approved(cmd) -> Result<CrfFormView, UsecaseError>`:
  1. Load the form (`DomainError::CrfFormNotFound(id)` if missing → 404).
  2. Call `form_repo.set_approved(id, approved)` and project to view.

No mission lookup, no issue count, no extra port methods.

### Persistence (`lib/crates/crf/src/adapter/persistence/postgres`)

- `CrfFormRow` gains `approved: bool`; the `From<CrfFormRow> for CrfForm` impl threads it.
- `CrfFormRepoPg::create` returns `approved` in its `RETURNING`; `find_by_id`, `list_by_version`, `search_by_version` `SELECT` it.
- `CrfFormRepoPg::update` (which still maps to `CrfFormUpdate`) is **unchanged** — does not touch `approved`. The `RETURNING *` clause naturally returns the new column too.
- New method `CrfFormRepoPg::set_approved(id, approved)` → `UPDATE crf_forms SET approved = $2 WHERE id = $1 RETURNING ...`; throws `DomainError::CrfFormNotFound(id)` if no row.
- `CrfBulkFormRepoPg::bulk_create` INSERT/RETURNING thread `approved`.

### Apis port (`lib/crates/apis/src/crf.rs`)

- `CrfFormView` gains `pub approved: bool`.
- `CreateCrfFormRequest.approved: bool` (required, same shape as `not_submitted` — clients must explicitly pass `false` on create).
- `UpdateCrfFormRequest` **does not** gain a field.
- New `SetCrfApprovedRequest { id: i64, approved: bool }`.
- New `CrfService::set_approved(req: SetCrfApprovedRequest) -> Result<CrfFormView, CrfApiError>` trait method.
- **No new `CrfApiError` variants.** The handler maps the existing `DomainError::CrfFormNotFound` to 404 — that's the only error the endpoint can produce.

### Facade (`lib/crates/crf/src/adapter/facade/in_memory/service.rs`)

- `From<usecase::CrfFormView> for ApiCrfFormView` includes `approved: f.approved`.
- `CrfServiceImpl::create_form` and `bulk_create_form` pass `approved` through.
- `CrfServiceImpl::update_form` is **unchanged** in shape — no `approved` plumbing.
- New `CrfServiceImpl::set_approved(req)` method — calls `usecase::set_approved`.

### Server wire (`apps/server/aegis-server/src/transport/http/dto.rs`)

- `CrfFormViewResponse.approved: bool` + `From` impl.
- `CreateCrfFormRequest.approved: bool`.
- `UpdateCrfFormRequest` **does not** gain a field.
- `BulkCreateCrfFormRequest` propagates `approved` (it re-uses `CreateCrfFormRequest`).
- New `SetCrfApprovedRequest { approved: bool }`.

### Server transport (`apps/server/aegis-server/src/transport/http/crf/handlers.rs`)

- `create_form` and `bulk_create_form` thread `approved` through.
- `update_form` is **unchanged** — no `approved` plumbing.
- New handler `set_approved` mounted at `POST /forms/{id}/approval`:
  - Body: `SetCrfApprovedRequest { approved: bool }`.
  - Response: `CrfFormViewResponse` (200).
  - 404 if the form does not exist.
  - No other error codes. The endpoint is a thin toggle.
- `router()` gains `.routes(routes!(handlers::set_approved))` under the CrfForm section.

### Authorization note

The existing CRF route handlers in this codebase do not yet enforce role-based authz at the handler layer (only `AuthClaims` is required). The `// TODO: reject or not base on the project role` comments throughout `crf/handlers.rs` mark this future work. For V1, `set_approved` accepts any authenticated caller; the QC + open-issues gate is enforced in the Tauri command. This matches the codebase's current convention.

### Tauri wire (`apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`)

- `CrfFormViewResponse.approved: bool`.
- `CreateCrfFormRequest.approved: bool`.
- `UpdateCrfFormRequest` **does not** gain a field.
- New `SetCrfApprovedRequest { approved: bool }`.
- New async fn `set_approved(c, id, body) -> Result<CrfFormViewResponse, ApiError>` — single server call, no gate logic. Returns the form unchanged-shape.
- Update the four wiremock-based unit tests so `form_view_json` includes `"approved": false`. The `update_request_skips_none_fields` test stays valid (the field doesn't exist).
- New wiremock test: `set_approved_hits_correct_path` mounts a `POST /api/crf/forms/{id}/approval` mock and asserts the response body is returned.

### Tauri commands (`apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`)

- `create_crf_form` body type unchanged but now receives `CreateCrfFormRequest` with `approved`.
- `update_crf_form` body type unchanged.
- New `#[tauri::command] set_crf_form_approved(client, id, approved: bool, open_issue_count: usize) -> Result<CrfFormViewResponse, ApiError>`:
  - If `approved && open_issue_count > 0`, return `Err(ApiError::Http { status: 409, code: "crf.open_issues_blocking_approval".into(), message: format!("{open_issue_count} open issue(s) blocking approval") })` — using the existing `ApiError::Http` struct variant (mirrors `{ kind: "http"; status; code; message }` in the TS wire).
  - Otherwise call `http::crf::form::set_approved(client, id, SetCrfApprovedRequest { approved }).await`.
  - The `open_issue_count` argument is a plain `usize` — the frontend already has it cached from `useListIssuesByMission`, so no extra network calls.

### TS types (`apps/desktop/aegis-desktop/src/shared/api/types.ts`)

- `CrfForm.approved: boolean`.
- `CreateCrfFormInput.approved: boolean` (required).
- `UpdateCrfFormInput` **does not** gain a field.

### Tests — Rust

- `lib/crates/crf/tests/public_api.rs` — the `CrfForm::new` and `CrfFormNew` literals gain `approved: false`. `CrfFormUpdate` fixtures unchanged.
- `lib/crates/crf/tests/integration_persistence.rs` — every `not_submitted: false` site gains a paired `approved: false` on the same `CrfFormNew` / `CrfForm` literal. Update assertions that compare view structs to include the `approved` field. New case for `set_approved` (toggle `false → true`, then `true → false`; 404 for unknown id).
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` — new unit test that wires a mock client: when `open_issue_count > 0 && approved == true`, the command returns the 409 `ApiError::Http` *without* calling the server (use a mock that asserts no call). When `open_issue_count == 0 || approved == false`, the mock receives exactly one `POST /api/crf/forms/{id}/approval` and the response is forwarded.

### Tests — TS

- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx` — fixture rows gain `approved: false`; expect "Pending" in the status column.
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` — new cases: chip renders "Approved" / "Pending" based on `form.approved`; chip is disabled for non-QC; chip is enabled for QC with `openIssueCount === 0`; chip is disabled for QC with `openIssueCount > 0` when current `approved === false`; chip position is immediately left of the tools-menu icon.
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx` — new cases for the aggregate chip: empty list → Pending; all-approved → Approved; mixed → Pending.

## UI changes

### `CrfDetailPage.tsx`

- New header element: an approval action chip-button, placed **immediately to the left of `<CrfToolsMenu />`** in the header toolbar (after the `<Box sx={{ flexGrow: 1 }} />` spacer, before the tools menu).
- The chip serves as **both visual indicator and toggle** — no separate read-only chip next to the form name.
- Renders one of two states based on `form.approved`:
  - `approved === true` → `<Chip color="success" icon={<VerifiedIcon />} label={t("crf.toolbar.statusApproved")} onClick={handleToggle} />` — label `"Approved"`.
  - `approved === false` → `<Chip color="warning" icon={<PendingActionsIcon />} label={t("crf.toolbar.statusPending")} onClick={handleToggle} />` — label `"Pending"`.
- `handleToggle` calls `setCrfFormApproved.mutate({ id, approved: !form.approved, openIssueCount })`, passing the cached `openIssueCount` from `useListIssuesByMission` so the Rust command can gate without an extra round trip.
- Gates (UI-side, visual feedback only — the authoritative gate is in the Tauri command):
  - Always disabled when `!isMissionQc`.
  - Disabled when `form.approved === false && openIssueCount > 0` (cannot approve while issues are open).
  - Always enabled for `form.approved === true` (un-approve has no issue gate).
  - Disabled while the mutation is in flight.
- Disabled visual: `onClick={undefined}` + `<Tooltip>` with the disable reason + `sx={{ opacity: 0.5, cursor: "not-allowed" }}`.
- Always rendered — non-QC users still see the status, they just can't click.
- `isMissionQc` and `openIssueCount` are already computed in `CrfDetailPage` (`isMissionQc` at line ~281, `openIssueCount` derived from `useListIssuesByMission`); no new queries needed.

### `CrfToolsMenu.tsx` — unchanged

The Global Search menu item is the only entry. No new props.

### `CrfFormTable.tsx` (row-level status chip)

The existing static `Pending` chip in the "Status" column becomes a read-only chip mirroring the form's `approved` field:
- `approved === true` → `<Chip color="success" icon={<VerifiedIcon />} label="Approved" size="small" variant="outlined" />`.
- `approved === false` → `<Chip color="warning" icon={<PendingActionsIcon />} label="Pending" size="small" variant="outlined" />`.

No `onClick`, no menu, no tooltip — purely visual.

### `CrfStatusChip.tsx` (aggregate chip next to the version dropdown)

Becomes derived rather than static:
- New prop `forms: CrfForm[]`.
- Pass `allRows` from the list page (so the aggregate reflects the version, not the user's filter).
- Computation: `const allApproved = forms.length > 0 && forms.every((f) => f.approved)`.
- Empty form list → "Pending" (warning).
- All approved → "Approved" (success).
- Mixed → "Pending" (warning).
- Removes the "status API is not ready" placeholder comment; replaces with a doc comment explaining the derivation.

### `CrfFormListPage.tsx`

- Pass `allRows` to `<CrfStatusChip forms={allRows} />`.
- No other changes.

### New data hook — `apps/desktop/aegis-desktop/src/features/crf/data/list.ts`

`useSetCrfFormApproved()` — TanStack mutation calling the new Tauri command. The hook signature accepts `openIssueCount: number` and passes it through. On success, invalidates `queryKeys.crf.formsByVersion(updated.versionId)` and `queryKeys.crf.form(updated.id)` — same pattern as `useUpdateCrfForm`.

### New shared API entry — `apps/desktop/aegis-desktop/src/shared/api/index.ts`

`api.setCrfFormApproved(id, approved, openIssueCount): Promise<CrfForm>` — wraps `invoke("set_crf_form_approved", { id, approved, openIssueCount })`.

### i18n strings (added to `en.ts` and `zhCN.ts`)

- `"crf.toolbar.statusApproved": "Approved"` (new).
- `"crf.toolbar.statusPending": "Pending"` (already exists).
- `"crf.toolbar.approveDisabled.notQc": "Only the QC assignee can approve this form."` (new).
- `"crf.toolbar.approveDisabled.openIssues": "Close all {{count}} open issue(s) before approving."` (new).

## Verification

### Rust

- `cargo check --workspace` — catches every site where the field was added.
- `cargo test -p crf` (no `--ignored`) — public-api compile fixture, in-memory facade tests, new `set_approved` persistence tests.
- `cargo test -p apis`, `cargo test -p aegis-server`, `cargo test -p aegis-desktop --lib` — wire DTO round-trips and Tauri wiremock tests (including the new `set_approved_hits_correct_path` and the gate-by-`open_issue_count` command test).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo fmt --all -- --check`.
- Live-DB integration (deliberate, not part of normal verification):
  - `cargo test -p crf -- --ignored --test-threads=1`
  - `cargo test -p aegis-server -- --ignored --test-threads=1`

### TS

- `pnpm --filter aegis-desktop typecheck`.
- `pnpm --filter aegis-desktop test` — covers all three updated test files.
- `pnpm --filter aegis-desktop build` (tsc + vite build).
- `pnpm --filter @aegis/ui typecheck` and `pnpm --filter @aegis-ui test` — i18n key additions.

### End-to-end manual smoke

1. Start desktop app + server with a seeded project.
2. Open a CRF version; confirm every row in the table shows "Pending" (warning); confirm the aggregate chip next to the version dropdown also shows "Pending".
3. Open a form's detail page; confirm the chip-toggle to the left of the tools-menu icon shows "Pending" (warning).
4. Create an issue on the form. Re-open the detail; confirm the chip is disabled with the open-issues tooltip.
5. Close the issue. Confirm the chip becomes enabled.
6. Click to approve; confirm: chip flips to "Approved" (success); the corresponding table row flips to "Approved" (success); the aggregate chip flips to "Approved" when all forms are approved; the request hit `POST /api/crf/forms/{id}/approval` (visible in devtools).
7. As a non-QC user (no QC role on the form's mission), confirm the chip is rendered but disabled with the not-QC tooltip.

## Out of scope

- Handler-level role-based authz (matches the codebase's current "AuthClaims-only" convention; `// TODO: reject or not base on the project role` markers exist throughout the existing handlers).
- Server-side enforcement of the open-issues gate (gate lives in the Tauri command by design — see "Shape of the change").
- Composing mission service / issue service into the crf usecase.
- Allowing project leaders to override the QC gate.
- Allowing un-approve to also require zero open issues.
- Auto-approving forms whose mission has been deleted.
- Filter-drawer status filtering by approval state (the existing `CrfStatusFilter` is held for future use; no change here).