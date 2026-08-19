# Terminology Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SDTM/ADaM terminology management pages to `aegis-desktop`, with the server wired through 15 new Tauri command shims, the frontend exposed through 13 React Query hooks, and admin/root-gated mutations.

**Architecture:** Frontend uses the existing TanStack Router file-based routing under `/_authed/_layout/terminology/{sdtm,adam}/...`, with a new `features/terminology/` feature folder mirroring the `user` and `project` patterns. Backend adds an `http/terminology/` module with three subresources (`version`, `code_list`, `code_item`) and a matching `commands/terminology/` module of one-line tauri shims. The 5 search endpoints are exposed on the tauri shim layer but unused by the desktop pages (future-proofing); the desktop uses list + client-side substring filter.

**Tech Stack:** Tauri 2, Rust 2021, wiremock (Rust tests), React 19, @tanstack/react-router 1, @tanstack/react-query 5, MUI 5, Vitest 2, Testing Library.

**Spec:** [docs/superpowers/specs/2026-08-19-terminology-page-design.md](../specs/2026-08-19-terminology-page-design.md)

## Global Constraints

- Follow the existing `http/user.rs` and `commands/user.rs` patterns 1:1 — module names, file layout, serde `rename_all`, test style.
- Tauri HTTP DTOs use `#[serde(rename_all = "camelCase")]` to match the existing `UserViewResponse` convention used by the desktop wire.
- The shared `TerminologyKind` enum lives in `http/dto.rs` (single source of truth); it uses `#[serde(rename_all = "lowercase")]` so the wire is `"sdtm"` / `"adam"`.
- The frontend types mirror the wire shape (snake_case → camelCase). `id: i64` on Rust → `id: number` on TS.
- Mutation buttons and drawers render only when `currentUser.data?.role === 'admin' || role === 'root'`; the server's existing `require_admin_or_root` is the final word.
- All 15 tauri commands are registered in `lib.rs` (5 version + 5 code_list + 5 code_item, including search_* which the spec says to expose for future use).
- Only 13 frontend `api.*` wrappers (no `search_*`); the search tauri commands are reserved.
- Tests live in `#[cfg(test)] mod tests` blocks alongside the code (Rust) or in `*.test.tsx` files alongside the component (TS).
- All work happens on the current branch (`feat/desktop_terminology`).
- Commit message prefix `feat(terminology):` for new behavior, `test(terminology):` for tests, `docs(terminology):` for doc-only.

---

## Task 1: Add `TerminologyKind` enum to `http/dto.rs`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs:1-10` (top of file, alongside `Role`)
- Test: same file, `#[cfg(test)] mod tests` block

**Interfaces:**
- Produces: `crate::http::dto::TerminologyKind { Sdtm, Adam }` with `serde(rename_all = "lowercase")`. Used by every file under `http/terminology/`.

- [ ] **Step 1: Write the failing test**

In `http/dto.rs`, inside the existing `mod tests` block, add:

```rust
#[test]
fn terminology_kind_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&super::TerminologyKind::Sdtm).unwrap(), "\"sdtm\"");
    assert_eq!(serde_json::to_string(&super::TerminologyKind::Adam).unwrap(), "\"adam\"");
}

#[test]
fn terminology_kind_deserializes_lowercase() {
    let k: super::TerminologyKind = serde_json::from_str("\"sdtm\"").unwrap();
    assert_eq!(k, super::TerminologyKind::Sdtm);
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p aegis-desktop terminology_kind`
Expected: FAIL with "cannot find type `TerminologyKind`".

- [ ] **Step 3: Add the enum at the top of `http/dto.rs`**

