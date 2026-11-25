# CrfForm `approved` Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `approved: bool` flag to `CrfForm` (default false), persist it end-to-end, and let QC toggle it from `CrfDetailPage` via a chip-toggle gated on zero open issues (gated in the Tauri command by re-fetching the live open-issue count from the server).

**Architecture:** Vertical slice from DB → lib crate `crf` (domain / usecase / facade) → lib crate `apis` → server wire + handler → Tauri wire + command → TS types + shared API + mutation hook → UI (chip-toggle + row chip + aggregate chip + drawer) → i18n → tests. The open-issues gate lives in the Tauri command (re-fetches via `http::mission::list_issues_by_mission`), not in the crf usecase or server endpoint. The server endpoint is a thin toggle (only error: `CrfFormNotFound` → 404).

**Tech Stack:** Rust 2024 (sqlx, axum, utoipa-axum, async-trait, chrono), Tauri 2, TanStack Query, React, MUI, vitest + @testing-library/react, wiremock (Rust), vitest (TS).

**Spec:** `docs/superpowers/specs/2026-09-23-crf-form-approved-field-design.md`

---

## File map

**Created**
- `lib/crates/crf/migrations/0008_add_crf_forms_approved.sql`

**Modified — Rust (lib)**
- `lib/crates/crf/src/domain/crf_form.rs`
- `lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs`
- `lib/crates/crf/src/adapter/persistence/postgres/crf_bulk_form_repo.rs`
- `lib/crates/crf/src/usecase/views.rs`
- `lib/crates/crf/src/usecase/commands.rs`
- `lib/crates/crf/src/usecase/crf_usecase.rs`
- `lib/crates/crf/src/usecase/mod.rs` *(re-export new `SetCrfApproved`)*
- `lib/crates/crf/src/adapter/facade/in_memory/service.rs`
- `lib/crates/apis/src/crf.rs`
- `lib/crates/crf/tests/public_api.rs`
- `lib/crates/crf/tests/integration_persistence.rs`

**Modified — Rust (server + desktop shell)**
- `apps/server/aegis-server/src/transport/http/dto.rs`
- `apps/server/aegis-server/src/transport/http/crf/handlers.rs`
- `apps/server/aegis-server/src/transport/http/crf/router.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`

**Modified — TS**
- `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- `apps/desktop/aegis-desktop/src/features/crf/data/list.ts`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfStatusChip.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx`
- `lib/packages/ui/src/i18n/locales/en.ts`
- `lib/packages/ui/src/i18n/locales/zhCN.ts`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

---

## Task 1: DB migration

**Files:**
- Create: `lib/crates/crf/migrations/0008_add_crf_forms_approved.sql`

- [ ] **Step 1: Create the migration file**

Write the following to `lib/crates/crf/migrations/0008_add_crf_forms_approved.sql`:

```sql
-- Adds the `approved` flag to crf_forms. Defaults to FALSE so every
-- pre-existing row counts as not-yet-approved. Idempotent so re-runs
-- on a partially-migrated database are safe.
ALTER TABLE crf_forms
    ADD COLUMN IF NOT EXISTS approved BOOLEAN NOT NULL DEFAULT FALSE;
```

- [ ] **Step 2: Verify the file is the next migration (idempotent check)**

Run: `ls lib/crates/crf/migrations/`
Expected: existing 0001…0007 + new `0008_add_crf_forms_approved.sql`. No gaps.

- [ ] **Step 3: Apply the migration to the dev DB (manual, not auto)**

