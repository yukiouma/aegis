# CrfDetailPage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the body of `CrfDetailPage` so the user can read and mutate `(domain_)annotations` for a CRF form, using the existing server endpoints and adding the missing Rust shim / Tauri command / TS API client layers.

**Architecture:** Mirror the existing `crf::form` shim pattern (DTO struct in `http/crf/*.rs` → `#[tauri::command]` in `commands/crf/*.rs` → wrapper in `shared/api/index.ts` → query hook in `features/crf/data/*` → presentational component). The page composes a header (back / code chip / name + hover popup / domain-annotation chips / spacer / tools menu), a form-annotation chip area, and a per-item row with its own annotation chips + unit + options.

**Tech Stack:** Rust 2024 (Tauri 2.x, reqwest, serde, wiremock), TypeScript (React 18, MUI 5, TanStack Query 5, TanStack Router, i18n via `@aegis/ui/i18n`), Vitest + Testing Library.

---

## File Structure

Files created:
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs`
- `apps/desktop/aegis-desktop/src/features/crf/data/detail.ts`
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/DeleteDomainAnnotationDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/DeleteAnnotationDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-domain-annotation-dialog.test.tsx`

Files modified:
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs` (register new modules)
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs` (add DTOs + `details` helper + test)
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs` (register new modules)
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` (add `get_crf_form_details` shim)
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (register new commands)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (add TS wire DTOs + input shapes)
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` (add wrappers + exports)
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (add `crf.formDetail` key)
- `apps/desktop/aegis-desktop/src/features/crf/components/index.ts` (export new components)
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` (compose the page)
- `lib/packages/ui/src/i18n/locales/en.ts` (add strings)
- `lib/packages/ui/src/i18n/locales/zhCN.ts` (add Chinese strings)
- `apps/desktop/aegis-desktop/src/test/shared/api.test.ts` (extend with new wrappers)

---

## Task 1: Add TS wire DTO types

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts:435-end`

- [ ] **Step 1: Append the new TS interfaces and unions to types.ts**

Append the following block to `apps/desktop/aegis-desktop/src/shared/api/types.ts` (after the existing `UpdateCrfFormInput`):

```ts
// ---- CRF detail / items / options / units ----

export type CrfItemKind =
  | "text"
  | "selection"
  | "checkbox"
  | "datetime"
  | "label";

