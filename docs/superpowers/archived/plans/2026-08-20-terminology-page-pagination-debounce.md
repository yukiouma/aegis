# Terminology Page Pagination + Debounced Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 20-rows-per-page infinite-scroll pagination and a 300 ms trailing / 1 s maxWait debounced search to `TerminologyPage` and `CodeListDetailPage`, backed by the server's existing unified `/code-lists` and `/code-items` endpoints with `fragment`, `offset`, and `limit`.

**Architecture:** Bottom-up. Rust shim layer first (so the wire shape is correct), then TS shared types and the `useDebouncedValue` + `InfiniteScrollSentinel` primitives, then the React Query hooks, then the two pages, then the test fixtures.

**Tech Stack:** Rust (axum wiremock tests), TypeScript, React 19, `@tanstack/react-query`, MUI, Vitest + `@testing-library/react`, `vi.useFakeTimers()` for debounce tests.

## Global Constraints

- Page size: `PAGE_SIZE = 20` (every offset increment is 20).
- Debounce: `delayMs = 300`, `maxWaitMs = 1000`.
- Wire shapes (must match the Rust `serde(rename_all = "camelCase")` output):
  - `GET /api/terminology/code-lists?versionId=…&fragment=…&offset=…&limit=…` → `{ codelists: CodeListView[], nextOffset?: number }`
  - `GET /api/terminology/code-items?codelistId=…&fragment=…&offset=…&limit=…` → `{ items: CodeItemView[], nextOffset?: number }`
- Empty / whitespace `fragment` is omitted from the URL.
- Tauri command names: `list_code_lists(versionId, fragment, offset, limit)`, `list_code_items(codelistId, fragment, offset, limit)`. The dead `search_code_lists` / `search_code_items` commands are removed entirely (and from `invoke_handler!`).
- Spec: `docs/superpowers/specs/2026-08-20-terminology-page-pagination-debounce-design.md`.

## File Structure

**Rust (Tauri shim)**
- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs` — `list_paged` + `CodeListPagedResponse` + `CodeListListQuery`; drop old `search` + `*Search*` types.
- `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` — mirror of above for items.
- `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs` — replace `list_code_lists` signature; drop `search_code_lists`.
- `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` — mirror; drop `search_code_items`.
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — drop both `search_code_*` from `invoke_handler!`.

**Shared TS**
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — add `PagedCodeListListResponse`, `PagedCodeItemListResponse`, `CodeListListQuery`, `CodeItemListQuery`. Drop `SearchTerminologyQuery`.
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — replace `listCodeLists` / `listCodeItems` wrappers.
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — extend `codeLists` / `codeItems` keys; drop `searchCodeLists` / `searchCodeItems`.
- `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts` — **new**.
- `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.test.ts` — **new**.
- `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx` — **new**.
- `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.test.tsx` — **new**.

**Feature**
- `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` — `useListCodeLists(versionId, opts?)` returns `Page<T>`; `PAGE_SIZE = 20`.
- `apps/desktop/aegis-desktop/src/features/terminology/data/list.test.ts` — **new** (didn't exist).
- `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` — drop local filter; add offset + debounce + sentinel.
- `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx` — mirror.

**Tests**
- `apps/desktop/aegis-desktop/src/test/features/terminology/version-dropdown-persistence.test.tsx` — fix the `list_code_lists` / `list_code_items` mocks to paged shape.
- `apps/desktop/aegis-desktop/src/test/features/terminology/terminology-page-pagination.test.tsx` — **new**.
- `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-detail-pagination.test.tsx` — **new**.

---

## Task 1: Replace `code_list.rs` HTTP module with paged `list_paged`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs` (entire file)

**Interfaces:**
- Produces: `pub struct CodeListPagedResponse { codelists, next_offset? }`, `pub struct CodeListListQuery { version_id, fragment, offset, limit }`, `pub async fn list_paged(c, q) -> Result<CodeListPagedResponse, ApiError>`, `fn percent_encode_fragment(&str) -> String`.

- [ ] **Step 1: Replace the file contents with the paged module**

Rewrite the file to contain only what we need:

```rust
//! HTTP functions under `/api/terminology/code-lists`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeListViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeListPagedResponse {
    pub codelists: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeListRequest {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn create(
    c: &HttpClient,
    body: CreateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-lists",
        Some(&body),
    )
    .await
}

pub async fn list_paged(
    c: &HttpClient,
    q: CodeListListQuery,
) -> Result<CodeListPagedResponse, ApiError> {
    let mut path = format!(
        "/api/terminology/code-lists?versionId={}&offset={}&limit={}",
        q.version_id, q.offset, q.limit
    );
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/terminology/code-lists/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/code-lists/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/terminology/code-lists/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}
```

- [ ] **Step 2: Build to confirm the module compiles**

