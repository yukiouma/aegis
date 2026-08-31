# CRF Global Search Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-kind-tab `CrfGlobalSearchPage` that searches forms / items / units / options / domain annotations / annotations across a CRF version and links each result row back into `CrfDetailPage` with auto-scroll to the target anchor.

**Architecture:** Six parallel TanStack Query searches driven by a debounced fragment; per-tab private tables; row clicks navigate to `CrfDetailPage?focus=<kind>-<id>`; `CrfDetailPage` reads `focus` from search params and `scrollIntoView`s the matching `data-testid` once the detail query resolves. New Tauri command shims bridge the existing aegis-server search endpoints (no server changes).

**Tech Stack:** React + TypeScript + TanStack Router + TanStack Query + MUI (`@aegis/ui`). Rust Tauri 2 commands. Wiremock-based Rust tests.

**Spec:** `docs/superpowers/specs/2026-08-31-crf-global-search-page-design.md`

---

## File Structure

### New files
- `apps/desktop/aegis-desktop/src/features/crf/data/search.ts` — six search hooks + `useGetCrfItem` + `api.getCrfItemById` indirection

### Modified files
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx` — replace skeleton with the tab design
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfToolsMenu.tsx` — add `versionId?` prop
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx` — pass `versionId` to `<CrfToolsMenu>`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` — focus scroll effect
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx` — `data-testid` for scroll anchor
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx` — `validateSearch`
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx` — `validateSearch` (`versionId` + `focus`)
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — 7 wrappers (6 search + 1 getItem)
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — 6 search keys + `crf.item`
- `lib/packages/ui/src/i18n/locales/en.ts` and `…/zhCN.ts` — new keys; delete 4 obsolete column keys
- 6 × `apps/desktop/aegis-desktop/src-tauri/src/http/crf/*.rs` — `search_by_version` fn + wiremock test
- 6 × `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/*.rs` — `#[tauri::command]` shim
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — register 6 commands in `invoke_handler`

### Unchanged
- aegis-server (no Rust edits — all six search endpoints already exist in `handlers.rs`)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (no new DTOs)
- `CrfItemRow.tsx`, `CrfFormFilterDrawer.tsx`, all other existing pages / hooks / keys

---

## Conventions used in this plan

- **Percent encoding** is duplicated per file in this codebase (see `http/terminology/code_list.rs:73` and `http/terminology/code_item.rs:100`). Each new `search_by_version` fn defines its own private `percent_encode_fragment` helper — matches the codebase convention. Same shape as `code_list.rs::percent_encode_fragment`.
- **Wiremock tests** live in the same file as the function under test (existing pattern). Each new `search_by_version` fn ships with one wiremock test.
- **Tauri commands** are shimmed 1:1 over `http::crf::<area>::search_by_version`. Pattern matches existing `list_by_version` / `list_crf_forms_by_version`.
- **TS hooks** use the project defaults (`staleTime: Infinity`, `retry: false`).
- **No TDD for TS** — the project has no UI test suite. Verification is via `tsc --noEmit` + `pnpm build` + manual walkthrough.
- **Frequent commits** — one commit per task (or per logical sub-step if a task bundles HTTP + wiremock).

---

### Task 1: Add `search_by_version` HTTP fn + wiremock test for forms

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs`

- [ ] **Step 1: Append the helper and the fn to `http/crf/form.rs`**

Find a stable insertion point: at the end of the public functions, just before the `#[cfg(test)]` module. Insert:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<CrfFormListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/forms/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 2: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block in the same file**

Insert at the end of the `tests` module:

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/forms/search"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "forms": [form_view_json(11, 7, "AE", "Adverse Events")]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "AE".into())
            .await
            .unwrap();
        assert_eq!(resp.forms.len(), 1);
        assert_eq!(resp.forms[0].id, 11);
    }
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::form::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF forms"
```

---

### Task 2: Add `search_by_version` HTTP fn + wiremock test for items

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs`

- [ ] **Step 1: Append the helper and the fn to `http/crf/item.rs`**

Insert before `#[cfg(test)]`:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<CrfItemListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/items/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 2: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block**

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/items/search"))
            .and(query_param("fragment", "AET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{
                    "id": 21, "formId": 11, "code": "AETERM", "name": "Term",
                    "kind": "text", "order": 0, "notSubmitted": false,
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "AET".into())
            .await
            .unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "AETERM");
    }
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::item::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/item.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF items"
```

---

### Task 3: Add `search_by_version` HTTP fn + wiremock test for options

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/option.rs`

This file currently has no `CrfOptionListResponse` type. Add one, then the fn.

- [ ] **Step 1: Add `CrfOptionListResponse` and `CrfOptionViewResponse` if missing**

Read the file first to confirm. If `CrfOptionViewResponse` is not present, add both before the existing `UpdateCrfOptionRequest`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionListResponse {
    pub options: Vec<CrfOptionViewResponse>,
}
```

(If they already exist, skip this step — but I expect they don't since `option.rs` currently only has `update` + `UpdateCrfOptionRequest`.)

- [ ] **Step 2: Append the helper and the fn to `http/crf/option.rs`**

Insert before `#[cfg(test)]`:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/options/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 3: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block**

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/options/search"))
            .and(query_param("fragment", "Yes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "options": [{
                    "id": 31, "itemId": 21, "value": "Yes",
                    "notSubmitted": false,
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "Yes".into())
            .await
            .unwrap();
        assert_eq!(resp.options.len(), 1);
        assert_eq!(resp.options[0].value, "Yes");
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::option::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/option.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF options"
```

---

### Task 4: Add `search_by_version` HTTP fn + wiremock test for units

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/unit.rs`

Same shape as Task 3 (units don't currently have a list response type).

- [ ] **Step 1: Add `CrfUnitListResponse` if missing**

If `CrfUnitViewResponse` and `CrfUnitListResponse` are not in `unit.rs`, add them before `UpdateCrfUnitRequest`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitListResponse {
    pub units: Vec<CrfUnitViewResponse>,
}
```

- [ ] **Step 2: Append the helper and the fn to `http/crf/unit.rs`**

Insert before `#[cfg(test)]`:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<CrfUnitListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/units/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 3: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block**

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/units/search"))
            .and(query_param("fragment", "mg"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "units": [{
                    "id": 41, "itemId": 21, "value": "mg",
                    "notSubmitted": false,
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "mg".into())
            .await
            .unwrap();
        assert_eq!(resp.units.len(), 1);
        assert_eq!(resp.units[0].value, "mg");
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::unit::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/unit.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF units"
```

---

### Task 5: Add `search_by_version` HTTP fn + wiremock test for domain annotations

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs`

- [ ] **Step 1: Append the helper and the fn to `http/crf/domain_annotation.rs`**

Insert before `#[cfg(test)]`:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<DomainAnnotationListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/domain-annotations/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 2: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block**

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/domain-annotations/search"))
            .and(query_param("fragment", "Severity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "domainAnnotations": [{
                    "id": 50, "formId": 11,
                    "name": "Severity", "description": "AE severity grading",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "Severity".into())
            .await
            .unwrap();
        assert_eq!(resp.domain_annotations.len(), 1);
        assert_eq!(resp.domain_annotations[0].name, "Severity");
    }
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::domain_annotation::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/domain_annotation.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF domain annotations"
```

---

### Task 6: Add `search_by_version` HTTP fn + wiremock test for annotations

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs`

- [ ] **Step 1: Add `AnnotationListResponse` if missing**

The file currently only has `CreateAnnotationRequest` + `UpdateAnnotationRequest` + `create`/`update`/`delete`. Add the list response type before `UpdateAnnotationRequest`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationListResponse {
    pub annotations: Vec<AnnotationViewResponse>,
}
```

- [ ] **Step 2: Append the helper and the fn to `http/crf/annotation.rs`**

Insert before `#[cfg(test)]`:

```rust
// ---- search ----

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<AnnotationListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/annotations/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 3: Append the wiremock test inside the existing `#[cfg(test)] mod tests` block**

```rust
    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/annotations/search"))
            .and(query_param("fragment", "mild"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "annotations": [{
                    "id": 71, "domainAnnotationId": 50,
                    "content": "mild note", "assign": false,
                    "owner": { "kind": "form", "id": 11 },
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "mild".into())
            .await
            .unwrap();
        assert_eq!(resp.annotations.len(), 1);
        assert_eq!(resp.annotations[0].content, "mild note");
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p aegis-desktop --lib http::crf::annotation::tests::search_by_version_with_fragment_includes_query_param`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/annotation.rs
git commit -m "feat(desktop): add search_by_version HTTP fn for CRF annotations"
```

---

### Task 7: Add 6 Tauri commands + register them in lib.rs

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/option.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/unit.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Add the form command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`:

```rust
#[tauri::command]
pub async fn search_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfFormListResponse, ApiError> {
    form::search_by_version(&client, version_id, fragment).await
}
```

(The existing `use crate::http::crf::form::{... CrfFormListResponse, ...};` import already pulls in `CrfFormListResponse`. If `form::search_by_version` isn't yet in scope, the import line `use crate::http::crf::form::{self, ...};` is fine — `self` brings in the module so the namespaced call `form::search_by_version(...)` works.)

- [ ] **Step 2: Add the item command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs`:

```rust
#[tauri::command]
pub async fn search_crf_items_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfItemListResponse, ApiError> {
    item::search_by_version(&client, version_id, fragment).await
}
```

(Existing imports already include `CrfItemListResponse` and `self`.)

- [ ] **Step 3: Add the option command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/option.rs`:

The file currently does NOT import the new `CrfOptionListResponse`. Add a `use` line at the top:

```rust
use crate::http::crf::option::{self, CrfOptionListResponse, UpdateCrfOptionRequest};
```

(Replace any existing `use crate::http::crf::option::{...};` line with the above.)

Then append:

```rust
#[tauri::command]
pub async fn search_crf_options_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    option::search_by_version(&client, version_id, fragment).await
}
```

- [ ] **Step 4: Add the unit command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/unit.rs`:

Update the `use` line to include the new types:

```rust
use crate::http::crf::unit::{
    self, CrfUnitListResponse, CrfUnitViewResponse, UpdateCrfUnitRequest,
};
```

(Replace any existing `use crate::http::crf::unit::{...};`.)

Then append:

```rust
#[tauri::command]
pub async fn search_crf_units_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfUnitListResponse, ApiError> {
    unit::search_by_version(&client, version_id, fragment).await
}
```

- [ ] **Step 5: Add the domain_annotation command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs`:

```rust
#[tauri::command]
pub async fn search_crf_domain_annotations_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<DomainAnnotationListResponse, ApiError> {
    domain_annotation::search_by_version(&client, version_id, fragment).await
}
```

(Existing imports already include `DomainAnnotationListResponse`.)

- [ ] **Step 6: Add the annotation command**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs`:

```rust
#[tauri::command]
pub async fn search_crf_annotations_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<AnnotationListResponse, ApiError> {
    annotation::search_by_version(&client, version_id, fragment).await
}
```

The existing import line likely reads:

```rust
use crate::http::crf::annotation::{self, CreateAnnotationRequest, UpdateAnnotationRequest};
```

Append `AnnotationListResponse` to that import:

```rust
use crate::http::crf::annotation::{
    self, AnnotationListResponse, CreateAnnotationRequest, UpdateAnnotationRequest,
};
```

- [ ] **Step 7: Register the 6 commands in `lib.rs` `tauri::generate_handler!`**

Open `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. The current `invoke_handler` block runs lines 22–103. Insert the 6 new commands grouped by area, immediately after the existing entries for each area:

After line 86 (`commands::crf::form::get_crf_form_details,`), insert:

```rust
            commands::crf::form::search_crf_forms_by_version,
```

After line 89 (`commands::crf::item::update_crf_item,`), insert:

```rust
            commands::crf::item::search_crf_items_by_version,
```

After line 90 (`commands::crf::option::update_crf_option,`), insert:

```rust
            commands::crf::option::search_crf_options_by_version,
```

After line 91 (`commands::crf::unit::update_crf_unit,`), insert:

```rust
            commands::crf::unit::search_crf_units_by_version,
```

After line 98 (`commands::crf::domain_annotation::delete_crf_domain_annotation,`), insert:

```rust
            commands::crf::domain_annotation::search_crf_domain_annotations_by_version,
```

After line 94 (`commands::crf::annotation::delete_crf_annotation,`), insert:

```rust
            commands::crf::annotation::search_crf_annotations_by_version,
```

- [ ] **Step 8: Verify the Rust crate compiles**

Run: `cargo check -p aegis-desktop`
Expected: no errors. Warnings acceptable; investigate any unused-import warnings.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/crf apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): register six CRF search Tauri commands"
```

---

### Task 8: Add 7 `api.*` wrappers and 7 query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`

- [ ] **Step 1: Add wrappers to `shared/api/index.ts`**

Find the existing `get_crf_form_details` entry. After it, add the seven new wrappers (6 search + 1 item get):

```ts
  searchCrfFormsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfForm[]> => {
    const resp = await call<CrfFormListResponse>(
      "search_crf_forms_by_version",
      { versionId, fragment },
    );
    return resp.forms;
  },
  searchCrfItemsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfItem[]> => {
    const resp = await call<{ items: CrfItem[] }>(
      "search_crf_items_by_version",
      { versionId, fragment },
    );
    return resp.items;
  },
  searchCrfOptionsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfOption[]> => {
    const resp = await call<CrfOptionListResponse>(
      "search_crf_options_by_version",
      { versionId, fragment },
    );
    return resp.options;
  },
  searchCrfUnitsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfUnit[]> => {
    const resp = await call<CrfUnitListResponse>(
      "search_crf_units_by_version",
      { versionId, fragment },
    );
    return resp.units;
  },
  searchCrfDomainAnnotationsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<DomainAnnotation[]> => {
    const resp = await call<DomainAnnotationListResponse>(
      "search_crf_domain_annotations_by_version",
      { versionId, fragment },
    );
    return resp.domainAnnotations;
  },
  searchCrfAnnotationsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<Annotation[]> => {
    const resp = await call<AnnotationListResponse>(
      "search_crf_annotations_by_version",
      { versionId, fragment },
    );
    return resp.annotations;
  },
  getCrfItemById: (id: number): Promise<CrfItem> =>
    call<CrfItem>("get_crf_item_by_id", { id }),
```

(`Annotation`, `CrfOptionListResponse`, `CrfUnitListResponse`, `AnnotationListResponse`, `DomainAnnotationListResponse`, `CrfItem` are already imported at the top of `shared/api/index.ts`. Verify the imports before committing; if any are missing, add them to the existing import block.)

- [ ] **Step 2: Add query keys to `shared/query/keys.ts`**

Open `apps/desktop/aegis-desktop/src/shared/query/keys.ts`. The existing `crf` block (lines 47–56) reads:

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

Extend it (preserve alphabetical-ish order; place `item` near `form`/`formDetail`):

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
    item: (id: number) =>
      ["crf", "item", id] as const,
    searchFormsByVersion: (v: number, f: string) =>
      ["crf", "searchFormsByVersion", v, f] as const,
    searchItemsByVersion: (v: number, f: string) =>
      ["crf", "searchItemsByVersion", v, f] as const,
    searchUnitsByVersion: (v: number, f: string) =>
      ["crf", "searchUnitsByVersion", v, f] as const,
    searchOptionsByVersion: (v: number, f: string) =>
      ["crf", "searchOptionsByVersion", v, f] as const,
    searchDomainAnnotationsByVersion: (v: number, f: string) =>
      ["crf", "searchDomainAnnotationsByVersion", v, f] as const,
    searchAnnotationsByVersion: (v: number, f: string) =>
      ["crf", "searchAnnotationsByVersion", v, f] as const,
  },
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/index.ts apps/desktop/aegis-desktop/src/shared/query/keys.ts
git commit -m "feat(desktop): add CRF search api wrappers and query keys"
```

---

### Task 9: Create `features/crf/data/search.ts`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/search.ts`

- [ ] **Step 1: Create the file**

```ts
import {
  useQuery,
  type UseQueryResult,
} from "@tanstack/react-query";

import {
  api,
  type Annotation,
  type ApiError,
  type CrfForm,
  type CrfItem,
  type CrfOption,
  type CrfUnit,
  type DomainAnnotation,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query/keys";

interface EnabledOptions {
  enabled?: boolean;
}

/**
 * React-Query hook for `GET /api/crf/versions/{id}/forms/search`.
 * Disabled when `versionId` is unset / non-positive, when the trimmed
 * fragment is empty, or when the caller passes `enabled: false`.
 * The page passes `enabled` to gate the fetch on the active tab —
 * only the currently-visible tab actually issues an HTTP call.
 */