```rust
/// CDISC terminology kind. Wire form is lowercase (`"sdtm"`, `"adam"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminologyKind {
    Sdtm,
    Adam,
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p aegis-desktop terminology_kind`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs
git commit -m "feat(terminology): add TerminologyKind enum to http dto"
```

---

## Task 2: `http/terminology/version.rs` — DTOs + 5 HTTP functions (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/version.rs`
- Test: same file, `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::http::client::HttpClient`, `crate::http::dto::{ApiError, TerminologyKind}`
- Produces:
  - `pub struct TerminologyVersionViewResponse { id: i64, kind: TerminologyKind, name: String, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`
  - `pub struct TerminologyVersionListResponse { versions: Vec<TerminologyVersionViewResponse> }`
  - `pub struct CreateTerminologyVersionRequest { kind: TerminologyKind, name: String }`
  - `pub struct UpdateTerminologyVersionRequest { kind: Option<TerminologyKind>, name: Option<String> }` (default + skip_if_none on each field)
  - `pub async fn create(c, body) -> Result<Term, ApiError>` — POST `/api/terminology/versions`
  - `pub async fn list(c) -> Result<Vec<Term>, ApiError>` — GET `/api/terminology/versions`
  - `pub async fn get_by_id(c, id: i64) -> Result<Term, ApiError>` — GET `/api/terminology/versions/{id}`
  - `pub async fn update(c, id: i64, body) -> Result<Term, ApiError>` — PATCH `/api/terminology/versions/{id}`
  - `pub async fn delete(c, id: i64) -> Result<(), ApiError>` — DELETE `/api/terminology/versions/{id}`

- [ ] **Step 1: Write the wiremock round-trip tests**

Mirror `http/user.rs` style. The full block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::super::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        std::mem::drop(store.set_access_token("AT"));
        std::mem::drop(store.set_refresh_token("RT"));
        HttpClient::new(server.uri(), store)
    }

    #[tokio::test]
    async fn list_returns_versions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": [{
                    "id": 1, "kind": "sdtm", "name": "2024-06-28",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let versions = list(&client(&server)).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].kind, TerminologyKind::Sdtm);
        assert_eq!(versions[0].name, "2024-06-28");
        assert_eq!(
            versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 7, "kind": "adam", "name": "2024-09-30",
                "createdAt": "2026-02-01T00:00:00Z",
                "updatedAt": "2026-02-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = create(&client(&server), CreateTerminologyVersionRequest {
            kind: TerminologyKind::Adam, name: "2024-09-30".into(),
        })
        .await
        .unwrap();
        assert_eq!(v.id, 7);
        assert_eq!(v.kind, TerminologyKind::Adam);
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 3, "kind": "sdtm", "name": "2023-12-15",
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2025-12-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = get_by_id(&client(&server), 3).await.unwrap();
        assert_eq!(v.id, 3);
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 3, "kind": "sdtm", "name": "renamed",
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2026-03-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = update(&client(&server), 3, UpdateTerminologyVersionRequest {
            name: Some("renamed".into()), ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(v.name, "renamed");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateTerminologyVersionRequest {
            name: Some("renamed".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}
```

- [ ] **Step 2: Run the tests and confirm they fail to compile**

Run: `cargo test -p aegis-desktop http::terminology::version::tests::list_returns_versions`
Expected: FAIL with "cannot find module `terminology`" (or unresolved import).

- [ ] **Step 3: Implement the module**

```rust
//! Versions under `/api/terminology/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionViewResponse {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionListResponse {
    pub versions: Vec<TerminologyVersionViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminologyVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TerminologyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/terminology/versions", Some(&body))
        .await
}

pub async fn list(c: &HttpClient) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    let resp: TerminologyVersionListResponse = c
        .request(reqwest::Method::GET, "/api/terminology/versions", None::<&()>)
        .await?;
    Ok(resp.versions)
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/terminology/versions/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/versions/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    c.request_no_body(reqwest::Method::DELETE, &format!("/api/terminology/versions/{id}"))
        .await
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p aegis-desktop http::terminology::version`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/version.rs
git commit -m "feat(terminology): http functions for terminology versions"
```

---

## Task 3: `http/terminology/code_list.rs` — DTOs + 5 HTTP functions (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_list.rs`
- Test: same file

**Interfaces:**
- Produces:
  - `CodeListViewResponse { id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at }`
  - `CodeListListResponse { codelists: Vec<CodeListViewResponse> }`
  - `CreateCodeListRequest { version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term }`
  - `UpdateCodeListRequest` — all fields optional
  - `CodeListSearchQuery { version_id, fragment, limit }` (request body for search; matches server)
  - `create / list(c, version_id) / update(c, id, body) / delete(c, id) / search(c, query)`

- [ ] **Step 1: Write the wiremock tests**

Pattern: same as Task 2. Six tests — `list_returns_codelists`, `create_returns_view`, `update_returns_view`, `delete_succeeds`, `search_returns_hits`, `update_request_skips_none_fields`. The `list_returns_codelists` test mounts the GET on `/api/terminology/code-lists` and asserts the response decodes. `search_returns_hits` mounts GET on the same path with a query string that the matcher allows via `wiremock::matchers::query`; the server response body matches `CodeListSearchHitsResponse` shape `{ hits: [{ codelist: { ... } }] }`.

- [ ] **Step 2: Run, confirm failure**
- [ ] **Step 3: Implement DTOs and 5 functions**

Follow Task 2's structure exactly. URLs:
- `POST/GET/PATCH/DELETE /api/terminology/code-lists[/id]`
- `GET /api/terminology/code-lists/search?versionId=…&fragment=…&limit=…`

For `search`, build the URL with `format!("/api/terminology/code-lists/search?versionId={}&fragment={}&limit={}", q.version_id, urlencoded_fragment, q.limit)`. URL-encode `fragment` via a small helper (e.g. `urlencoding::encode`, or `q.fragment.replace(' ', "+")` for this minimal case — confirm against the server's expected encoding). If unsure, check the server's `handlers::search_code_lists` for how the `fragment` query is read.

- [ ] **Step 4: Run, confirm pass** — `cargo test -p aegis-desktop http::terminology::code_list`
- [ ] **Step 5: Commit**

```bash
git commit -m "feat(terminology): http functions for code lists"
```

---

## Task 4: `http/terminology/code_item.rs` — DTOs + 5 HTTP functions (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`

**Interfaces:**
- Produces:
  - `CodeItemViewResponse { id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at }` (no `extensible`, no `name`)
  - `CodeItemListResponse { items: Vec<CodeItemViewResponse> }`
  - `CreateCodeItemRequest { codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term }`
  - `UpdateCodeItemRequest` — all fields optional
  - `create / list(c, codelist_id) / update(c, id, body) / delete(c, id) / search(c, query)`

- [ ] **Steps 1–5:** Mirror Tasks 2 and 3 exactly. URLs `/api/terminology/code-items[/id]` and `/api/terminology/code-items/search`. Six wiremock tests + one `update_request_skips_none_fields` test.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): http functions for code items"
```

---

## Task 5: `http/terminology.rs` module + register in `http.rs`

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http.rs:6-14` (add `pub mod terminology;`)

- [ ] **Step 1: Write `terminology.rs`**

```rust
//! Terminology HTTP client. One submodule per resource.
pub mod code_item;
pub mod code_list;
pub mod version;
```

- [ ] **Step 2: Wire it into `http.rs`**

Add `pub mod terminology;` in alphabetical position in `apps/desktop/aegis-desktop/src-tauri/src/http.rs`.

- [ ] **Step 3: Run all terminology tests once more**

Run: `cargo test -p aegis-desktop http::terminology`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http.rs apps/desktop/aegis-desktop/src-tauri/src/http/terminology.rs
git commit -m "feat(terminology): declare http/terminology module"
```

---

## Task 6: `commands/terminology/version.rs` — 5 Tauri shims (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/version.rs`

**Interfaces:**
- Consumes: `crate::http::terminology::version::{self, CreateTerminologyVersionRequest, UpdateTerminologyVersionRequest, TerminologyVersionViewResponse}`
- Produces 5 `#[tauri::command]` async functions: `create_terminology_version(kind, name)`, `list_terminology_versions`, `get_terminology_version_by_id(id)`, `update_terminology_version(id, body)`, `delete_terminology_version(id)`. All take `State<'_, HttpClient>` first, return `Result<_, ApiError>`.

- [ ] **Step 1: Write a test confirming each shim forwards**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use crate::http::client::{HttpClient, MemoryStore, TokenStore};
    use crate::http::dto::TerminologyKind;

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        std::mem::drop(store.set_access_token("AT"));
        std::mem::drop(store.set_refresh_token("RT"));
        HttpClient::new(server.uri(), store)
    }

    #[tokio::test]
    async fn list_terminology_versions_forwards_to_http() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions": []})))
            .mount(&server).await;
        let c = client(&server);
        let versions = list_terminology_versions(State::from(&c)).await.unwrap();
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn create_terminology_version_forwards_args() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 1, "kind": "sdtm", "name": "2024",
                "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
            })))
            .mount(&server).await;
        let c = client(&server);
        let v = create_terminology_version(State::from(&c), TerminologyKind::Sdtm, "2024".to_string())
            .await.unwrap();
        assert_eq!(v.name, "2024");
    }

    // (and similarly for get_by_id, update, delete)
}
```

Note: `State::from(&HttpClient)` constructs a State wrapper around a reference for direct calls; if the wrapper isn't exposed, test via `crate::http::terminology::version::list(&c).await` instead. Match whichever pattern the existing `commands/user.rs` tests use.

- [ ] **Step 2: Run, confirm failure**
- [ ] **Step 3: Implement the shims**

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::version::{
    self, CreateTerminologyVersionRequest, UpdateTerminologyVersionRequest,
    TerminologyVersionViewResponse,
};

#[tauri::command]
pub async fn create_terminology_version(
    client: State<'_, HttpClient>,
    kind: TerminologyKind,
    name: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::create(&client, CreateTerminologyVersionRequest { kind, name }).await
}

#[tauri::command]
pub async fn list_terminology_versions(
    client: State<'_, HttpClient>,
) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    version::list(&client).await
}

#[tauri::command]
pub async fn get_terminology_version_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_terminology_version(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_terminology_version(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    version::delete(&client, id).await
}
```

- [ ] **Step 4: Run, confirm pass** — `cargo test -p aegis-desktop commands::terminology::version`
- [ ] **Step 5: Commit**

```bash
git commit -m "feat(terminology): tauri commands for terminology versions"
```

---

## Task 7: `commands/terminology/code_list.rs` — 5 Tauri shims (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs`