export interface CrfItem {
  id: number;
  formId: number;
  code: string;
  name: string;
  kind: CrfItemKind;
  order: number;
  notSubmitted: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CrfOption {
  id: number;
  itemId: number;
  value: string;
  notSubmitted: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CrfUnit {
  id: number;
  itemId: number;
  value: string;
  notSubmitted: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface DomainAnnotation {
  id: number;
  formId: number;
  name: string;
  description: string;
  createdAt: string;
  updatedAt: string;
}

export type AnnotationOwner =
  | { kind: "form"; id: number }
  | { kind: "item"; id: number }
  | { kind: "option"; id: number }
  | { kind: "unit"; id: number };

export interface Annotation {
  id: number;
  domainAnnotationId: number;
  content: string;
  assign: boolean;
  owner: AnnotationOwner;
  createdAt: string;
  updatedAt: string;
}

export interface CrfOptionDetail {
  option: CrfOption;
  annotations: Annotation[];
}

export interface CrfUnitDetail {
  unit: CrfUnit;
  annotations: Annotation[];
}

export interface CrfItemDetail {
  item: CrfItem;
  options: CrfOptionDetail[];
  units: CrfUnitDetail[];
  annotations: Annotation[];
}

export interface CrfFormDetail {
  form: CrfForm;
  formAnnotations: Annotation[];
  items: CrfItemDetail[];
  domainAnnotations: DomainAnnotation[];
}

export interface CreateDomainAnnotationInput {
  name: string;
  description: string;
}

export interface UpdateDomainAnnotationInput {
  name?: string;
  description?: string;
}

export interface CreateAnnotationInput {
  domainAnnotationId: number;
  content: string;
  assign: boolean;
  owner: AnnotationOwner;
}

export interface UpdateAnnotationInput {
  content?: string;
  assign?: boolean;
}
```

- [ ] **Step 2: Run typecheck to verify the new types compile**

Run from repo root:
```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS (no errors).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts
git commit -m "feat(crf): add detail-page wire DTO types

Adds CrfItem / CrfOption / CrfUnit / DomainAnnotation / Annotation /
CrfFormDetail plus Create/Update input shapes mirroring the server
DTOs in apps/server/aegis-server/src/transport/http/dto.rs.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Extend Rust form.rs with detail DTOs + helper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`

- [ ] **Step 1: Add new DTO structs and the `details` HTTP helper**

Append the following to `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs` (after the existing `get_by_id`):

```rust
// ---- detail composition ----

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AnnotationOwner {
    Form { id: i64 },
    Item { id: i64 },
    #[serde(rename = "option")]
    Option { id: i64 },
    Unit { id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationViewResponse {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionDetailResponse {
    pub option: CrfOptionViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitDetailResponse {
    pub unit: CrfUnitViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemDetailResponse {
    pub item: CrfItemViewResponse,
    pub options: Vec<CrfOptionDetailResponse>,
    pub units: Vec<CrfUnitDetailResponse>,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormDetailResponse {
    pub form: CrfFormViewResponse,
    pub form_annotations: Vec<AnnotationViewResponse>,
    pub items: Vec<CrfItemDetailResponse>,
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

pub async fn details(
    c: &HttpClient,
    id: i64,
) -> Result<CrfFormDetailResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{id}/details"),
        None::<&()>,
    )
    .await
}
```

(The `use serde::...` import is already in scope from the existing file — remove the duplicate if present; the file already has `use serde::{Deserialize, Serialize};`.)

- [ ] **Step 2: Add a wiremock test for `details`**

Append the following to the existing `mod tests` in `form.rs`:

```rust
#[tokio::test]
async fn details_returns_composed_view() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/crf/forms/11/details"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "form": form_view_json(11, 7, "AE", "Adverse Events"),
            "formAnnotations": [{
                "id": 100,
                "domainAnnotationId": 50,
                "content": "form-level note",
                "assign": false,
                "owner": { "kind": "form", "id": 11 },
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-02T00:00:00Z"
            }],
            "items": [{
                "item": {
                    "id": 21, "formId": 11, "code": "AETERM", "name": "Term",
                    "kind": "text", "order": 0, "notSubmitted": false,
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                },
                "options": [],
                "units": [],
                "annotations": []
            }],
            "domainAnnotations": [{
                "id": 50, "formId": 11,
                "name": "Adverse Events", "description": "AE",
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-02T00:00:00Z"
            }]
        })))
        .mount(&server)
        .await;
    let resp = details(&client(&server), 11).await.unwrap();
    assert_eq!(resp.form.id, 11);
    assert_eq!(resp.form_annotations.len(), 1);
    assert_eq!(resp.items.len(), 1);
    assert_eq!(resp.items[0].item.code, "AETERM");
    assert_eq!(resp.domain_annotations.len(), 1);
    assert_eq!(resp.domain_annotations[0].name, "Adverse Events");
}
```

- [ ] **Step 3: Run the test**

Run from repo root:
```bash
cargo test -p aegis-desktop --lib http::crf::form::tests::details_returns_composed_view
```
Expected: PASS.

- [ ] **Step 4: Run cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --lib -- -D warnings
```
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs
git commit -m "feat(aegis-desktop): add CrfFormDetailResponse + details helper

Mirrors the server DTO in apps/server/aegis-server/src/transport/http/dto.rs:
AnnotationOwner, CrfItem/Option/UnitViewResponse, AnnotationViewResponse,
DomainAnnotationViewResponse, plus the composed CrfFormDetailResponse and a
GET /api/crf/forms/{id}/details helper. Includes a wiremock test.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Add Rust http/crf/item.rs

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs`

- [ ] **Step 1: Create the module**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs` with:

```rust
//! HTTP functions under `/api/crf/items/{id}` and `/api/crf/forms/{id}/items`.

use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemListResponse {
    pub items: Vec<CrfItemViewResponse>,
}

pub async fn list_by_form(
    c: &HttpClient,
    form_id: i64,
) -> Result<CrfItemListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{form_id}/items"),
        None::<&()>,
    )
    .await
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<CrfItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/items/{id}"),
        None::<&()>,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn item_view_json(id: i64, form_id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "formId": form_id, "code": code, "name": code,
            "kind": "text", "order": 0, "notSubmitted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_by_form_returns_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_view_json(21, 11, "AETERM")]
            })))
            .mount(&server)
            .await;
        let resp = list_by_form(&client(&server), 11).await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "AETERM");
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/items/21"))
            .respond_with(ResponseTemplate::new(200).set_body_json(item_view_json(21, 11, "AETERM")))
            .mount(&server)
            .await;
        let resp = get_by_id(&client(&server), 21).await.unwrap();
        assert_eq!(resp.id, 21);
        assert_eq!(
            resp.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }
}
```

- [ ] **Step 2: Register the new module**

Edit `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs`:

```rust
//! HTTP functions for the CRF namespace. One submodule per resource.
pub mod annotation;
pub mod domain_annotation;
pub mod form;
pub mod item;
pub mod version;
```

- [ ] **Step 3: Run the tests**

```bash
cargo test -p aegis-desktop --lib http::crf::item::tests
```
Expected: PASS (both tests).

- [ ] **Step 4: Run cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --lib -- -D warnings
```
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/ \
        apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs
git commit -m "feat(aegis-desktop): add http/crf/item shim

Mirrors the per-item endpoints used by future flows and the search
boxes the global-search page will need.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Add Rust http/crf/annotation.rs

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs`

- [ ] **Step 1: Create the module**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs` with:

```rust
//! HTTP functions for the polymorphic `Annotation` resource.
//!
//! The polymorphic owner lives in the request body, so
//! `CreateAnnotationRequest` carries `AnnotationOwner`. Reads are
//! keyed by form / item / option / unit id per the server.

use serde::{Deserialize, Serialize};

use super::form::AnnotationOwner;
use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign: Option<bool>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/crf/annotations",
        Some(&body),
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/annotations/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/annotations/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

// We re-use the AnnotationViewResponse defined in form.rs to keep
// the wire type in one place (avoids accidentally drifting the
// serde rename between two copies).
pub use super::form::AnnotationViewResponse;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn annotation_json(id: i64, owner: AnnotationOwner) -> serde_json::Value {
        let owner_json = match owner {
            AnnotationOwner::Form { id } => serde_json::json!({ "kind": "form", "id": id }),
            AnnotationOwner::Item { id } => serde_json::json!({ "kind": "item", "id": id }),
            AnnotationOwner::Option { id } => serde_json::json!({ "kind": "option", "id": id }),
            AnnotationOwner::Unit { id } => serde_json::json!({ "kind": "unit", "id": id }),
        };
        serde_json::json!({
            "id": id,
            "domainAnnotationId": 50,
            "content": "note",
            "assign": false,
            "owner": owner_json,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/annotations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(annotation_json(100, AnnotationOwner::Form { id: 11 })))
            .mount(&server)
            .await;
        let view = create(
            &client(&server),
            CreateAnnotationRequest {
                domain_annotation_id: 50,
                content: "note".into(),
                assign: false,
                owner: AnnotationOwner::Form { id: 11 },
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 100);
        match view.owner {
            AnnotationOwner::Form { id } => assert_eq!(id, 11),
            _ => panic!("expected form owner"),
        }
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/annotations/100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(annotation_json(100, AnnotationOwner::Item { id: 21 })))
            .mount(&server)
            .await;
        let view = update(
            &client(&server),
            100,
            UpdateAnnotationRequest {
                content: Some("renamed".into()),
                assign: None,
            },
        )
        .await
        .unwrap();
        match view.owner {
            AnnotationOwner::Item { id } => assert_eq!(id, 21),
            _ => panic!("expected item owner"),
        }
    }

    #[tokio::test]
    async fn update_request_skips_none_fields() {
        let body = UpdateAnnotationRequest {
            content: Some("renamed".into()),
            assign: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"content":"renamed"}"#);
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/annotations/100"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 100).await.unwrap();
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo test -p aegis-desktop --lib http::crf::annotation::tests
```
Expected: PASS (4 tests).

- [ ] **Step 3: Run cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --lib -- -D warnings
```
Expected: no warnings.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs
git commit -m "feat(aegis-desktop): add http/crf/annotation shim

Owner travels in the body (polymorphic). Re-uses
AnnotationViewResponse from form.rs to keep the wire type in one
place.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Add Rust http/crf/domain_annotation.rs

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs`

- [ ] **Step 1: Create the module**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs` with:

```rust
//! HTTP functions for `DomainAnnotation`. CRUD plus a list-by-form
//! helper used by the detail page's domain-annotation chip row.

use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

pub use super::form::DomainAnnotationViewResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainAnnotationRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationListResponse {
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

pub async fn create(
    c: &HttpClient,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/forms/{form_id}/domain-annotations"),
        Some(&body),
    )
    .await
}

pub async fn list_by_form(
    c: &HttpClient,
    form_id: i64,
) -> Result<DomainAnnotationListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{form_id}/domain-annotations"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/domain-annotations/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/domain-annotations/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn domain_json(id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id, "formId": 11,
            "name": "Adverse Events", "description": "AE",
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/domain-annotations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(domain_json(50)))
            .mount(&server)
            .await;
        let view = create(
            &client(&server),
            11,
            CreateDomainAnnotationRequest {
                name: "Adverse Events".into(),
                description: "AE".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 50);
    }

    #[tokio::test]
    async fn list_by_form_returns_views() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11/domain-annotations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "domainAnnotations": [domain_json(50), domain_json(51)]
            })))
            .mount(&server)
            .await;
        let resp = list_by_form(&client(&server), 11).await.unwrap();
        assert_eq!(resp.domain_annotations.len(), 2);
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/domain-annotations/50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(domain_json(50)))
            .mount(&server)
            .await;
        let view = update(
            &client(&server),
            50,
            UpdateDomainAnnotationRequest {
                name: Some("Renamed".into()),
                description: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(view.name, "Adverse Events");
    }

    #[tokio::test]
    async fn update_request_skips_none_fields() {
        let body = UpdateDomainAnnotationRequest {
            name: Some("renamed".into()),
            description: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/domain-annotations/50"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 50).await.unwrap();
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo test -p aegis-desktop --lib http::crf::domain_annotation::tests
```
Expected: PASS (5 tests).

- [ ] **Step 3: Run cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --lib -- -D warnings
```
Expected: no warnings.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs
git commit -m "feat(aegis-desktop): add http/crf/domain_annotation shim

CRUD plus list_by_form. Re-uses DomainAnnotationViewResponse from
form.rs.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Add Rust command shims and register in lib.rs

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Add `get_crf_form_details` shim to commands/crf/form.rs**

Replace `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` with:

```rust
//! Tauri command shims for `http::crf::form`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::form::{
    self, CreateCrfFormRequest, CrfFormDetailResponse, CrfFormListResponse,
    CrfFormViewResponse, UpdateCrfFormRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<CrfFormListResponse, ApiError> {
    form::list_by_version(&client, version_id).await
}

#[tauri::command]
pub async fn create_crf_form(
    client: State<'_, HttpClient>,
    version_id: i64,
    body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::create(&client, version_id, body).await
}

#[tauri::command]
pub async fn update_crf_form(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_form(client: State<'_, HttpClient>, id: i64) -> Result<(), ApiError> {
    form::delete(&client, id).await
}

#[tauri::command]
pub async fn get_crf_form_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    form::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn get_crf_form_details(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfFormDetailResponse, ApiError> {
    form::details(&client, id).await
}
```

- [ ] **Step 2: Create commands/crf/item.rs**

```rust
//! Tauri command shims for `http::crf::item`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::item::{self, CrfItemListResponse, CrfItemViewResponse};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_items_by_form(
    client: State<'_, HttpClient>,
    form_id: i64,
) -> Result<CrfItemListResponse, ApiError> {
    item::list_by_form(&client, form_id).await
}

#[tauri::command]
pub async fn get_crf_item_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfItemViewResponse, ApiError> {
    item::get_by_id(&client, id).await
}
```

- [ ] **Step 3: Create commands/crf/annotation.rs**

```rust
//! Tauri command shims for `http::crf::annotation`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::annotation::{
    self, CreateAnnotationRequest, UpdateAnnotationRequest,
};
use crate::http::crf::form::AnnotationViewResponse;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_crf_annotation(
    client: State<'_, HttpClient>,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    annotation::create(&client, body).await
}

#[tauri::command]
pub async fn update_crf_annotation(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    annotation::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_annotation(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    annotation::delete(&client, id).await
}
```

- [ ] **Step 4: Create commands/crf/domain_annotation.rs**

```rust
//! Tauri command shims for `http::crf::domain_annotation`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::domain_annotation::{
    self, CreateDomainAnnotationRequest, UpdateDomainAnnotationRequest,
};
use crate::http::crf::form::DomainAnnotationViewResponse;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_crf_domain_annotation(
    client: State<'_, HttpClient>,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    domain_annotation::create(&client, form_id, body).await
}

#[tauri::command]
pub async fn list_crf_domain_annotations_by_form(
    client: State<'_, HttpClient>,
    form_id: i64,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    domain_annotation::list_by_form(&client, form_id).await
}

#[tauri::command]
pub async fn update_crf_domain_annotation(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    domain_annotation::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_domain_annotation(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    domain_annotation::delete(&client, id).await
}
```

- [ ] **Step 5: Register the new submodules**

Edit `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs`:

```rust
//! Tauri command shims for the CRF HTTP layer.
pub mod annotation;
pub mod domain_annotation;
pub mod form;
pub mod item;
pub mod version;
```

- [ ] **Step 6: Register the new commands in lib.rs**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, expand the `// crf` block of `generate_handler!` to:

```rust
            // crf
            commands::crf::version::list_crf_versions,
            commands::crf::form::list_crf_forms_by_version,
            commands::crf::form::create_crf_form,
            commands::crf::form::update_crf_form,
            commands::crf::form::delete_crf_form,
            commands::crf::form::get_crf_form_by_id,
            commands::crf::form::get_crf_form_details,
            commands::crf::item::list_crf_items_by_form,
            commands::crf::item::get_crf_item_by_id,
            commands::crf::annotation::create_crf_annotation,
            commands::crf::annotation::update_crf_annotation,
            commands::crf::annotation::delete_crf_annotation,
            commands::crf::domain_annotation::create_crf_domain_annotation,
            commands::crf::domain_annotation::list_crf_domain_annotations_by_form,
            commands::crf::domain_annotation::update_crf_domain_annotation,
            commands::crf::domain_annotation::delete_crf_domain_annotation,
```

- [ ] **Step 7: Verify the Tauri shell still compiles**

```bash
cargo check -p aegis-desktop
```
Expected: no errors.

- [ ] **Step 8: Run cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```
Expected: no warnings.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/
git commit -m "feat(aegis-desktop): wire new CRF command shims

Adds get_crf_form_details, create/update/delete annotation,
create/list/update/delete domain annotation, and list/get item
shims. Registers every command in lib.rs.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: Add TS API client wrappers and tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/test/shared/api.test.ts`

- [ ] **Step 1: Add the new imports in shared/api/index.ts**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, extend the `import type { ... } from "./types";` block to add:

```ts
import type {
  ...
  CreateAnnotationInput,
  CreateDomainAnnotationInput,
  CreateCrfFormInput,
  ...
  Annotation,
  AnnotationOwner,
  CrfFormDetail,
  CrfItem,
  DomainAnnotation,
  ...
  UpdateAnnotationInput,
  UpdateCrfFormInput,
  UpdateDomainAnnotationInput,
} from "./types";
```

(Keep the existing imports — only add the missing ones.)

- [ ] **Step 2: Add the wrappers after the existing `deleteCrfForm`**

Append the following to the `api` object literal in `shared/api/index.ts` (right after `deleteCrfForm: ...`):

```ts
  getCrfFormDetails: (id: number): Promise<CrfFormDetail> =>
    call<CrfFormDetail>("get_crf_form_details", { id }),
  listCrfItemsByForm: async (formId: number): Promise<CrfItem[]> => {
    const resp = await call<{ items: CrfItem[] }>("list_crf_items_by_form", { formId });
    return resp.items;
  },
  createCrfDomainAnnotation: (
    formId: number,
    body: CreateDomainAnnotationInput,
  ): Promise<DomainAnnotation> =>
    call<DomainAnnotation>("create_crf_domain_annotation", { formId, body: { ...body } }),
  updateCrfDomainAnnotation: (
    id: number,
    body: UpdateDomainAnnotationInput,
  ): Promise<DomainAnnotation> =>
    call<DomainAnnotation>("update_crf_domain_annotation", { id, body: { ...body } }),
  deleteCrfDomainAnnotation: (id: number): Promise<void> =>
    call<void>("delete_crf_domain_annotation", { id }),
  createCrfAnnotation: (body: CreateAnnotationInput): Promise<Annotation> =>
    call<Annotation>("create_crf_annotation", { body: { ...body } }),
  updateCrfAnnotation: (id: number, body: UpdateAnnotationInput): Promise<Annotation> =>
    call<Annotation>("update_crf_annotation", { id, body: { ...body } }),
  deleteCrfAnnotation: (id: number): Promise<void> =>
    call<void>("delete_crf_annotation", { id }),
```

- [ ] **Step 3: Add the new types to the export block**

In the `export type { ... } from "./types";` block at the bottom of `index.ts`, add:

```ts
  Annotation,
  AnnotationOwner,
  CreateAnnotationInput,
  CreateDomainAnnotationInput,
  CrfFormDetail,
  CrfItem,
  CrfItemDetail,
  CrfItemKind,
  CrfOption,
  CrfOptionDetail,
  CrfUnit,
  CrfUnitDetail,
  DomainAnnotation,
  UpdateAnnotationInput,
  UpdateDomainAnnotationInput,
```

- [ ] **Step 4: Extend api.test.ts with wrapper assertions**

Append the following to `apps/desktop/aegis-desktop/src/test/shared/api.test.ts` (inside the existing `describe("api wrappers", ...)` block):

```ts
  it("getCrfFormDetails -> invoke('get_crf_form_details', { id })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getCrfFormDetails(11);
    expect(mockInvoke).toHaveBeenCalledWith("get_crf_form_details", { id: 11 });
  });

  it("createCrfDomainAnnotation -> invoke('create_crf_domain_annotation', { formId, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createCrfDomainAnnotation(11, { name: "AE", description: "Adverse Events" });
    expect(mockInvoke).toHaveBeenCalledWith("create_crf_domain_annotation", {
      formId: 11,
      body: { name: "AE", description: "Adverse Events" },
    });
  });

  it("updateCrfDomainAnnotation -> invoke('update_crf_domain_annotation', { id, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateCrfDomainAnnotation(50, { name: "renamed" });
    expect(mockInvoke).toHaveBeenCalledWith("update_crf_domain_annotation", {
      id: 50,
      body: { name: "renamed" },
    });
  });

  it("deleteCrfDomainAnnotation -> invoke('delete_crf_domain_annotation', { id })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.deleteCrfDomainAnnotation(50);
    expect(mockInvoke).toHaveBeenCalledWith("delete_crf_domain_annotation", { id: 50 });
  });

  it("createCrfAnnotation -> invoke('create_crf_annotation', { body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createCrfAnnotation({
      domainAnnotationId: 50,
      content: "note",
      assign: false,
      owner: { kind: "item", id: 21 },
    });
    expect(mockInvoke).toHaveBeenCalledWith("create_crf_annotation", {
      body: {
        domainAnnotationId: 50,
        content: "note",
        assign: false,
        owner: { kind: "item", id: 21 },
      },
    });
  });

  it("updateCrfAnnotation -> invoke('update_crf_annotation', { id, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateCrfAnnotation(100, { content: "renamed", assign: true });
    expect(mockInvoke).toHaveBeenCalledWith("update_crf_annotation", {
      id: 100,
      body: { content: "renamed", assign: true },
    });
  });

  it("deleteCrfAnnotation -> invoke('delete_crf_annotation', { id })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.deleteCrfAnnotation(100);
    expect(mockInvoke).toHaveBeenCalledWith("delete_crf_annotation", { id: 100 });
  });
```

- [ ] **Step 5: Run the tests**

```bash
pnpm --filter aegis-desktop test -- src/test/shared/api.test.ts
```
Expected: PASS (existing + 7 new tests).

- [ ] **Step 6: Run typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/index.ts \
        apps/desktop/aegis-desktop/src/test/shared/api.test.ts
git commit -m "feat(crf): expose detail-page API wrappers

Adds api.getCrfFormDetails, the create/update/delete pairs for both
(domain_)annotations, and api.listCrfItemsByForm. Tests assert the
invoke call shape for each new wrapper.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: Add query key and data/detail.ts hooks

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/detail.ts`

- [ ] **Step 1: Add the new query key**

In `apps/desktop/aegis-desktop/src/shared/query/keys.ts`, extend the `crf` block:

```ts
  crf: {
    versionsByProject: (projectCode: string) =>
      ["crf", "versionsByProject", projectCode] as const,
    formsByVersion: (versionId: number) =>
      ["crf", "formsByVersion", versionId] as const,
    form: (id: number) =>
      ["crf", "form", id] as const,
    formDetail: (id: number) =>
      ["crf", "formDetail", id] as const,
  },
```

- [ ] **Step 2: Create data/detail.ts**

Create `apps/desktop/aegis-desktop/src/features/crf/data/detail.ts` with:

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateAnnotationInput,
  type CreateDomainAnnotationInput,
  type CrfFormDetail,
  type UpdateAnnotationInput,
  type UpdateDomainAnnotationInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query/keys";

export function useCrfFormDetail(id: number | null) {
  return useQuery<CrfFormDetail, ApiError>({
    queryKey: queryKeys.crf.formDetail(id ?? 0),
    queryFn: () => api.getCrfFormDetails(id!),
    enabled: id != null && id > 0,
  });
}

export function useCreateDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.createCrfDomainAnnotation>>,
    ApiError,
    { formId: number; body: CreateDomainAnnotationInput }
  >({
    mutationFn: ({ formId, body }) => api.createCrfDomainAnnotation(formId, body),
    onSuccess: (_d, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useUpdateDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.updateCrfDomainAnnotation>>,
    ApiError,
    { id: number; formId: number; body: UpdateDomainAnnotationInput }
  >({
    mutationFn: ({ id, body }) => api.updateCrfDomainAnnotation(id, body),
    onSuccess: (_d, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useDeleteDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; formId: number }>({
    mutationFn: ({ id }) => api.deleteCrfDomainAnnotation(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useCreateAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.createCrfAnnotation>>,
    ApiError,
    { formId: number; body: CreateAnnotationInput }
  >({
    mutationFn: ({ body }) => api.createCrfAnnotation(body),
    onSuccess: (_a, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useUpdateAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.updateCrfAnnotation>>,
    ApiError,
    { id: number; formId: number; body: UpdateAnnotationInput }
  >({
    mutationFn: ({ id, body }) => api.updateCrfAnnotation(id, body),
    onSuccess: (_a, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useDeleteAnnotation() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; formId: number }>({
    mutationFn: ({ id }) => api.deleteCrfAnnotation(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}
```

- [ ] **Step 3: Run typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/query/keys.ts \
        apps/desktop/aegis-desktop/src/features/crf/data/detail.ts
git commit -m "feat(crf): add form-detail query + annotation mutations

useCrfFormDetail fetches the composed form payload; the six
mutations invalidate formDetail(formId) on success so chip state
reflects server state immediately.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: Add AnnotationChip component and color helper

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx` with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AnnotationChip, annotationColor } from "../../../features/crf/components/AnnotationChip";

afterEach(() => cleanup());

describe("annotationColor", () => {
  it("cycles info -> warning -> success -> error -> info", () => {
    expect(annotationColor(0)).toBe("info");
    expect(annotationColor(1)).toBe("warning");
    expect(annotationColor(2)).toBe("success");
    expect(annotationColor(3)).toBe("error");
    expect(annotationColor(4)).toBe("info");
    expect(annotationColor(-1)).toBe("default");
  });
});

describe("AnnotationChip", () => {
  const baseAnnotation = {
    id: 100,
    domainAnnotationId: 50,
    content: "form-level note",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "",
    updatedAt: "",
  };

  it("renders the annotation content", () => {
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText("form-level note")).toBeInTheDocument();
  });

  it("clicking the chip body calls onEdit", () => {
    const onEdit = vi.fn();
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={onEdit}
        onDelete={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByText("form-level note"));
    expect(onEdit).toHaveBeenCalledTimes(1);
  });

  it("clicking the delete icon calls onDelete", () => {
    const onDelete = vi.fn();
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={onDelete}
      />,
    );
    fireEvent.click(screen.getByTestId("annotation-chip-delete"));
    expect(onDelete).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx
```
Expected: FAIL — AnnotationChip is not exported.

- [ ] **Step 3: Implement AnnotationChip**

Create `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx` with:

```tsx
import { Chip } from "@aegis/ui/mui";
import type { ChipProps } from "@mui/material/Chip";
import type { Annotation } from "../../../shared/api";

/**
 * Map an index (the position of the owning domain annotation in the
 * form's domain-annotation list) to a Chip color. Cycles every 4
 * domain annotations. A negative index (the owning domain annotation
 * is not in the loaded list) falls back to the default colour.
 */
export function annotationColor(index: number): ChipProps["color"] {
  if (index < 0) return "default";
  const palette: ChipProps["color"][] = ["info", "warning", "success", "error"];
  return palette[index % palette.length];
}

interface Props {
  annotation: Annotation;
  /**
   * Index of the owning domain annotation in the form's
   * `domainAnnotations` array, or -1 if not found. Negative falls
   * through to the default palette slot.
   */
  colorIndex: number;
  onEdit: () => void;
  onDelete: () => void;
}

export function AnnotationChip({ annotation, colorIndex, onEdit, onDelete }: Props) {
  return (
    <Chip
      label={annotation.content}
      color={annotationColor(colorIndex)}
      onClick={onEdit}
      onDelete={onDelete}
      deleteIcon={
        <span data-testid="annotation-chip-delete" aria-hidden>
          ×
        </span>
      }
      size="small"
    />
  );
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx
```
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx \
        apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx
git commit -m "feat(crf): add AnnotationChip with info/warning/success/error cycle

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Add DomainAnnotationDialog component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-domain-annotation-dialog.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-domain-annotation-dialog.test.tsx` with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { DomainAnnotationDialog } from "../../../features/crf/components/DomainAnnotationDialog";

afterEach(() => cleanup());

function renderDialog(props: Partial<React.ComponentProps<typeof DomainAnnotationDialog>> = {}) {
  const onSubmit = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <DomainAnnotationDialog
        open
        mode="create"
        onClose={() => undefined}
        onSubmit={onSubmit}
        mutationError={null}
        mutationPending={false}
        {...props}
      />
    </AegisI18nProvider>,
  );
  return { onSubmit, ...utils };
}

describe("DomainAnnotationDialog", () => {
  it("submit is disabled while name is empty", () => {
    const { onSubmit } = renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Name/i), { target: { value: "AE" } });
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    expect(onSubmit).toHaveBeenCalledWith({ name: "AE", description: "" });
  });

  it("edit mode pre-fills from row", () => {
    const onSubmit = vi.fn();
    renderDialog({
      mode: "edit",
      row: { id: 50, formId: 11, name: "AE", description: "Adverse Events", createdAt: "", updatedAt: "" },
      onSubmit,
    });
    fireEvent.change(screen.getByLabelText(/Name/i), { target: { value: "Renamed" } });
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({ name: "Renamed", description: "Adverse Events" });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-domain-annotation-dialog.test.tsx
```
Expected: FAIL — DomainAnnotationDialog not exported.

- [ ] **Step 3: Implement DomainAnnotationDialog**

Create `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx`:

```tsx
import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  CreateDomainAnnotationInput,
  DomainAnnotation,
  UpdateDomainAnnotationInput,
} from "../../../shared/api";

type SubmitBody = CreateDomainAnnotationInput | UpdateDomainAnnotationInput;

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  onClose: () => void;
  onSubmit: (body: SubmitBody) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: SubmitBody = { name: "", description: "" };

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  onClose,
  onSubmit,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setName(row.name);
      setDescription(row.description);
    } else {
      setName(EMPTY.name as string);
      setDescription(EMPTY.description as string);
    }
  }, [open, mode, row]);

  const submitDisabled = mutationPending || name.trim() === "";

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({ name: name.trim(), description: description.trim() });
  }

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t(
            mode === "create"
              ? "crf.domainDialog.create.title"
              : "crf.domainDialog.edit.title",
          )}
        </Typography>
        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("crf.domainDialog.field.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
          <TextField
            size="small"
            label={t("crf.domainDialog.field.description")}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            multiline
            minRows={2}
          />
        </Stack>
        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}
        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={mutationPending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={submitDisabled}
          >
            {t(
              mode === "create"
                ? "crf.domainDialog.submit.create"
                : "crf.domainDialog.submit.save",
            )}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 4: Add the i18n strings now (so the test passes)**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.domainDialog.create.title": "Create domain annotation",
  "crf.domainDialog.edit.title": "Edit domain annotation",
  "crf.domainDialog.field.name": "Name",
  "crf.domainDialog.field.description": "Description",
  "crf.domainDialog.submit.create": "Create",
  "crf.domainDialog.submit.save": "Save",
```

Append the matching translations to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.domainDialog.create.title": "创建域注释",
  "crf.domainDialog.edit.title": "编辑域注释",
  "crf.domainDialog.field.name": "名称",
  "crf.domainDialog.field.description": "描述",
  "crf.domainDialog.submit.create": "创建",
  "crf.domainDialog.submit.save": "保存",
```

- [ ] **Step 5: Run the test**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-domain-annotation-dialog.test.tsx
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx \
        apps/desktop/aegis-desktop/src/test/features/crf/crf-domain-annotation-dialog.test.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add DomainAnnotationDialog + i18n strings

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 11: Add AnnotationDialog component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx` with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { AnnotationDialog } from "../../../features/crf/components/AnnotationDialog";
import type { DomainAnnotation, AnnotationOwner } from "../../../shared/api";

afterEach(() => cleanup());

const owner: AnnotationOwner = { kind: "form", id: 11 };
const domainAnnotations: DomainAnnotation[] = [
  { id: 50, formId: 11, name: "AE", description: "Adverse Events", createdAt: "", updatedAt: "" },
  { id: 51, formId: 11, name: "VS", description: "Vital Signs", createdAt: "", updatedAt: "" },
];

function renderDialog(props: Partial<React.ComponentProps<typeof AnnotationDialog>> = {}) {
  const onSubmit = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <AnnotationDialog
        open
        mode="create"
        owner={owner}
        availableDomainAnnotations={domainAnnotations}
        onClose={() => undefined}
        onSubmit={onSubmit}
        mutationError={null}
        mutationPending={false}
        {...props}
      />
    </AegisI18nProvider>,
  );
  return { onSubmit, ...utils };
}

describe("AnnotationDialog", () => {
  it("submit is disabled until content is non-empty", () => {
    const { onSubmit } = renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Content/i), { target: { value: "note" } });
    expect(submit).not.toBeDisabled();
  });

  it("edit mode disables the domain annotation select and the assign checkbox toggle is preserved", () => {
    const onSubmit = vi.fn();
    renderDialog({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old note",
        assign: true,
        owner,
        createdAt: "",
        updatedAt: "",
      },
      onSubmit,
    });
    // Domain annotation Select is disabled
    const combobox = screen.getByRole("combobox");
    expect(combobox).toBeDisabled();
    // Content is pre-filled
    expect(screen.getByDisplayValue("old note")).toBeInTheDocument();
    // Assign checkbox is checked
    const assign = screen.getByRole("checkbox");
    expect(assign).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({ domainAnnotationId: 50, content: "old note", assign: true });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx
```
Expected: FAIL.

- [ ] **Step 3: Implement AnnotationDialog**

Create `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`:

```tsx
import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Drawer,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  Annotation,
  AnnotationOwner,
  ApiError,
  CreateAnnotationInput,
  DomainAnnotation,
  UpdateAnnotationInput,
} from "../../../shared/api";

export interface AnnotationDialogBody {
  domainAnnotationId: number;
  content: string;
  assign: boolean;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  owner: AnnotationOwner;
  row?: Annotation;
  availableDomainAnnotations: DomainAnnotation[];
  onClose: () => void;
  /**
   * Called with the dialog body. The page composes the full
   * CreateAnnotationInput by merging the owner at the call site.
   */
  onSubmit: (body: AnnotationDialogBody) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: AnnotationDialogBody = { domainAnnotationId: 0, content: "", assign: false };

export function AnnotationDialog({
  open,
  mode,
  owner: _owner,
  row,
  availableDomainAnnotations,
  onClose,
  onSubmit,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<AnnotationDialogBody>(EMPTY);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        domainAnnotationId: row.domainAnnotationId,
        content: row.content,
        assign: row.assign,
      });
    } else {
      setBody({
        domainAnnotationId: availableDomainAnnotations[0]?.id ?? 0,
        content: "",
        assign: false,
      });
    }
  }, [open, mode, row, availableDomainAnnotations]);

  const submitDisabled =
    mutationPending ||
    body.content.trim() === "" ||
    body.domainAnnotationId === 0;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
    });
  }

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t(
            mode === "create"
              ? "crf.annotationDialog.create.title"
              : "crf.annotationDialog.edit.title",
          )}
        </Typography>
        <Stack spacing={2}>
          <FormControl size="small" disabled={mode === "edit"}>
            <InputLabel id="annotation-domain-annotation-label">
              {t("crf.annotationDialog.field.domainAnnotation")}
            </InputLabel>
            <Select
              labelId="annotation-domain-annotation-label"
              label={t("crf.annotationDialog.field.domainAnnotation")}
              value={body.domainAnnotationId || ""}
              onChange={(e) =>
                setBody((b) => ({
                  ...b,
                  domainAnnotationId: Number(e.target.value) || 0,
                }))
              }
              required
            >
              {availableDomainAnnotations.length === 0 && (
                <MenuItem value="" disabled>
                  {t("crf.annotationDialog.domainAnnotation.none")}
                </MenuItem>
              )}
              {availableDomainAnnotations.map((d) => (
                <MenuItem key={d.id} value={d.id}>
                  {d.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <TextField
            size="small"
            label={t("crf.annotationDialog.field.content")}
            value={body.content}
            onChange={(e) => setBody((b) => ({ ...b, content: e.target.value }))}
            multiline
            minRows={3}
            required
          />
          <FormControlLabel
            control={
              <Checkbox
                checked={body.assign}
                onChange={(e) => setBody((b) => ({ ...b, assign: e.target.checked }))}
              />
            }
            label={t("crf.annotationDialog.field.assign")}
          />
        </Stack>
        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}
        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={mutationPending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={submitDisabled}
          >
            {t(
              mode === "create"
                ? "crf.annotationDialog.submit.create"
                : "crf.annotationDialog.submit.save",
            )}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

(Note: `_owner` is currently unused in the dialog body — the page composes the full request — but the prop is kept so callers don't need to track ownership separately.)

- [ ] **Step 4: Add the i18n strings**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.annotationDialog.create.title": "Create annotation",
  "crf.annotationDialog.edit.title": "Edit annotation",
  "crf.annotationDialog.field.domainAnnotation": "Domain annotation",
  "crf.annotationDialog.field.content": "Content",
  "crf.annotationDialog.field.assign": "Assigned",
  "crf.annotationDialog.submit.create": "Create",
  "crf.annotationDialog.submit.save": "Save",
  "crf.annotationDialog.domainAnnotation.none": "No domain annotations on this form",
```

Append to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.annotationDialog.create.title": "创建注释",
  "crf.annotationDialog.edit.title": "编辑注释",
  "crf.annotationDialog.field.domainAnnotation": "域注释",
  "crf.annotationDialog.field.content": "内容",
  "crf.annotationDialog.field.assign": "已分配",
  "crf.annotationDialog.submit.create": "创建",
  "crf.annotationDialog.submit.save": "保存",
  "crf.annotationDialog.domainAnnotation.none": "该表单暂无域注释",
```

- [ ] **Step 5: Run the test**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx \
        apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add AnnotationDialog + i18n strings

Domain-annotation Select is disabled in edit mode (the spec fixes
the owner + domain annotation at create time). Page composes the
final CreateAnnotationInput by merging the dialog body with the
owner at the call site.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 12: Add DeleteDomainAnnotationDialog + DeleteAnnotationDialog

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/DeleteDomainAnnotationDialog.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/DeleteAnnotationDialog.tsx`

- [ ] **Step 1: Create DeleteDomainAnnotationDialog**

Create `apps/desktop/aegis-desktop/src/features/crf/components/DeleteDomainAnnotationDialog.tsx`:

```tsx
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, DomainAnnotation } from "../../../shared/api";

interface Props {
  open: boolean;
  row: DomainAnnotation | null;
  onClose: () => void;
  onConfirm: (row: DomainAnnotation) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

export function DeleteDomainAnnotationDialog({
  open,
  row,
  onClose,
  onConfirm,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>{t("crf.deleteDomain.title")}</DialogTitle>
      <DialogContent>
        {row && (
          <Alert severity="warning">
            {t("crf.deleteDomain.message", { name: row.name })}
          </Alert>
        )}
        {mutationError && (
          <Alert severity="error" sx={{ mt: 2 }}>
            {errorMessage(mutationError)}
          </Alert>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="contained"
          color="error"
          disabled={mutationPending || !row}
          onClick={() => row && onConfirm(row)}
        >
          {t("crf.deleteDomain.submit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 2: Create DeleteAnnotationDialog**

Create `apps/desktop/aegis-desktop/src/features/crf/components/DeleteAnnotationDialog.tsx`:

```tsx
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { Annotation, ApiError } from "../../../shared/api";

interface Props {
  open: boolean;
  row: Annotation | null;
  onClose: () => void;
  onConfirm: (row: Annotation) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

export function DeleteAnnotationDialog({
  open,
  row,
  onClose,
  onConfirm,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>{t("crf.deleteAnnotation.title")}</DialogTitle>
      <DialogContent>
        {row && (
          <>
            <Alert severity="warning" sx={{ mb: 1 }}>
              {t("crf.deleteAnnotation.message")}
            </Alert>
            <Typography variant="body2" sx={{ color: "text.secondary" }}>
              {row.content}
            </Typography>
          </>
        )}
        {mutationError && (
          <Alert severity="error" sx={{ mt: 2 }}>
            {errorMessage(mutationError)}
          </Alert>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="contained"
          color="error"
          disabled={mutationPending || !row}
          onClick={() => row && onConfirm(row)}
        >
          {t("crf.deleteAnnotation.submit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 3: Add the i18n strings**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.deleteDomain.title": "Delete domain annotation",
  "crf.deleteDomain.message":
    "Delete domain annotation \"{name}\" and all annotations using it? This cannot be undone.",
  "crf.deleteDomain.submit": "Delete",
  "crf.deleteAnnotation.title": "Delete annotation",
  "crf.deleteAnnotation.message":
    "Delete this annotation? This cannot be undone.",
  "crf.deleteAnnotation.submit": "Delete",
```

Append to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.deleteDomain.title": "删除域注释",
  "crf.deleteDomain.message": "删除域注释 \"{name}\" 及其所有关联注释？此操作不可撤销。",
  "crf.deleteDomain.submit": "删除",
  "crf.deleteAnnotation.title": "删除注释",
  "crf.deleteAnnotation.message": "删除此注释？此操作不可撤销。",
  "crf.deleteAnnotation.submit": "删除",
```

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/DeleteDomainAnnotationDialog.tsx \
        apps/desktop/aegis-desktop/src/features/crf/components/DeleteAnnotationDialog.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add delete dialogs for annotation and domain annotation

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 13: Add CrfAnnotationArea (form-level chips)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx`

- [ ] **Step 1: Implement CrfAnnotationArea**

Create `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx`:

```tsx
import { Box, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { Annotation } from "../../../shared/api";
import { AnnotationChip } from "./AnnotationChip";

interface Props {
  annotations: Annotation[];
  colorByDomainAnnotationId: Map<number, number>;
  onEdit: (annotation: Annotation) => void;
  onDelete: (annotation: Annotation) => void;
}

/**
 * Renders the form-level annotation chips. The list lives directly
 * under the header, above the item rows.
 */
export function CrfAnnotationArea({
  annotations,
  colorByDomainAnnotationId,
  onEdit,
  onDelete,
}: Props) {
  const { t } = useI18n();
  if (annotations.length === 0) return null;
  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <Typography variant="subtitle2" sx={{ color: "text.secondary" }}>
        {t("crf.detail.formAnnotationsHeading")}
      </Typography>
      <Stack direction="row" flexWrap="wrap" gap={1}>
        {annotations.map((a) => (
          <AnnotationChip
            key={a.id}
            annotation={a}
            colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
            onEdit={() => onEdit(a)}
            onDelete={() => onDelete(a)}
          />
        ))}
      </Stack>
    </Box>
  );
}
```

- [ ] **Step 2: Add the i18n string**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.detail.formAnnotationsHeading": "Form annotations",
```

Append to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.detail.formAnnotationsHeading": "表单级注释",
```

- [ ] **Step 3: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add CrfAnnotationArea (form-level chips)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 14: Add CrfItemRow component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx`

- [ ] **Step 1: Implement CrfItemRow**

Create `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx`:

```tsx
import { Box, Chip, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { Annotation, CrfItemDetail } from "../../../shared/api";
import { AnnotationChip } from "./AnnotationChip";

interface Props {
  itemDetail: CrfItemDetail;
  colorByDomainAnnotationId: Map<number, number>;
  /**
   * Open the new-annotation dialog for the given owner. The page
   * holds the dialog state so the caller's owner kind/id stays in
   * one place.
   */
  onCreateAnnotation: (owner: Annotation["owner"]) => void;
  onEditAnnotation: (annotation: Annotation) => void;
  onDeleteAnnotation: (annotation: Annotation) => void;
}

export function CrfItemRow({
  itemDetail,
  colorByDomainAnnotationId,
  onCreateAnnotation,
  onEditAnnotation,
  onDeleteAnnotation,
}: Props) {
  const { t } = useI18n();
  const { item, options, units, annotations } = itemDetail;
  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        gap: 1,
        p: 2,
        border: 1,
        borderColor: "divider",
        borderRadius: 1,
      }}
      data-testid={`crf-item-row-${item.id}`}
    >
      <Box
        sx={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 1,
          flexWrap: "wrap",
        }}
      >
        <Chip label={item.code} variant="outlined" size="small" />
        <Typography
          variant="subtitle1"
          sx={{ cursor: "pointer", "&:hover": { textDecoration: "underline" } }}
          onClick={() => onCreateAnnotation({ kind: "item", id: item.id })}
          data-testid={`crf-item-name-${item.id}`}
        >
          {item.name}
        </Typography>
        <Stack direction="row" flexWrap="wrap" gap={1} sx={{ flexGrow: 1 }}>
          {annotations.map((a) => (
            <AnnotationChip
              key={a.id}
              annotation={a}
              colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
              onEdit={() => onEditAnnotation(a)}
              onDelete={() => onDeleteAnnotation(a)}
            />
          ))}
        </Stack>
        {/* Unit on the right side */}
        {units.map((u) => (
          <Box
            key={u.unit.id}
            sx={{ display: "flex", alignItems: "center", gap: 1 }}
          >
            <Stack direction="row" flexWrap="wrap" gap={1}>
              {u.annotations.map((a) => (
                <AnnotationChip
                  key={a.id}
                  annotation={a}
                  colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
                  onEdit={() => onEditAnnotation(a)}
                  onDelete={() => onDeleteAnnotation(a)}
                />
              ))}
            </Stack>
            <Typography
              variant="body2"
              sx={{ cursor: "pointer", "&:hover": { textDecoration: "underline" } }}
              onClick={() => onCreateAnnotation({ kind: "unit", id: u.unit.id })}
              data-testid={`crf-unit-${u.unit.id}`}
            >
              {t("crf.detail.unitLabel", { value: u.unit.value })}
            </Typography>
          </Box>
        ))}
      </Box>
      {options.length > 0 && (
        <Box sx={{ pl: 4, display: "flex", flexDirection: "column", gap: 1 }}>
          <Typography variant="body2" sx={{ color: "text.secondary" }}>
            {t("crf.detail.optionsHeading")}
          </Typography>
          {options.map((o) => (
            <Box
              key={o.option.id}
              sx={{ display: "flex", alignItems: "center", gap: 1 }}
            >
              <Typography
                variant="body2"
                sx={{
                  flexGrow: 1,
                  cursor: "pointer",
                  "&:hover": { textDecoration: "underline" },
                }}
                onClick={() => onCreateAnnotation({ kind: "option", id: o.option.id })}
                data-testid={`crf-option-${o.option.id}`}
              >
                {t("crf.detail.optionLabel", { value: o.option.value })}
              </Typography>
              <Stack direction="row" flexWrap="wrap" gap={1}>
                {o.annotations.map((a) => (
                  <AnnotationChip
                    key={a.id}
                    annotation={a}
                    colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
                    onEdit={() => onEditAnnotation(a)}
                    onDelete={() => onDeleteAnnotation(a)}
                  />
                ))}
              </Stack>
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
}
```

- [ ] **Step 2: Add the i18n strings**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.detail.optionsHeading": "Options",
  "crf.detail.optionLabel": "Option {value}",
  "crf.detail.unitLabel": "Unit: {value}",
```

Append to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.detail.optionsHeading": "选项",
  "crf.detail.optionLabel": "选项 {value}",
  "crf.detail.unitLabel": "单位：{value}",
```

- [ ] **Step 3: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add CrfItemRow with chips, options, unit

Unit + unit annotations live on the right; options (selection
items only) render indented under the item name with their chips
to the right. All clickable surfaces open the new-annotation dialog
with the right owner kind.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 15: Compose CrfDetailPage with all new pieces

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/index.ts`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Export the new components**

Replace `apps/desktop/aegis-desktop/src/features/crf/components/index.ts` with:

```ts
export * from "./AnnotationChip";
export * from "./AnnotationDialog";
export * from "./CrfAnnotationArea";
export * from "./CrfAssignTakersDrawer";
export * from "./CrfFormDrawer";
export * from "./CrfFormFilterDrawer";
export * from "./CrfFormTable";
export * from "./CrfItemRow";
export * from "./CrfStatusChip";
export * from "./CrfToolsMenu";
export * from "./CrfVersionDropdown";
export * from "./DeleteAnnotationDialog";
export * from "./DeleteCrfFormDialog";
export * from "./DeleteDomainAnnotationDialog";
export * from "./DomainAnnotationDialog";
```

- [ ] **Step 2: Add the i18n strings**

Append to `lib/packages/ui/src/i18n/locales/en.ts`:

```ts
  "crf.detail.menu.newDomain": "New domain",
  "crf.detail.menu.newAnnotation": "New annotation",
  "crf.detail.domainChip.label": "{name} ({description})",
  "crf.detail.empty": "No items yet",
  "crf.detail.loadFailed": "Failed to load form detail: {message}",
```

Append to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  "crf.detail.menu.newDomain": "新建域注释",
  "crf.detail.menu.newAnnotation": "新建注释",
  "crf.detail.domainChip.label": "{name}（{description}）",
  "crf.detail.empty": "暂无项目",
  "crf.detail.loadFailed": "加载表单详情失败：{message}",
```

- [ ] **Step 3: Replace CrfDetailPage.tsx**

Replace `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` with:

```tsx
import { useMemo, useState } from "react";
import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  MenuItem,
  Popover,
  Stack,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams } from "@tanstack/react-router";

import {
  AnnotationChip,
  AnnotationDialog,
  CrfAnnotationArea,
  CrfItemRow,
  CrfToolsMenu,
  DeleteAnnotationDialog,
  DeleteDomainAnnotationDialog,
  DomainAnnotationDialog,
} from "../components";
import { useGetCrfForm } from "../data/list";
import {
  useCrfFormDetail,
  useCreateAnnotation,
  useCreateDomainAnnotation,
  useDeleteAnnotation,
  useDeleteDomainAnnotation,
  useUpdateAnnotation,
  useUpdateDomainAnnotation,
} from "../data/detail";
import type {
  Annotation,
  AnnotationOwner,
  DomainAnnotation,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";

type DomainDialogState =
  | { mode: "create" }
  | { mode: "edit"; row: DomainAnnotation }
  | null;

type AnnotationDialogState =
  | { mode: "create"; owner: AnnotationOwner }
  | { mode: "edit"; row: Annotation; owner: AnnotationOwner }
  | null;

export function CrfDetailPage() {
  const { t } = useI18n();
  const { projectCode, formId } = useParams({ strict: false }) as {
    projectCode: string;
    formId?: string;
  };
  const navigate = useNavigate();
  const id =
    formId != null && Number.isFinite(Number(formId)) && Number(formId) > 0
      ? Number(formId)
      : null;

  const query = useGetCrfForm(id);
  const detailQuery = useCrfFormDetail(id);

  const createDomain = useCreateDomainAnnotation();
  const updateDomain = useUpdateDomainAnnotation();
  const deleteDomain = useDeleteDomainAnnotation();
  const createAnnotation = useCreateAnnotation();
  const updateAnnotation = useUpdateAnnotation();
  const deleteAnnotation = useDeleteAnnotation();

  const [domainDialog, setDomainDialog] = useState<DomainDialogState>(null);
  const [annotationDialog, setAnnotationDialog] =
    useState<AnnotationDialogState>(null);
  const [confirmDeleteDomain, setConfirmDeleteDomain] =
    useState<DomainAnnotation | null>(null);
  const [confirmDeleteAnnotation, setConfirmDeleteAnnotation] =
    useState<Annotation | null>(null);
  const [formNameMenuAnchor, setFormNameMenuAnchor] =
    useState<HTMLElement | null>(null);

  const colorByDomainAnnotationId = useMemo(() => {
    const map = new Map<number, number>();
    detailQuery.data?.domainAnnotations.forEach((d, i) => map.set(d.id, i));
    return map;
  }, [detailQuery.data]);

  const back = () =>
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: (prev: Record<string, unknown>) => prev,
    });

  if (id == null) {
    return (
      <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
          <IconButton aria-label={t("crf.detail.back")} onClick={back}>
            <ArrowBackIcon />
          </IconButton>
          <Typography variant="h4">{t("crf.detail.title")}</Typography>
        </Box>
        <Alert severity="error">{t("common.invalidId")}</Alert>
      </Box>
    );
  }

  const form = query.data;
  const detail = detailQuery.data;

  const activeDomainMutation =
    createDomain.error ?? updateDomain.error ?? deleteDomain.error ?? null;
  const domainMutationPending =
    createDomain.isPending || updateDomain.isPending || deleteDomain.isPending;

  const activeAnnotationMutation =
    createAnnotation.error ?? updateAnnotation.error ?? deleteAnnotation.error ?? null;
  const annotationMutationPending =
    createAnnotation.isPending ||
    updateAnnotation.isPending ||
    deleteAnnotation.isPending;

  const openCreateAnnotation = (owner: AnnotationOwner) =>
    setAnnotationDialog({ mode: "create", owner });

  const openEditAnnotation = (row: Annotation, owner: AnnotationOwner) =>
    setAnnotationDialog({ mode: "edit", row, owner });

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          flexWrap: "wrap",
          gap: 2,
        }}
      >
        <IconButton aria-label={t("crf.detail.back")} onClick={back}>
          <ArrowBackIcon />
        </IconButton>
        {form?.code && <Chip label={form.code} variant="outlined" />}
        <Typography
          variant="h5"
          onMouseEnter={(e) => setFormNameMenuAnchor(e.currentTarget)}
          onMouseLeave={() => setFormNameMenuAnchor(null)}
          sx={{ cursor: "default" }}
          data-testid="crf-form-name"
        >
          {form?.name ?? t("crf.detail.title")}
        </Typography>
        <Popover
          open={Boolean(formNameMenuAnchor)}
          anchorEl={formNameMenuAnchor}
          onClose={() => setFormNameMenuAnchor(null)}
          anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
          disableAutoFocus
          disableEnforceFocus
          slotProps={{
            paper: {
              onMouseLeave: () => setFormNameMenuAnchor(null),
              sx: { minWidth: 200 },
            },
          }}
        >
          <MenuItem
            onClick={() => {
              setFormNameMenuAnchor(null);
              setDomainDialog({ mode: "create" });
            }}
          >
            {t("crf.detail.menu.newDomain")}
          </MenuItem>
          <MenuItem
            onClick={() => {
              setFormNameMenuAnchor(null);
              openCreateAnnotation({ kind: "form", id });
            }}
          >
            {t("crf.detail.menu.newAnnotation")}
          </MenuItem>
        </Popover>
        {/* Domain annotation chips, right of name */}
        {detail && detail.domainAnnotations.length > 0 && (
          <Stack direction="row" flexWrap="wrap" gap={1}>
            {detail.domainAnnotations.map((d) => (
              <Chip
                key={d.id}
                label={t("crf.detail.domainChip.label", {
                  name: d.name,
                  description: d.description,
                })}
                onClick={() => setDomainDialog({ mode: "edit", row: d })}
                onDelete={() => setConfirmDeleteDomain(d)}
                size="small"
                data-testid={`domain-annotation-chip-${d.id}`}
              />
            ))}
          </Stack>
        )}
        <Box sx={{ flexGrow: 1 }} />
        <CrfToolsMenu projectCode={projectCode} />
      </Box>

      {query.isFetching && !form && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      {query.isError && (
        <Alert severity="error">{errorMessage(query.error)}</Alert>
      )}
      {detailQuery.isError && (
        <Alert severity="error">
          {t("crf.detail.loadFailed", { message: errorMessage(detailQuery.error) })}
        </Alert>
      )}
      {detailQuery.isFetching && !detail && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      {/* Form-level annotation chips */}
      {detail && (
        <CrfAnnotationArea
          annotations={detail.formAnnotations}
          colorByDomainAnnotationId={colorByDomainAnnotationId}
          onEdit={(a) => openEditAnnotation(a, { kind: "form", id })}
          onDelete={(a) => setConfirmDeleteAnnotation(a)}
        />
      )}

      {/* Item list */}
      {detail && (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {detail.items.length === 0 ? (
            <Alert severity="info">{t("crf.detail.empty")}</Alert>
          ) : (
            detail.items.map((itemDetail) => (
              <CrfItemRow
                key={itemDetail.item.id}
                itemDetail={itemDetail}
                colorByDomainAnnotationId={colorByDomainAnnotationId}
                onCreateAnnotation={openCreateAnnotation}
                onEditAnnotation={(a) => {
                  const owner: AnnotationOwner = a.owner;
                  openEditAnnotation(a, owner);
                }}
                onDeleteAnnotation={(a) => setConfirmDeleteAnnotation(a)}
              />
            ))
          )}
        </Box>
      )}

      {/* Dialogs */}
      <DomainAnnotationDialog
        open={domainDialog != null}
        mode={domainDialog?.mode ?? "create"}
        row={domainDialog?.mode === "edit" ? domainDialog.row : undefined}
        onClose={() => setDomainDialog(null)}
        onSubmit={(body) => {
          if (domainDialog?.mode === "edit") {
            updateDomain.mutate(
              { id: domainDialog.row.id, formId: id, body },
              { onSuccess: () => setDomainDialog(null) },
            );
          } else {
            createDomain.mutate(
              { formId: id, body },
              { onSuccess: () => setDomainDialog(null) },
            );
          }
        }}
        mutationError={activeDomainMutation}
        mutationPending={domainMutationPending}
      />

      <AnnotationDialog
        open={annotationDialog != null}
        mode={annotationDialog?.mode ?? "create"}
        owner={
          annotationDialog
            ? annotationDialog.owner
            : { kind: "form", id }
        }
        row={annotationDialog?.mode === "edit" ? annotationDialog.row : undefined}
        availableDomainAnnotations={detail?.domainAnnotations ?? []}
        onClose={() => setAnnotationDialog(null)}
        onSubmit={(body) => {
          if (annotationDialog?.mode === "edit") {
            updateAnnotation.mutate(
              { id: annotationDialog.row.id, formId: id, body },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          } else {
            const owner = annotationDialog?.owner ?? { kind: "form", id };
            createAnnotation.mutate(
              {
                formId: id,
                body: { ...body, owner },
              },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          }
        }}
        mutationError={activeAnnotationMutation}
        mutationPending={annotationMutationPending}
      />

      <DeleteDomainAnnotationDialog
        open={confirmDeleteDomain != null}
        row={confirmDeleteDomain}
        onClose={() => setConfirmDeleteDomain(null)}
        onConfirm={(row) =>
          deleteDomain.mutate(
            { id: row.id, formId: id },
            { onSuccess: () => setConfirmDeleteDomain(null) },
          )
        }
        mutationError={deleteDomain.error}
        mutationPending={deleteDomain.isPending}
      />

      <DeleteAnnotationDialog
        open={confirmDeleteAnnotation != null}
        row={confirmDeleteAnnotation}
        onClose={() => setConfirmDeleteAnnotation(null)}
        onConfirm={(row) =>
          deleteAnnotation.mutate(
            { id: row.id, formId: id },
            { onSuccess: () => setConfirmDeleteAnnotation(null) },
          )
        }
        mutationError={deleteAnnotation.error}
        mutationPending={deleteAnnotation.isPending}
      />
    </Box>
  );
}
```

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx \
        apps/desktop/aegis-desktop/src/features/crf/components/index.ts \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): compose CrfDetailPage with annotations

Header now renders code chip / name (hover popup) / domain
annotation chips / spacer / tools menu. Below: form-level
annotation chips, then item rows with their own chips / unit /
options. Six mutations wired with formDetail(formId) invalidation.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 16: Add end-to-end CrfDetailPage test

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`

- [ ] **Step 1: Write the test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function renderPage(initialEntries: string[]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("CrfDetailPage", () => {
  it("renders header + domain-annotation chip + form annotation chip", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "u",
        name: "U",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      get_crf_form_by_id: () => ({
        id: 11,
        versionId: 7,
        code: "AE",
        name: "Adverse Events",
        order: 0,
        notSubmitted: false,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      get_crf_form_details: () => ({
        form: {
          id: 11,
          versionId: 7,
          code: "AE",
          name: "Adverse Events",
          order: 0,
          notSubmitted: false,
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-01T00:00:00Z",
        },
        formAnnotations: [
          {
            id: 200,
            domainAnnotationId: 50,
            content: "form-level note",
            assign: false,
            owner: { kind: "form", id: 11 },
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
        items: [],
        domainAnnotations: [
          {
            id: 50,
            formId: 11,
            name: "Adverse Events",
            description: "AE",
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
    });

    renderPage(["/project/abc/crf/11"]);

    expect(await screen.findByText("Adverse Events")).toBeInTheDocument();
    // Domain annotation chip renders as "name (description)"
    expect(await screen.findByText("Adverse Events (AE)")).toBeInTheDocument();
    // Form annotation chip renders the content
    expect(await screen.findByText("form-level note")).toBeInTheDocument();
  });

  it("opens the new-annotation dialog from the hover menu", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "u",
        name: "U",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      get_crf_form_by_id: () => ({
        id: 11,
        versionId: 7,
        code: "AE",
        name: "Adverse Events",
        order: 0,
        notSubmitted: false,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      get_crf_form_details: () => ({
        form: {
          id: 11,
          versionId: 7,
          code: "AE",
          name: "Adverse Events",
          order: 0,
          notSubmitted: false,
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-01T00:00:00Z",
        },
        formAnnotations: [],
        items: [],
        domainAnnotations: [
          {
            id: 50,
            formId: 11,
            name: "AE",
            description: "Adverse Events",
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
    });

    const user = userEvent.setup();
    renderPage(["/project/abc/crf/11"]);

    const formName = await screen.findByTestId("crf-form-name");
    await user.hover(formName);
    const newAnnotation = await screen.findByText(/New annotation/i);
    await user.click(newAnnotation);
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: /Create annotation/i })).toBeInTheDocument(),
    );
  });
});
```

- [ ] **Step 2: Run the test**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```
Expected: PASS (2 tests).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx
git commit -m "test(crf): cover CrfDetailPage render and hover menu

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 17: Final verification

**Files:** none — runs commands only.

- [ ] **Step 1: Typecheck the desktop app**

```bash
pnpm --filter aegis-desktop typecheck
```
Expected: PASS.

- [ ] **Step 2: Typecheck the UI package**

```bash
pnpm --filter @aegis/ui typecheck
```
Expected: PASS.

- [ ] **Step 3: Run cargo fmt + clippy + tests for the desktop shim**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop --lib http::crf::
```
Expected: no warnings; tests pass.

- [ ] **Step 4: Run the full vitest suite for the crf feature**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/
```
Expected: all tests pass.

- [ ] **Step 5: Build the desktop app to confirm Tauri shell compiles**

```bash
cargo check -p aegis-desktop
```
Expected: no errors.

- [ ] **Step 6: Commit a docs CHANGELOG entry if the project keeps one**

If `docs/superpowers/CHANGELOG.md` exists, append a one-line entry under the current date summarising the change. Otherwise skip this step.

---

## Self-Review

**Spec coverage:**
- §2.1 Header with hover-popup (New Domain / New Annotation): Task 15.
- §2.1 Domain-annotation chips in header (right of name): Task 15.
- §2.1 Form annotation chips: Tasks 13 + 15.
- §2.1 Item rows (code chip / name / annotation chips / unit right / unit annotations left / options): Task 14 + 15.
- §2.2 Color cycle info→warning→success→error: Task 9 (`annotationColor` helper, used in Tasks 13/14/15).
- §2.3 Mutation success invalidates formDetail: Task 8.
- §2.4 Mutation errors via `errorMessage(...)`: Tasks 10/11/12/15.
- §4 Rust shim layer: Tasks 2/3/4/5/6.
- §5 TS wire DTO mirror: Task 1.
- §6 Query keys + data hooks: Task 8.
- §7 Components: Tasks 9/10/11/12/13/14.
- §9 i18n strings: Tasks 10/11/12/13/14/15.
- §10 Tests: Tasks 7 (api), 9 (chip), 10 (domain dialog), 11 (annotation dialog), 16 (page).

**Placeholder scan:** No "TBD" / "TODO" / "implement later" patterns. Every step shows complete code with exact file paths. The hover-popup implementation switches from `Menu` to `Popover` inline in Task 15 with the rationale documented inline.

**Type consistency:**
- `AnnotationOwner` kind tags (`form` / `item` / `option` / `unit`) match between Rust (Task 4) and TS (Task 1 + 7).
- `DomainAnnotation` schema (`id`, `formId`, `name`, `description`, `createdAt`, `updatedAt`) matches across Rust (Tasks 2/5), TS types (Task 1), and mock JSON in tests (Tasks 9/10/11/16).
- `Annotation` schema (`id`, `domainAnnotationId`, `content`, `assign`, `owner`, `createdAt`, `updatedAt`) matches across all three.
- `CrfFormDetail` shape (`form` / `formAnnotations` / `items` / `domainAnnotations`) matches the server DTO and the page composition in Task 15.
- Mutation hook signatures: `useCreateAnnotation({ formId, body })`, `useUpdateAnnotation({ id, formId, body })`, `useDeleteAnnotation({ id, formId })` — consistent across Tasks 8 and 15.
- Dialog body shapes: `DomainAnnotationDialog` body is `{ name, description }`; `AnnotationDialog` body is `{ domainAnnotationId, content, assign }` (owner composed at the call site in Task 15).