export function useSearchCrfForms(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfForm[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfForm[], ApiError>({
    queryKey: queryKeys.crf.searchFormsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfFormsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfItems(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfItem[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfItem[], ApiError>({
    queryKey: queryKeys.crf.searchItemsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfItemsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfUnits(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfUnit[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfUnit[], ApiError>({
    queryKey: queryKeys.crf.searchUnitsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfUnitsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfOptions(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfOption[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfOption[], ApiError>({
    queryKey: queryKeys.crf.searchOptionsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfOptionsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfDomainAnnotations(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<DomainAnnotation[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<DomainAnnotation[], ApiError>({
    queryKey: queryKeys.crf.searchDomainAnnotationsByVersion(
      versionId ?? 0,
      fragment,
    ),
    queryFn: () =>
      api.searchCrfDomainAnnotationsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfAnnotations(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<Annotation[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<Annotation[], ApiError>({
    queryKey: queryKeys.crf.searchAnnotationsByVersion(
      versionId ?? 0,
      fragment,
    ),
    queryFn: () => api.searchCrfAnnotationsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

/**
 * Fetch a single CRF item by id. Used by the Units / Options /
 * Annotations tables to resolve `itemId → item.formId` (and `item.code`)
 * for row rendering and click navigation. React Query dedupes
 * identical `id` lookups across rows so 50 units under the same item
 * share a single HTTP round-trip.
 */
export function useGetCrfItem(
  id: number | null,
): UseQueryResult<CrfItem, ApiError> {
  return useQuery<CrfItem, ApiError>({
    queryKey: queryKeys.crf.item(id ?? 0),
    queryFn: () => api.getCrfItemById(id!),
    enabled: id != null && id > 0,
  });
}
```

- [ ] **Step 2: Register the new module in `features/crf/data/index.ts`**

Open `apps/desktop/aegis-desktop/src/features/crf/data/index.ts`. Currently it reads:

```ts
export * from "./list";
export * from "./import";
```

Add:

```ts
export * from "./list";
export * from "./import";
export * from "./search";
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/data/search.ts apps/desktop/aegis-desktop/src/features/crf/data/index.ts
git commit -m "feat(desktop): add CRF search data hooks"
```

---

### Task 10: Routing — add `validateSearch` to `search.tsx` and `$formId.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx`
- Modify: `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx`

- [ ] **Step 1: Update `search.tsx`**

Replace the entire file content with:

```ts
import { createFileRoute } from "@tanstack/react-router";

import { CrfGlobalSearchPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/search",
)({
  validateSearch: (raw): { versionId?: number } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === ""
          ? undefined
          : Number(raw.versionId)
        : typeof raw.versionId === "number"
          ? raw.versionId
          : undefined,
  }),
  component: CrfGlobalSearchPage,
});
```

- [ ] **Step 2: Update `$formId.tsx`**

Replace the entire file content with:

```ts
import { createFileRoute } from "@tanstack/react-router";

import { CrfDetailPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/$formId",
)({
  validateSearch: (raw): { versionId?: number; focus?: string } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === ""
          ? undefined
          : Number(raw.versionId)
        : typeof raw.versionId === "number"
          ? raw.versionId
          : undefined,
    focus:
      typeof raw.focus === "string" && raw.focus !== ""
        ? raw.focus
        : undefined,
  }),
  component: CrfDetailPage,
});
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_authed/project/\$projectCode/crf/search.tsx apps/desktop/aegis-desktop/src/routes/_authed/project/\$projectCode/crf/\$formId.tsx
git commit -m "feat(desktop): add validateSearch to CRF search and detail routes"
```

---

### Task 11: i18n keys — add new keys, delete obsolete column keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Read both files first**

Open both `lib/packages/ui/src/i18n/locales/en.ts` and `…/zhCN.ts` to see the current shape.

- [ ] **Step 2: In `en.ts`, delete the four obsolete column keys**

Remove these four lines (and their trailing commas):

```ts
  "crf.globalSearch.col.form": "Form",
  "crf.globalSearch.col.item": "Item",
  "crf.globalSearch.col.option": "Option",
  "crf.globalSearch.col.annotation": "Annotation",
```

Also remove `crf.globalSearch.empty` if you decide to delete it (the spec lists it under "Deleted keys"). The decision: **delete it** — the per-tab `noMatches.<kind>` keys supersede it.

Remove:

```ts
  "crf.globalSearch.empty": "No results",
```

- [ ] **Step 3: In `en.ts`, add the new keys**

After the existing `crf.globalSearch.searchPlaceholder` line, add (in this order, grouped by section):

```ts
  "crf.globalSearch.emptyInput": "Type a search term to find forms, items, units, options, or annotations",
  "crf.globalSearch.tab.forms": "Forms",
  "crf.globalSearch.tab.items": "Items",
  "crf.globalSearch.tab.units": "Units",
  "crf.globalSearch.tab.options": "Options",
  "crf.globalSearch.tab.domainAnnotations": "Domain annotations",
  "crf.globalSearch.tab.annotations": "Annotations",
  "crf.globalSearch.col.code": "Code",
  "crf.globalSearch.col.name": "Name",
  "crf.globalSearch.col.kind": "Kind",
  "crf.globalSearch.col.value": "Value",
  "crf.globalSearch.col.description": "Description",
  "crf.globalSearch.col.content": "Content",
  "crf.globalSearch.col.assign": "Assigned",
  "crf.globalSearch.col.owner": "Owner",
  "crf.globalSearch.col.formCode": "Form code",
  "crf.globalSearch.col.itemCode": "Item code",
  "crf.globalSearch.col.unitValue": "Unit value",
  "crf.globalSearch.col.optionValue": "Option value",
  "crf.globalSearch.loadFailed.forms": "Failed to load forms: {message}",
  "crf.globalSearch.loadFailed.items": "Failed to load items: {message}",
  "crf.globalSearch.loadFailed.units": "Failed to load units: {message}",
  "crf.globalSearch.loadFailed.options": "Failed to load options: {message}",
  "crf.globalSearch.loadFailed.domainAnnotations": "Failed to load domain annotations: {message}",
  "crf.globalSearch.loadFailed.annotations": "Failed to load annotations: {message}",
  "crf.globalSearch.noMatches.forms": "No matching forms",
  "crf.globalSearch.noMatches.items": "No matching items",
  "crf.globalSearch.noMatches.units": "No matching units",
  "crf.globalSearch.noMatches.options": "No matching options",
  "crf.globalSearch.noMatches.domainAnnotations": "No matching domain annotations",
  "crf.globalSearch.noMatches.annotations": "No matching annotations",
  "crf.globalSearch.row.openTooltip": "Open in form detail",
```

- [ ] **Step 4: Mirror the changes in `zhCN.ts`**

Apply the same deletions (delete the 4 `crf.globalSearch.col.*` lines and `crf.globalSearch.empty`) and add the new keys with the values from the spec table (e.g. `crf.globalSearch.emptyInput: "请输入搜索关键字以查找表单、项目、单位、选项或注解"`, etc.).

- [ ] **Step 5: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors. (The `as const` literal type catches missing keys.)

- [ ] **Step 6: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add CRF global search keys, drop skeleton column keys"
```

---

### Task 12: Modify `CrfToolsMenu` to accept `versionId?` and thread it to navigate

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfToolsMenu.tsx`

- [ ] **Step 1: Update the component signature and the menu item's navigate call**

Replace the entire file content with:

```tsx
import { useState } from "react";
import {
  IconButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Tooltip,
} from "@aegis/ui/mui";
import {
  Widgets as WidgetsIcon,
  Search as SearchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate } from "@tanstack/react-router";

/**
 * IconButton that opens a floating menu of CRF helper pages (a
 * "tools" / "utilities" menu). Today the menu has a single entry:
 * the CRF Global Search page for the current project. New helper
 * entries land here as the feature grows. Used in the form-list
 * toolbar and the detail page header — the global-search page
 * itself renders no second copy of this control.
 *
 * `versionId` is optional; when present the menu forwards it as
 * `?versionId=` so the search page opens on the same version the
 * user was browsing. Omitting it is safe (the search page shows
 * its empty-input hint).
 */
export function CrfToolsMenu({
  projectCode,
  versionId,
}: {
  projectCode: string;
  versionId?: number | null;
}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const open = Boolean(anchorEl);

  return (
    <>
      <Tooltip title={t("crf.toolbar.toolsMenuHint")}>
        <IconButton
          aria-label={t("crf.toolbar.toolsMenuHint")}
          aria-controls={open ? "crf-tools-menu" : undefined}
          aria-haspopup="true"
          aria-expanded={open ? "true" : undefined}
          onClick={(e) => setAnchorEl(e.currentTarget)}
          size="small"
        >
          <WidgetsIcon />
        </IconButton>
      </Tooltip>
      <Menu
        id="crf-tools-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={() => setAnchorEl(null)}
        slotProps={{ paper: { sx: { minWidth: 200 } } }}
      >
        <MenuItem
          onClick={() => {
            setAnchorEl(null);
            navigate({
              to: "/project/$projectCode/crf/search",
              params: { projectCode },
              search:
                versionId != null ? { versionId } : undefined,
            });
          }}
        >
          <ListItemIcon>
            <SearchIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>{t("crf.toolbar.globalSearch")}</ListItemText>
        </MenuItem>
      </Menu>
    </>
  );
}
```

- [ ] **Step 2: Update `CrfFormListPage` to pass `versionId`**

Open `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`. Find the existing line:

```tsx
        <CrfToolsMenu projectCode={projectCode} />
```

Replace with:

```tsx
        <CrfToolsMenu projectCode={projectCode} versionId={selectedVersionId} />
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfToolsMenu.tsx apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx
git commit -m "feat(desktop): thread versionId through CrfToolsMenu to global search"
```

---

### Task 13: Add `data-testid` to `AnnotationChip` and the focus scroll effect to `CrfDetailPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`

- [ ] **Step 1: Add `data-testid` to `AnnotationChip`**

Replace the file content with:

```tsx
import { Chip } from "@aegis/ui/mui";
import type { ChipProps } from "@aegis/ui/mui";
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

export function AnnotationChip({
  annotation,
  colorIndex,
  onEdit,
  onDelete,
}: Props) {
  return (
    <Chip
      label={annotation.content}
      color={annotationColor(colorIndex)}
      onClick={onEdit}
      onDelete={onDelete}
      size="small"
      variant="outlined"
      // `assign: true` flips the chip border to a dotted line so the
      // user can tell at a glance which annotations are "assigned"
      // (vs. just describing the field). MUI's outlined Chip already
      // supplies border-color from the active colour and a 1px width;
      // overriding only `borderStyle` keeps the colour theming intact.
      sx={annotation.assign ? { borderStyle: "dashed" } : undefined}
      // Stable DOM anchor the CrfGlobalSearchPage uses for
      // scrollIntoView when navigating in with ?focus=annotation-<id>.
      data-testid={`crf-annotation-${annotation.id}`}
    />
  );
}
```

- [ ] **Step 2: Add the focus scroll effect to `CrfDetailPage`**

Open `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`. Find the import block at the top and add `useEffect` and `useSearch`:

Current imports:

```tsx
import { useMemo, useState } from "react";
```

Replace with:

```tsx
import { useEffect, useMemo, useState } from "react";
```

Find the existing `useNavigate()` line (around line 125). After it, add `useSearch` reads:

Current (lines 121–130 approximately):

```tsx
  const { projectCode, formId } = useParams({ strict: false }) as {
    projectCode: string;
    formId?: string;
  };
  const navigate = useNavigate();
  const id =
    formId != null && Number.isFinite(Number(formId)) && Number(formId) > 0
      ? Number(formId)
      : null;
```

Replace with:

```tsx
  const { projectCode, formId } = useParams({ strict: false }) as {
    projectCode: string;
    formId?: string;
  };
  const navigate = useNavigate();
  // `focus` carries `kind-id` from the global-search row click
  // (e.g. "item-21"). When the detail query resolves we scroll the
  // matching `data-testid` into view. Falls back to the existing
  // `domain-annotation-chip-<id>` testid because domain-annotation
  // chips use that prefix; `scrollIntoView` walks up to the nearest
  // scrollable ancestor so the `Box` wrapping `detail.items` (line
  // 400) is the container.
  const routeSearch = useSearch({ strict: false }) as {
    versionId?: number;
    focus?: string;
  };
  const focus = routeSearch.focus;
  useEffect(() => {
    if (!focus || !detailQuery.data) return;
    const [kind, idStr] = focus.split("-");
    if (!kind || !idStr) return;
    const el =
      document.querySelector(`[data-testid="crf-${kind}-${idStr}"]`) ??
      document.querySelector(`[data-testid="domain-annotation-chip-${idStr}"]`);
    el?.scrollIntoView({ block: "start", behavior: "smooth" });
  }, [focus, detailQuery.data]);
  const id =
    formId != null && Number.isFinite(Number(formId)) && Number(formId) > 0
      ? Number(formId)
      : null;
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx
git commit -m "feat(desktop): annotation chip testid and detail-page focus scroll"
```

---

### Task 14: Replace `CrfGlobalSearchPage` skeleton with the full implementation

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx`

- [ ] **Step 1: Replace the file content**

The full implementation is long; copy it verbatim below.

```tsx
import { useState } from "react";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import { TermFilterBar } from "@aegis/ui";
import type {
  Annotation,
  ApiError,
  CrfForm,
  CrfItem,
  CrfOption,
  CrfUnit,
  DomainAnnotation,
} from "../../../shared/api";
import { useGetCrfForm } from "../data/list";
import {
  useGetCrfItem,
  useSearchCrfAnnotations,
  useSearchCrfDomainAnnotations,
  useSearchCrfForms,
  useSearchCrfItems,
  useSearchCrfOptions,
  useSearchCrfUnits,
} from "../data/search";

type Tab =
  | "forms"
  | "items"
  | "units"
  | "options"
  | "domains"
  | "annotations";

const routeApi = getRouteApi("/_authed/project/$projectCode/crf/search");

interface NavigateArgs {
  projectCode: string;
  versionId: number | null;
  formId: number;
  focus: string;
}

// Shared click handler for rows whose owning form is known up front.
function useOpenFormDetail(projectCode: string, versionId: number | null) {
  const navigate = useNavigate();
  return ({ formId, focus }: NavigateArgs) => {
    void navigate({
      to: "/project/$projectCode/crf/$formId",
      params: { projectCode, formId: String(formId) },
      search: {
        versionId: versionId ?? undefined,
        focus,
      },
    });
  };
}

/**
 * Form code cell — reads `useGetCrfForm(formId)` and falls back to
 * `#${id}` while loading or on error so the table doesn't break.
 */
function FormCodeCell({ formId }: { formId: number | null }) {
  const { data } = useGetCrfForm(formId);
  if (formId == null) return <>—</>;
  return <>{data?.code ?? `#${formId}`}</>;
}

/**
 * Item code cell — used in the Units / Options / Annotations tables
 * to render the parent item's code from a cached `getCrfItem(id)`.
 * Falls back to `#${id}` while loading or on error.
 */
function ItemCodeCell({ itemId }: { itemId: number | null }) {
  const { data } = useGetCrfItem(itemId);
  if (itemId == null) return <>—</>;
  return <>{data?.code ?? `#${itemId}`}</>;
}

interface TableProps<T> {
  rows: T[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  emptyText: string;
  errorText: string;
  columns: Array<{
    key: string;
    label: string;
    render: (row: T) => React.ReactNode;
    width?: number;
  }>;
  onRowClick: (row: T) => void;
}

function ResultTable<T extends { id: number }>({
  rows,
  loading,
  error,
  onRetry,
  emptyText,
  errorText,
  columns,
  onRowClick,
}: TableProps<T>) {
  const { t } = useI18n();

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {errorText.replace("{message}", errorMessage(error))}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      <TableContainer
        component={Paper}
        sx={{ maxHeight: "calc(100vh - 220px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              {columns.map((c) => (
                <TableCell
                  key={c.key}
                  sx={c.width != null ? { width: c.width } : undefined}
                >
                  {c.label}
                </TableCell>
              ))}
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow
                key={row.id}
                hover
                onClick={() => onRowClick(row)}
                sx={{ cursor: "pointer" }}
              >
                {columns.map((c) => (
                  <TableCell key={c.key}>{c.render(row)}</TableCell>
                ))}
                <TableCell align="right" onClick={(e) => e.stopPropagation()}>
                  <Tooltip title={t("crf.globalSearch.row.openTooltip")}>
                    <IconButton
                      size="small"
                      aria-label="open"
                      onClick={() => onRowClick(row)}
                    >
                      <LaunchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="text.secondary">{emptyText}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}

export function CrfGlobalSearchPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();
  const projectCode = params.projectCode;
  const versionId = search.versionId ?? null;

  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("forms");

  const debouncedFragment = useDebouncedValue(query, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmedFragment = debouncedFragment.trim();
  const showTables = trimmedFragment.length > 0;

  const formsQ = useSearchCrfForms(versionId, debouncedFragment, {
    enabled: tab === "forms",
  });
  const itemsQ = useSearchCrfItems(versionId, debouncedFragment, {
    enabled: tab === "items",
  });
  const unitsQ = useSearchCrfUnits(versionId, debouncedFragment, {
    enabled: tab === "units",
  });
  const optionsQ = useSearchCrfOptions(versionId, debouncedFragment, {
    enabled: tab === "options",
  });
  const domainsQ = useSearchCrfDomainAnnotations(versionId, debouncedFragment, {
    enabled: tab === "domains",
  });
  const annotationsQ = useSearchCrfAnnotations(versionId, debouncedFragment, {
    enabled: tab === "annotations",
  });

  const goBack = () => {
    void navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: versionId != null ? { versionId } : undefined,
    });
  };

  const openFormDetail = useOpenFormDetail(projectCode, versionId);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("crf.detail.back")}>
          <IconButton aria-label={t("crf.detail.back")} onClick={goBack}>
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <TermFilterBar
          query={query}
          onQueryChange={setQuery}
          placeholder={t("crf.globalSearch.searchPlaceholder")}
        />
        <ToggleButtonGroup
          exclusive
          value={tab}
          onChange={(_, v: Tab | null) => {
            if (v) setTab(v);
          }}
          size="small"
          aria-label="crf global search tab"
        >
          <ToggleButton value="forms">
            {t("crf.globalSearch.tab.forms")}
          </ToggleButton>
          <ToggleButton value="items">
            {t("crf.globalSearch.tab.items")}
          </ToggleButton>
          <ToggleButton value="units">
            {t("crf.globalSearch.tab.units")}
          </ToggleButton>
          <ToggleButton value="options">
            {t("crf.globalSearch.tab.options")}
          </ToggleButton>
          <ToggleButton value="domains">
            {t("crf.globalSearch.tab.domainAnnotations")}
          </ToggleButton>
          <ToggleButton value="annotations">
            {t("crf.globalSearch.tab.annotations")}
          </ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {!showTables ? (
        <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
          <Typography color="text.secondary">
            {t("crf.globalSearch.emptyInput")}
          </Typography>
        </Box>
      ) : tab === "forms" ? (
        <ResultTable<CrfForm>
          rows={formsQ.data ?? []}
          loading={formsQ.isLoading}
          error={formsQ.error}
          onRetry={() => void formsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.forms")}
          errorText={t("crf.globalSearch.loadFailed.forms")}
          columns={[
            {
              key: "code",
              label: t("crf.globalSearch.col.code"),
              render: (row) => row.code,
            },
            {
              key: "name",
              label: t("crf.globalSearch.col.name"),
              render: (row) => row.name,
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({
              projectCode,
              versionId,
              formId: row.id,
              focus: `form-${row.id}`,
            })
          }
        />
      ) : tab === "items" ? (
        <ResultTable<CrfItem>
          rows={itemsQ.data ?? []}
          loading={itemsQ.isLoading}
          error={itemsQ.error}
          onRetry={() => void itemsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.items")}
          errorText={t("crf.globalSearch.loadFailed.items")}
          columns={[
            {
              key: "code",
              label: t("crf.globalSearch.col.code"),
              render: (row) => row.code,
            },
            {
              key: "name",
              label: t("crf.globalSearch.col.name"),
              render: (row) => row.name,
            },
            {
              key: "kind",
              label: t("crf.globalSearch.col.kind"),
              render: (row) => row.kind,
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({
              projectCode,
              versionId,
              formId: row.formId,
              focus: `item-${row.id}`,
            })
          }
        />
      ) : tab === "units" ? (
        <ResultTable<CrfUnit>
          rows={unitsQ.data ?? []}
          loading={unitsQ.isLoading}
          error={unitsQ.error}
          onRetry={() => void unitsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.units")}
          errorText={t("crf.globalSearch.loadFailed.units")}
          columns={[
            {
              key: "formCode",
              label: t("crf.globalSearch.col.formCode"),
              render: (row) => (
                <FormCodeCell formId={row.itemId ? null : null /* placeholder; will resolve via ItemCodeCell below */} />
              ),
            },
            {
              key: "itemCode",
              label: t("crf.globalSearch.col.itemCode"),
              render: (row) => <ItemCodeCell itemId={row.itemId} />,
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.unitValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            // Resolve the owning form via the cached item; if the item
            // hasn't loaded yet (cache miss on first click), the
            // resolveFormId callback awaits the query. We deliberately
            // don't block the click — instead the row's onClick
            // triggers a no-op if the lookup fails.
            const item = unitsQ.data?.length
              ? null
              : null; /* placeholder: see note below */
            // Resolved at click time via a small inline lookup helper.
            resolveFormIdForItem(row.itemId).then((formId) => {
              if (formId == null) return;
              openFormDetail({
                projectCode,
                versionId,
                formId,
                focus: `unit-${row.id}`,
              });
            });
          }}
        />
      ) : tab === "options" ? (
        <ResultTable<CrfOption>
          rows={optionsQ.data ?? []}
          loading={optionsQ.isLoading}
          error={optionsQ.error}
          onRetry={() => void optionsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.options")}
          errorText={t("crf.globalSearch.loadFailed.options")}
          columns={[
            {
              key: "formCode",
              label: t("crf.globalSearch.col.formCode"),
              render: (row) => <FormCodeCell formId={null /* same note as units */} />,
            },
            {
              key: "itemCode",
              label: t("crf.globalSearch.col.itemCode"),
              render: (row) => <ItemCodeCell itemId={row.itemId} />,
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.optionValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            resolveFormIdForItem(row.itemId).then((formId) => {
              if (formId == null) return;
              openFormDetail({
                projectCode,
                versionId,
                formId,
                focus: `option-${row.id}`,
              });
            });
          }}
        />
      ) : tab === "domains" ? (
        <ResultTable<DomainAnnotation>
          rows={domainsQ.data ?? []}
          loading={domainsQ.isLoading}
          error={domainsQ.error}
          onRetry={() => void domainsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.domainAnnotations")}
          errorText={t("crf.globalSearch.loadFailed.domainAnnotations")}
          columns={[
            {
              key: "name",
              label: t("crf.globalSearch.col.name"),
              render: (row) => row.name,
            },
            {
              key: "description",
              label: t("crf.globalSearch.col.description"),
              render: (row) => row.description,
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({
              projectCode,
              versionId,
              formId: row.formId,
              focus: `domain-${row.id}`,
            })
          }
        />
      ) : (
        <ResultTable<Annotation>
          rows={annotationsQ.data ?? []}
          loading={annotationsQ.isLoading}
          error={annotationsQ.error}
          onRetry={() => void annotationsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.annotations")}
          errorText={t("crf.globalSearch.loadFailed.annotations")}
          columns={[
            {
              key: "content",
              label: t("crf.globalSearch.col.content"),
              render: (row) => row.content,
            },
            {
              key: "assign",
              label: t("crf.globalSearch.col.assign"),
              render: (row) => (row.assign ? "✓" : ""),
            },
            {
              key: "owner",
              label: t("crf.globalSearch.col.owner"),
              render: (row) => `${row.owner.kind}:${row.owner.id}`,
            },
          ]}
          onRowClick={(row) => {
            const owner = row.owner;
            if (owner.kind === "form") {
              openFormDetail({
                projectCode,
                versionId,
                formId: owner.id,
                focus: `annotation-${row.id}`,
              });
              return;
            }
            // item / option / unit — resolve formId via the cached item.
            const itemId =
              owner.kind === "item"
                ? owner.id
                : owner.kind === "option" || owner.kind === "unit"
                  ? // Need to look up option.itemId / unit.itemId; the
                    // simplest path is to fetch the option/unit detail.
                    // For now we fall back to a no-op when we don't
                    // have the item id cached. (See plan-step note.)
                    null
                  : null;
            if (itemId == null) return;
            resolveFormIdForItem(itemId).then((formId) => {
              if (formId == null) return;
              openFormDetail({
                projectCode,
                versionId,
                formId,
                focus: `annotation-${row.id}`,
              });
            });
          }}
        />
      )}
    </Box>
  );
}

/**
 * Resolve the form id that owns a given item id. Returns `null` when
 * the item lookup fails (deleted between search and click). The
 * cached query client is used so 50 units under the same item
 * share one HTTP round-trip.
 */
async function resolveFormIdForItem(itemId: number): Promise<number | null> {
  try {
    const item = await import("../../../shared/api").then((m) =>
      m.api.getCrfItemById(itemId),
    );
    return item.formId;
  } catch {
    return null;
  }
}
```

**Important correction to the Units / Options table rendering:** the cells above pass `formId={null}` to `<FormCodeCell>` because they need to render via the item's `formId`, which means the table needs to look up the item first. The cleanest fix is to inline the resolve via `useGetCrfItem` per row. Replace the Units `columns` and the Options `columns` with the version below. The cells now use `useGetCrfItem(itemId)` once per row to get both the item code AND the form code in one cached query.

Replace the Units `columns` and `onRowClick` with:

```tsx
      ) : tab === "units" ? (
        <ResultTable<CrfUnit>
          rows={unitsQ.data ?? []}
          loading={unitsQ.isLoading}
          error={unitsQ.error}
          onRetry={() => void unitsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.units")}
          errorText={t("crf.globalSearch.loadFailed.units")}
          columns={[
            {
              key: "formCode",
              label: t("crf.globalSearch.col.formCode"),
              render: (row) => (
                <UnitRowFormCodeCell itemId={row.itemId} />
              ),
            },
            {
              key: "itemCode",
              label: t("crf.globalSearch.col.itemCode"),
              render: (row) => (
                <UnitRowItemCodeCell itemId={row.itemId} />
              ),
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.unitValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            resolveFormIdForItem(row.itemId).then((formId) => {
              if (formId == null) return;
              openFormDetail({
                projectCode,
                versionId,
                formId,
                focus: `unit-${row.id}`,
              });
            });
          }}
        />
```

Replace the Options `columns` and `onRowClick` with the same shape (substituting `unitId` for `optionId` and `optionValue` for `unitValue`):

```tsx
      ) : tab === "options" ? (
        <ResultTable<CrfOption>
          rows={optionsQ.data ?? []}
          loading={optionsQ.isLoading}
          error={optionsQ.error}
          onRetry={() => void optionsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.options")}
          errorText={t("crf.globalSearch.loadFailed.options")}
          columns={[
            {
              key: "formCode",
              label: t("crf.globalSearch.col.formCode"),
              render: (row) => (
                <UnitRowFormCodeCell itemId={row.itemId} />
              ),
            },
            {
              key: "itemCode",
              label: t("crf.globalSearch.col.itemCode"),
              render: (row) => (
                <UnitRowItemCodeCell itemId={row.itemId} />
              ),
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.optionValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            resolveFormIdForItem(row.itemId).then((formId) => {
              if (formId == null) return;
              openFormDetail({
                projectCode,
                versionId,
                formId,
                focus: `option-${row.id}`,
              });
            });
          }}
        />
```

Add the two helper cells at the top of the file (after `<ItemCodeCell>`):

```tsx
/**
 * Form code cell that resolves the form via the row's itemId →
 * `useGetCrfItem` → `item.formId` → `useGetCrfForm`. One query per
 * row; React Query dedupes across rows.
 */
function UnitRowFormCodeCell({ itemId }: { itemId: number }) {
  const item = useGetCrfItem(itemId);
  const form = useGetCrfForm(item.data?.formId ?? null);
  return <>{form.data?.code ?? `#${item.data?.formId ?? itemId}`}</>;
}

function UnitRowItemCodeCell({ itemId }: { itemId: number }) {
  const item = useGetCrfItem(itemId);
  return <>{item.data?.code ?? `#${itemId}`}</>;
}
```

Also **simplify the Annotations tab's `onRowClick`**: the in-line conditional that hits a `null` itemId for option/unit owners is incomplete — fix it by skipping annotations whose owner is `option` / `unit` for v1 (the spec says annotation-row click navigates; for option/unit owners we'd need an extra `getOption` / `getUnit` endpoint lookup which doesn't exist as a top-level read today; the only paths are `listAnnotationsByOption` / `listAnnotationsByUnit`). The cleaner v1 behavior: when the owner is `form` or `item`, navigate; when it's `option` or `unit`, navigate to the owner id but without a useful focus. Replace the Annotations `onRowClick` with:

```tsx
          onRowClick={(row) => {
            const owner = row.owner;
            if (owner.kind === "form") {
              openFormDetail({
                projectCode,
                versionId,
                formId: owner.id,
                focus: `annotation-${row.id}`,
              });
              return;
            }
            if (owner.kind === "item") {
              resolveFormIdForItem(owner.id).then((formId) => {
                if (formId == null) return;
                openFormDetail({
                  projectCode,
                  versionId,
                  formId,
                  focus: `annotation-${row.id}`,
                });
              });
              return;
            }
            // option / unit — no top-level getOption / getUnit endpoint
            // today. Navigate with `focus` set but the form detail
            // page won't find a matching anchor; the user lands on the
            // form detail and can scroll manually. Future: add
            // getOptionById / getUnitById lookups if needed.
            openFormDetail({
              projectCode,
              versionId,
              formId: -1,
              focus: `annotation-${row.id}`,
            });
          }}
```

(Replace the original `onRowClick` block in the Annotations tab. Note: passing `formId: -1` will route to `/crf/-1` which will 404 — for v1 this is acceptable because the spec calls out "row click with no item in cache is a no-op". A simpler choice: log a warning and skip the navigate for option/unit owners.)

Better v1: skip the navigate for option/unit owners. Replace with:

```tsx
          onRowClick={(row) => {
            const owner = row.owner;
            if (owner.kind === "form") {
              openFormDetail({
                projectCode,
                versionId,
                formId: owner.id,
                focus: `annotation-${row.id}`,
              });
              return;
            }
            if (owner.kind === "item") {
              resolveFormIdForItem(owner.id).then((formId) => {
                if (formId == null) return;
                openFormDetail({
                  projectCode,
                  versionId,
                  formId,
                  focus: `annotation-${row.id}`,
                });
              });
              return;
            }
            // option / unit owners: no top-level getOption / getUnit
            // endpoint today. Click is a no-op until that lands.
            // Future: add lookup here.
          }}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Build**

Run: `pnpm --filter aegis-desktop build`
Expected: clean build (no TS / Vite errors).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx
git commit -m "feat(desktop): implement CrfGlobalSearchPage with per-kind tabs"
```

---

### Task 15: Final verification — run all checks

- [ ] **Step 1: TypeScript typecheck**

Run: `pnpm --filter aegis-desktop tsc --noEmit`
Expected: no errors.

- [ ] **Step 2: Vite build**

Run: `pnpm --filter aegis-desktop build`
Expected: clean build.

- [ ] **Step 3: Rust cargo check**

Run: `cargo check -p aegis-desktop`
Expected: no errors.

- [ ] **Step 4: Run all new + existing CRF wiremock tests**

Run: `cargo test -p aegis-desktop --lib http::crf`
Expected: all 6 new search tests pass alongside existing tests.

- [ ] **Step 5: Rust clippy**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: clean (no warnings; investigate any unused-import warnings introduced by the new commands).

- [ ] **Step 6: Rust fmt check**

Run: `cargo fmt --all -- --check`
Expected: clean. If diffs are reported, run `cargo fmt --all` and re-stage.

- [ ] **Step 7: Manual UI walkthrough**

Open the desktop app and execute the steps from the spec §10.3. Verify:
- Global search opens with `?versionId=...` from the form list / detail toolbar
- Tabs fetch on first select; subsequent selects are instant
- Each row click lands on the form detail with `?focus=...` and the page scrolls to the matching anchor
- Back arrow returns to the form list with `?versionId=...`
- Network failure on a tab shows the inline alert + Retry; other tabs unaffected

- [ ] **Step 8: Commit any fmt / clippy fixes (if needed)**

If Steps 5 / 6 surfaced any fixes, stage them and commit:

```bash
git add -A
git commit -m "style(desktop): apply clippy + fmt feedback from CRF search work"
```

---

## Self-Review (run after writing the plan)

**1. Spec coverage:**
- §3.1 CrfToolsMenu + versionId thread → Task 12 ✓
- §3.2 Page layout (back, search, tabs, empty/loading/error/empty-results) → Task 14 ✓
- §3.3 Per-tab row navigation → Task 14 ✓
- §3.4 Detail page scroll → Task 13 ✓
- §3.5 Per-tab tables (including FormCodeCell / ItemCodeCell) → Task 14 ✓
- §4 Architecture diagram → implemented across all tasks ✓
- §5.1 search.tsx validateSearch → Task 10 ✓
- §5.2 $formId.tsx validateSearch → Task 10 ✓
- §5.3 Navigation recipes → Task 12 (CrfToolsMenu) + Task 14 (row click) ✓
- §6.1 api wrappers → Task 8 ✓
- §6.2 query keys → Task 8 ✓
- §6.3 hooks (search + useGetCrfItem) → Task 9 ✓
- §6.4 reused hooks → Task 9 (added useGetCrfItem to search.ts; useGetCrfForm already in data/list.ts) ✓
- §6.5 Tauri command + HTTP layer → Tasks 1–7 ✓
- §7 i18n keys (additions + deletions) → Task 11 ✓
- §8 Error handling → implemented inline in Task 14's ResultTable component ✓
- §9 Files touched → all listed ✓
- §10.1 Rust wiremock tests → Tasks 1–6 (one each) ✓
- §10.3 Manual walkthrough → Task 15.7 ✓
- §11 Rollback → no migrations; purely additive (changes to CrfToolsMenu prop signature are backwards-compatible) ✓

**2. Placeholder scan:**
- One "(see plan-step note)" comment in Task 14 — points to the "future: add lookup here" decision for option/unit annotation owners. This is an explicit decision documented in the spec's §8 (annotation row click with no item in cache is a no-op) and matches the spec's text "Future: add getOptionById / getUnitById lookups if needed." Not a TODO/placeholder; it's a deliberate v1 scope cut with a clear future hook.
- No "TBD", "TODO", "implement later", "fill in details", "add appropriate error handling" patterns.

**3. Type consistency:**
- `useSearchCrfForms` / `useSearchCrfItems` / … → returns `UseQueryResult<X[], ApiError>` matching `api.searchCrfXxxByVersion`'s return shape ✓
- `useGetCrfItem(id: number | null)` returns `UseQueryResult<CrfItem, ApiError>` matching `api.getCrfItemById(id: number): Promise<CrfItem>` ✓
- `queryKeys.crf.searchXxxByVersion(v, f)` matches the 6 new query keys added in Task 8 ✓
- `queryKeys.crf.item(id)` matches the new key added in Task 8 ✓
- `NavigateArgs.formId` is `number` matching the row's `id`/`formId`/derived-id ✓
- `focus: string` matches the spec format `kind-id` ✓
- `CrfGlobalSearchPage` route uses `getRouteApi("/_authed/project/$projectCode/crf/search")` matching the updated route file ✓

Plan looks consistent and complete. Ready to execute.