**Interfaces:**
- Produces 5 shims:
  - `create_code_list(client, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term)`
  - `list_code_lists(client, version_id) -> Vec<CodeListViewResponse>`
  - `update_code_list(client, id, body: UpdateCodeListRequest)`
  - `delete_code_list(client, id)`
  - `search_code_lists(client, version_id, fragment, limit)`

- [ ] **Steps 1–5:** Mirror Task 6. Build the `CreateCodeListRequest` from positional args inside each shim; the search shim takes the three query fields directly.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): tauri commands for code lists"
```

---

## Task 8: `commands/terminology/code_item.rs` — 5 Tauri shims (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`

**Interfaces:**
- Same shape as Task 7 minus `extensible` / `name` (code items don't carry those). `create_code_item(client, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term)`.

- [ ] **Steps 1–5:** Mirror Tasks 6 and 7.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): tauri commands for code items"
```

---

## Task 9: `commands/terminology.rs` module + register in `commands.rs`

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`

- [ ] **Step 1: Write `terminology.rs`**

```rust
//! Tauri command shims for the terminology HTTP layer.
pub mod code_item;
pub mod code_list;
pub mod version;
```

- [ ] **Step 2: Wire it into `commands.rs`**

Add `pub mod terminology;` in alphabetical position.

- [ ] **Step 3: Run all command tests**

Run: `cargo test -p aegis-desktop commands::terminology`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(terminology): declare commands/terminology module"
```

---

## Task 10: Register 15 commands in `lib.rs`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:21-48`

- [ ] **Step 1: Add 15 entries to `invoke_handler!`**

Insert after the project block, before `commands::healthz::healthz`:

```rust
            // terminology
            commands::terminology::version::create_terminology_version,
            commands::terminology::version::list_terminology_versions,
            commands::terminology::version::get_terminology_version_by_id,
            commands::terminology::version::update_terminology_version,
            commands::terminology::version::delete_terminology_version,
            commands::terminology::code_list::create_code_list,
            commands::terminology::code_list::list_code_lists,
            commands::terminology::code_list::update_code_list,
            commands::terminology::code_list::delete_code_list,
            commands::terminology::code_list::search_code_lists,
            commands::terminology::code_item::create_code_item,
            commands::terminology::code_item::list_code_items,
            commands::terminology::code_item::update_code_item,
            commands::terminology::code_item::delete_code_item,
            commands::terminology::code_item::search_code_items,
```

- [ ] **Step 2: Verify it builds**

Run: `cargo check -p aegis-desktop`
Expected: success.

- [ ] **Step 3: Run all tauri tests**

Run: `cargo test -p aegis-desktop`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(terminology): register 15 commands in invoke_handler"
```

---

## Task 11: Add terminology types to `shared/api/types.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (append at end)

- [ ] **Step 1: Append the types**