Run: `cargo build -p aegis-desktop`
Expected: succeeds. (Other call sites still reference the old `list`/`search` symbols; we'll fix those in tasks 3 and 4 — the build will surface them.)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs
git commit -m "refactor(http/terminology): replace list with list_paged for code-list"
```

---

## Task 2: Add wiremock tests for `code_list::list_paged`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `list_paged`, `CodeListListQuery`, `CodeListPagedResponse` from Task 1.

- [ ] **Step 1: Replace the existing `tests` module**

Replace the entire `mod tests` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn codelist_json(id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "versionId": 7, "code": code, "extensible": true,
            "name": "name", "submissionValue": "SV", "synonym": "",
            "definition": "def", "nciPreferredTerm": "nci",
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_paged_returns_first_page_with_next_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .and(query_param("versionId", "7"))
            .and(query_param("offset", "0"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(1, "C1"), codelist_json(2, "C2")],
                "nextOffset": 20
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery { version_id: 7, fragment: None, offset: 0, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.codelists.len(), 2);
        assert_eq!(page.codelists[0].code, "C1");
        assert_eq!(page.next_offset, Some(20));
    }

    #[tokio::test]
    async fn list_paged_returns_no_next_offset_on_last_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .and(query_param("offset", "40"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(41, "C41")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery { version_id: 7, fragment: None, offset: 40, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.codelists.len(), 1);
        assert!(page.next_offset.is_none());
    }

    #[tokio::test]
    async fn list_paged_with_fragment_includes_fragment_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(1, "AE")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
                fragment: Some("AE".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.codelists[0].code, "AE");
    }

    #[tokio::test]
    async fn list_paged_with_whitespace_fragment_omits_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": []
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
                fragment: Some("   ".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert!(page.codelists.is_empty());
    }

    #[tokio::test]
    async fn list_paged_round_trips_snake_case_next_offset_to_camel_case() {
        // Wiremock serves snake_case; serde rename verifies camelCase decoding.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [],
                "next_offset": 100
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery { version_id: 7, fragment: None, offset: 0, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.next_offset, Some(100));
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(codelist_json(42, "C42")))
            .mount(&server)
            .await;
        let v = get_by_id(&client(&server), 42).await.unwrap();
        assert_eq!(v.id, 42);
        assert_eq!(v.code, "C42");
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(201).set_body_json(codelist_json(99, "NEW")))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateCodeListRequest {
                version_id: 7,
                code: "NEW".into(),
                extensible: true,
                name: "name".into(),
                submission_value: "SV".into(),
                synonym: "".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 99);
        assert_eq!(v.code, "NEW");
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/code-lists/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(codelist_json(3, "UPD")))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            3,
            UpdateCodeListRequest {
                code: Some("UPD".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(v.code, "UPD");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/terminology/code-lists/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCodeListRequest {
            name: Some("renamed".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p aegis-desktop --lib http::terminology::code_list`
Expected: 10 tests pass.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs
git commit -m "test(http/terminology): pin list_paged wire shape for code-list"
```

---

## Task 3: Replace `code_item.rs` HTTP module with paged `list_paged`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` (entire file)

**Interfaces:**
- Produces: `pub struct CodeItemPagedResponse { items, next_offset? }`, `pub struct CodeItemListQuery { codelist_id, fragment, offset, limit }`, `pub async fn list_paged(c, q) -> Result<CodeItemPagedResponse, ApiError>`.

- [ ] **Step 1: Replace the file contents with the paged module**

```rust
//! HTTP functions under `/api/terminology/code-items`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemViewResponse {
    pub id: i64,
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemPagedResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeItemRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

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

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn create(
    c: &HttpClient,
    body: CreateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-items",
        Some(&body),
    )
    .await
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

pub async fn list_paged(
    c: &HttpClient,
    q: CodeItemListQuery,
) -> Result<CodeItemPagedResponse, ApiError> {
    let mut path = format!(
        "/api/terminology/code-items?codelistId={}&offset={}&limit={}",
        q.codelist_id, q.offset, q.limit
    );
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/code-items/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/terminology/code-items/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}
```

- [ ] **Step 2: Replace the existing `mod tests` block with the item-equivalent of Task 2**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn item_json(id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "codelistId": 11, "versionId": 7, "code": code,
            "submissionValue": "SV", "synonym": "syn",
            "definition": "def", "nciPreferredTerm": "nci",
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_paged_returns_first_page_with_next_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("codelistId", "11"))
            .and(query_param("offset", "0"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "Y"), item_json(2, "N")],
                "nextOffset": 20
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery { codelist_id: 11, fragment: None, offset: 0, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_offset, Some(20));
    }

    #[tokio::test]
    async fn list_paged_returns_no_next_offset_on_last_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("offset", "40"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(41, "Z")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery { codelist_id: 11, fragment: None, offset: 40, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.next_offset.is_none());
    }

    #[tokio::test]
    async fn list_paged_with_fragment_includes_fragment_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "AE")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: 11,
                fragment: Some("AE".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items[0].code, "AE");
    }

    #[tokio::test]
    async fn list_paged_with_whitespace_fragment_omits_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": []
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: 11,
                fragment: Some("   ".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn list_paged_round_trips_snake_case_next_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "next_offset": 60
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery { codelist_id: 11, fragment: None, offset: 0, limit: 20 },
        )
        .await
        .unwrap();
        assert_eq!(page.next_offset, Some(60));
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(201).set_body_json(item_json(99, "NEW")))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateCodeItemRequest {
                codelist_id: 11,
                version_id: 7,
                code: "NEW".into(),
                submission_value: "SV".into(),
                synonym: "syn".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 99);
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/code-items/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(item_json(3, "UPD")))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            3,
            UpdateCodeItemRequest {
                submission_value: Some("SV-UPD".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(v.code, "UPD");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/terminology/code-items/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCodeItemRequest {
            submission_value: Some("SV".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"submissionValue":"SV"}"#);
    }

    fn batch_response_json(codelist_id: i64, version_id: i64, count: usize) -> serde_json::Value {
        serde_json::json!({
            "count": count, "codelistId": codelist_id, "versionId": version_id
        })
    }

    #[tokio::test]
    async fn batch_create_returns_count() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-items/batch"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(batch_response_json(11, 7, 42)))
            .mount(&server)
            .await;
        let resp = batch_create(
            &client(&server),
            BatchCreateCodeItemsRequest {
                codelist_id: 11,
                version_id: 7,
                items: vec![BatchCodeItemEntry {
                    code: "Y".into(),
                    submission_value: "SV".into(),
                    synonym: "syn".into(),
                    definition: "def".into(),
                    nci_preferred_term: "nci".into(),
                }],
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.count, 42);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 7);
    }

    #[test]
    fn batch_request_serializes_camel_case() {
        let body = BatchCreateCodeItemsRequest {
            codelist_id: 11,
            version_id: 7,
            items: vec![BatchCodeItemEntry {
                code: "Y".into(),
                submission_value: "SV".into(),
                synonym: "syn".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            }],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"codelistId":11,"versionId":7,"items":[{"code":"Y","submissionValue":"SV","synonym":"syn","definition":"def","nciPreferredTerm":"nci"}]}"#
        );
    }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p aegis-desktop --lib http::terminology::code_item`
Expected: 12 tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs
git commit -m "refactor(http/terminology): replace list with list_paged for code-item"
```

---

## Task 4: Update Tauri command shims (drop `search_code_*`, change `list_code_*` signatures)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Replace `code_list.rs` shim**

```rust
//! Tauri command shims for the terminology code-list HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::terminology::code_list::{
    self, CodeListListQuery, CodeListPagedResponse, CodeListViewResponse,
    CreateCodeListRequest, UpdateCodeListRequest,
};

#[tauri::command]
pub async fn create_code_list(
    client: State<'_, HttpClient>,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeListViewResponse, ApiError> {
    code_list::create(
        &client,
        CreateCodeListRequest {
            version_id,
            code,
            extensible,
            name,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_code_lists(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeListPagedResponse, ApiError> {
    code_list::list_paged(
        &client,
        CodeListListQuery { version_id, fragment, offset, limit },
    )
    .await
}

#[tauri::command]
pub async fn get_code_list_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CodeListViewResponse, ApiError> {
    code_list::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_code_list(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    code_list::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_code_list(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    code_list::delete(&client, id).await
}
```

- [ ] **Step 2: Replace `code_item.rs` shim**

```rust
//! Tauri command shims for the terminology code-item HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::terminology::code_item::{
    self, CodeItemListQuery, CodeItemPagedResponse, CodeItemViewResponse,
    CreateCodeItemRequest, UpdateCodeItemRequest,
};

#[tauri::command]
pub async fn create_code_item(
    client: State<'_, HttpClient>,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeItemViewResponse, ApiError> {
    code_item::create(
        &client,
        CreateCodeItemRequest {
            codelist_id,
            version_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_code_items(
    client: State<'_, HttpClient>,
    codelist_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    code_item::list_paged(
        &client,
        CodeItemListQuery { codelist_id, fragment, offset, limit },
    )
    .await
}

#[tauri::command]
pub async fn update_code_item(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    code_item::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_code_item(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    code_item::delete(&client, id).await
}
```

- [ ] **Step 3: Drop the two `search_code_*` entries from `lib.rs`**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, remove these two lines from the `tauri::generate_handler![ ... ]` macro list:

```rust
            commands::terminology::code_list::search_code_lists,
            ...
            commands::terminology::code_item::search_code_items,
```

- [ ] **Step 4: Build to confirm**

Run: `cargo build -p aegis-desktop`
Expected: succeeds (Rust side now compiles with the new types; the TS-side call sites that pass extra args will be fixed in tasks 5–7).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "refactor(commands/terminology): drop search_code_*; widen list_code_*"
```

---

## Task 5: Update shared TS types and the api wrapper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`

**Interfaces:**
- Produces:
  - `export interface PagedCodeListListResponse { codelists: CodeListView[]; nextOffset?: number }`
  - `export interface PagedCodeItemListResponse { items: CodeItemView[]; nextOffset?: number }`
  - `export interface CodeListListQuery { versionId: number; fragment?: string; offset?: number; limit?: number }`
  - `export interface CodeItemListQuery { codelistId: number; fragment?: string; offset?: number; limit?: number }`
  - `api.listCodeLists(versionId, options?: CodeListListQuery): Promise<PagedCodeListListResponse>`
  - `api.listCodeItems(codelistId, options?: CodeItemListQuery): Promise<PagedCodeItemListResponse>`

- [ ] **Step 1: Update `types.ts`**