The server does not auto-migrate. Run:
```bash
cd /Users/yukichen/Coding/Projects/aegis
export AEGIS_CRF_DATABASE_URL=postgres://user:pass@localhost/aegis_crf
sqlx migrate run --source lib/crates/crf/migrations
```
Expected: `0008_add_crf_forms_approved.sql` applied. (Skip if the dev DB isn't running locally — the live-DB integration tests in Task 20 will apply it on their own run.)

- [ ] **Step 4: Commit**

```bash
git add lib/crates/crf/migrations/0008_add_crf_forms_approved.sql
git commit -m "feat(crf): add approved column migration to crf_forms"
```

---

## Task 2: Domain — `CrfForm` gains `approved`

**Files:**
- Modify: `lib/crates/crf/src/domain/crf_form.rs`

- [ ] **Step 1: Add `approved` to the `CrfForm` struct**

In `lib/crates/crf/src/domain/crf_form.rs`, add `pub approved: bool,` after `pub not_submitted: bool,` (around line 16). Update the `Debug` impl to also `.field("approved", &self.approved)`.

- [ ] **Step 2: Add `approved` to `CrfForm::new` and `for_repository`**

Change the signatures and bodies to thread `approved: bool` as the new arg (placed after `not_submitted` in both, to match the field order). Example:

```rust
pub fn new(
    version_id: i64,
    code: String,
    name: String,
    order: i32,
    not_submitted: bool,
    approved: bool,
) -> Result<Self, DomainError> {
    if code.trim().is_empty() {
        return Err(DomainError::EmptyCode);
    }
    if name.trim().is_empty() {
        return Err(DomainError::EmptyName);
    }
    Ok(Self {
        id: 0,
        version_id,
        code,
        name,
        order,
        not_submitted,
        approved,
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    })
}
```

And:

```rust
#[allow(clippy::too_many_arguments)]
pub(crate) fn for_repository(
    id: i64,
    version_id: i64,
    code: String,
    name: String,
    order: i32,
    not_submitted: bool,
    approved: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Self {
    Self {
        id,
        version_id,
        code,
        name,
        order,
        not_submitted,
        approved,
        created_at,
        updated_at,
    }
}
```

- [ ] **Step 3: Add `approved` to `CrfFormNew`**

Change `CrfFormNew` (around line 92) to:

```rust
#[derive(Debug, Clone)]
pub struct CrfFormNew {
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub approved: bool,
}
```

`CrfFormUpdate` stays unchanged (the partial-update path does not touch `approved` — see design §"Domain").

- [ ] **Step 4: Compile-check**

Run: `cargo check -p crf`
Expected: FAILURE with "this function takes 7 arguments but 6 were supplied" or "missing field `approved`" — every caller of `CrfForm::new`, `CrfForm::for_repository`, and `CrfFormNew` literal now needs an `approved` arg. These callers will be fixed in subsequent tasks.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/domain/crf_form.rs
git commit -m "feat(crf): add approved field to CrfForm aggregate"
```

---

## Task 3: Persistence — thread `approved` through `CrfFormRepoPg`

**Files:**
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs`
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/crf_bulk_form_repo.rs`

- [ ] **Step 1: Update the bulk repo's `CrfFormRow` and From impl**

In `lib/crates/crf/src/adapter/persistence/postgres/crf_bulk_form_repo.rs`:

- Add `approved: bool,` to `struct CrfFormRow` (around line 17).
- Update `From<CrfFormRow> for CrfForm` to thread it: `CrfForm::for_repository(r.id, r.version_id, r.code, r.name, r.order, r.not_submitted, r.approved, r.created_at, r.updated_at)`.
- Update the bulk insert's INSERT and RETURNING SQL (around line 96) to:

```rust
"INSERT INTO crf_forms (version_id, code, name, \"order\", not_submitted, approved)
 VALUES ($1, $2, $3, $4, $5, $6)
 RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
```

Add `.bind(input.form.approved)` to the bind chain.

- [ ] **Step 2: Update the single-form repo's `CrfFormRow` and From impl**

In `lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs`:

- Add `approved: bool,` to `struct CrfFormRow` (around line 8).
- Update `From<CrfFormRow> for CrfForm` to thread it.
- Update `create`'s INSERT and RETURNING SQL:

```rust
"INSERT INTO crf_forms (version_id, code, name, \"order\", not_submitted, approved)
 VALUES ($1, $2, $3, $4, $5, $6)
 RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
```

Add `.bind(input.approved)` to the bind chain.

- [ ] **Step 3: Update SELECT statements**

For `find_by_id`, `list_by_version`, and `search_by_version`, change `SELECT id, version_id, code, name, \"order\", not_submitted, created_at, updated_at` to `SELECT id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at`. `update`'s RETURNING clause also gains `approved`.

- [ ] **Step 4: Compile-check**

Run: `cargo check -p crf`
Expected: still FAILURE — `update_form`'s `CrfFormUpdate` binding (`bind(input.not_submitted)`) needs the order check, but the file compiles structurally. The remaining failures are from `usecase::create_form` / `update_form` and `CrfServiceImpl::create_form` / `bulk_create_form` not yet threading `approved`.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs lib/crates/crf/src/adapter/persistence/postgres/crf_bulk_form_repo.rs
git commit -m "feat(crf): thread approved through form and bulk-form repos"
```

---

## Task 4: Persistence — new `CrfFormRepoPg::set_approved` + integration test

**Files:**
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs`
- Modify: `lib/crates/crf/tests/integration_persistence.rs`

- [ ] **Step 1: Add `set_approved` to the `CrfFormRepository` port**

In `lib/crates/crf/src/domain/crf_form.rs`, add to the `CrfFormRepository` trait (around line 113):

```rust
async fn set_approved(
    &self,
    id: i64,
    approved: bool,
) -> Result<CrfForm, DomainError>;
```

- [ ] **Step 2: Implement `set_approved` on `CrfFormRepoPg`**

In `lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs`, add inside the `impl CrfFormRepository for CrfFormRepoPg` block (after `delete`):

```rust
async fn set_approved(
    &self,
    id: i64,
    approved: bool,
) -> Result<CrfForm, DomainError> {
    let row: CrfFormRow = sqlx::query_as::<_, CrfFormRow>(
        "UPDATE crf_forms SET approved = $2
         WHERE id = $1
         RETURNING id, version_id, code, name, \"order\", not_submitted, approved, created_at, updated_at",
    )
    .bind(id)
    .bind(approved)
    .fetch_optional(&self.pool)
    .await
    .map_err(map_db_err)?
    .ok_or(DomainError::CrfFormNotFound(id))?;
    Ok(row.into())
}
```

- [ ] **Step 3: Add a failing integration test**

In `lib/crates/crf/tests/integration_persistence.rs`, add a new `#[tokio::test] #[ignore]` (matching the existing pattern around the live-DB tests):

```rust
#[tokio::test]
#[ignore]
async fn set_approved_toggles_flag_and_404s_on_unknown_id() {
    // Boot a live DB via .env / AEGIS_CRF_DATABASE_URL the same way the
    // existing ignored tests do. Insert a form, toggle approved true
    // via repo.set_approved, assert flag is true; toggle back; assert
    // false; call set_approved on a non-existent id and assert
    // CrfFormNotFound(0). Mirror the wiring the other ignored tests
    // already use.
    todo!("see existing #[ignore] tests in this file for wiring")
}
```

- [ ] **Step 4: Run the test (will fail with `todo!()` panic)**

Run: `cargo test -p crf -- --ignored --test-threads=1 set_approved_toggles_flag_and_404s_on_unknown_id`
Expected: panic at `todo!()`. Confirms the test is wired and will be filled in during execution.

- [ ] **Step 5: Replace the `todo!()` with the actual body**

Implement the body by copying the wiring from any nearby `#[ignore]` test in the same file (look for an existing `set_up_pool()` or pattern), insert a `CrfFormNew { ..., approved: false }`, call `repo.create(...)`, then `repo.set_approved(form.id, true)` and assert the returned form has `approved == true`, then `repo.set_approved(form.id, false)` and assert `approved == false`. Then `repo.set_approved(0, true)` and assert `Err(DomainError::CrfFormNotFound(0))`.

- [ ] **Step 6: Run the test against the dev DB**

Run: `cargo test -p crf -- --ignored --test-threads=1 set_approved_toggles_flag_and_404s_on_unknown_id`
Expected: PASS. Live-DB tests are destructive (drop tables) — see CLAUDE.md.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/crf/src/adapter/persistence/postgres/crf_form_repo.rs lib/crates/crf/tests/integration_persistence.rs
git commit -m "feat(crf): persistence port method set_approved"
```

---

## Task 5: Usecase — `CrfFormView`, `SetCrfApproved`, `set_approved`

**Files:**
- Modify: `lib/crates/crf/src/usecase/views.rs`
- Modify: `lib/crates/crf/src/usecase/commands.rs`
- Modify: `lib/crates/crf/src/usecase/crf_usecase.rs`
- Modify: `lib/crates/crf/src/usecase/mod.rs` *(re-export)*

- [ ] **Step 1: Add `approved` to `CrfFormView`**

In `lib/crates/crf/src/usecase/views.rs`, add `pub approved: bool,` to `CrfFormView` (around line 34) and the matching field in `impl From<CrfForm> for CrfFormView` (around line 45).

- [ ] **Step 2: Add `approved` to `CreateCrfForm`**

In `lib/crates/crf/src/usecase/commands.rs`, change `CreateCrfForm` (around line 26) to include `pub approved: bool,` after `not_submitted`. Update the `From<CreateCrfBulkForm> for DomainCrfBulkCreateForm` impl (around line 185) to thread `approved: cmd.form.approved` into the `CrfFormNew` literal. `UpdateCrfForm` stays unchanged.

- [ ] **Step 3: Add `SetCrfApproved` command**

In `lib/crates/crf/src/usecase/commands.rs`, add after `UpdateCrfForm`:

```rust
/// Toggle the `approved` flag on a CRF form. The gate (zero open
/// issues) lives in the Tauri command, not here — this usecase is
/// intentionally simple and does not compose mission / issue
/// services.
pub struct SetCrfApproved {
    pub id: i64,
    pub approved: bool,
}
```

- [ ] **Step 4: Add `set_approved` to `CrfUsecase`**

In `lib/crates/crf/src/usecase/crf_usecase.rs`, add after `update_form` (around line 232):

```rust
pub async fn set_approved(
    &self,
    cmd: SetCrfApproved,
) -> Result<CrfFormView, UsecaseError> {
    let f = self.form_repo.set_approved(cmd.id, cmd.approved).await?;
    Ok(f.into())
}
```

- [ ] **Step 5: Thread `approved` through `create_form` and `update_form`**

In `lib/crates/crf/src/usecase/crf_usecase.rs`:

- `create_form` (line 187): in the `CrfFormNew { ... }` literal, add `approved: cmd.approved,` (and add the field to `CreateCrfForm` already done in Step 2 — if `cmd` doesn't have `approved`, the compile failure here will surface it).
- `update_form` (line 219): unchanged — `CrfFormUpdate` doesn't carry `approved`.

- [ ] **Step 6: Re-export `SetCrfApproved`**

In `lib/crates/crf/src/usecase/mod.rs`, add `SetCrfApproved` to the `pub use commands::` line alongside `CreateCrfForm`, `UpdateCrfForm`, etc.

- [ ] **Step 7: Compile-check**

Run: `cargo check -p crf`
Expected: still FAILURE — `apis::crf::CrfFormView`, `CrfServiceImpl::create_form`, `bulk_create_form` need updating next. Persistence side and usecase side are now in sync.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/crf/src/usecase/
git commit -m "feat(crf): usecase approved threading and set_approved"
```

---

## Task 6: Facade — thread `approved` through `CrfServiceImpl`

**Files:**
- Modify: `lib/crates/crf/src/adapter/facade/in_memory/service.rs`

- [ ] **Step 1: Update `From<usecase::CrfFormView> for ApiCrfFormView`**

Around line 704, add `approved: f.approved,` to the `Self { ... }` literal.

- [ ] **Step 2: Update `create_form`**

Around line 161, change the inner `crate::usecase::CreateCrfForm { ... }` literal to include `approved: req.approved,` (and add `approved: bool` to `apis::crf::CreateCrfFormRequest` — see Task 7).

- [ ] **Step 3: Update `bulk_create_form`**

Around line 175, in the `crate::usecase::CreateCrfForm { ... }` literal (inside the bulk mapping), add `approved: req.form.approved,`.

- [ ] **Step 4: `update_form` is unchanged**

`UpdateCrfFormRequest` does NOT gain `approved`. Leave the body untouched.

- [ ] **Step 5: Add `set_approved` to `CrfServiceImpl`**

After `update_form` (around line 274), add:

```rust
async fn set_approved(
    &self,
    req: apis::crf::SetCrfApprovedRequest,
) -> Result<ApiCrfFormView, CrfApiError> {
    self.usecase
        .set_approved(crate::usecase::SetCrfApproved {
            id: req.id,
            approved: req.approved,
        })
        .await
        .map(Into::into)
        .map_err(map_error)
}
```

(The `apis::crf::SetCrfApprovedRequest` struct + trait method are added in Task 7. This will not compile until then — that's fine; we wire it before compile-checking in Task 7.)

- [ ] **Step 6: Commit (do not compile-check yet)**

```bash
git add lib/crates/crf/src/adapter/facade/in_memory/service.rs
git commit -m "feat(crf): facade threading and set_approved"
```

---

## Task 7: Apis port — DTOs + trait method

**Files:**
- Modify: `lib/crates/apis/src/crf.rs`

- [ ] **Step 1: Add `approved` to `apis::crf::CrfFormView`**

Around line 58, add `pub approved: bool,` to `CrfFormView`.

- [ ] **Step 2: Add `approved` to `CreateCrfFormRequest`**

Around line 146, change `CreateCrfFormRequest` to include `pub approved: bool,`. `UpdateCrfFormRequest` (around line 225) stays unchanged.

- [ ] **Step 3: Add `SetCrfApprovedRequest` and the trait method**

After `UpdateCrfFormRequest` (around line 231), add:

```rust
/// Input DTO for [`CrfService::set_approved`]. The gate (zero open
/// issues) is enforced by the Tauri command, not by this port —
/// `set_approved` is a thin toggle on the server.
#[derive(Debug, Clone)]
pub struct SetCrfApprovedRequest {
    pub id: i64,
    pub approved: bool,
}
```

Then add to the `CrfService` trait (after `update_form`, around line 589):

```rust
/// Toggle the `approved` flag on the form identified by `req.id`.
/// Thin server-side toggle — no gate logic. Returns
/// `CrfApiError::CrfFormNotFound(req.id)` if the form does not
/// exist. The Tauri command re-fetches the open-issue count and
/// rejects before calling this endpoint, so an "approve with open
/// issues" call should never reach the server.
async fn set_approved(
    &self,
    req: SetCrfApprovedRequest,
) -> Result<CrfFormView, CrfApiError>;
```

- [ ] **Step 4: Compile-check the workspace**

Run: `cargo check --workspace`
Expected: SUCCESS. After this task, the entire lib crate surface (domain → usecase → facade → apis) compiles cleanly with `approved`.

- [ ] **Step 5: Run crf crate unit + facade tests**

Run: `cargo test -p crf`
Expected: PASS (the existing tests don't touch the new field — the fixtures still use the *old* `CrfForm::new` arity in Task 4's test, which we'll fix in Task 20; but the test we wrote in Task 4 with the new arity already exercises the new field).

If the existing public_api test in `lib/crates/crf/tests/public_api.rs` references `CrfFormNew` literals, those will be fixed in Task 20. Compilation success is enough here.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/apis/src/crf.rs
git commit -m "feat(crf): apis port threading and set_approved trait method"
```

---

## Task 8: Server wire DTOs + handler + router

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/crf/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/crf/router.rs`

- [ ] **Step 1: Update `CrfFormViewResponse` in server wire DTO**

In `dto.rs` (around line 1572), add `pub approved: bool,` to the struct and to the `From<apis::crf::CrfFormView> for CrfFormViewResponse` impl.

- [ ] **Step 2: Update `CreateCrfFormRequest`**

Around line 1792, add `pub approved: bool,` to the struct.

- [ ] **Step 3: Add `SetCrfApprovedRequest`**

After `UpdateCrfFormRequest` (around line 1931), add:

```rust
/// Body for `POST /api/crf/forms/{id}/approval`. The id is in
/// the path; the body carries only the desired flag value.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetCrfApprovedRequest {
    pub approved: bool,
}
```

- [ ] **Step 4: Update `create_form` and `bulk_create_form` handlers**

In `handlers.rs`, in the `create_form` handler (around line 193), add `approved: req.approved,` to the inner `apis::crf::CreateCrfFormRequest { ... }` literal. In `bulk_create_form` (around line 236), add `approved: req.form.approved,` to the inner `apis::crf::CreateCrfFormRequest { ... }` literal inside the bulk mapping.

- [ ] **Step 5: Add `set_approved` handler**

After `update_form` in `handlers.rs`, add:

```rust
/// `POST /api/crf/forms/{id}/approval` — toggle the `approved`
/// flag on the form. Thin endpoint: the only error is
/// `CrfFormNotFound` → 404. The open-issues gate lives in the
/// Tauri command (which re-fetches the live open-issue count and
/// rejects before reaching this endpoint).
#[utoipa::path(
    post, path = "/forms/{id}/approval", tag = "crf",
    operation_id = "crf_set_form_approved",
    params(
        ("id" = i64, Path, description = "Form id"),
    ),
    request_body = dto::SetCrfApprovedRequest,
    responses(
        (status = 200, description = "Form approval toggled", body = dto::CrfFormViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF form not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn set_approved(
    State(state): State<AppState>,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::SetCrfApprovedRequest>,
) -> Result<Json<dto::CrfFormViewResponse>, ApiError> {
    // TODO: reject or not base on the project role
    let view = state
        .crf
        .set_approved(apis::crf::SetCrfApprovedRequest { id, approved: req.approved })
        .await?;
    Ok(Json(view.into()))
}
```

(Use the `Path<CrfPathId>` extractor the same way `update_form` does around line 429. Match the `CrfPathId` pattern from `create_form`.)

- [ ] **Step 6: Register the route**

In `router.rs`, add `.routes(routes!(handlers::set_approved))` in the `// ---- CrfForm ----` section, after `.routes(routes!(handlers::delete_form))`. Update the URL-map doc comment at the top of the file (around line 25) to add `POST /forms/{id}/approval set_approved`.

- [ ] **Step 7: Compile-check + tests**

Run: `cargo check --workspace && cargo test -p aegis-server --lib`
Expected: PASS. (The server's existing tests don't reference `approved` on form types directly, but the wire DTOs are exercised through round-trip serde.)

- [ ] **Step 8: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs apps/server/aegis-server/src/transport/http/crf/handlers.rs apps/server/aegis-server/src/transport/http/crf/router.rs
git commit -m "feat(server): crf set_approved endpoint and approved threading"
```

---

## Task 9: Tauri wire — DTOs + http fn + wiremock tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`

- [ ] **Step 1: Add `approved` to `CrfFormViewResponse`**

Around line 10, add `pub approved: bool,` to the struct.

- [ ] **Step 2: Add `approved` to `CreateCrfFormRequest`**

Around line 31, add `pub approved: bool,` to the struct. `UpdateCrfFormRequest` (around line 102) stays unchanged.

- [ ] **Step 3: Add `SetCrfApprovedRequest` and the http fn**

After `update` (around line 153), add:

```rust
/// Body for `POST /api/crf/forms/{id}/approval`. The id is in
/// the path; the body carries only the desired flag value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCrfApprovedRequest {
    pub approved: bool,
}

pub async fn set_approved(
    c: &HttpClient,
    id: i64,
    body: SetCrfApprovedRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/forms/{id}/approval"),
        Some(&body),
    )
    .await
}
```

- [ ] **Step 4: Update the wiremock test fixture**

In the `tests` module (around line 353), the helper `form_view_json` returns a JSON value. Add `"approved": false,` to the literal (after `"notSubmitted": false`). The four tests that use this helper (`list_by_version_returns_forms`, `create_returns_view`, `get_by_id_returns_view`, `details_returns_composed_view`, `search_by_version_with_fragment_includes_query_param`) will all round-trip the new field automatically.

- [ ] **Step 5: Add a new wiremock test for `set_approved`**

At the end of the `tests` module:

```rust
#[tokio::test]
async fn set_approved_hits_correct_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/crf/forms/11/approval"))
        .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(
            11, 7, "AE", "Adverse Events",
        )))
        .mount(&server)
        .await;
    let f = set_approved(
        &client(&server),
        11,
        SetCrfApprovedRequest { approved: true },
    )
    .await
    .unwrap();
    assert_eq!(f.id, 11);
    assert!(f.approved);
}
```

- [ ] **Step 6: Run the wiremock tests**

Run: `cargo test -p aegis-desktop --lib http::crf::form`
Expected: PASS. All five tests green.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs
git commit -m "feat(desktop): tauri wire set_approved http fn"
```

---

## Task 10: Tauri command — `set_crf_form_approved` with fresh-issue-fetch gate

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`

This is the core gate logic. TDD applies: write tests first.

- [ ] **Step 1: Add a failing test for the gate trip (open issues present)**

The codebase convention for Tauri-command tests is to **call the underlying http fn directly** via `HttpClient::new(server.uri(), store)` + a `MemoryStore` of tokens (see `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs::current_user_resolves_sub_to_user_view`, lines 78-106). The `tauri::State` extractor is not involved in tests — the gate logic is. To make that work, extract the gate into a testable inner function `set_approved_impl` and have the `#[tauri::command]` shim delegate to it.

Add a `#[cfg(test)] mod tests` block at the bottom of the file. The helper builds a `MemoryStore` + `HttpClient` against a `wiremock::MockServer`, then calls `set_approved_impl` directly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::{HttpClient, MemoryStore, TokenStore};
    use crate::http::dto::ApiError;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        HttpClient::new(server.uri(), store)
    }

    fn form_view_json(id: i64, approved: bool) -> serde_json::Value {
        serde_json::json!({
            "id": id, "versionId": 7, "code": "AE", "name": "Adverse Events",
            "order": 0, "notSubmitted": false, "approved": approved,
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    fn open_issue(id: i64, mission_id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id, "missionId": mission_id,
            "issuer": "alice", "description": "needs review",
            "state": "opened", "comments": [],
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn gate_trips_when_open_issues_exist() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/mission/by-mission/42/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [open_issue(1, 42)]
            })))
            .mount(&server)
            .await;
        // Approval POST mounted but should NEVER be hit (strict: zero calls allowed).
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/approval"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, true)))
            .expect(0)
            .mount(&server)
            .await;

        let err = set_approved_impl(&client(&server).await, 11, true, 42)
            .await
            .unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 409);
                assert_eq!(code, "crf.open_issues_blocking_approval");
            }
            other => panic!("expected ApiError::Http 409, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test -p aegis-desktop --lib commands::crf::form`
Expected: FAIL — `set_crf_form_approved` doesn't exist yet.

- [ ] **Step 3: Implement `set_crf_form_approved` (with extracted `set_approved_impl`)**

At the top of the file, update the imports:

```rust
use crate::http::crf::form::{
    self, CreateCrfFormRequest, CrfFormDetailResponse, CrfFormListResponse, CrfFormViewResponse,
    SetCrfApprovedRequest, UpdateCrfFormRequest,
};
use crate::http::mission::{self, IssueState};
```

Add the new command **and its extracted impl** after `get_crf_form_details`:

```rust
/// Toggle the `approved` flag on a CRF form. Re-fetches the current
/// open issues for the form's mission from the server (the
/// TanStack-Query-cached count is intentionally NOT trusted) and
/// rejects `approved=true` when at least one open issue exists.
/// Returns 409 `ApiError::Http { status: 409, code:
/// "crf.open_issues_blocking_approval", message }` in that case;
/// the server approve endpoint is **not** called.
///
/// `approved=false` (un-approve) bypasses the issue fetch entirely
/// — un-approving has no gate.
#[tauri::command]
pub async fn set_crf_form_approved(
    client: State<'_, HttpClient>,
    id: i64,
    approved: bool,
    mission_id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    set_approved_impl(&client, id, approved, mission_id).await
}

/// Gate + proxy implementation. `#[tauri::command]` shims over
/// `tauri::State`, which is hard to construct in tests; the real
/// logic lives here so tests can drive it directly with a real
/// `HttpClient` against `wiremock`. Mirrors the codebase's
/// convention in `commands/user.rs::current_user_resolves_sub_to_user_view`.
pub async fn set_approved_impl(
    client: &HttpClient,
    id: i64,
    approved: bool,
    mission_id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    if approved {
        let issues = mission::list_issues_by_mission(
            client,
            mission_id,
            Some(IssueState::Opened),
        )
        .await?;
        if !issues.is_empty() {
            return Err(ApiError::Http {
                status: 409,
                code: "crf.open_issues_blocking_approval".into(),
                message: format!(
                    "{} open issue(s) blocking approval",
                    issues.len()
                ),
            });
        }
    }
    form::set_approved(client, id, SetCrfApprovedRequest { approved }).await
}
```

(`mission::list_issues_by_mission` signature is `pub async fn list_issues_by_mission(c: &HttpClient, mission_id: i64, state: Option<IssueState>) -> Result<Vec<IssueViewResponse>, ApiError>` — verified in `apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs:216-234`. The body unwraps the inner `.issues` for callers.)

- [ ] **Step 4: Implement the remaining two failing tests**

```rust
#[tokio::test]
async fn gate_passes_when_no_open_issues() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/mission/by-mission/42/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": []
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/crf/forms/11/approval"))
        .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, true)))
        .expect(1)
        .mount(&server)
        .await;
    let f = set_approved_impl(&client(&server).await, 11, true, 42).await.unwrap();
    assert_eq!(f.id, 11);
    assert!(f.approved);
}