The shapes from spec section 7.4, exported under `// Terminology`. All `id: number`, snake_case wire keys but camelCase TS identifiers (per existing convention — verify by re-reading the file's header comment).

```ts
// Terminology
export type TerminologyKind = 'sdtm' | 'adam';

export interface TerminologyVersionView {
  id: number;
  kind: TerminologyKind;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface TerminologyVersionListResponse {
  versions: TerminologyVersionView[];
}

export interface CreateTerminologyVersionInput {
  kind: TerminologyKind;
  name: string;
}

export interface UpdateTerminologyVersionInput {
  kind?: TerminologyKind;
  name?: string;
}

export interface CodeListView {
  id: number;
  versionId: number;
  code: string;
  extensible: boolean;
  name: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
  createdAt: string;
  updatedAt: string;
}

export interface CodeListListResponse {
  codelists: CodeListView[];
}

export interface CreateCodeListInput {
  versionId: number;
  code: string;
  extensible: boolean;
  name: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface UpdateCodeListInput {
  code?: string;
  extensible?: boolean;
  name?: string;
  submissionValue?: string;
  synonym?: string;
  definition?: string;
  nciPreferredTerm?: string;
}

export interface CodeItemView {
  id: number;
  codelistId: number;
  versionId: number;
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
  createdAt: string;
  updatedAt: string;
}

export interface CodeItemListResponse {
  items: CodeItemView[];
}

export interface CreateCodeItemInput {
  codelistId: number;
  versionId: number;
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface UpdateCodeItemInput {
  code?: string;
  submissionValue?: string;
  synonym?: string;
  definition?: string;
  nciPreferredTerm?: string;
}

export interface SearchTerminologyQuery {
  versionId: number;
  fragment: string;
  limit?: number;
}
```

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts
git commit -m "feat(terminology): add terminology types to shared api"
```

---

## Task 12: Add 13 wrappers to `shared/api/index.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`

- [ ] **Step 1: Append the wrappers + type re-exports**

Add a `// terminology` block to the `api` object (13 wrappers, no `search_*`):

```ts
  // terminology
  listTerminologyVersions: (): Promise<TerminologyVersionView[]> =>
    call<TerminologyVersionView[]>('list_terminology_versions'),
  createTerminologyVersion: (input: CreateTerminologyVersionInput): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>('create_terminology_version', { ...input }),
  getTerminologyVersionById: (id: number): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>('get_terminology_version_by_id', { id }),
  updateTerminologyVersion: (id: number, body: UpdateTerminologyVersionInput): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>('update_terminology_version', { id, body: { ...body } }),
  deleteTerminologyVersion: (id: number): Promise<void> =>
    call<void>('delete_terminology_version', { id }),

  listCodeLists: (versionId: number): Promise<CodeListView[]> =>
    call<CodeListView[]>('list_code_lists', { versionId }),
  createCodeList: (input: CreateCodeListInput): Promise<CodeListView> =>
    call<CodeListView>('create_code_list', { ...input }),
  updateCodeList: (id: number, body: UpdateCodeListInput): Promise<CodeListView> =>
    call<CodeListView>('update_code_list', { id, body: { ...body } }),
  deleteCodeList: (id: number): Promise<void> =>
    call<void>('delete_code_list', { id }),

  listCodeItems: (codelistId: number): Promise<CodeItemView[]> =>
    call<CodeItemView[]>('list_code_items', { codelistId }),
  createCodeItem: (input: CreateCodeItemInput): Promise<CodeItemView> =>
    call<CodeItemView>('create_code_item', { ...input }),
  updateCodeItem: (id: number, body: UpdateCodeItemInput): Promise<CodeItemView> =>
    call<CodeItemView>('update_code_item', { id, body: { ...body } }),
  deleteCodeItem: (id: number): Promise<void> =>
    call<void>('delete_code_item', { id }),
```

Update the imports at the top of the file to pull the new types. Update the bottom `export type {}` block to re-export them.

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(terminology): add terminology wrappers to shared api"
```

---

## Task 13: Add `terminology` keys to `shared/query/keys.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`

- [ ] **Step 1: Append the `terminology` family**

```ts
  terminology: {
    versions: () => ['terminology', 'versions'] as const,
    version: (id: number) => ['terminology', 'version', id] as const,
    codeLists: (versionId: number) => ['terminology', 'codeLists', versionId] as const,
    codeList: (id: number) => ['terminology', 'codeList', id] as const,
    codeItems: (codelistId: number) => ['terminology', 'codeItems', codelistId] as const,
    searchCodeLists: (versionId: number, fragment: string) =>
      ['terminology', 'searchCodeLists', versionId, fragment] as const,
    searchCodeItems: (versionId: number, fragment: string) =>
      ['terminology', 'searchCodeItems', versionId, fragment] as const,
  },
```

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(terminology): add terminology query keys"
```

---

## Task 14: Verify the desktop builds end-to-end

**Files:** none (verification only)

- [ ] **Step 1: Tauri check**

Run: `cargo check -p aegis-desktop`
Expected: success.

- [ ] **Step 2: Frontend typecheck + vitest baseline**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck && pnpm test --run`
Expected: typecheck succeeds; all existing tests pass (no new tests yet).

- [ ] **Step 3: Commit if any fix-ups were required**

If anything was tweaked to make typecheck/tests pass, commit under `chore(terminology): baseline build fixes`. If clean, skip this commit.

---

## Task 15: `features/terminology/data/list.ts` — hooks (no separate test file yet)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`

**Interfaces:**
- Produces 11 hooks, one per line in spec section 7.6:
  - `useListTerminologyVersions() -> UseQueryResult<TerminologyVersionView[], ApiError>`
  - `useCreateTerminologyVersion() -> UseMutationResult<TerminologyVersionView, ApiError, CreateTerminologyVersionInput>`
  - `useUpdateTerminologyVersion()`
  - `useDeleteTerminologyVersion()`
  - `useListCodeLists(versionId: number | null)`
  - `useCreateCodeList()`
  - `useUpdateCodeList()`
  - `useDeleteCodeList()`
  - `useListCodeItems(codelistId: number | null)`
  - `useCreateCodeItem()`
  - `useUpdateCodeItem()`
  - `useDeleteCodeItem()`

- [ ] **Step 1: Implement the hooks file**

Mirror the spec section 7.6 verbatim. Use `useQuery`, `useMutation`, `useQueryClient` from `@tanstack/react-query`. Pull `queryKeys` from `../../../shared/query` and the types / `api` from `../../../shared/api`.

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/data/list.ts
git commit -m "feat(terminology): react-query hooks for terminology data"
```

---

## Task 16: `data/list.test.ts` — hook tests with mocked `api`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/data/list.test.ts`

- [ ] **Step 1: Write the failing tests**

Pattern: use `vi.mock("../../../shared/api")` to stub the api, then assert each hook calls the right function with the right args and invalidates the right keys on success.

```ts
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi } from 'vitest';
import React from 'react';

vi.mock('../../../shared/api', () => ({
  api: {
    listTerminologyVersions: vi.fn(),
    listCodeLists: vi.fn(),
    listCodeItems: vi.fn(),
    createCodeList: vi.fn(),
    deleteCodeList: vi.fn(),
    createCodeItem: vi.fn(),
    deleteCodeItem: vi.fn(),
    // ... all other api fns as vi.fn()
  },
}));

import { api } from '../../../shared/api';
import { queryKeys } from '../../../shared/query';
import {
  useListCodeLists,
  useCreateCodeList,
  useDeleteCodeList,
  useListCodeItems,
  useCreateCodeItem,
  useDeleteCodeItem,
} from './list';

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe('useListCodeLists', () => {
  it('fetches when versionId is non-zero', async () => {
    vi.mocked(api.listCodeLists).mockResolvedValue([]);
    const { result } = renderHook(() => useListCodeLists(7), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(api.listCodeLists).toHaveBeenCalledWith(7);
  });

  it('is disabled when versionId is 0', async () => {
    vi.mocked(api.listCodeLists).mockResolvedValue([]);
    renderHook(() => useListCodeLists(0), { wrapper: makeWrapper() });
    expect(api.listCodeLists).not.toHaveBeenCalled();
  });
});

describe('useCreateCodeList', () => {
  it('invalidates the codeLists cache for the created version', async () => {
    vi.mocked(api.createCodeList).mockResolvedValue({
      id: 1, versionId: 7, code: 'C1', extensible: true, name: 'n',
      submissionValue: 's', synonym: '', definition: '', nciPreferredTerm: '',
      createdAt: 'x', updatedAt: 'x',
    });
    const invalidateSpy = vi.fn();
    // build the wrapper using a real QueryClient and spy on invalidateQueries
    const qc = new QueryClient();
    vi.spyOn(qc, 'invalidateQueries').mockImplementation(invalidateSpy);
    const { result } = renderHook(() => useCreateCodeList(), {
      wrapper: ({ children }) => <QueryClientProvider client={qc}>{children}</QueryClientProvider>,
    });
    result.current.mutate({
      versionId: 7, code: 'C1', extensible: true, name: 'n',
      submissionValue: 's', synonym: '', definition: '', nciPreferredTerm: '',
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: queryKeys.terminology.codeLists(7) });
  });
});

// mirror for useDeleteCodeList (needs versionId passed via mutation variables), useListCodeItems (enabled when codelistId > 0), useCreateCodeItem (invalidates codeItems(codelistId)), useDeleteCodeItem (invalidates codeItems(codelistId))
```

- [ ] **Step 2: Run, confirm failure**

Run: `cd apps/desktop/aegis-desktop && pnpm test --run data/list`
Expected: FAIL with "Cannot find module './list'".

- [ ] **Step 3: Confirm green after Task 15 is committed**

Run: `cd apps/desktop/aegis-desktop && pnpm test --run data/list`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "test(terminology): react-query hook tests"
```

---

## Task 17: `components/DescriptionsCell.tsx` + `ExtensibleIcon.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/DescriptionsCell.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/DescriptionsCell.test.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/ExtensibleIcon.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/ExtensibleIcon.test.tsx`

**Interfaces:**
- `<DescriptionsCell synonym: string; definition: string; nciPreferredTerm: string />` — renders 0–3 rows of `SYN/DEF/NCI` chip + value, skipping whitespace-empty fields.
- `<ExtensibleIcon visible: boolean />` — renders nothing when `!visible`; renders `<Tooltip title={t('terminology.extensible')}><NorthEastIcon fontSize="small" /></Tooltip>` otherwise.

- [ ] **Step 1: Write `DescriptionsCell.test.tsx`**

```tsx
import { render, screen } from '@testing-library/react';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { DescriptionsCell } from './DescriptionsCell';

function wrap(ui: React.ReactNode) {
  return render(<AegisI18nProvider locale="en">{ui}</AegisI18nProvider>);
}

it('renders SYN/DEF/NCI rows when all values are present', () => {
  wrap(<DescriptionsCell synonym="alt" definition="def" nciPreferredTerm="nci" />);
  expect(screen.getByText('SYN')).toBeInTheDocument();
  expect(screen.getByText('DEF')).toBeInTheDocument();
  expect(screen.getByText('NCI')).toBeInTheDocument();
  expect(screen.getByText('alt')).toBeInTheDocument();
  expect(screen.getByText('def')).toBeInTheDocument();
  expect(screen.getByText('nci')).toBeInTheDocument();
});

it('skips rows whose value is empty or whitespace', () => {
  wrap(<DescriptionsCell synonym="alt" definition="" nciPreferredTerm="  " />);
  expect(screen.getByText('SYN')).toBeInTheDocument();
  expect(screen.queryByText('DEF')).not.toBeInTheDocument();
  expect(screen.queryByText('NCI')).not.toBeInTheDocument();
});

it('renders nothing when every value is empty', () => {
  const { container } = wrap(<DescriptionsCell synonym="" definition="" nciPreferredTerm="" />);
  expect(container.firstChild?.firstChild?.childNodes.length ?? 0).toBe(0);
});
```

- [ ] **Step 2: Write `ExtensibleIcon.test.tsx`**

```tsx
import { render, screen } from '@testing-library/react';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { ExtensibleIcon } from './ExtensibleIcon';

function wrap(ui: React.ReactNode) {
  return render(<AegisI18nProvider locale="en">{ui}</AegisI18nProvider>);
}

it('renders nothing when visible=false', () => {
  const { container } = wrap(<ExtensibleIcon visible={false} />);
  expect(container.firstChild).toBeNull();
});

it('renders the icon when visible=true', () => {
  wrap(<ExtensibleIcon visible />);
  expect(screen.getByLabelText(/extensible/i)).toBeInTheDocument();
});
```

- [ ] **Step 3: Run, confirm failure**

Run: `cd apps/desktop/aegis-desktop && pnpm test --run terminology/components`
Expected: FAIL.

- [ ] **Step 4: Implement `DescriptionsCell.tsx`**

```tsx
import { Box, Chip, Typography } from '@aegis/ui/mui';
import { useI18n } from '@aegis/ui/i18n';

export interface DescriptionsCellProps {
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export function DescriptionsCell({ synonym, definition, nciPreferredTerm }: DescriptionsCellProps) {
  const rows: Array<[string, string]> = [
    ['SYN', synonym],
    ['DEF', definition],
    ['NCI', nciPreferredTerm],
  ].filter(([, v]) => v.trim() !== '');

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
      {rows.map(([label, value]) => (
        <Box key={label} sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
          <Chip label={label} size="small" />
          <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>{value}</Typography>
        </Box>
      ))}
    </Box>
  );
}
```

- [ ] **Step 5: Implement `ExtensibleIcon.tsx`**

```tsx
import { Tooltip } from '@aegis/ui/mui';
import { NorthEast as NorthEastIcon } from '@aegis/ui/icons';
import { useI18n } from '@aegis/ui/i18n';

export interface ExtensibleIconProps {
  visible: boolean;
}

export function ExtensibleIcon({ visible }: ExtensibleIconProps) {
  const { t } = useI18n();
  if (!visible) return null;
  return (
    <Tooltip title={t('terminology.extensible')}>
      <NorthEastIcon fontSize="small" aria-label={t('terminology.extensible')} sx={{ ml: 0.5, verticalAlign: 'middle' }} />
    </Tooltip>
  );
}
```

- [ ] **Step 6: Run, confirm pass** — `pnpm test --run terminology/components`
- [ ] **Step 7: Commit**

```bash
git commit -m "feat(terminology): DescriptionsCell and ExtensibleIcon components"
```

---

## Task 18: `TermFilterBar.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/TermFilterBar.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/TermFilterBar.test.tsx`

**Interfaces:**
- `TermFilterBar(query: string; onQueryChange: (v: string) => void; placeholder?: string)`. Mirrors `UserFilterBar` shape with a customizable placeholder (default `t('terminology.codelist.search.placeholder')`).

- [ ] **Step 1: Write the test**

```tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { TermFilterBar } from './TermFilterBar';

it('calls onQueryChange when the user types', async () => {
  const onChange = vi.fn();
  render(
    <AegisI18nProvider locale="en">
      <TermFilterBar query="" onQueryChange={onChange} />
    </AegisI18nProvider>,
  );
  await userEvent.type(screen.getByRole('textbox'), 'a');
  expect(onChange).toHaveBeenCalledWith('a');
});

it('uses a custom placeholder when provided', () => {
  render(
    <AegisI18nProvider locale="en">
      <TermFilterBar query="" onQueryChange={() => {}} placeholder="Find by code" />
    </AegisI18nProvider>,
  );
  expect(screen.getByPlaceholderText('Find by code')).toBeInTheDocument();
});
```

- [ ] **Steps 2–6:** Mirror Task 17 (test fails → implement → pass → commit).

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): TermFilterBar search component"
```

---

## Task 19: `VersionDropdown.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/VersionDropdown.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/VersionDropdown.test.tsx`

**Interfaces:**
- `VersionDropdown({ kind: TerminologyKind; versions: TerminologyVersionView[]; value: number | null; onChange: (id: number | null) => void; disabled?: boolean })`. The dropdown filters `versions` by `kind`. When `versions` (filtered) is empty the select is disabled and helper text reads `t('terminology.version.placeholder')` (`'No versions yet'`).

- [ ] **Step 1: Write the test**

Three cases:
1. Empty list → select is disabled; helper text shown.
2. Non-empty list filtered by `kind` → renders only matching options.
3. Selecting an option calls `onChange` with the chosen id.

- [ ] **Steps 2–6:** TDD cycle. Use `FormControl` + `Select` from `@aegis/ui/mui`. Use the `Select<TerminologyVersionView>` typed pattern from `SettingsPage.tsx`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): VersionDropdown component"
```

---

## Task 20: `ImportButton.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.test.tsx`

**Interfaces:**
- `ImportButton()` — an `IconButton` with `<AddIcon />`; on click opens a `<Snackbar open autoHideDuration={3000} onClose={...} message={t('terminology.importComingSoon')} />`.

- [ ] **Step 1: Write the test**

```tsx
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { ImportButton } from './ImportButton';

