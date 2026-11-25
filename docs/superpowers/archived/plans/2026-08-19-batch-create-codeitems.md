# Batch Create CodeItems — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `POST /api/terminology/code-items/batch` that creates multiple code items in a single SQL `INSERT`, all under one codelist/version, with fail-fast validation.

**Architecture:** Add `bulk_create` to `CodeItemRepository` trait (single SQL `INSERT`), then layer `batch_create_code_items` through the usecase and service to the HTTP handler. Validation fails on first empty `code` with position annotation.

**Tech Stack:** Rust, axum, async-trait, sqlx (Postgres backend), serde, utoipa

---

## File Inventory

| File | Change |
|------|--------|
| `lib/crates/terminology/src/domain/error.rs` | Add `EmptyCodeAtPosition(usize)` to `DomainError` |
| `lib/crates/terminology/src/domain/repository.rs` | Add `bulk_create` method to `CodeItemRepository` trait |
| `lib/crates/terminology/src/usecase/commands.rs` | Add `BatchCreateCodeItems` command struct |
| `lib/crates/terminology/src/usecase/terminology_usecase.rs` | Add `batch_create_code_items` method + `validate_batch_code_items` |
| `lib/crates/terminology/src/adapter/facade/in_memory/service.rs` | Add `bulk_create` to in-memory repo impl; implement `batch_create_code_items` on `TerminologyServiceImpl` |
| `lib/crates/apis/src/terminology.rs` | Add `BatchCreateCodeItemsRequest`, `BatchCodeItemEntry`, `BatchCreateCodeItemsResponse` DTOs; add `batch_create_code_items` to `TerminologyService` trait |
| `apps/server/aegis-server/src/transport/http/dto.rs` | Add wire DTOs for batch request/response |
| `apps/server/aegis-server/src/transport/http/terminology/handlers.rs` | Add `batch_create_code_items` handler |
| `apps/server/aegis-server/src/transport/http/terminology/router.rs` | Register `POST /api/terminology/code-items/batch` route |
| `lib/crates/terminology/src/adapter/repository/` | Implement `bulk_create` on the Postgres `CodeItemRepo` |

---

## Task 1: Domain Error — `EmptyCodeAtPosition`

**Files:**
- Modify: `lib/crates/terminology/src/domain/error.rs:6-48`

- [ ] **Step 1: Add `EmptyCodeAtPosition` variant to `DomainError`**

Find the closing `}` of `DomainError` at line 48 and add the new variant before it (after line 29 `CodeItemNotFound`):

```rust
#[error("code at position {0} must not be empty")]
EmptyCodeAtPosition(usize),
```

- [ ] **Step 2: Commit**

```bash
git add lib/crates/terminology/src/domain/error.rs
git commit -m "feat(terminology): add EmptyCodeAtPosition domain error

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Repository Trait — Add `bulk_create`

**Files:**
- Modify: `lib/crates/terminology/src/domain/repository.rs:59-83`

- [ ] **Step 1: Add `bulk_create` method to `CodeItemRepository` trait**

After the existing `search` method (line 82), add:

```rust
/// Insert several `CodeItem` rows in a single SQL statement.
/// Returns the number of rows inserted on success. The backend
/// must execute this atomically — if any row violates a constraint
/// the entire call fails and zero rows are inserted.
async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError>;
```

- [ ] **Step 2: Commit**

```bash
git add lib/crates/terminology/src/domain/repository.rs
git commit -m "feat(terminology): add bulk_create to CodeItemRepository

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Commands — Add `BatchCreateCodeItems`

**Files:**
- Modify: `lib/crates/terminology/src/usecase/commands.rs:42-62`

- [ ] **Step 1: Add `BatchCreateCodeItems` command after the `UpdateCodeItem` struct**

At the end of the file (after line 62, before any `#[cfg(test)]` block), add:

```rust
// Batch

pub struct BatchCreateCodeItems {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<CreateCodeItem>,
}
```

- [ ] **Step 2: Commit**