#[tokio::test]
async fn unapprove_skips_issue_fetch() {
    let server = MockServer::start().await;
    // Issues endpoint mounted but MUST NOT be hit on un-approve.
    Mock::given(method("GET"))
        .and(path("/api/mission/by-mission/42/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": [open_issue(1, 42)]
        })))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/crf/forms/11/approval"))
        .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, false)))
        .expect(1)
        .mount(&server)
        .await;
    let f = set_approved_impl(&client(&server).await, 11, false, 42).await.unwrap();
    assert!(!f.approved);
}
```

- [ ] **Step 5: Run all three tests**

Run: `cargo test -p aegis-desktop --lib commands::crf::form`
Expected: all 3 PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs
git commit -m "feat(desktop): tauri command set_crf_form_approved with gate"
```

---

## Task 11: TS types + shared API + mutation hook

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/crf/data/list.ts`

- [ ] **Step 1: Add `approved` to `CrfForm`, `CreateCrfFormInput`**

In `types.ts` (around line 442), add `approved: boolean;` to the `CrfForm` interface after `notSubmitted`. Around line 473, add `approved: boolean;` to `CreateCrfFormInput`. `UpdateCrfFormInput` (around line 480) stays unchanged.

- [ ] **Step 2: Add `api.setCrfFormApproved`**

In `index.ts`, in the `// CRF` section (after `deleteCrfForm`, around line 326), add:

```ts
setCrfFormApproved: (
  id: number,
  approved: boolean,
  missionId: number,
): Promise<CrfForm> =>
  call<CrfForm>("set_crf_form_approved", { id, approved, missionId }),
```

- [ ] **Step 3: Add `useSetCrfFormApproved` hook**

In `features/crf/data/list.ts`, at the bottom of the file, add:

```ts
export function useSetCrfFormApproved() {
  const qc = useQueryClient();
  return useMutation<
    CrfForm,
    ApiError,
    { id: number; approved: boolean; missionId: number }
  >({
    mutationFn: ({ id, approved, missionId }) =>
      api.setCrfFormApproved(id, approved, missionId),
    onSuccess: (updated) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(updated.versionId),
      });
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.form(updated.id),
      });
    },
  });
}
```

- [ ] **Step 4: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts apps/desktop/aegis-desktop/src/shared/api/index.ts apps/desktop/aegis-desktop/src/features/crf/data/list.ts
git commit -m "feat(desktop): ts types, shared api, mutation hook for approved"
```

---

## Task 12: CrfFormDrawer — add `approved: false` to onCreate body

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx`

- [ ] **Step 1: Add `approved: false` to the `onCreate` body**

In `CrfFormDrawer.tsx` (around line 72), change:

```tsx
onCreate({
  code: code.trim(),
  name: name.trim(),
  order: 0,
  notSubmitted: false,
  approved: false, // new forms are not pre-approved
});
```