it('opens the coming-soon snackbar on click', async () => {
  render(<AegisI18nProvider locale="en"><ImportButton /></AegisI18nProvider>);
  await userEvent.click(screen.getByRole('button'));
  expect(screen.getByText(/coming soon/i)).toBeInTheDocument();
});
```

- [ ] **Steps 2–6:** TDD. Use `useState` for snackbar open. Use `Snackbar` from `@aegis/ui/mui`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): placeholder ImportButton with coming-soon snackbar"
```

---

## Task 21: `CodeListTable.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListTable.test.tsx`

**Interfaces:**
- Spec section 6.3. Type the props as the union:

```ts
export type CodeListTableProps =
  | {
      mode: 'list';
      rows: CodeListView[];
      loading: boolean;
      mutationLoading: boolean;
      error: ApiError | null;
      canMutate: boolean;
      onRetry: () => void;
      onCreate: () => void;
      onEdit: (row: CodeListView) => void;
      onDelete: (row: CodeListView) => void;
      onOpen: (row: CodeListView) => void;
    }
  | {
      mode: 'single';
      rows: CodeListView[]; // length 1
      loading: boolean;
      mutationLoading: boolean;
      error: ApiError | null;
      canMutate: boolean;
      onRetry: () => void;
      onEdit: (row: CodeListView) => void;
    };
```

- [ ] **Step 1: Write the tests**

Three test files inside `CodeListTable.test.tsx`:

1. `mode='list' + canMutate=false` — rows render but no header `+` and no edit/delete icons. (Use `vi.fn()` for callbacks; assert `screen.queryByRole('button', { name: /add/i })` is null; assert edit / delete buttons are not rendered.)
2. `mode='list' + canMutate=true` — header `+` button + per-row edit + delete + open buttons render; clicking edit invokes `onEdit` with the row.
3. `mode='single'` — never renders header `+`; renders edit only; empty list shows error Alert + Retry when `error != null`.