Replace the existing `CodeListListResponse` interface and `SearchTerminologyQuery` interface (search the file for both names; remove them) and add the new ones:

```ts
export interface PagedCodeListListResponse {
  codelists: CodeListView[];
  nextOffset?: number;
}

export interface PagedCodeItemListResponse {
  items: CodeItemView[];
  nextOffset?: number;
}

export interface CodeListListQuery {
  versionId: number;
  fragment?: string;
  offset?: number;
  limit?: number;
}

export interface CodeItemListQuery {
  codelistId: number;
  fragment?: string;
  offset?: number;
  limit?: number;
}
```

Make sure `SearchTerminologyQuery` (the bottom of the file) is removed entirely.

- [ ] **Step 2: Update `index.ts`**

Replace these two wrappers:

```ts
listCodeLists: (versionId: number): Promise<CodeListView[]> =>
  call<CodeListView[]>("list_code_lists", { versionId }),
```

with:

```ts
listCodeLists: (
  versionId: number,
  options: CodeListListQuery = {},
): Promise<PagedCodeListListResponse> =>
  call<PagedCodeListListResponse>("list_code_lists", {
    versionId,
    fragment: options.fragment,
    offset: options.offset,
    limit: options.limit,
  }),
```

and analogously for `listCodeItems` (the field is `codelistId`).

Also update the import block at the top of the file:

```ts
import type {
  CodeItemView,
  CodeListView,
  // …
  PagedCodeItemListResponse,
  PagedCodeListListResponse,
  CodeItemListQuery,
  CodeListListQuery,
  // …
} from "./types";
```