```bash
git add lib/crates/terminology/src/usecase/commands.rs
git commit -m "feat(terminology): add BatchCreateCodeItems command

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Usecase — `batch_create_code_items` + validation

**Files:**
- Modify: `lib/crates/terminology/src/usecase/terminology_usecase.rs`

- [ ] **Step 1: Add `BatchCreateCodeItemsResponse` view struct and `BatchCreateCodeItems` import**

Add to the imports from `super::views` at line 12:
```rust
BatchCreateCodeItemsResponse,
```

Add a new `use` for the new command in the commands import at line 7-10:
```rust
use super::commands::{
    BatchCreateCodeItems, CreateCodeItem, CreateCodeList, CreateTerminologyVersion,
    UpdateCodeItem, UpdateCodeList, UpdateTerminologyVersion,
};
```

- [ ] **Step 2: Add `BatchCreateCodeItemsResponse` view struct**

After the `impl From<CodeItem> for CodeItemView` block in `views.rs` (after line 88), add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}
```

- [ ] **Step 3: Add `batch_create_code_items` usecase method**

Add after the existing `search_code_items` method (after line 269, before the `// ---- pre-flight validation ----` comment at line 272):

```rust
pub async fn batch_create_code_items(
    &self,
    cmd: BatchCreateCodeItems,
) -> Result<BatchCreateCodeItemsResponse, UsecaseError> {
    validate_batch_code_items(&cmd)?;

    let inputs: Vec<CodeItemNew> = cmd
        .items
        .iter()
        .map(|item| CodeItemNew {
            codelist_id: cmd.codelist_id,
            version_id: cmd.version_id,
            code: item.code.clone(),
            submission_value: item.submission_value.clone(),
            synonym: item.synonym.clone(),
            definition: item.definition.clone(),
            nci_preferred_term: item.nci_preferred_term.clone(),
        })
        .collect();

    let count = self.code_item_repo.bulk_create(inputs).await?;

    Ok(BatchCreateCodeItemsResponse {
        count,
        codelist_id: cmd.codelist_id,
        version_id: cmd.version_id,
    })
}
```

- [ ] **Step 4: Add `validate_batch_code_items` validation function**

Add after `validate_update_code_item` (after line 319, before the `// ---- search-query sanitation ----` comment at line 322):

```rust
fn validate_batch_code_items(cmd: &BatchCreateCodeItems) -> Result<(), UsecaseError> {
    for (i, item) in cmd.items.iter().enumerate() {
        if item.code.trim().is_empty() {
            return Err(UsecaseError::Validation(
                DomainError::EmptyCodeAtPosition(i),
            ));
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Commit**

```bash
git add lib/crates/terminology/src/usecase/terminology_usecase.rs lib/crates/terminology/src/usecase/views.rs
git commit -m "feat(terminology): add batch_create_code_items usecase method

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: API Types — DTOs + `TerminologyService` trait method

**Files:**
- Modify: `lib/crates/apis/src/terminology.rs`

- [ ] **Step 1: Add `BatchCodeItemEntry`, `BatchCreateCodeItemsRequest`, `BatchCreateCodeItemsResponse` DTOs**

Add after the existing `UpdateCodeItemRequest` struct (after line 203, before the `Outbound port` comment at line 205):

```rust
/// One entry inside a batch request. All fields map 1-to-1 with
/// `CreateCodeItemRequest` minus codelist_id/version_id
/// (those are fixed at the batch level).
#[derive(Debug, Clone)]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for [`TerminologyService::batch_create_code_items`].
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

/// Response for [`TerminologyService::batch_create_code_items`].
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}
```

- [ ] **Step 2: Add `batch_create_code_items` to `TerminologyService` trait**

Add after the existing `search_code_items` method (after line 338, before the closing `}` of the trait at line 339):

```rust
/// Create several `CodeItem`s in one logical operation. All items
/// must share `req.codelist_id` and `req.version_id`. If any item
/// fails validation the entire batch is rolled back and the first
/// error is returned with the failing item's position annotated:
/// `"item 23: code cannot be empty"`.
async fn batch_create_code_items(
    &self,
    req: BatchCreateCodeItemsRequest,
) -> Result<BatchCreateCodeItemsResponse, TerminologyApiError>;
```

- [ ] **Step 3: Commit**

```bash
git add lib/crates/apis/src/terminology.rs
git commit -m "feat(apis): add batch create code items request/response DTOs and service trait method

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: In-Memory Repo — `bulk_create` + Service Implementation

**Files:**
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/service.rs`
- Create: `lib/crates/terminology/src/adapter/facade/in_memory/` (check if repo file exists)