- [ ] **Steps 2–6:** TDD. Layout: top-level `<Box>` with optional error Alert, then `<TableContainer component={Paper}><Table size="small">`. Header columns: `code, name, submissionValue, descriptions, operation`. Empty-state message: centered `<Typography>` when `rows.length === 0 && !loading && !error`. Loading spinner: centered `<CircularProgress />` when `loading && rows.length === 0`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): CodeListTable presentational component"
```

---

## Task 22: `CodeItemTable.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.test.tsx`

**Interfaces:**
- `CodeItemTableProps`: `mode` is always `"list"` here (no single-row variant). Same shape as `CodeListTable` minus `mode`, `onOpen`, `onCreate`; has `onCreate`/`onEdit`/`onDelete`. (Header `+` is always present for admin/root.)

- [ ] **Steps 1–6:** Mirror Task 21. Difference: no `extensible` column on code items; columns are `code, name (items don't have `name` field — use `submissionValue` for the name slot? No, items have no `name`. Use `submissionValue` as the second column and skip `name`. Verify against the wire DTO: `CodeItemView` has no `name` field.) Columns: `code, submissionValue, descriptions, operation`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): CodeItemTable presentational component"
```

---

## Task 23: `CodeListDrawer.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListDrawer.test.tsx`

**Interfaces:**
- `CodeListDrawer({ open: boolean; mode: 'create' | 'edit'; row?: CodeListView; versions: TerminologyVersionView[]; versionId: number; onClose: () => void; onCreate: (input: CreateCodeListInput) => void; onUpdate: (id: number, body: UpdateCodeListInput) => void; canMutate: boolean; mutationError: ApiError | null; mutationPending: boolean })`.

Form fields: `code, extensible (Switch), name, submissionValue, synonym, definition, nciPreferredTerm`. Submit button label switches on `mode` (`t('terminology.codelist.action.create')` vs `t('...save')`). Submit disabled when `code.trim() === ''` or when `mutationPending`. When `!canMutate` the title reads `t('terminology.codelist.readOnly')` and every input is `disabled`.

- [ ] **Step 1: Write tests**

Three cases:
1. Submit disabled when `code` is whitespace.
2. Read-only mode (`canMutate=false`): inputs are disabled and the title shows `Read-only`.
3. Edit mode calls `onUpdate` with `id` + `body` derived from the form on submit.

- [ ] **Steps 2–6:** TDD. Use `useState` for each field, initialized from `row` when `mode === 'edit'` else empty. Use `<Drawer anchor="right">` from `@aegis/ui/mui`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): CodeListDrawer create/edit form"
```

---

## Task 24: `CodeItemDrawer.tsx` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemDrawer.test.tsx`

**Interfaces:**
- Same shape as `CodeListDrawer` minus `extensible` / `name` fields (code items don't carry those).

- [ ] **Steps 1–6:** Mirror Task 23. Form fields: `code, submissionValue, synonym, definition, nciPreferredTerm`.

- [ ] **Commit:**

```bash
git commit -m "feat(terminology): CodeItemDrawer create/edit form"
```

---

## Task 25: Feature barrel + `pages/TerminologyPage.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/data/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`

**Interfaces:**
- `TerminologyPage({ kind: 'sdtm' | 'adam' })`. Layout from spec section 6.1.

- [ ] **Step 1: Write the barrel files**

`features/terminology/index.ts`:
```ts
export * from './pages/TerminologyPage';
export * from './pages/CodeListDetailPage';
```

`features/terminology/data/index.ts`:
```ts
export * from './list';
```

- [ ] **Step 2: Implement `TerminologyPage.tsx`**

Full page per spec section 6.1. State: `selectedVersionId, search, drawerState`. Memoized `filteredCodeLists` (case-insensitive substring over `code|name|submissionValue|synonym|definition|nciPreferredTerm`). `canMutate = role === 'admin' || role === 'root'`. Title: `t('terminology.heading', { kind })`.

Delete confirmation: open a `<Dialog>` showing `t('terminology.action.delete.confirmTitle')` + the codelist `code`/`name`, with `Confirm`/`Cancel` buttons. On confirm, call `deleteCodelist.mutate({ id, versionId })`.

- [ ] **Step 3: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(terminology): TerminologyPage component"
```

---

## Task 26: `pages/CodeListDetailPage.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx`

**Interfaces:**
- Reads `codelistId` from `Route.useParams()`. Uses the cached `useListTerminologyVersions()` to derive `kind` + breadcrumb. Layout from spec section 6.2.

- [ ] **Step 1: Implement**

```tsx
import { useMemo, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  Alert, Box, Button, CircularProgress, Dialog, DialogActions, DialogContent,
  DialogContentText, DialogTitle, IconButton, Tooltip, Typography,
} from '@aegis/ui/mui';
import { ArrowBack as ArrowBackIcon } from '@aegis/ui/icons';
import { useI18n } from '@aegis/ui/i18n';

import { errorMessage } from '../../../shared/api/error';
import { useCurrentUser } from '../../auth';
import {
  useListTerminologyVersions,
  useListCodeLists,
  useListCodeItems,
  useDeleteCodeItem,
  useUpdateCodeList,
  useCreateCodeItem,
  useUpdateCodeItem,
} from '../data';
import { CodeListTable } from '../components/CodeListTable';
import { CodeItemTable } from '../components/CodeItemTable';
import { CodeListDrawer } from '../components/CodeListDrawer';
import { CodeItemDrawer } from '../components/CodeItemDrawer';
import { TermFilterBar } from '../components/TermFilterBar';
import type {
  CodeListView, CodeItemView, UpdateCodeListInput, UpdateCodeItemInput,
} from '../../../shared/api';

type ItemDrawerState =
  | { mode: 'create' }
  | { mode: 'edit'; row: CodeItemView }
  | null;

export function CodeListDetailPage() {
  // The route file's parseParams coerces to number; we cast accordingly.
  const params = (Route as unknown as { useParams: () => { codelistId: number } }).useParams();
  const codelistId = params.codelistId;
  const navigate = useNavigate();
  const { t } = useI18n();
  const currentUser = useCurrentUser();
  const versionsQuery = useListTerminologyVersions();
  const codeListsQuery = useListCodeLists(null); // re-fetched once we know the version
  const itemsQuery = useListCodeItems(codelistId);

  const role = currentUser.data?.role;
  const canMutate = role === 'admin' || role === 'root';

  // Pick the codelist out of whichever version's list happens to contain it.
  const allLists = codeListsQuery.data ?? [];
  const codelist = useMemo(
    () => allLists.find((cl) => cl.id === codelistId),
    [allLists, codelistId],
  );

  // Derive the kind for the back-link + breadcrumb.
  const version = useMemo(
    () => (codelist
      ? (versionsQuery.data ?? []).find((v) => v.id === codelist.versionId)
      : undefined),
    [codelist, versionsQuery.data],
  );
  const kind = version?.kind
    ?? (typeof window !== 'undefined' && window.location.pathname.startsWith('/terminology/adam')
      ? 'adam' : 'sdtm');
  const backLink = `/terminology/${kind}`;

  const [search, setSearch] = useState('');
  const [editCodelistDrawerOpen, setEditCodelistDrawerOpen] = useState(false);
  const [itemDrawer, setItemDrawer] = useState<ItemDrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeItemView | null>(null);

  const updateCodelist = useUpdateCodeList();
  const createItem = useCreateCodeItem();
  const updateItem = useUpdateCodeItem();
  const deleteItem = useDeleteCodeItem();

  const filteredItems = useMemo(() => {
    const list = itemsQuery.data ?? [];
    const q = search.trim().toLowerCase();
    if (!q) return list;
    return list.filter((it) =>
      it.code.toLowerCase().includes(q) ||
      it.submissionValue.toLowerCase().includes(q) ||
      it.synonym.toLowerCase().includes(q) ||
      it.definition.toLowerCase().includes(q) ||
      it.nciPreferredTerm.toLowerCase().includes(q),
    );
  }, [itemsQuery.data, search]);

  const mutationLoading =
    updateCodelist.isPending || createItem.isPending || updateItem.isPending || deleteItem.isPending;

  const error = codeListsQuery.error ?? itemsQuery.error;

  if (error && !codelist) {
    return (
      <Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 1 }}>
        <Alert severity="error">{t('terminology.codeitem.loadFailed', { message: errorMessage(error) })}</Alert>
        <Box>
          <Button onClick={() => navigate({ to: backLink })}>{t('common.back')}</Button>
        </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <Tooltip title={t('common.back')}>
          <span>
            <IconButton onClick={() => navigate({ to: backLink })} disabled={!backLink}>
              <ArrowBackIcon />
            </IconButton>
          </span>
        </Tooltip>
        <Typography variant="h6">
          {codelist
            ? t('terminology.detail.heading', { kind: kind.toUpperCase(), code: codelist.code })
            : t('common.retry')}
        </Typography>
      </Box>

      {codelist && (
        <CodeListTable
          mode="single"
          rows={[codelist]}
          loading={codeListsQuery.isLoading}
          mutationLoading={mutationLoading}
          error={null}
          canMutate={canMutate}
          onRetry={() => codeListsQuery.refetch()}
          onEdit={() => setEditCodelistDrawerOpen(true)}
        />
      )}

      <TermFilterBar query={search} onQueryChange={setSearch} placeholder={t('terminology.codeitem.search.placeholder')} />

      <CodeItemTable
        rows={filteredItems}
        loading={itemsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={itemsQuery.error}
        canMutate={canMutate}
        onRetry={itemsQuery.refetch}
        onCreate={() => setItemDrawer({ mode: 'create' })}
        onEdit={(row) => setItemDrawer({ mode: 'edit', row })}
        onDelete={(row) => setConfirmDelete(row)}
        emptyMessage={search ? t('terminology.codeitem.noMatches') : t('terminology.codeitem.empty')}
      />

      {codelist && (
        <CodeListDrawer
          open={editCodelistDrawerOpen}
          mode="edit"
          row={codelist}
          versions={versionsQuery.data ?? []}
          versionId={codelist.versionId}
          onClose={() => setEditCodelistDrawerOpen(false)}
          onCreate={() => { /* unreachable in edit mode */ }}
          onUpdate={(_id, body: UpdateCodeListInput) => {
            updateCodelist.mutate(
              { id: codelist.id, body },
              { onSuccess: () => setEditCodelistDrawerOpen(false) },
            );
          }}
          canMutate={canMutate}
          mutationError={updateCodelist.error}
          mutationPending={updateCodelist.isPending}
        />
      )}

      <CodeItemDrawer
        open={itemDrawer !== null}
        mode={itemDrawer?.mode ?? 'create'}
        row={itemDrawer?.mode === 'edit' ? itemDrawer.row : undefined}
        codelistId={codelistId}
        versionId={codelist?.versionId ?? 0}
        onClose={() => setItemDrawer(null)}
        onCreate={(input) => {
          createItem.mutate(input, { onSuccess: () => setItemDrawer(null) });
        }}
        onUpdate={(id, body: UpdateCodeItemInput) => {
          updateItem.mutate({ id, body }, { onSuccess: () => setItemDrawer(null) });
        }}
        canMutate={canMutate}
        mutationError={createItem.error ?? updateItem.error}
        mutationPending={createItem.isPending || updateItem.isPending}
      />

      <Dialog open={confirmDelete !== null} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>{t('terminology.codeitem.action.delete.confirmTitle')}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t('terminology.codeitem.action.delete.confirmMessage')}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)}>{t('common.cancel')}</Button>
          <Button
            color="error"
            onClick={() => {
              if (!confirmDelete) return;
              deleteItem.mutate(
                { id: confirmDelete.id, codelistId: confirmDelete.codelistId },
                { onSuccess: () => setConfirmDelete(null) },
              );
            }}
          >
            {t('common.confirm')}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
```

Notes embedded in the code:

- `useListCodeLists(null)` is set to `enabled = false` (per its signature); we let `codeListsQuery` stay disabled until we know the version. To make this work, the hook must accept `null` and short-circuit; if the hook signature already requires a number, replace with a two-step "fetch all codelists for the first version, then narrow down" approach using `useListCodeLists(codelist?.versionId ?? 0)`.
- The `Route.useParams` cast compensates for the file-route macro not registering the typed params yet at the time this plan was authored; the project should keep an eye on whether TanStack Router 1.170+ emits typed `useParams()` automatically. If it does, replace the cast with a direct `Route.useParams()` call.

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(terminology): CodeListDetailPage component"
```

---

## Task 27: 4 route files

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/sdtm/codelists/$codelistId.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/adam/codelists/$codelistId.tsx`

- [ ] **Step 1: `sdtm.tsx`**

```tsx
import { createFileRoute } from '@tanstack/react-router';
import { TerminologyPage } from '../../../../../features/terminology';

export const Route = createFileRoute('/_authed/_layout/terminology/sdtm')({
  component: () => <TerminologyPage kind="sdtm" />,
});
```

- [ ] **Step 2: `adam.tsx`** — identical except `kind="adam"` and the file path.

- [ ] **Step 3: `sdtm/codelists/$codelistId.tsx`**

```tsx
import { createFileRoute } from '@tanstack/react-router';
import { CodeListDetailPage } from '../../../../../../features/terminology';

export const Route = createFileRoute(
  '/_authed/_layout/terminology/sdtm/codelists/$codelistId',
)({
  parseParams: (raw) => ({ codelistId: Number(raw.codelistId) }),
  stringifyParams: ({ codelistId }) => ({ codelistId: String(codelistId) }),
  component: CodeListDetailPage,
});
```

- [ ] **Step 4: `adam/codelists/$codelistId.tsx`** — same with the `adam` segment.

- [ ] **Step 5: Generate the route tree**

Run: `cd apps/desktop/aegis-desktop && pnpm dev --host 127.0.0.1 --port 0` for a moment (or just `pnpm build` to trigger the plugin). Alternatively run `pnpm exec tsr generate` if the project exposes it; otherwise the plugin regenerates on `vite dev`/`vite build`.

Run: `cd apps/desktop/aegis-desktop && pnpm build`
Expected: typecheck + build succeed; `src/routes/routeTree.gen.ts` contains the new routes.

- [ ] **Step 6: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git commit -m "feat(terminology): routes for sdtm/adam terminology pages"
```

---

## Task 28: Add `terminologyEntry` to `AppLayout.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`

- [ ] **Step 1: Add icons + menu entry**

```tsx
import {
  AdminPanelSettings as AdminPanelSettingsIcon,
  Analytics as AnalyticsIcon,
  Home as HomeIcon,
  MenuBook as MenuBookIcon,
  People as PeopleIcon,
  Settings as SettingsIcon,
  Storage as StorageIcon,
  Workspaces as WorkspacesIcon,
} from '@aegis/ui/icons';

// inside AppLayout:
const TerminologyMenuIcon = () => <MenuBookIcon />;
const SdtmMenuIcon = () => <StorageIcon />;
const AdamMenuIcon = () => <AnalyticsIcon />;

const terminologyEntry: MenuItem = {
  link: '#',
  title: t('nav.terminology'),
  icon: TerminologyMenuIcon,
  subMenu: [
    { link: '/terminology/sdtm', title: t('nav.terminology.sdtm'), icon: SdtmMenuIcon },
    { link: '/terminology/adam', title: t('nav.terminology.adam'), icon: AdamMenuIcon },
  ],
};

const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 2),     // Home, Projects
      terminologyEntry,             // Terminology (submenu: SDTM, ADaM)
      managementEntry,              // Management (submenu: Users)
      ...baseMenu.slice(2),         // Settings
    ]
  : baseMenu;
```

- [ ] **Step 2: Typecheck + visual sanity check**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(terminology): add terminology menu entry to sidebar"
```

---

## Task 29: Add ~25 i18n keys to `en.ts` and `zhCN.ts`

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Append keys to `en.ts`**

The full list from spec section 10. Place under a `// Terminology` comment, after the existing `user.*` keys.

- [ ] **Step 2: Append matching zh-CN translations to `zhCN.ts`**

Final translations (commit verbatim — do not paraphrase):

```ts
'nav.terminology': '术语',
'nav.terminology.sdtm': 'SDTM',
'nav.terminology.adam': 'ADaM',
'terminology.heading': '术语 — {kind}',
'terminology.detail.heading': '术语 — {kind} › {code}',
'terminology.version.placeholder': '暂无版本',
'terminology.version.helper': '选择术语版本',
'terminology.extensible': '可扩展',
'terminology.importComingSoon': '术语导入功能即将上线',
'terminology.codelist.search.placeholder': '按代码、名称、提交值或描述搜索',
'terminology.codelist.field.code': '代码',
'terminology.codelist.field.name': '名称',
'terminology.codelist.field.submissionValue': '提交值',
'terminology.codelist.field.descriptions': '描述',
'terminology.codelist.field.extensible': '可扩展',
'terminology.codelist.field.synonym': '同义词',
'terminology.codelist.field.definition': '定义',
'terminology.codelist.field.nciPreferredTerm': 'NCI 首选术语',
'terminology.codelist.empty': '该版本下暂无代码列表',
'terminology.codelist.noMatches': '未找到匹配的代码列表',
'terminology.codelist.loadFailed': '加载代码列表失败：{message}',
'terminology.codelist.create.title': '创建代码列表',
'terminology.codelist.edit.title': '编辑代码列表',
'terminology.codelist.action.create': '创建',
'terminology.codelist.action.save': '保存',
'terminology.codelist.readOnly': '只读',
'terminology.codeitem.search.placeholder': '按代码、提交值或描述搜索',
'terminology.codeitem.field.code': '代码',
'terminology.codeitem.field.submissionValue': '提交值',
'terminology.codeitem.field.descriptions': '描述',
'terminology.codeitem.field.synonym': '同义词',
'terminology.codeitem.field.definition': '定义',
'terminology.codeitem.field.nciPreferredTerm': 'NCI 首选术语',
'terminology.codeitem.empty': '该代码列表下暂无代码项',
'terminology.codeitem.noMatches': '未找到匹配的代码项',
'terminology.codeitem.loadFailed': '加载代码项失败：{message}',
'terminology.codeitem.create.title': '创建代码项',
'terminology.codeitem.edit.title': '编辑代码项',
'terminology.codeitem.action.create': '创建',
'terminology.codeitem.action.save': '保存',
'terminology.codeitem.readOnly': '只读',
'terminology.action.delete.confirmTitle': '删除代码列表',
'terminology.action.delete.confirmMessage': '确认删除此代码列表及其全部代码项？此操作不可撤销。',
'terminology.codeitem.action.delete.confirmTitle': '删除代码项',
'terminology.codeitem.action.delete.confirmMessage': '确认删除此代码项？此操作不可撤销。',
'common.confirm': '确认',
'common.cancel': '取消',
'common.retry': '重试',
'common.back': '返回',
```

Note: `common.cancel`, `common.retry` may already exist in the locales — verify and skip duplicates.

- [ ] **Step 3: Verify translations are typed**

The `en` and `zhCN` exports are typed `as const`. The new keys must be added to BOTH files for `useI18n()` to typecheck.

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(terminology): i18n keys for terminology pages"
```

---

## Task 30: Smoke verification

**Files:** none.

- [ ] **Step 1: Run the full test suite**

Run: `cd apps/desktop/aegis-desktop && pnpm test --run`
Expected: all tests pass.

- [ ] **Step 2: Run the Rust test suite**

Run: `cargo test -p aegis-desktop`
Expected: all tests pass.

- [ ] **Step 3: Typecheck + build**

Run: `cd apps/desktop/aegis-desktop && pnpm build`
Expected: typecheck + vite build succeed.

- [ ] **Step 4: Manual smoke test**

Run: `cd apps/desktop/aegis-desktop && pnpm tauri dev`
Open the app, log in as a `root` user. Verify:
- Sidebar shows Terminology → SDTM and Terminology → ADaM entries.
- Clicking either navigates to the right page; the dropdown is disabled when no versions exist.
- Seed at least one version (via the existing server / curl) and confirm the dropdown populates; selecting it loads the code list table.
- Add / edit / delete codelist works (admin/root only). As a `general` user, the action buttons are hidden and the drawers don't open.
- The detail page (`/terminology/sdtm/codelists/<id>`) loads the code items; add / edit / delete work as admin.
- Search field filters the table client-side.
- Descriptions cell renders chip+value rows for non-empty fields and skips empty fields.
- Extensible icon shows next to codes where `extensible === true`.

If any step fails, file an issue and fix forward; do not close the task until every step passes.

- [ ] **Step 5: Commit any smoke-test fixes**

If any code change was needed to make the manual smoke test pass, commit it under `fix(terminology): …`.

---

## Done Criteria

- All 30 tasks complete and committed on `feat/desktop_terminology`.
- `cargo test -p aegis-desktop` green.
- `pnpm test --run` green.
- `pnpm typecheck` green.
- `pnpm build` green.
- Manual smoke test passes for both admin/root and general roles.
- Spec coverage: every spec section has at least one task; the 5 deferred items from spec section 11 remain explicitly out of scope.