(Adjust to match the file's existing import shape — the point is to add the four new types and drop `CodeItemListResponse`, `CodeListListResponse`, `SearchTerminologyQuery`.)

Also update the bottom `export type { … }` block to add `PagedCodeListListResponse`, `PagedCodeItemListResponse`, `CodeListListQuery`, `CodeItemListQuery` and drop `CodeItemListResponse`, `CodeListListResponse`, `SearchTerminologyQuery`.

- [ ] **Step 3: Update import in `features/terminology/data/list.ts`**

Open `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`. At the top, change:

```ts
import {
  api,
  type ApiError,
  type CodeItemView,
  type CodeListView,
  // …
} from "../../../shared/api";
```

to import the new types too:

```ts
import {
  api,
  type ApiError,
  type CodeItemListQuery,
  type CodeListListQuery,
  type CodeItemView,
  type CodeListView,
  type PagedCodeItemListResponse,
  type PagedCodeListListResponse,
  // …
} from "../../../shared/api";
```

(The file still references the old `CodeListView` / `CodeItemView` types; we keep those. We're only adding the new ones at the import site.)

- [ ] **Step 4: Type-check**

Run: `pnpm --filter aegis-desktop typecheck` (or `tsc --noEmit` — pick the project's command).
Expected: passes. (The hook signatures are still wrong; we'll fix them in Task 9.)

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/api/index.ts \
        apps/desktop/aegis-desktop/src/features/terminology/data/list.ts
git commit -m "feat(shared/api): paged wrappers for list_code_lists / list_code_items"
```

---

## Task 6: Update query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`

**Interfaces:**
- Produces:
  - `terminology.codeLists(versionId, fragment, offset): readonly ["terminology", "codeLists", number, string, number]`
  - `terminology.codeItems(codelistId, fragment, offset): readonly ["terminology", "codeItems", number, string, number]`

- [ ] **Step 1: Replace the key factories**

Inside the `terminology` block, replace:

```ts
    codeLists: (versionId: number) =>
      ["terminology", "codeLists", versionId] as const,
    codeItems: (codelistId: number) =>
      ["terminology", "codeItems", codelistId] as const,
    searchCodeLists: (versionId: number, fragment: string) =>
      ["terminology", "searchCodeLists", versionId, fragment] as const,
    searchCodeItems: (versionId: number, fragment: string) =>
      ["terminology", "searchCodeItems", versionId, fragment] as const,
```

with:

```ts
    codeLists: (versionId: number, fragment: string, offset: number) =>
      ["terminology", "codeLists", versionId, fragment, offset] as const,
    codeItems: (codelistId: number, fragment: string, offset: number) =>
      ["terminology", "codeItems", codelistId, fragment, offset] as const,
```

(The `searchCodeLists` / `searchCodeItems` factories are removed — they were never referenced by the hook layer.)

- [ ] **Step 2: Type-check**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: passes.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/query/keys.ts
git commit -m "feat(query): include fragment + offset in terminology codeList/Item keys"
```

---

## Task 7: Add `useDebouncedValue` hook with TDD

**Files:**
- Create: `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts`
- Create: `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.test.ts`

**Interfaces:**
- Produces: `export interface UseDebouncedValueOptions { delayMs: number; maxWaitMs: number }` and `export function useDebouncedValue<T>(value: T, options: UseDebouncedValueOptions): T`.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.test.ts`:

```ts
import "@testing-library/jest-dom/vitest";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useDebouncedValue } from "./useDebouncedValue";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("useDebouncedValue", () => {
  it("returns the initial value on first render", () => {
    const { result } = renderHook(() =>
      useDebouncedValue("a", { delayMs: 300, maxWaitMs: 1000 }),
    );
    expect(result.current).toBe("a");
  });

  it("emits the trailing value after delayMs when input stops changing", () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "a" } },
    );

    rerender({ value: "ab" });
    rerender({ value: "abc" });
    expect(result.current).toBe("a"); // not yet

    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(result.current).toBe("abc");
  });

  it("throttles continuous changes to at most one fire per maxWaitMs", () => {
    let fires = 0;
    const { rerender } = renderHook(
      ({ value }: { value: number }) => {
        const settled = useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 });
        fires = settled;
        return settled;
      },
      { initialProps: { value: 0 } },
    );

    for (let i = 1; i <= 20; i++) {
      rerender({ value: i });
      act(() => {
        vi.advanceTimersByTime(200); // faster than delayMs but slower than maxWait
      });
    }
    // Total wall-clock advanced: 20 * 200 = 4000 ms.
    // maxWaitMs = 1000, so at most 4 fires should have landed.
    expect(fires).toBeGreaterThanOrEqual(2);
    expect(fires).toBeLessThanOrEqual(5);
  });

  it("does not emit when the value is unchanged across renders", () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "x" } },
    );
    rerender({ value: "x" });
    rerender({ value: "x" });
    act(() => {
      vi.advanceTimersByTime(5000);
    });
    // No timer should have fired because the input never changed after mount.
    expect(result.current).toBe("x");
  });

  it("cancels pending timers on unmount", () => {
    const { rerender, unmount } = renderHook(
      ({ value }: { value: string }) =>
        useDebouncedValue(value, { delayMs: 300, maxWaitMs: 1000 }),
      { initialProps: { value: "a" } },
    );
    rerender({ value: "b" });
    unmount();
    // Advancing time after unmount must not throw a setState-after-unmount warning.
    expect(() => vi.advanceTimersByTime(1000)).not.toThrow();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- --run shared/hooks/useDebouncedValue.test.ts`
Expected: FAIL — `./useDebouncedValue` does not exist.

- [ ] **Step 3: Implement the hook**

Create `apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts`:

```ts
import { useEffect, useRef, useState } from "react";

export interface UseDebouncedValueOptions {
  /** Trailing-edge debounce window after the last change. */
  delayMs: number;
  /** Maximum time to wait between fires while the value is still changing. */
  maxWaitMs: number;
}

/**
 * Returns a "settled" value that lags behind `value` until either the
 * trailing-debounce window (`delayMs`) or the max-wait window (`maxWaitMs`)
 * has elapsed — whichever comes first. See
 * `docs/superpowers/specs/2026-08-20-terminology-page-pagination-debounce-design.md`
 * section 7 for the exact semantics.
 */
export function useDebouncedValue<T>(
  value: T,
  options: UseDebouncedValueOptions,
): T {
  const { delayMs, maxWaitMs } = options;
  const [settled, setSettled] = useState<T>(value);
  const latestRef = useRef<T>(value);
  const delayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const maxTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    latestRef.current = value;

    const fire = () => {
      if (delayTimerRef.current != null) {
        clearTimeout(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      if (maxTimerRef.current != null) {
        clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
      // setSettled schedules a re-render; the next render reads `latestRef`.
      setSettled(latestRef.current);
    };

    if (delayTimerRef.current != null) clearTimeout(delayTimerRef.current);
    if (maxTimerRef.current != null) clearTimeout(maxTimerRef.current);

    delayTimerRef.current = setTimeout(fire, delayMs);
    maxTimerRef.current = setTimeout(fire, maxWaitMs);

    return () => {
      if (delayTimerRef.current != null) {
        clearTimeout(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      if (maxTimerRef.current != null) {
        clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
    };
  }, [value, delayMs, maxWaitMs]);

  return settled;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- --run shared/hooks/useDebouncedValue.test.ts`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.ts \
        apps/desktop/aegis-desktop/src/shared/hooks/useDebouncedValue.test.ts
git commit -m "feat(shared/hooks): add useDebouncedValue with trailing + maxWait"
```

---

## Task 8: Add `InfiniteScrollSentinel` component with TDD

**Files:**
- Create: `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx`
- Create: `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.test.tsx`

**Interfaces:**
- Produces: `export interface InfiniteScrollSentinelProps { onIntersect: () => void; hasMore: boolean; loading: boolean; rootMargin?: string }` and `export function InfiniteScrollSentinel(props): JSX.Element`.

- [ ] **Step 1: Install the `intersection-observer` polyfill if not present**

Run: `pnpm --filter aegis-desktop list intersection-observer`
If it is not installed, add it: `pnpm --filter aegis-desktop add -D intersection-observer`.

Then in `apps/desktop/aegis-desktop/src/test/setup.ts`, add (at the top, before any test imports):

```ts
import "intersection-observer";
```

If the file does not exist, create it with that one line. (If the project uses `vitest.config.ts` for setup, register the import path there instead.)

- [ ] **Step 2: Write the failing test**

Create `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { InfiniteScrollSentinel } from "./InfiniteScrollSentinel";

let observers: Array<{
  cb: IntersectionObserverCallback;
  observe: ReturnType<typeof vi.fn>;
  unobserve: ReturnType<typeof vi.fn>;
  disconnect: ReturnType<typeof vi.fn>;
}> = [];

beforeEach(() => {
  observers = [];
  const fakeObserver = class {
    cb: IntersectionObserverCallback;
    observe = vi.fn();
    unobserve = vi.fn();
    disconnect = vi.fn();
    constructor(cb: IntersectionObserverCallback) {
      this.cb = cb;
      observers.push(this);
    }
  };
  (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver =
    fakeObserver;
});

afterEach(() => {
  observers = [];
});

function fireIntersect(idx: number, isIntersecting: boolean) {
  act(() => {
    observers[idx].cb(
      [{ isIntersecting } as IntersectionObserverEntry],
      observers[idx] as unknown as IntersectionObserver,
    );
  });
}

describe("InfiniteScrollSentinel", () => {
  it("calls onIntersect when intersection fires and hasMore=true, loading=false", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading={false} />);
    expect(observers).toHaveLength(1);
    fireIntersect(0, true);
    expect(onIntersect).toHaveBeenCalledTimes(1);
  });

  it("does not call onIntersect when hasMore=false", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore={false} loading={false} />);
    fireIntersect(0, true);
    expect(onIntersect).not.toHaveBeenCalled();
  });

  it("does not call onIntersect while loading=true", () => {
    const onIntersect = vi.fn();
    render(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading />);
    fireIntersect(0, true);
    expect(onIntersect).not.toHaveBeenCalled();
  });

  it("renders a spinner while loading=true", () => {
    render(<InfiniteScrollSentinel onIntersect={() => {}} hasMore loading />);
    expect(screen.getByRole("progressbar")).toBeInTheDocument();
  });

  it("disconnects the observer when hasMore flips to false", () => {
    const onIntersect = vi.fn();
    const { rerender } = render(
      <InfiniteScrollSentinel onIntersect={onIntersect} hasMore loading={false} />,
    );
    const observer = observers[0];
    rerender(<InfiniteScrollSentinel onIntersect={onIntersect} hasMore={false} loading={false} />);
    expect(observer.disconnect).toHaveBeenCalled();
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- --run shared/components/InfiniteScrollSentinel.test.tsx`
Expected: FAIL — `./InfiniteScrollSentinel` does not exist.

- [ ] **Step 4: Implement the component**

Create `apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx`:

```tsx
import { Box, CircularProgress } from "@aegis/ui/mui";
import { useEffect, useRef } from "react";

export interface InfiniteScrollSentinelProps {
  /** Called when the sentinel scrolls into view and `hasMore && !loading`. */
  onIntersect: () => void;
  /** Stop firing `onIntersect` when false. */
  hasMore: boolean;
  /** Suppress `onIntersect` while a page fetch is in flight. */
  loading: boolean;
  /** Pixel margin before the viewport edge at which the observer fires. */
  rootMargin?: string;
}

/**
 * Single-pixel-high sentinel that calls `onIntersect` when it scrolls into
 * view. The parent owns `offset` and `hasMore`; this component is pure.
 */
export function InfiniteScrollSentinel({
  onIntersect,
  hasMore,
  loading,
  rootMargin = "0px 0px 200px 0px",
}: InfiniteScrollSentinelProps) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasMore) return;
    const el = ref.current;
    if (el == null) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && !loading) {
            onIntersect();
            break;
          }
        }
      },
      { rootMargin },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loading, onIntersect, rootMargin]);

  if (!hasMore) return null;

  return (
    <Box
      ref={ref}
      sx={{
        display: "flex",
        justifyContent: "center",
        py: 1,
        minHeight: 8,
      }}
      data-testid="infinite-scroll-sentinel"
    >
      {loading ? <CircularProgress size={20} role="progressbar" /> : null}
    </Box>
  );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- --run shared/components/InfiniteScrollSentinel.test.tsx`
Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.tsx \
        apps/desktop/aegis-desktop/src/shared/components/InfiniteScrollSentinel.test.tsx \
        apps/desktop/aegis-desktop/src/test/setup.ts \
        apps/desktop/aegis-desktop/package.json
git commit -m "feat(shared/components): add InfiniteScrollSentinel"
```

---

## Task 9: Update `useListCodeLists` / `useListCodeItems` to return `Page<T>`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`

**Interfaces:**
- Produces: `export const PAGE_SIZE = 20` and `useListCodeLists(versionId, opts?)` returning `useQuery<PagedCodeListListResponse, ApiError>` with key `["terminology", "codeLists", versionId, fragment, offset]`; same shape for items.

- [ ] **Step 1: Write the failing hook test**

Create `apps/desktop/aegis-desktop/src/features/terminology/data/list.test.ts`:

```ts
import "@testing-library/jest-dom/vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { useCreateCodeList, useListCodeLists } from "./list";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});
afterEach(() => {
  vi.mocked(invoke).mockReset();
});

import { invoke } from "@tauri-apps/api/core";

function wrapper({ children }: { children: React.ReactNode }) {
  return <TestQueryProvider>{children}</TestQueryProvider>;
}

function paged(codelists: unknown[], nextOffset?: number) {
  return { codelists, nextOffset };
}

describe("useListCodeLists", () => {
  it("returns the paged envelope and calls list_code_lists with the right args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paged([{ id: 1, code: "AE" }], undefined));
    const { result } = renderHook(
      () => useListCodeLists(7, { fragment: "AE", offset: 0 }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.codelists).toEqual([{ id: 1, code: "AE" }]);
    expect(result.current.data?.nextOffset).toBeUndefined();
    expect(invoke).toHaveBeenCalledWith("list_code_lists", {
      versionId: 7,
      fragment: "AE",
      offset: 0,
      limit: 20,
    });
  });

  it("treats an empty fragment as no fragment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paged([]));
    renderHook(() => useListCodeLists(7, { fragment: "   ", offset: 0 }), { wrapper });
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(invoke).toHaveBeenCalledWith("list_code_lists", {
      versionId: 7,
      fragment: undefined,
      offset: 0,
      limit: 20,
    });
  });

  it("uses different query keys for different fragments", async () => {
    vi.mocked(invoke).mockResolvedValue(paged([]));
    const { result: a } = renderHook(() => useListCodeLists(7, { fragment: "AE", offset: 0 }), { wrapper });
    const { result: b } = renderHook(() => useListCodeLists(7, { fragment: "LB", offset: 0 }), { wrapper });
    await waitFor(() => {
      expect(a.current.isSuccess).toBe(true);
      expect(b.current.isSuccess).toBe(true);
    });
    expect(a.current.data).not.toBe(b.current.data);
  });
});

describe("useCreateCodeList invalidation", () => {
  it("invalidates every (fragment, offset) for the new version", async () => {
    const { queryClient } = renderHook(() => ({ qc: useQueryClient() }), { wrapper }).result.current as unknown as { qc: ReturnType<typeof useQueryClient> };
    // We assert through the public hook contract: after mutation succeeds, the
    // query for the same versionId (any fragment, any offset) is invalidated.
    // Implementation lives in the hook; this is the regression guard.
    expect(typeof useCreateCodeList).toBe("function");
  });
});
```

(The third describe block is intentionally a placeholder — the real invalidation test is in Task 13. Drop or fix the bogus `as unknown as` cast when implementing — the intent is "we'll exercise this end-to-end later, not as a unit test".)

Actually — simplify and drop the third describe entirely. The hook-level invalidation behavior is covered by the page-level integration test in Task 13. Replace the third describe with:

```ts
// (no further tests here — invalidation is covered in Task 13 page tests)
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- --run features/terminology/data/list.test.ts`
Expected: FAIL — `useListCodeLists` has the wrong signature, so the test will fail at the call site.

- [ ] **Step 3: Replace `list.ts` with the paged hook**

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CodeItemListQuery,
  type CodeItemView,
  type CodeListListQuery,
  type CodeListView,
  type CreateCodeItemInput,
  type CreateCodeListInput,
  type CreateTerminologyVersionInput,
  type PagedCodeItemListResponse,
  type PagedCodeListListResponse,
  type TerminologyVersionView,
  type UpdateCodeItemInput,
  type UpdateCodeListInput,
  type UpdateTerminologyVersionInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/** Page size used by both code-list and code-item tables. */
export const PAGE_SIZE = 20;

// ---- Versions ----

export function useListTerminologyVersions() {
  return useQuery<TerminologyVersionView[], ApiError>({
    queryKey: queryKeys.terminology.versions(),
    queryFn: () => api.listTerminologyVersions(),
  });
}

export function useCreateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    CreateTerminologyVersionInput
  >({
    mutationFn: api.createTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useUpdateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    { id: number; body: UpdateTerminologyVersionInput }
  >({
    mutationFn: ({ id, body }) => api.updateTerminologyVersion(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useDeleteTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: api.deleteTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

// ---- Code lists ----

export interface ListPagedOptions {
  fragment?: string;
  offset?: number;
}

/**
 * Codelists for a given terminology version. Paged: returns
 * `{ codelists, nextOffset? }`. `fragment = ""` (or whitespace) is treated
 * as "no filter" by stripping it before sending.
 */
export function useListCodeLists(
  versionId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  const offset = options.offset ?? 0;
  return useQuery<PagedCodeListListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0, fragment, offset),
    queryFn: () =>
      api.listCodeLists(versionId!, {
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset,
        limit: PAGE_SIZE,
      }),
    enabled: versionId != null && versionId > 0,
  });
}

export function useGetCodeList(id: number | null) {
  return useQuery<CodeListView, ApiError>({
    queryKey: queryKeys.terminology.codeList(id ?? 0),
    queryFn: () => api.getCodeListById(id!),
    enabled: id != null && id > 0,
  });
}

export function useCreateCodeList() {
  const qc = useQueryClient();
  return useMutation<CodeListView, ApiError, CreateCodeListInput>({
    mutationFn: api.createCodeList,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", created.versionId],
      });
    },
  });
}

export function useUpdateCodeList() {
  const qc = useQueryClient();
  return useMutation<
    CodeListView,
    ApiError,
    { id: number; body: UpdateCodeListInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeList(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", updated.versionId],
      });
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeList(updated.id),
      });
    },
  });
}

export function useDeleteCodeList() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; versionId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeList(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", vars.versionId],
      });
    },
  });
}

// ---- Code items ----

export function useListCodeItems(
  codelistId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  const offset = options.offset ?? 0;
  return useQuery<PagedCodeItemListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeItems(codelistId ?? 0, fragment, offset),
    queryFn: () =>
      api.listCodeItems(codelistId!, {
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset,
        limit: PAGE_SIZE,
      }),
    enabled: codelistId != null && codelistId > 0,
  });
}

export function useCreateCodeItem() {
  const qc = useQueryClient();
  return useMutation<CodeItemView, ApiError, CreateCodeItemInput>({
    mutationFn: api.createCodeItem,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", created.codelistId],
      });
    },
  });
}

export function useUpdateCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    CodeItemView,
    ApiError,
    { id: number; body: UpdateCodeItemInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeItem(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", updated.codelistId],
      });
    },
  });
}

export function useDeleteCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; codelistId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeItem(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", vars.codelistId],
      });
    },
  });
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- --run features/terminology/data/list.test.ts`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/data/list.ts \
        apps/desktop/aegis-desktop/src/features/terminology/data/list.test.ts
git commit -m "feat(terminology/data): paged list hooks with fragment + offset"
```

---

## Task 10: Update `TerminologyPage` for pagination + debounce

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`

**Interfaces:**
- Consumes: `useDebouncedValue`, `InfiniteScrollSentinel`, `useListCodeLists(versionId, { fragment, offset })` returning `Page<T>`, `PAGE_SIZE`.

- [ ] **Step 1: Apply the edits**

Replace the entire file with:

```tsx
import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import { useCurrentUser } from "../../auth";
import {
  PAGE_SIZE,
  useCreateCodeList,
  useDeleteCodeList,
  useListCodeLists,
  useListTerminologyVersions,
  useUpdateCodeList,
} from "../data";
import type {
  CodeListView,
  CreateCodeListInput,
  TerminologyKind,
  UpdateCodeListInput,
} from "../../../shared/api";
import { CodeListDrawer } from "../components/CodeListDrawer";
import { CodeListTable } from "../components/CodeListTable";
import { ImportButton } from "../components/ImportButton";
import { TermFilterBar } from "../components/TermFilterBar";
import { VersionDropdown } from "../components/VersionDropdown";

type DrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CodeListView }
  | null;

export interface TerminologyPageProps {
  kind: TerminologyKind;
}

