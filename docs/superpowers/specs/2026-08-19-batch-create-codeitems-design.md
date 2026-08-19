# Batch Create CodeItems — Design Spec

## Status

Approved 2026-08-19.

## Overview

Add a `POST /api/terminology/code-items/batch` endpoint that accepts a single `codelist_id` + `version_id` and a list of item fields, creating all items in a single SQL `INSERT`. Fail-fast validation: if any item is invalid, zero items are persisted.

---

## Decisions

| # | Question | Decision |
|---|----------|----------|
| 1 | Are all batch items for the same codelist/version? | Yes — `codelist_id` and `version_id` are fixed at the batch level |
| 2 | On validation failure? | Fail-fast: return error for first invalid item, persist nothing |
| 3 | Success response shape? | `{ count, codelist_id, version_id }` — no per-item detail |
| 4 | Maximum batch size? | No hard limit; underlying DB/server timeout is the practical ceiling |
| 5 | Error message includes failing item position? | Yes — `"item 23: code cannot be empty"` |

---

## Changes by Layer

### 1. API Types — `lib/crates/apis/src/terminology.rs`

**New DTOs:**
```rust
/// Input DTO for `TerminologyService::batch_create_code_items`.
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

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

/// Response for `TerminologyService::batch_create_code_items`.
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}
```

**New service trait method:**
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

---

### 2. Domain Error — `lib/crates/terminology/src/domain/error.rs`

**New variant on `DomainError`:**
```rust
EmptyCodeAtPosition(usize),
```

---

### 3. Commands — `lib/crates/terminology/src/usecase/commands.rs`

**New command struct:**
```rust
pub struct BatchCreateCodeItems {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<CreateCodeItem>,
}
```

---

### 4. Repository Trait — `lib/crates/terminology/src/domain/repository.rs`

**New method on `CodeItemRepository`:**
```rust
/// Insert several `CodeItem` rows in a single SQL statement.
/// Returns the number of rows inserted on success. The backend
/// must execute this atomically — if any row violates a constraint
/// the entire call fails and zero rows are inserted.
async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError>;
```

---

### 5. Usecase — `lib/crates/terminology/src/usecase/terminology_usecase.rs`

**New method on `TerminologyUsecase`:**
```rust
pub async fn batch_create_code_items(
    &self,
    cmd: BatchCreateCodeItems,
) -> Result<BatchCreateCodeItemsResponse, UsecaseError>
```

Calls `validate_batch_code_items` first, then maps the command items to `Vec<CodeItemNew>` and calls `code_item_repo.bulk_create(inputs)` once.

**New validation function:**
```rust
fn validate_batch_code_items(cmd: &BatchCreateCodeItems) -> Result<(), UsecaseError>
```

Iterates items in order; on first empty `code`, returns `UsecaseError::Validation(DomainError::EmptyCodeAtPosition(i))`.

---

### 6. Service Implementation — `lib/crates/terminology/src/adapter/facade/in_memory/service.rs`

Implement `batch_create_code_items` on `TerminologyServiceImpl` by mapping the API request to the usecase command and calling the usecase method, translating `UsecaseError` → `TerminologyApiError` via the existing `translate_usecase_error` helper.

---

### 7. HTTP Handler — `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`

```rust
pub async fn batch_create_code_items(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::BatchCreateCodeItemsRequest>,
) -> Result<(StatusCode, Json<dto::BatchCreateCodeItemsResponse>), ApiError>
```

Requires `admin` or `root` role. Returns `201 CREATED` on success.

---

### 8. HTTP DTO — `apps/server/aegis-server/src/transport/http/terminology/dto.rs`

Add `BatchCreateCodeItemsRequest` and `BatchCreateCodeItemsResponse` DTOs mirroring the `apis` types (with serde annotations).

---

### 9. HTTP Router — `apps/server/aegis-server/src/transport/http/terminology/router.rs`

Register:
```rust
post("/api/terminology/code-items/batch").handler(batch_create_code_items)
```

---

## Files Changed

| File | Change |
|------|--------|
| `lib/crates/apis/src/terminology.rs` | Add `BatchCreateCodeItemsRequest`, `BatchCodeItemEntry`, `BatchCreateCodeItemsResponse` DTOs; add `batch_create_code_items` to `TerminologyService` trait |
| `lib/crates/terminology/src/domain/error.rs` | Add `EmptyCodeAtPosition(usize)` to `DomainError` |
| `lib/crates/terminology/src/usecase/commands.rs` | Add `BatchCreateCodeItems` command |
| `lib/crates/terminology/src/domain/repository.rs` | Add `bulk_create` method to `CodeItemRepository` trait |
| `lib/crates/terminology/src/usecase/terminology_usecase.rs` | Add `batch_create_code_items` usecase method + `validate_batch_code_items` |
| `lib/crates/terminology/src/adapter/facade/in_memory/service.rs` | Implement `batch_create_code_items` on `TerminologyServiceImpl` |
| `apps/server/aegis-server/src/transport/http/terminology/handlers.rs` | Add `batch_create_code_items` HTTP handler |
| `apps/server/aegis-server/src/transport/http/terminology/dto.rs` | Add request/response DTOs |
| `apps/server/aegis-server/src/transport/http/terminology/router.rs` | Register `POST /api/terminology/code-items/batch` route |