- [ ] **Step 2: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx
git commit -m "feat(desktop): drawer sets approved false on create"
```

---

## Task 13: CrfStatusChip — derived from forms prop

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfStatusChip.tsx`

- [ ] **Step 1: Rewrite as derived**

Replace the entire file with:

```tsx
import { Chip } from "@aegis/ui/mui";
import {
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { CrfForm } from "../../../shared/api";

/**
 * Aggregate approval chip for a CRF version. Derives its label
 * and color from the supplied forms:
 *   - empty list          → "Pending" (warning)
 *   - all approved        → "Approved" (success)
 *   - mixed / any pending → "Pending" (warning)
 *
 * The list page passes the unfiltered `allRows` so the chip
 * reflects the version, not the user's filter.
 */
export function CrfStatusChip({ forms }: { forms: CrfForm[] }) {
  const { t } = useI18n();
  const allApproved = forms.length > 0 && forms.every((f) => f.approved);
  if (allApproved) {
    return (
      <Chip
        icon={<VerifiedIcon />}
        label={t("crf.toolbar.statusApproved")}
        color="success"
        variant="outlined"
        size="small"
      />
    );
  }
  return (
    <Chip
      icon={<PendingActionsIcon />}
      label={t("crf.toolbar.statusPending")}
      color="warning"
      variant="outlined"
      size="small"
    />
  );
}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: error from `CrfFormListPage` (which calls `<CrfStatusChip />` without the new prop) — fixed in Task 15.

- [ ] **Step 3: Commit (do not typecheck-cleanly yet)**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfStatusChip.tsx
git commit -m "feat(desktop): CrfStatusChip derives state from forms"
```