- [ ] **Step 1: Find the in-memory repository file**

Run: `find /root/projects/aegis/lib/crates/terminology/src/adapter -name "*.rs" | xargs grep -l "CodeItemRepository" | head -5`

Read that file to find where the in-memory `CodeItemRepository` impl lives.

- [ ] **Step 2: Add `bulk_create` to the in-memory `CodeItemRepository` impl**

The in-memory repo likely holds items in a `Vec<CodeItem>`. Add to its impl:

```rust
async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError> {
    // Generate IDs (find max existing id + 1 per new row)
    // For the in-memory fake, this is a simplified insert
    // that appends to the vec. No actual FK/unique constraint checking needed.
    let count = inputs.len();
    for input in inputs {
        let item = CodeItem { /* map fields */ };
        self.items.lock().unwrap().push(item);
    }
    Ok(count)
}
```

- [ ] **Step 3: Add `batch_create_code_items` to `TerminologyServiceImpl`**

Add to the `impl<V, L, I> TerminologyService for TerminologyServiceImpl<V, L, I>` block, after `search_code_items`:

```rust
async fn batch_create_code_items(
    &self,
    req: BatchCreateCodeItemsRequest,
) -> Result<BatchCreateCodeItemsResponse, TerminologyApiError> {
    let cmd = BatchCreateCodeItems {
        codelist_id: req.codelist_id,
        version_id: req.version_id,
        items: req
            .items
            .into_iter()
            .map(|e| CreateCodeItem {
                codelist_id: req.codelist_id,
                version_id: req.version_id,
                code: e.code,
                submission_value: e.submission_value,
                synonym: e.synonym,
                definition: e.definition,
                nci_preferred_term: e.nci_preferred_term,
            })
            .collect(),
    };
    let resp = self
        .usecase
        .batch_create_code_items(cmd)
        .await
        .map_err(TerminologyApiError::from)?;
    Ok(BatchCreateCodeItemsResponse {
        count: resp.count,
        codelist_id: resp.codelist_id,
        version_id: resp.version_id,
    })
}
```

Also add `BatchCreateCodeItems` and `BatchCreateCodeItemsResponse` to the imports from `crate::usecase`.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/terminology/src/adapter/facade/in_memory/service.rs
git add <the in-memory repo file found in step 1>
git commit -m "feat(terminology): implement bulk_create in-memory and batch_create_code_items service

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: HTTP DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

- [ ] **Step 1: Add wire DTOs for batch request and response**

Add after the existing `UpdateCodeItemRequest` struct (after line 734, before the `// -- terminology search query DTOs --` comment at line 736):

```rust
/// Wire-level request body for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

/// Wire-level entry inside a batch request.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level response for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}
```

- [ ] **Step 2: Add roundtrip tests**

In the `#[cfg(test)]` mod at the bottom of `dto.rs`, add:

