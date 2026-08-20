# Terminology — Unified list+search with pagination — Design

**Date:** 2026-08-20
**Status:** Approved (pending spec review)
**Scope:** Collapse the four list/search endpoints on codelists and items (`GET /code-lists`, `GET /code-lists/search`, `GET /code-items`, `GET /code-items/search`) into two unified list+search endpoints with optional full-text `fragment` and offset/limit pagination. Keep `GET /code-items/by-version-and-code` unchanged — its cross-codelist natural-key semantics are different from a list-with-filter. Server-side only: touches every layer from the domain up through the HTTP router. The Tauri desktop client is explicitly out of scope (see section 9).

---

## 1. Goals

1. Replace `list_code_lists` + `search_code_lists` on the usecase / apis trait / http with one method `list_code_lists(CodeListListQuery)` that takes `(version_id, fragment?, offset, limit)` and returns a `Page<CodeListView>`.
2. Replace `list_code_items` + `search_code_items` on the same layers with one method `list_code_items(CodeItemListQuery)` that takes `(codelist_id, fragment?, offset, limit)` and returns a `Page<CodeItemView>`.
3. Add offset/limit pagination to both. `next_offset` in the response is the cursor for the next page; absent when the page was the last.
4. Keep the existing FTS prefix-match semantics (`to_tsquery(fragment || ':*')`, ranked by `ts_rank DESC`, with `id ASC` as tiebreaker).
5. Defend against tsquery syntax errors (fragments containing `& | ! ( ) :`) by rejecting them at the usecase with a new `DomainError::InvalidFragment` → 400.
6. Keep `list_code_items_by_version_and_code` and `GET /code-items/by-version-and-code` exactly as-is.
7. Out of scope: cursor-based pagination, total count, fuzzy / substring search, configurable sort order, additional full-text fields, deprecation period (the old search endpoints are removed in the same change; the Tauri `search_*` commands that called them are dormant and are non-blocking to remove in a follow-up — see section 9).

---

## 2. Wire contract

### 2.1 `GET /api/terminology/code-lists`

Query parameters (all optional except `versionId`):

| Param | Type | Default | Notes |
| --- | --- | --- | --- |
| `versionId` | i64 | required | parent version |
| `fragment` | string | absent | optional FTS prefix match |
| `offset` | u32 | 0 | skipped when 0 |
| `limit` | u32 | 0 → clamped to 50 (max 500) | clamped by the usecase |

Response — 200:

```json
{
  "codelists": [ { /* CodeListViewResponse */ }, … ],
  "nextOffset": 100   // omitted when this is the last page
}
```

Errors: 400 (InvalidFragment), 401 (auth), 500 (repository).

### 2.2 `GET /api/terminology/code-items`

Same query params with `codelistId` as the required parent.

Response — 200:

```json
{
  "items": [ { /* CodeItemViewResponse */ }, … ],
  "nextOffset": 100   // omitted when this is the last page
}
```

Errors: 400 (InvalidFragment), 401, 500.

### 2.3 `GET /api/terminology/code-items/by-version-and-code`

Unchanged.

### 2.4 Endpoints removed

- `GET /api/terminology/code-lists/search`
- `GET /api/terminology/code-items/search`

---

## 3. Domain types

### 3.1 Added

```rust
// domain/paging.rs
pub struct Page<T> {
    pub items: Vec<T>,
    /// `Some(offset + limit)` when more rows exist beyond this page;
    /// `None` when this page is the last one.
    pub next_offset: Option<u32>,
}
```

```rust
// domain/code_list.rs
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

```rust
// domain/code_item.rs
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

### 3.2 Removed

- `CodeListSearchQuery`
- `CodeListSearchHit`
- `CodeItemSearchQuery`
- `CodeItemSearchHit`
- `DomainError::EmptyFragment` (no longer needed; `fragment = None` encodes "no filter")

### 3.3 Added `DomainError` variant

```rust
pub enum DomainError {
    // …existing variants…
    /// Search fragment contained characters reserved by Postgres
    /// `to_tsquery` (`& | ! ( ) :`).
    #[error("search fragment contains reserved tsquery characters: & | ! ( ) :")]
    InvalidFragment,
}
```

---

## 4. Repository contract

### 4.1 `CodeListRepository`