---

## Task 14: CrfFormTable — row-level status chip

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`

- [ ] **Step 1: Replace the static `Pending` chip with a derived one**

In the `DraggableRow` component (around line 187), replace:

```tsx
<TableCell>
  <Chip
    icon={<PendingActionsIcon />}
    label={t("crf.toolbar.statusPending")}
    size="small"
    color="warning"
    variant="outlined"
  />
</TableCell>
```

with:

```tsx
<TableCell>
  <Chip
    icon={
      row.approved ? <VerifiedIcon /> : <PendingActionsIcon />
    }
    label={
      row.approved
        ? t("crf.toolbar.statusApproved")
        : t("crf.toolbar.statusPending")
    }
    size="small"
    color={row.approved ? "success" : "warning"}
    variant="outlined"
  />
</TableCell>
```

- [ ] **Step 2: Update the icon import line**

Around line 23, add `Verified as VerifiedIcon` to the import:

```ts
import {
  Add as AddIcon,
  AssignmentInd as AssignmentIndIcon,
  Delete as DeleteIcon,
  DragIndicator as DragIndicatorIcon,
  Edit as EditIcon,
  FilterList as FilterListIcon,
  Launch as LaunchIcon,
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx
git commit -m "feat(desktop): CrfFormTable row chip mirrors form approved"
```

---

## Task 15: CrfFormListPage — pass `forms` to CrfStatusChip

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`

- [ ] **Step 1: Pass `allRows` to `<CrfStatusChip />`**

Around line 246, change:

```tsx
<CrfStatusChip />
```

to:

```tsx
<CrfStatusChip forms={allRows} />
```

(`allRows` is the unfiltered list at line 123 — the spec wants the aggregate to reflect the version, not the user's filter.)

- [ ] **Step 2: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx
git commit -m "feat(desktop): list page passes forms to status chip"
```

---

## Task 16: CrfDetailPage — chip-toggle, immediately left of `<CrfToolsMenu />`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`

This is the user-facing chip-toggle. The chip serves as both visual indicator and toggle.

- [ ] **Step 1: Add imports**

Add to the `@aegis/ui/icons` import block (around line 16):

```ts
import {
  ArrowBack as ArrowBackIcon,
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
```

Add to the `useSetCrfFormApproved` import line near the other data hooks:

```ts
import {
  useCrfFormDetail,
  useCreateAnnotation,
  useCreateDomainAnnotation,
  useDeleteAnnotation,
  useDeleteDomainAnnotation,
  useSetCrfFormApproved, // new
  useUpdateAnnotation,
  useUpdateDomainAnnotation,
  useUpdateOwnerNotSubmitted,
} from "../data/detail";
```

Add `useSetCrfFormApproved` to the data hook imports from `../data/list`:

```ts
import {
  useGetCrfForm,
  useSetCrfFormApproved, // new
} from "../data/list";
```

(If `useSetCrfFormApproved` is exported from `../data/list` — yes per Task 11 — only one import is needed; remove the duplicate if added.)

- [ ] **Step 2: Add the mutation hook call and toggle handler**

After the `appendComment` mutation hook (around line 311), add:

```ts
const setApproved = useSetCrfFormApproved();

const openIssueCount =
  openIssueCountByTarget.get(null) ?? 0; // form-scoped count from existing useMemo

const approvedDisabled =
  setApproved.isPending ||
  !isMissionQc ||
  (form && !form.approved && openIssueCount > 0);

const approvedTooltipReason = !isMissionQc
  ? t("crf.toolbar.approveDisabled.notQc")
  : form && !form.approved && openIssueCount > 0
    ? t("crf.toolbar.approveDisabled.openIssues", { count: openIssueCount })
    : "";

const handleToggleApproved = () => {
  if (!form || !formMission) return;
  setApproved.mutate({
    id,
    approved: !form.approved,
    missionId: formMission.id,
  });
};
```

- [ ] **Step 3: Insert the chip-toggle in the header toolbar**

Find the line that reads `<CrfToolsMenu projectCode={projectCode} versionId={routeSearch.versionId ?? null} />` (around line 556). Immediately **before** that line (i.e. immediately to the left of the tools-menu icon), insert:

```tsx
{form && (
  <Tooltip title={approvedTooltipReason} disableHoverListener={!approvedDisabled || !approvedTooltipReason}>
    <span>
      <Chip
        icon={form.approved ? <VerifiedIcon /> : <PendingActionsIcon />}
        label={
          form.approved
            ? t("crf.toolbar.statusApproved")
            : t("crf.toolbar.statusPending")
        }
        color={form.approved ? "success" : "warning"}
        variant="outlined"
        size="small"
        onClick={approvedDisabled ? undefined : handleToggleApproved}
        sx={approvedDisabled ? { opacity: 0.5, cursor: "not-allowed" } : undefined}
        data-testid="crf-approval-toggle"
      />
    </span>
  </Tooltip>
)}
```

(`<Chip onClick={undefined}>` makes MUI render it as non-interactive; the `cursor: not-allowed` style is the disabled affordance.)

- [ ] **Step 4: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx
git commit -m "feat(desktop): CrfDetailPage chip-toggle left of CrfToolsMenu"
```

---

## Task 17: i18n strings

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Add the three new English keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, in the `crf.toolbar` block (around line 360), add the new keys after `"crf.toolbar.statusPending": "Pending"`:

```ts
"crf.toolbar.statusApproved": "Approved",
"crf.toolbar.approveDisabled.notQc": "Only the QC assignee can approve this form.",
"crf.toolbar.approveDisabled.openIssues": "Close all {{count}} open issue(s) before approving.",
```

(Keep `statusPending` as is — already exists.)

- [ ] **Step 2: Add the matching Chinese keys**

In `lib/packages/ui/src/i18n/locales/zhCN.ts` (around line 349), add:

```ts
"crf.toolbar.statusApproved": "已批准",
"crf.toolbar.approveDisabled.notQc": "只有该表单的 QC 负责人才能批准。",
"crf.toolbar.approveDisabled.openIssues": "请先关闭全部 {{count}} 个未解决问题再批准。",
```

- [ ] **Step 3: Typecheck the UI package**

Run: `pnpm --filter @aegis/ui typecheck`
Expected: PASS (locales are typed via `as const`, so the new keys are checked at the import site).

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): i18n keys for approved status and disabled tooltips"
```

---

## Task 18: Rust tests — public_api + integration_persistence fixtures

**Files:**
- Modify: `lib/crates/crf/tests/public_api.rs`
- Modify: `lib/crates/crf/tests/integration_persistence.rs`

After Tasks 2–7, every `CrfForm::new(...)` call, `CrfForm::for_repository(...)` call, and `CrfFormNew { ... }` literal must include the new `approved: bool` field. The tests in `public_api.rs` and `integration_persistence.rs` already have such literals — they will fail to compile until updated.

- [ ] **Step 1: Run the tests to surface failures**

Run: `cargo test -p crf`
Expected: compile errors. Note every reported site.

- [ ] **Step 2: Update `lib/crates/crf/tests/public_api.rs`**

This file has **two** kinds of site:

- `CrfForm::new(1, "F1".into(), "Form 1".into(), 0, false).unwrap()` at line 134 — this is a direct call to `CrfForm::new`; add the new `approved` arg so the call becomes `CrfForm::new(1, "F1".into(), "Form 1".into(), 0, false, false).unwrap()`.
- One `CrfFormNew { ... }` literal at lines 149-155 — add `approved: false,` after `not_submitted: false,`.

The other `CrfFormNew`-like literals on lines 162, 167, 172 are for `CrfItemNew` / `CrfOptionNew` / `CrfUnitNew` (different aggregate), which do NOT gain `approved`. Leave them alone.

- [ ] **Step 3: Update every `CrfFormNew { ... }` literal in `integration_persistence.rs`**

**6 `CrfFormNew { ... }` literals** need `approved: false,` added — at the start of each literal block (i.e. where `forms.create(CrfFormNew {` begins):
- line 86 (inside `cascade_delete_form_with_version`)
- line 153 (inside `polymorphic_owner_check_rejects_two_owners`)
- line 232 (inside `options_cascade_delete_with_item`)
- line 293 (inside `units_cascade_delete_with_item`)
- line 358 (inside `polymorphic_owner_round_trip`)
- line 503 (inside `get_form_detail_batch_ports_round_trip`)

Note: a grep for `not_submitted: false,` in this file will also surface 12 hits inside `CrfItemNew { ... }`, `CrfOptionNew { ... }`, and `CrfUnitNew { ... }` literals — those do NOT need updating. Filter by the enclosing literal type before editing.

- [ ] **Step 4: Update any view assertions**

If any test compares the returned `CrfFormView` to a literal (e.g. `assert_eq!(f.approved, false)`), update the comparison to include `approved: false`.

- [ ] **Step 5: Run non-ignored tests**

Run: `cargo test -p crf`
Expected: PASS.

- [ ] **Step 6: Run ignored tests against the dev DB**

Run: `cargo test -p crf -- --ignored --test-threads=1`
Expected: PASS. Live-DB tests are destructive (see CLAUDE.md).

- [ ] **Step 7: Commit**

```bash
git add lib/crates/crf/tests/public_api.rs lib/crates/crf/tests/integration_persistence.rs
git commit -m "test(crf): update fixtures for CrfForm.approved"
```

---

## Task 19: TS tests — fixtures + new cases

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

- [ ] **Step 1: Update `crf-form-table.test.tsx` fixtures**

Find the `CrfForm` fixtures (around lines 28, 38). Add `approved: false,` after `notSubmitted: false,` in each row literal.

- [ ] **Step 2: Add row-chip expectations**

In the same file, add a new `it` block asserting that a row with `approved: true` shows the "Approved" chip, and the default row shows "Pending". Use `screen.getByText(...)` against the chip's label and an icon-name check via `data-testid` if present, otherwise via text content.

- [ ] **Step 3: Update `crf-detail-page.test.tsx` fixtures and add chip-toggle cases**

Add `approved: false,` to the `CrfForm` fixture. Add new `it` cases:

- `chip renders "Pending" when form.approved is false`
- `chip renders "Approved" when form.approved is true`
- `chip is disabled for non-QC users (no QC role on mission)`
- `chip is enabled for QC with zero open issues and approved=false`
- `chip is disabled for QC with open issues > 0 when approved=false`
- `chip position is immediately left of CrfToolsMenu icon` — `<CrfToolsMenu>` exposes **no `data-testid`** today (its inner IconButton only has an `aria-label` of the localized `crf.toolbar.toolsMenuHint`). Query the chip by `data-testid="crf-approval-toggle"` (added in Task 16) and the menu icon by `getByRole("button", { name: t("crf.toolbar.toolsMenuHint") })`. Assert the chip's element `compareDocumentPosition` shows it appears BEFORE the menu icon button.

Use the existing test patterns in this file — look at how `isMissionQc` and `openIssueCount` are currently mocked, copy the pattern.

- [ ] **Step 4: Update `crf-form-list-page.test.tsx` fixtures and add aggregate-chip cases**

Add `approved: false,` to the row fixtures. Update the existing `CrfStatusChip` test (if any) to use the new `forms` prop. Add new cases:

- empty `forms` list → chip says "Pending"
- all-approved `forms` → chip says "Approved"
- mixed `forms` → chip says "Pending"

- [ ] **Step 5: Run the desktop tests**

Run: `pnpm --filter aegis-desktop test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/
git commit -m "test(desktop): approved status fixtures and chip cases"
```

---

## Task 20: Final verification

**Files:** none — verification only.

- [ ] **Step 1: Workspace compile**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 2: Workspace clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS.

- [ ] **Step 3: Workspace fmt**

Run: `cargo fmt --all -- --check`
Expected: PASS (run `cargo fmt --all` first if it complains).

- [ ] **Step 4: Full Rust test run**

Run: `cargo test --workspace`
Expected: PASS (non-ignored only).

- [ ] **Step 5: Desktop typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 6: Desktop build**

Run: `pnpm --filter aegis-desktop build`
Expected: PASS (tsc + vite).

- [ ] **Step 7: UI package typecheck + test**

Run: `pnpm --filter @aegis/ui typecheck && pnpm --filter @aegis/ui test`
Expected: PASS.

- [ ] **Step 8: Live-DB integration (deliberate, optional)**

Run:
```bash
cargo test -p crf -- --ignored --test-threads=1
cargo test -p aegis-server -- --ignored --test-threads=1
```
Expected: PASS against a local dev DB. These tests are destructive (drop tables + `_sqlx_migrations` per crate — see CLAUDE.md).

- [ ] **Step 9: Manual smoke (optional but recommended)**

If a local environment is available:

1. Start the server: `cargo run -p aegis-server` (after applying migrations).
2. Start the desktop app: `pnpm --filter aegis-desktop tauri dev`.
3. Open a CRF version: every row shows "Pending"; the aggregate chip shows "Pending".
4. Open a form's detail page: the chip-toggle left of the tools-menu icon shows "Pending".
5. Create an issue; confirm the chip is disabled with the open-issues tooltip.
6. Close the issue; click the chip; confirm it flips to "Approved", the row chip flips, and the aggregate chip flips when all forms are approved.
7. As a non-QC user (no QC role on the form's mission), confirm the chip is rendered but disabled.

- [ ] **Step 10: Final commit (only if fmt from Step 3 modified anything)**

```bash
git status
# If anything was formatted:
git add -u
git commit -m "style: cargo fmt"
```

---

## Self-review notes (filled in during planning)

**Spec coverage check:** Every section in [2026-09-23-crf-form-approved-field-design.md](docs/superpowers/specs/2026-09-23-crf-form-approved-field-design.md) maps to a task:
- DB → Task 1
- Domain → Task 2
- Persistence (CrfFormRepoPg threading + bulk) → Tasks 3, 4
- Usecase → Task 5
- Facade → Task 6
- Apis port → Task 7
- Server wire/handlers/router → Task 8
- Tauri wire (http fns + tests) → Task 9
- Tauri command + gate tests → Task 10
- TS types / shared API / mutation hook → Task 11
- Drawer `approved: false` → Task 12
- CrfStatusChip derived → Task 13
- CrfFormTable row chip → Task 14
- CrfFormListPage wiring → Task 15
- CrfDetailPage chip-toggle → Task 16
- i18n → Task 17
- Rust tests / fixtures → Task 18
- TS tests / fixtures → Task 19
- Verification → Task 20

No spec gaps.

**Placeholder scan:** No TBD / TODO / "implement later" / "add appropriate error handling" / "similar to Task N" / "fill in details" patterns.

**Type consistency:**
- `approved: bool` end-to-end: domain → persistence → usecase view → facade → apis → server wire → tauri wire → TS types.
- `SetCrfApprovedRequest` Rust struct (apis) ↔ `SetCrfApprovedRequest` tauri wire (same name, same field `approved: bool`, both renamed camelCase on wire).
- Tauri command signature `(id: i64, approved: bool, mission_id: i64)` matches the `invoke` call shape `{ id, approved, missionId }`.
- Mission-id from `formMission.id` in CrfDetailPage flows to `useSetCrfFormApproved` mutation variables then to Tauri command.
- i18n keys consistent between en.ts and zhCN.ts.
- `queryKeys.crf.formsByVersion(updated.versionId)` + `queryKeys.crf.form(updated.id)` invalidation matches `useUpdateCrfForm` pattern.
- `CrfStatusChip` new `forms: CrfForm[]` prop; `CrfFormListPage` passes `allRows` (the unfiltered, version-wide set per spec).
- `useListIssuesByMission` already provides `openIssueCountByTarget`; the chip-toggle uses the form-scoped value (`get(null) ?? 0`), matching existing detail-page patterns.