```rust
#[test]
fn batch_create_code_items_request_roundtrip() {
    let json = r#"{"codelistId":11,"versionId":1,"items":[{"code":"C1","submissionValue":"Y","synonym":"","definition":"","nciPreferredTerm":"Yes"}]}"#;
    let req: BatchCreateCodeItemsRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.codelist_id, 11);
    assert_eq!(req.version_id, 1);
    assert_eq!(req.items.len(), 1);
    assert_eq!(req.items[0].code, "C1");
    assert_eq!(serde_json::to_string(&req).unwrap(), json);
}

#[test]
fn batch_create_code_items_response_roundtrip() {
    let json = r#"{"count":3,"codelistId":11,"versionId":1}"#;
    let resp: BatchCreateCodeItemsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.count, 3);
    assert_eq!(resp.codelist_id, 11);
    assert_eq!(resp.version_id, 1);
    assert_eq!(serde_json::to_string(&resp).unwrap(), json);
}
```

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(server): add wire DTOs for batch create code items

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: HTTP Handler + Router

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/terminology/router.rs`

- [ ] **Step 1: Add `batch_create_code_items` handler**

Add after the existing `create_code_item` handler (after line 403, before `list_code_items`):

```rust
/// `POST /api/terminology/code-items/batch` — create several code items
/// in a single SQL statement.
#[utoipa::path(
    post, path = "/code-items/batch", tag = "terminology",
    operation_id = "terminology_batch_create_code_items",
    request_body = dto::BatchCreateCodeItemsRequest,
    responses(
        (status = 201, description = "Items created", body = dto::BatchCreateCodeItemsResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn batch_create_code_items(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::BatchCreateCodeItemsRequest>,
) -> Result<(StatusCode, Json<dto::BatchCreateCodeItemsResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let resp = state
        .terminology
        .batch_create_code_items(apis::terminology::BatchCreateCodeItemsRequest {
            codelist_id: req.codelist_id,
            version_id: req.version_id,
            items: req
                .items
                .into_iter()
                .map(|e| apis::terminology::BatchCodeItemEntry {
                    code: e.code,
                    submission_value: e.submission_value,
                    synonym: e.synonym,
                    definition: e.definition,
                    nci_preferred_term: e.nci_preferred_term,
                })
                .collect(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(dto::BatchCreateCodeItemsResponse {
        count: resp.count,
        codelist_id: resp.codelist_id,
        version_id: resp.version_id,
    })))
}
```

- [ ] **Step 2: Register route in router**

Add to the `// ---- CodeItem ----` section in `router.rs` after line 33:

```rust
.routes(routes!(handlers::batch_create_code_items))
```

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/terminology/handlers.rs apps/server/aegis-server/src/transport/http/terminology/router.rs
git commit -m "feat(server): add POST /api/terminology/code-items/batch handler and route

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: Postgres `CodeItemRepo` — `bulk_create`

**Files:**
- Modify: `<the postgres codeitem repo file>`

- [ ] **Step 1: Find the Postgres `CodeItemRepo` file**

Run: `find /root/projects/aegis -name "*.rs" | xargs grep -l "CodeItemRepo" | head -5`

Read the file to find the existing `create` method implementation.

- [ ] **Step 2: Add `bulk_create` implementation**

Implement `bulk_create` using sqlx `query_builder` to produce a single `INSERT INTO code_items (codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term) VALUES (...), (...), ...` statement. Use `sqlx::query_builder::QueryBuilder` with `separated_by`.

Example pattern (adapt to actual table column names):
```rust
async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError> {
    if inputs.is_empty() {
        return Ok(0);
    }
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "INSERT INTO code_items (codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term) "
    );
    qb.push_values(inputs, |mut b, item| {
        b.push_bind(item.codelist_id)
            .push_bind(item.version_id)
            .push_bind(&item.code)
            .push_bind(&item.submission_value)
            .push_bind(&item.synonym)
            .push_bind(&item.definition)
            .push_bind(&item.nci_preferred_term);
    });
    let query = qb.build();
    let count = query.execute(&*self.pool).await?;
    Ok(count.rows_affected() as usize)
}
```

- [ ] **Step 3: Commit**

```bash
git add <postgres codeitem repo file>
git commit -m "feat(terminology): implement bulk_create in postgres CodeItemRepo

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Verify Compile + Tests

- [ ] **Step 1: Run cargo check**

```bash
cargo check --all
```

Expected: no errors.

- [ ] **Step 2: Run terminology tests**

```bash
cargo test -p terminology
```

Expected: all tests pass.

- [ ] **Step 3: Run full test suite**

```bash
cargo test --all
```

Expected: all tests pass.

- [ ] **Step 4: Commit final verification**

```bash
git add -A
git commit -m "test: verify batch-create-codeitems compiles and all tests pass

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|------|------|
| Same codelist/version for all items | Task 3 (command), Task 4 (usecase) |
| Fail-fast on validation error | Task 4 (`validate_batch_code_items`) |
| `EmptyCodeAtPosition(usize)` error | Task 1 |
| Error includes item position | Task 1 (`EmptyCodeAtPosition`) + Task 4 (usecase maps to `UsecaseError::Validation`) |
| Summary-only response `{count, codelist_id, version_id}` | Task 4 (`BatchCreateCodeItemsResponse`), Task 5 (API DTO), Task 7 (wire DTO) |
| No hard batch size limit | No artificial cap added — underlying DB is the practical limit |
| Single SQL `INSERT` | Task 2 (trait), Task 9 (postgres impl) |
| Service trait updated | Task 5 |
| Service impl updated | Task 6 |
| HTTP handler + route | Task 8 |