```rust
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;

    /// Unified list+search under a version. Returns a single page.
    /// - `fragment = None`           → `WHERE version_id = $1 ORDER BY id ASC`
    /// - `fragment = Some(_)`        → `WHERE version_id = $1 AND tsv @@ to_tsquery('english', $2 || ':*')
    ///                                  ORDER BY ts_rank(tsv, to_tsquery('english', $2 || ':*')) DESC, id ASC`
    /// The implementation fetches `limit + 1` rows to compute `next_offset`.
    async fn search_or_list(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeList>, DomainError>;
}
```

Removed from the trait: `list_by_version`, `search`.

### 4.2 `CodeItemRepository`

Same shape — `search_or_list(CodeItemListQuery) -> Result<Page<CodeItem>, DomainError>` replaces `list_by_codelist` and `search`. `list_by_version_and_code` stays as the natural-key lookup.

### 4.3 Postgres adapter — implementation sketch

```rust
async fn search_or_list(&self, q: CodeListListQuery) -> Result<Page<CodeList>, DomainError> {
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at FROM code_lists WHERE version_id = ",
    );
    qb.push_bind(q.version_id);

    if let Some(frag) = q.fragment.as_deref() {
        qb.push(" AND tsv @@ to_tsquery('english', ");
        qb.push_bind(format!("{frag}:*"));
        qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
        qb.push_bind(format!("{frag}:*"));
        qb.push(")) DESC, id ASC LIMIT ");
    } else {
        qb.push(" ORDER BY id ASC LIMIT ");
    }
    qb.push_bind((q.limit as i64) + 1);
    qb.push(" OFFSET ");
    qb.push_bind(q.offset as i64);

    let mut rows: Vec<CodeListRow> = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_db_error_simple)?;
    let next_offset = if rows.len() as u32 > q.limit { rows.pop(); Some(q.offset + q.limit) } else { None };
    let items = rows.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, next_offset })
}
```

Same pattern for `CodeItemRepo::search_or_list` (different table / columns).

The existing `code_lists_tsv_idx` GIN index continues to be used for the `fragment = Some(_)` branch. ORDER BY `id ASC` tiebreaker is essential for stable pagination on rank ties.

### 4.4 In-memory adapter — implementation sketch

```rust
async fn search_or_list(&self, q: CodeListListQuery) -> Result<Page<CodeList>, DomainError> {
    let mut all: Vec<CodeList> = self.state.lock().unwrap().by_id.values().cloned().collect();
    all.retain(|cl| cl.version_id == q.version_id);

    if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        let needle = frag.to_lowercase();
        all.retain(|cl| cl.name.to_lowercase().contains(&needle)
            || cl.submission_value.to_lowercase().contains(&needle)
            || cl.synonym.to_lowercase().contains(&needle)
            || cl.definition.to_lowercase().contains(&needle)
            || cl.nci_preferred_term.to_lowercase().contains(&needle));
    }

    all.sort_by_key(|cl| cl.id);

    let limit = q.limit as usize;
    let offset = q.offset as usize;
    let take = (all.len().saturating_sub(offset)).min(limit + 1);
    let mut page = all.into_iter().skip(offset).take(take).collect::<Vec<_>>();
    let next_offset = if page.len() > limit { page.pop(); Some(q.offset + q.limit) } else { None };
    Ok(Page { items: page, next_offset })
}
```

The in-memory backend doesn't rank — fragments are matched case-insensitively across the same five text fields the existing in-memory search uses. Results are sorted by id ASC (the only stable order in HashMap-backed storage).

---

## 5. Usecase

```rust
pub async fn list_code_lists(
    &self,
    query: CodeListListQuery,
) -> Result<Page<CodeListView>, UsecaseError> {
    let limit = clamp_limit(query.limit);
    let fragment = match query.fragment.as_deref() {
        None => None,
        Some(s) if s.trim().is_empty() => None,
        Some(s) => Some(validate_tsquery_fragment(s)?.to_owned()),
    };
    let q = CodeListListQuery { fragment, offset: query.offset, limit, ..query };
    let page = self.code_list_repo.search_or_list(q).await?;
    Ok(Page {
        items: page.items.into_iter().map(Into::into).collect(),
        next_offset: page.next_offset,
    })
}
```

`validate_tsquery_fragment`:

```rust
fn validate_tsquery_fragment(s: &str) -> Result<&str, UsecaseError> {
    if s.chars().any(|c| matches!(c, '&' | '|' | '!' | '(' | ')' | ':')) {
        return Err(UsecaseError::Validation(DomainError::InvalidFragment));
    }
    Ok(s)
}
```

`clamp_limit` is unchanged (`0 → 50`, `>500 → 500`).

`list_code_items` mirrors the same shape. `list_code_items_by_version_and_code` is unchanged.

`search_code_lists` and `search_code_items` are removed from the usecase.

---

## 6. API contract (`apis` crate)

```rust
// apis/src/terminology.rs
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u32>,
}

pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

pub struct CodeItemListQuery {
    pub codelist_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

`TerminologyService` trait:

```rust
async fn list_code_lists(
    &self,
    query: CodeListListQuery,
) -> Result<Page<CodeListView>, TerminologyApiError>;

async fn list_code_items(
    &self,
    query: CodeItemListQuery,
) -> Result<Page<CodeItemView>, TerminologyApiError>;

async fn list_code_items_by_version_and_code(
    &self,
    version_id: i64,
    code: &str,
) -> Result<Vec<CodeItemView>, TerminologyApiError>; // unchanged
```

Removed: `search_code_lists`, `search_code_items`. Removed types: `CodeListSearchQuery`, `CodeListSearchHit`, `CodeItemSearchQuery`, `CodeItemSearchHit`.

`From<UsecaseError> for TerminologyApiError` already maps `UsecaseError::Validation(_)` → `TerminologyApiError::Validation(String)`, so `InvalidFragment` flows through unchanged.

---

## 7. Service adapter

```rust
async fn list_code_lists(
    &self,
    query: CodeListListQuery,
) -> Result<Page<CodeListView>, TerminologyApiError> {
    let page = self.usecase.list_code_lists(query).await?;
    Ok(Page {
        items: page.items.into_iter().map(code_list_view_from_internal).collect(),
        next_offset: page.next_offset,
    })
}
```

`list_code_items` mirrors the same shape. `search_code_lists` / `search_code_items` impls are removed.

---

## 8. HTTP layer

### 8.1 Wire DTOs

```rust
// apps/server/aegis-server/src/transport/http/dto.rs
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeListListQuery {
    pub version_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeListsResponse {
    pub codelists: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeItemsResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}
```

The existing `CodeListListResponse` / `CodeItemListResponse` (which only had the array field) are removed in favour of the paged variants. The old response envelopes (`{ codelists: […] }`, `{ items: […] }`) keep the same JSON field names so existing Tauri/TS consumers continue to deserialize; `nextOffset` is an additive optional field.

Removed: `TerminologySearchBaseQuery`, `CodeListSearchQueryRequest`, `CodeItemSearchQueryRequest`, `CodeListSearchHitResponse`, `CodeItemSearchHitResponse`, `CodeListSearchHitsResponse`, `CodeItemSearchHitsResponse`.

### 8.2 Handlers

```rust
pub async fn list_code_lists(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(q): Query<dto::CodeListListQuery>,
) -> Result<Json<dto::PagedCodeListsResponse>, ApiError> {
    let page = state.terminology.list_code_lists(q.into()).await?;
    Ok(Json(dto::PagedCodeListsResponse {
        codelists: page.items.into_iter().map(Into::into).collect(),
        next_offset: page.next_offset,
    }))
}
```

`list_code_items` mirrors the same shape. `list_code_items_by_version_and_code` is unchanged. `search_code_lists` and `search_code_items` handlers are removed.

### 8.3 Router

```rust
// apps/server/aegis-server/src/transport/http/terminology/router.rs
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- TerminologyVersion ---- (unchanged)
        // ---- CodeList ----
        .routes(routes!(handlers::create_code_list))
        .routes(routes!(handlers::list_code_lists))
        .routes(routes!(handlers::get_code_list_by_id))
        .routes(routes!(handlers::update_code_list))
        .routes(routes!(handlers::delete_code_list))
        // ---- CodeItem ----
        .routes(routes!(handlers::create_code_item))
        .routes(routes!(handlers::list_code_items))
        .routes(routes!(handlers::list_code_items_by_version_and_code))
        .routes(routes!(handlers::update_code_item))
        .routes(routes!(handlers::delete_code_item))
}
```

`search_code_lists` and `search_code_items` route entries are removed.

---

## 9. Out of scope: Tauri desktop client

This refactor is server-side only. **No files under `apps/desktop/aegis-desktop/` are touched.**

The wire shape is backwards-compatible by design:
- The list endpoints keep the same path (`GET /code-lists`, `GET /code-items`) and the same required query param (`versionId`, `codelistId`). New params are all optional.
- The response envelope keeps the same JSON field names (`codelists`, `items`). The new `nextOffset` field is an optional, additive field that the existing Tauri HTTP client (`http/terminology/code_list.rs::list`, `http/terminology/code_item.rs::list`) ignores when it unwraps `resp.codelists` / `resp.items` to a `Vec`.
- The existing Tauri commands (`list_code_lists(versionId) -> Vec<CodeListView>`, `list_code_items(codelistId) -> Vec<CodeItemView>`) keep their signatures and continue to call the HTTP endpoints with only the required param.
- The two `useMemo` client-side filters in `TerminologyPage.tsx` and `CodeListDetailPage.tsx` are unchanged; they continue to filter the full loaded list in memory.

The Tauri search commands (`search_code_lists`, `search_code_items`) are already unreferenced from the TS layer but stay in the Rust command surface until a follow-up issue removes them. They hit the now-removed `/code-lists/search` and `/code-items/search` server endpoints, so calling them returns 404; this matches their existing dormant state.

Future work (out of scope here): remove the Tauri search commands and switch the frontend's `useMemo` filters to server-side searches using the new `fragment` query param.

---

## 10. Error handling summary

| Input | Behaviour | Error |
| --- | --- | --- |
| `fragment = None` or `Some("")` or whitespace-only | treated as no fragment (plain list path) | — |
| `fragment = Some(s)` with reserved tsquery char (`& \| ! ( ) :`) | reject | `UsecaseError::Validation(DomainError::InvalidFragment)` → 400 |
| `limit = 0` | clamp to 50 | — |
| `limit > 500` | clamp to 500 | — |
| `offset >= total` | repo returns empty page | `next_offset = None` |
| `parent_id` has no rows | repo returns empty page | `next_offset = None` |

---

## 11. Files changed (summary)

**Domain (`lib/crates/terminology/src/domain/`)**
- `code_list.rs` — add `CodeListListQuery`, drop `CodeListSearchQuery` / `CodeListSearchHit`
- `code_item.rs` — add `CodeItemListQuery`, drop `CodeItemSearchQuery` / `CodeItemSearchHit`
- `error.rs` — drop `EmptyFragment`, add `InvalidFragment`
- `repository.rs` — replace `list_by_*` + `search` with `search_or_list`
- `paging.rs` (new) — `Page<T>`

**Postgres (`lib/crates/terminology/src/adapter/persistence/postgres/`)**
- `code_list_repo.rs` — replace `list_by_version` + `search` with `search_or_list`
- `code_item_repo.rs` — replace `list_by_codelist` + `search` with `search_or_list`

**Usecase (`lib/crates/terminology/src/usecase/`)**
- `terminology_usecase.rs` — replace `list_code_lists` / `search_code_lists` with one method, same for items; drop `validate_fragment`, add `validate_tsquery_fragment`; drop `DomainError::EmptyFragment` reference
- `views.rs` — drop the `pub use crate::domain::{CodeItemSearchHit, CodeListSearchHit};` re-export at the bottom of the file (these types no longer exist)
- `commands.rs` — no change

**API (`lib/crates/apis/src/terminology.rs`)**
- Add `Page<T>`, `CodeListListQuery`, `CodeItemListQuery`
- Drop `CodeListSearchQuery`, `CodeListSearchHit`, `CodeItemSearchQuery`, `CodeItemSearchHit`
- Replace `list_code_lists` / `search_code_lists` / `list_code_items` / `search_code_items` trait methods

**Service adapter (`lib/crates/terminology/src/adapter/facade/in_memory/service.rs`)**
- Replace impls of the four methods with two
- `code_list_view_from_internal` / `code_item_view_from_internal` unchanged

**In-memory fakes (`lib/crates/terminology/src/usecase/tests.rs` + `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs`)**
- Update `FakeCodeListRepo::search_or_list` / `FakeCodeItemRepo::search_or_list`
- Update `InMemoryCodeListRepo::search_or_list` / `InMemoryCodeItemRepo::search_or_list`
- Drop `search_code_lists` / `search_code_items` test cases

**HTTP (`apps/server/aegis-server/src/transport/http/terminology/`)**
- `dto.rs` (or wherever shared DTOs live): add `CodeListListQuery`, `CodeItemListQuery`, `PagedCodeListsResponse`, `PagedCodeItemsResponse`; drop `TerminologySearchBaseQuery`, `*SearchHitResponse`, `*SearchHitsResponse`, `*SearchQueryRequest`
- `handlers.rs` — replace `list_code_lists` / `search_code_lists` / `list_code_items` / `search_code_items` handlers with two; keep `list_code_items_by_version_and_code`
- `router.rs` — drop search route entries
- `handlers::tests` — drop search tests, add pagination/fragment tests
- `openapi.rs` — drop search schema references

**Stubs (carry the trait impl)**
- `apps/server/aegis-server/src/state.rs` (`NullTerminologyService`)
- `apps/server/aegis-server/src/transport/http/router.rs` (`StubTerminologyService`)
- `apps/server/aegis-server/tests/integration_auth.rs` (`NullTerminologyService`)

**Integration tests (`lib/crates/terminology/tests/integration_persistence.rs`)**
- Add: `list_code_lists_paginates_across_multiple_pages`, `list_code_lists_with_fragment_returns_ranked_matches`, `list_code_lists_with_empty_fragment_returns_plain_list`, `list_code_lists_rejects_invalid_fragment`, plus the same four for items.
- Remove: existing search-method integration checks (search moves into the unified method).

**No files under `apps/desktop/aegis-desktop/` are touched by this refactor.** See section 9.

---

## 12. Testing

### 12.1 Unit (terminology crate)

`usecase/tests.rs` — usecase-level, uses fakes:

- `list_code_lists_returns_empty_page_when_no_codelists_exist` → `{ items: [], next_offset: None }`.
- `list_code_lists_with_no_fragment_orders_by_id` — seed 5 rows with non-sequential ids, expect ascending.
- `list_code_lists_with_fragment_filters_results` — seed 5 rows where 2 contain the fragment, expect those 2.
- `list_code_lists_paginates_with_offset_and_limit` — seed 5 rows; `offset=2, limit=2` returns rows 3-4 with `next_offset = Some(4)`; `offset=4, limit=2` returns 1 row with `next_offset = None`.
- `list_code_lists_rejects_tsquery_metacharacters` — `fragment = Some("foo&bar")` → `UsecaseError::Validation(DomainError::InvalidFragment)`.
- `list_code_lists_clamps_limit_to_default_when_zero` — `limit = 0` → internal call uses 50.
- `list_code_lists_clamps_limit_to_max_when_exceeded` — `limit = 10_000` → clamped to 500.
- Same shape for `list_code_items`.

`adapter/facade/in_memory/tests.rs` — service-adapter level:

- `list_code_lists_returns_first_page_with_next_offset_when_more_pages_exist`.
- `list_code_lists_returns_no_next_offset_when_page_is_last`.
- `list_code_lists_with_fragment_filters_via_adapter`.
- `list_code_lists_returns_validation_error_for_invalid_fragment` → `TerminologyApiError::Validation(_)`.
- Same shape for `list_code_items`.

### 12.2 Integration (Postgres)

`lib/crates/terminology/tests/integration_persistence.rs`:

- `list_code_lists_paginates_across_multiple_pages` — seed 7 rows, page through with `limit=3`, verify ordering and `next_offset` transitions.
- `list_code_lists_with_fragment_returns_ranked_matches` — seed rows with overlapping tsv content; verify `ts_rank DESC, id ASC` order.
- `list_code_lists_with_empty_fragment_returns_plain_list` — `fragment = Some("")` matches `fragment = None`.
- `list_code_lists_rejects_invalid_fragment` — usecase-level, but exercised through the full Postgres wiring.
- Same shape for `list_code_items`.

Run with `cargo test -p terminology --test integration_persistence -- --ignored` and the live `AEGIS_TERMINOLOGY_DATABASE_URL`.

### 12.3 HTTP (aegis-server)

`apps/server/aegis-server/src/transport/http/terminology/handlers::tests`:

- Drop the existing `search_*_handler_*` tests.
- Add `list_code_lists_returns_first_page_with_next_offset`, `list_code_lists_returns_empty_page_when_no_codelists`, `list_code_lists_with_fragment_filters`, `list_code_lists_with_invalid_fragment_returns_400`, `list_code_lists_paginates` — and the matching five for items.

### 12.4 Tauri coverage

No Tauri (`apps/desktop/aegis-desktop/`) test fixtures are updated by this refactor. The existing Tauri wiremock tests for the list endpoints keep their un-paged JSON fixtures and still pass because the wire shape is backwards-compatible (the client unwraps `codelists` / `items` to a `Vec` and ignores `nextOffset`).

---

## 13. Verification

- `cargo build -p apis -p terminology -p aegis-server --tests`
- `cargo test -p terminology --lib` (60 tests today → grows by ~14 new tests)
- `cargo test -p aegis-server --lib transport::http::terminology`
- `cargo test -p terminology --test integration_persistence -- --ignored` against live Postgres
- `cargo test -p aegis-desktop --lib` (sanity check — must remain green; no fixtures are modified)

## 14. Out of scope

- Cursor / keyset pagination
- Total count in the response
- Substring / ILIKE search (FTS only)
- Configurable sort order
- New tsv columns
- Touching any file under `apps/desktop/aegis-desktop/` (see section 9)
- Replacing the Tauri `search_code_lists` / `search_code_items` commands with server-side search wiring