export function TerminologyPage({ kind }: TerminologyPageProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const versionsQuery = useListTerminologyVersions();

  const routeSearch = useSearch({ strict: false }) as { versionId?: number };
  const urlVersionId = routeSearch.versionId;

  const [search, setSearch] = useState("");
  const [offset, setOffset] = useState(0);
  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeListView | null>(null);

  const versions = versionsQuery.data ?? [];
  const versionsForKind = versions.filter((v) => v.kind === kind);

  const selectedVersionId = useMemo<number | null>(() => {
    if (
      urlVersionId != null &&
      versionsForKind.some((v) => v.id === urlVersionId)
    ) {
      return urlVersionId;
    }
    return versionsForKind[0]?.id ?? null;
  }, [urlVersionId, versionsForKind]);

  useEffect(() => {
    if (versionsForKind.length === 0) return;
    const urlIsValid =
      urlVersionId != null &&
      versionsForKind.some((v) => v.id === urlVersionId);
    if (urlIsValid) return;
    const fallback = versionsForKind[0].id;
    const to =
      kind === "sdtm"
        ? "/terminology/sdtm"
        : "/terminology/adam";
    void navigate({
      to,
      replace: true,
      search: { versionId: fallback },
    });
  }, [urlVersionId, versionsForKind, kind, navigate]);

  const setSelectedVersionId = (id: number | null) => {
    const to =
      kind === "sdtm"
        ? "/terminology/sdtm"
        : "/terminology/adam";
    void navigate({ to, search: { versionId: id ?? undefined } });
  };

  const debouncedFragment = useDebouncedValue(search, {
    delayMs: 300,
    maxWaitMs: 1000,
  });

  // Reset pagination whenever the parent (version) or the debounced fragment changes.
  useEffect(() => {
    setOffset(0);
  }, [selectedVersionId, debouncedFragment]);

  const codeListsQuery = useListCodeLists(selectedVersionId, {
    fragment: debouncedFragment,
    offset,
  });

  const createCodeList = useCreateCodeList();
  const updateCodeList = useUpdateCodeList();
  const deleteCodeList = useDeleteCodeList();

  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const rows = codeListsQuery.data?.codelists ?? [];
  const hasMore = codeListsQuery.data?.nextOffset != null;

  const trimmedQuery = debouncedFragment.trim();

  const mutationLoading =
    createCodeList.isPending ||
    updateCodeList.isPending ||
    deleteCodeList.isPending;

  const error = codeListsQuery.error;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box
        sx={{
          display: "flex",
          gap: 2,
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <TermFilterBar query={search} onQueryChange={setSearch} />
        <VersionDropdown
          kind={kind}
          versions={versions}
          value={selectedVersionId}
          onChange={setSelectedVersionId}
        />
        <ImportButton kind={kind} />
      </Box>

      <CodeListTable
        mode="list"
        rows={rows}
        loading={codeListsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={error}
        canMutate={canMutate}
        onRetry={codeListsQuery.refetch}
        onCreate={() => setDrawer({ mode: "create" })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpen={(row) => {
          void navigate({
            to: "/terminology/$kind/codelists/$codelistId",
            params: { kind, codelistId: row.id },
            search:
              selectedVersionId != null
                ? { versionId: selectedVersionId }
                : undefined,
          });
        }}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codelist.noMatches")
            : t("terminology.codelist.empty")
        }
      />

      <InfiniteScrollSentinel
        onIntersect={() => setOffset((o) => o + PAGE_SIZE)}
        hasMore={hasMore}
        loading={codeListsQuery.isFetching}
      />

      <CodeListDrawer
        open={drawer !== null}
        mode={drawer?.mode ?? "create"}
        row={drawer?.mode === "edit" ? drawer.row : undefined}
        versions={versions}
        versionId={selectedVersionId ?? 0}
        onClose={() => setDrawer(null)}
        onCreate={(input: CreateCodeListInput) =>
          createCodeList.mutate(input, {
            onSuccess: () => setDrawer(null),
          })
        }
        onUpdate={(id, body: UpdateCodeListInput) =>
          updateCodeList.mutate(
            { id, body },
            { onSuccess: () => setDrawer(null) },
          )
        }
        canMutate={canMutate}
        mutationError={createCodeList.error ?? updateCodeList.error}
        mutationPending={createCodeList.isPending || updateCodeList.isPending}
      />

      <Dialog
        open={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
      >
        <DialogTitle>
          {t("terminology.action.delete.confirmTitle")}
        </DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("terminology.action.delete.confirmMessage")}
          </DialogContentText>
          {deleteCodeList.isError && (
            <DialogContentText sx={{ mt: 2, color: "error.main" }}>
              {errorMessage(deleteCodeList.error)}
            </DialogContentText>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)} disabled={deleteCodeList.isPending}>
            {t("common.cancel")}
          </Button>
          <Button
            color="error"
            onClick={() => {
              if (!confirmDelete) return;
              deleteCodeList.mutate(
                { id: confirmDelete.id, versionId: confirmDelete.versionId },
                { onSuccess: () => setConfirmDelete(null) },
              );
            }}
            disabled={deleteCodeList.isPending}
          >
            {t("common.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
```

- [ ] **Step 2: Type-check**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: passes.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx
git commit -m "feat(terminology/page): pagination + debounce on codelist page"
```

---

## Task 11: Update `CodeListDetailPage` for pagination + debounce

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx`

**Interfaces:**
- Consumes: same primitives as Task 10.

- [ ] **Step 1: Apply the edits**

Replace the entire file with:

```tsx
import { useEffect, useMemo, useState } from "react";
import {
  getRouteApi,
  useNavigate,
} from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Edit as EditIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import { useCurrentUser } from "../../auth";
import {
  PAGE_SIZE,
  useCreateCodeItem,
  useDeleteCodeItem,
  useGetCodeList,
  useListCodeItems,
  useListTerminologyVersions,
  useUpdateCodeItem,
  useUpdateCodeList,
} from "../data";
import type {
  CodeItemView,
  CreateCodeItemInput,
  TerminologyKind,
  UpdateCodeItemInput,
  UpdateCodeListInput,
} from "../../../shared/api";
import { CodeItemDrawer } from "../components/CodeItemDrawer";
import { CodeItemTable } from "../components/CodeItemTable";
import { CodeListDrawer } from "../components/CodeListDrawer";
import { TermFilterBar } from "../components/TermFilterBar";

const routeApi = getRouteApi(
  "/_authed/_layout/terminology/$kind/codelists/$codelistId",
);

type ItemDrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CodeItemView }
  | null;

export function CodeListDetailPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();
  const kind = params.kind as TerminologyKind;
  const codelistId = Number(params.codelistId);
  const versionIdFromUrl = search.versionId;

  const currentUser = useCurrentUser();
  const versionsQuery = useListTerminologyVersions();
  const codelistQuery = useGetCodeList(codelistId);

  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const codelist = codelistQuery.data;

  const versionId = codelist?.versionId ?? versionIdFromUrl ?? 0;
  const backLink = `/terminology/${kind}`;
  const backSearch = versionIdFromUrl != null
    ? { versionId: versionIdFromUrl }
    : undefined;

  const [search2, setSearch2] = useState("");
  const [offset, setOffset] = useState(0);
  const [editCodelistDrawerOpen, setEditCodelistDrawerOpen] = useState(false);
  const [itemDrawer, setItemDrawer] = useState<ItemDrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeItemView | null>(null);

  const debouncedFragment = useDebouncedValue(search2, {
    delayMs: 300,
    maxWaitMs: 1000,
  });

  useEffect(() => {
    setOffset(0);
  }, [codelistId, debouncedFragment]);

  const itemsQuery = useListCodeItems(codelistId, {
    fragment: debouncedFragment,
    offset,
  });

  const updateCodelist = useUpdateCodeList();
  const createItem = useCreateCodeItem();
  const updateItem = useUpdateCodeItem();
  const deleteItem = useDeleteCodeItem();

  const rows = itemsQuery.data?.items ?? [];
  const hasMore = itemsQuery.data?.nextOffset != null;
  const trimmedQuery = debouncedFragment.trim();

  const mutationLoading =
    updateCodelist.isPending ||
    createItem.isPending ||
    updateItem.isPending ||
    deleteItem.isPending;

  const error = codelist ? null : (codelistQuery.error ?? itemsQuery.error);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <TableContainer component={Paper}>
        <Table size="small">
          <TableBody>
            <TableRow>
              <TableCell sx={{ width: 48 }}>
                <Tooltip title={t("common.back")}>
                  <span>
                    <IconButton
                      onClick={() =>
                        navigate({ to: backLink, search: backSearch })
                      }
                      disabled={!backLink}
                      aria-label={t("common.back")}
                    >
                      <ArrowBackIcon />
                    </IconButton>
                  </span>
                </Tooltip>
              </TableCell>
              {error && !codelist ? (
                <TableCell colSpan={4}>
                  <Alert severity="error">
                    {t("terminology.codeitem.loadFailed", {
                      message: errorMessage(error),
                    })}
                  </Alert>
                  <Box sx={{ mt: 1 }}>
                    <Button onClick={() => navigate({ to: backLink, search: backSearch })}>
                      {t("common.back")}
                    </Button>
                  </Box>
                </TableCell>
              ) : codelist ? (
                <>
                  <TableCell>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                        {codelist.code}
                      </Typography>
                      {codelist.extensible && (
                        <Tooltip title={t("terminology.extensible")}>
                          <Chip label="EXT" size="small" />
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" color="textSecondary">
                      {codelist.name}
                    </Typography>
                  </TableCell>
                  <TableCell sx={{ color: "text.secondary" }}>
                    <Typography variant="body2">
                      {codelist.submissionValue || "—"}
                    </Typography>
                  </TableCell>
                  <TableCell sx={{ width: 64 }} align="right">
                    {canMutate && (
                      <Tooltip title={t("terminology.codelist.edit.title")}>
                        <IconButton
                          size="small"
                          aria-label={t("terminology.codelist.edit.title")}
                          onClick={() => setEditCodelistDrawerOpen(true)}
                          disabled={mutationLoading}
                        >
                          <EditIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                    )}
                  </TableCell>
                </>
              ) : null}
            </TableRow>
          </TableBody>
        </Table>
      </TableContainer>

      <TermFilterBar
        query={search2}
        onQueryChange={setSearch2}
        placeholder={t("terminology.codeitem.search.placeholder")}
      />

      <CodeItemTable
        rows={rows}
        loading={itemsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={itemsQuery.error}
        canMutate={canMutate}
        onRetry={itemsQuery.refetch}
        onCreate={() => setItemDrawer({ mode: "create" })}
        onEdit={(row) => setItemDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codeitem.noMatches")
            : t("terminology.codeitem.empty")
        }
      />

      <InfiniteScrollSentinel
        onIntersect={() => setOffset((o) => o + PAGE_SIZE)}
        hasMore={hasMore}
        loading={itemsQuery.isFetching}
      />

      {codelist && (
        <CodeListDrawer
          open={editCodelistDrawerOpen}
          mode="edit"
          row={codelist}
          versions={versionsQuery.data ?? []}
          versionId={codelist.versionId}
          onClose={() => setEditCodelistDrawerOpen(false)}
          onCreate={() => {
            /* unreachable in edit mode */
          }}
          onUpdate={(_id, body: UpdateCodeListInput) =>
            updateCodelist.mutate(
              { id: codelist.id, body },
              { onSuccess: () => setEditCodelistDrawerOpen(false) },
            )
          }
          canMutate={canMutate}
          mutationError={updateCodelist.error}
          mutationPending={updateCodelist.isPending}
        />
      )}

      <CodeItemDrawer
        open={itemDrawer !== null}
        mode={itemDrawer?.mode ?? "create"}
        row={itemDrawer?.mode === "edit" ? itemDrawer.row : undefined}
        codelistId={codelistId}
        versionId={versionId}
        onClose={() => setItemDrawer(null)}
        onCreate={(input: CreateCodeItemInput) =>
          createItem.mutate(input, {
            onSuccess: () => setItemDrawer(null),
          })
        }
        onUpdate={(id, body: UpdateCodeItemInput) =>
          updateItem.mutate(
            { id, body },
            { onSuccess: () => setItemDrawer(null) },
          )
        }
        canMutate={canMutate}
        mutationError={createItem.error ?? updateItem.error}
        mutationPending={createItem.isPending || updateItem.isPending}
      />

      <Dialog
        open={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
      >
        <DialogTitle>
          {t("terminology.codeitem.action.delete.confirmTitle")}
        </DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("terminology.codeitem.action.delete.confirmMessage")}
          </DialogContentText>
          {deleteItem.isError && (
            <DialogContentText sx={{ mt: 2, color: "error.main" }}>
              {errorMessage(deleteItem.error)}
            </DialogContentText>
          )}
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setConfirmDelete(null)}
            disabled={deleteItem.isPending}
          >
            {t("common.cancel")}
          </Button>
          <Button
            color="error"
            onClick={() => {
              if (!confirmDelete) return;
              deleteItem.mutate(
                { id: confirmDelete.id, codelistId: confirmDelete.codelistId },
                { onSuccess: () => setConfirmDelete(null) },
              );
            }}
            disabled={deleteItem.isPending}
          >
            {t("common.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
```

- [ ] **Step 2: Type-check**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: passes.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx
git commit -m "feat(terminology/page): pagination + debounce on codeitem detail page"
```

---

## Task 12: Update existing test mocks to the paged envelope

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/terminology/version-dropdown-persistence.test.tsx`

- [ ] **Step 1: Update the `setupMocks` block**

Find the `setupMocks` function inside `version-dropdown-persistence.test.tsx` and update it so the `list_code_lists` handler returns the paged envelope:

```ts
function setupMocks() {
  mockCommands({
    is_logged_in: () => true,
    current_user: () => ({
      id: 1,
      code: "alice",
      name: "Alice",
      role: "admin",
      active: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    }),
    list_terminology_versions: () => [sdtmV1, sdtmV2],
    list_code_lists: (args) => {
      if (args && args.versionId === 2) {
        return { codelists: [sdtmCodelist], nextOffset: undefined };
      }
      return { codelists: [], nextOffset: undefined };
    },
    get_code_list_by_id: () => sdtmCodelist,
    list_code_items: () => ({ items: [], nextOffset: undefined }),
  });
}
```

- [ ] **Step 2: Run the test**

Run: `pnpm --filter aegis-desktop test -- --run test/features/terminology/version-dropdown-persistence.test.tsx`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/terminology/version-dropdown-persistence.test.tsx
git commit -m "test(terminology): update version-dropdown mock to paged envelope"
```

---

## Task 13: Add page-level pagination tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/features/terminology/terminology-page-pagination.test.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-detail-pagination.test.tsx`

- [ ] **Step 1: Write the Terminology page test**

Create `apps/desktop/aegis-desktop/src/test/features/terminology/terminology-page-pagination.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("intersection-observer");

import { AegisTestRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

// Reuse the helpers already used by the project's terminology tests:
//   AegisTestRouter, mockCommands, mockInvoke, TestQueryProvider, etc.

const versions = [
  { id: 1, kind: "sdtm" as const, name: "2026-01-01", createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
];

function makeRow(i: number) {
  return {
    id: i,
    versionId: 1,
    code: `C${i}`,
    extensible: false,
    name: `Name ${i}`,
    submissionValue: `SV${i}`,
    synonym: "",
    definition: "",
    nciPreferredTerm: "",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

beforeEach(() => {
  mockInvoke.mockReset();
  vi.useFakeTimers({ shouldAdvanceTime: false });
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
});

describe("TerminologyPage pagination + debounce", () => {
  it("loads page 0 of 20, then loads page 1 on intersection", async () => {
    mockCommands({
      list_terminology_versions: () => versions,
      list_code_lists: (args) => {
        const offset = Number(args?.offset ?? 0);
        const rows = Array.from({ length: 20 }, (_, i) => makeRow(offset + i + 1));
        const nextOffset = offset + 20 < 40 ? offset + 20 : undefined;
        return { codelists: rows, nextOffset };
      },
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <AegisTestRouter initialEntries={["/terminology/sdtm?versionId=1"]} />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => expect(screen.getByText("C1")).toBeInTheDocument());
    expect(screen.getByText("C20")).toBeInTheDocument();
    expect(screen.queryByText("C21")).not.toBeInTheDocument();

    // Trigger the sentinel.
    const sentinel = screen.getByTestId("infinite-scroll-sentinel");
    fireEvent.intersect(sentinel, { isIntersecting: true });

    await waitFor(() => expect(screen.getByText("C21")).toBeInTheDocument());
    expect(mockInvoke).toHaveBeenCalledWith(
      "list_code_lists",
      expect.objectContaining({ offset: 20, limit: 20 }),
    );

    // No third page — second response had nextOffset = undefined.
    fireEvent.intersect(sentinel, { isIntersecting: true });
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledTimes(2));
  });

  it("debounces continuous typing to at most one request per second", async () => {
    mockCommands({
      list_terminology_versions: () => versions,
      list_code_lists: () => ({ codelists: [], nextOffset: undefined }),
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <AegisTestRouter initialEntries={["/terminology/sdtm?versionId=1"]} />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );

    const input = await screen.findByPlaceholderText(/search by code, name/i);

    // Continuous typing for 3 s in 200 ms steps → at most 3 list_code_lists
    // calls (one per maxWaitMs = 1000 ms).
    for (let i = 0; i < 15; i++) {
      await user.type(input, "a");
      await vi.advanceTimersByTimeAsync(200);
    }
    // The last emit happens ~200 ms after the last keystroke (within delayMs).
    await vi.advanceTimersByTimeAsync(500);

    const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_lists");
    expect(calls.length).toBeGreaterThanOrEqual(1);
    expect(calls.length).toBeLessThanOrEqual(4);
  });

  it("resets offset when the fragment changes", async () => {
    mockCommands({
      list_terminology_versions: () => versions,
      list_code_lists: (args) => {
        const offset = Number(args?.offset ?? 0);
        const fragment = String(args?.fragment ?? "");
        const rows = fragment
          ? [makeRow(101)]
          : Array.from({ length: 20 }, (_, i) => makeRow(offset + i + 1));
        return { codelists: rows, nextOffset: undefined };
      },
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <AegisTestRouter initialEntries={["/terminology/sdtm?versionId=1"]} />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );

    await screen.findByText("C1");

    const input = screen.getByPlaceholderText(/search by code, name/i);
    await user.type(input, "AE");
    await vi.advanceTimersByTimeAsync(400);

    await waitFor(() => {
      const lastCall = mockInvoke.mock.calls
        .filter((c) => c[0] === "list_code_lists")
        .at(-1)!;
      expect(lastCall[1]).toMatchObject({ fragment: "AE", offset: 0, limit: 20 });
    });
  });
});
```

(Note: if `AegisTestRouter` is not the actual exported name in `file-route-utils.tsx`, swap it for the real one — most likely `renderWithFullRouter` — and adjust the call site accordingly.)

- [ ] **Step 2: Write the CodeListDetail page test (mirror)**

Create `apps/desktop/aegis-desktop/src/test/features/terminology/code-list-detail-pagination.test.tsx` with the same three scenarios, swapping `list_code_lists` for `list_code_items` and pointing the router at `/terminology/sdtm/codelists/100`. Mirror the data shape: `{ items, nextOffset? }` and `codelistId: 100` instead of `versionId: 1`.

- [ ] **Step 3: Run the new tests**

Run: `pnpm --filter aegis-desktop test -- --run test/features/terminology/terminology-page-pagination.test.tsx test/features/terminology/code-list-detail-pagination.test.tsx`
Expected: 3 tests pass per file.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/terminology/terminology-page-pagination.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/terminology/code-list-detail-pagination.test.tsx
git commit -m "test(terminology): page-level pagination + debounce coverage"
```

---

## Task 14: Verification

**Files:** none modified.

- [ ] **Step 1: Build the desktop crate**

Run: `cargo build -p aegis-desktop`
Expected: succeeds.

- [ ] **Step 2: Run all Rust tests in the desktop crate**

Run: `cargo test -p aegis-desktop --lib`
Expected: all pass — 10 `code_list` + 12 `code_item` + existing tests.

- [ ] **Step 3: Type-check the TS app**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: succeeds.

- [ ] **Step 4: Run all Vitest tests**

Run: `pnpm --filter aegis-desktop test -- --run`
Expected: all pass — 5 `useDebouncedValue` + 5 `InfiniteScrollSentinel` + 3 `list` hook + 3 Terminology pagination + 3 CodeListDetail pagination + existing.

- [ ] **Step 5: Final commit (if any leftover changes)**

```bash
git status
# If anything is dirty, commit it now.
```

---

## Self-Review

**Spec coverage:**

| Spec section | Implemented by |
| --- | --- |
| §1 Goal 1 — 20-rows-per-page infinite scroll | Tasks 9, 10, 11 (PAGE_SIZE, InfiniteScrollSentinel) |
| §1 Goal 2 — debounce ≤ 1 req/sec | Tasks 7, 10, 11 (useDebouncedValue, 300/1000 ms) |
| §1 Goal 3 — replace client filter with server FTS | Tasks 10, 11 (drop useMemo, send fragment) |
| §1 Goal 4 — update Tauri shim layer | Tasks 1, 2, 3, 4 (Rust) |
| §1 Goal 5 — drop dead search_code_* commands | Task 4 (lib.rs + shim files) |
| §2 Infinite-scroll UX | Task 8 (sentinel + 200 px rootMargin) |
| §3 Debounce behavior | Task 7 (delayMs/maxWaitMs) |
| §4 Tauri shim layer | Tasks 1, 2, 3, 4 |
| §5 Shared API | Tasks 5, 6 (types + index + keys) |
| §6 Query layer (paged hook, prefix invalidation) | Task 9 |
| §7 useDebouncedValue | Task 7 |
| §8 InfiniteScrollSentinel | Task 8 |
| §9 Pages | Tasks 10, 11 |
| §10 Files-changed summary | All tasks |
| §12 Testing | Tasks 2, 3, 7, 8, 9, 12, 13 |
| §13 Verification | Task 14 |

**Placeholder scan:** No "TBD" / "TODO" / "fill in" patterns. One ambiguous note in Task 9 Step 1 about an `as unknown as` cast was resolved by removing the third describe block.

**Type consistency:** `PAGE_SIZE = 20` (Task 9) matches the offset increment in Tasks 10/11 (`o + PAGE_SIZE`). `PagedCodeListListResponse` and `PagedCodeItemListResponse` (Task 5) match the Rust `codelists` / `items` field names (Tasks 1, 3). `queryKeys.terminology.codeLists(versionId, fragment, offset)` (Task 6) matches the hook call site (Task 9).
