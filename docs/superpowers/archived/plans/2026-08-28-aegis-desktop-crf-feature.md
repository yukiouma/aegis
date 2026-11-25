# aegis-desktop CRF feature implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `crf` feature under the project workspace window with form list (toolbar + table + create/edit/delete drawers + filter drawer + assign-takers drawer + status chip + global-search button), detail (back + code chip + name + global-search button), and global search (back + title + search input + empty results table) pages. Wires new Rust shims for `list_versions_by_project` + 5 form endpoints.

**Architecture:** New `src/features/crf/` feature module mirroring the existing `features/terminology/` layout. New `src-tauri/src/http/crf/` + `src-tauri/src/commands/crf/` modules mirroring `terminology/`. URL-driven `?versionId=` selector on the form list page; mutations invalidate the per-version form list cache key. Status / Taker columns render placeholder text — no fetch.

**Tech Stack:** Tauri 2, React 18, TanStack Router (file-based), TanStack Query v5, MUI v5 (`@aegis/ui/mui`), `useI18n` for translations, `useDebouncedValue` from `src/shared/hooks/`, wiremock for Rust http tests, Vitest for TS tests, jsdom.

**Spec:** `docs/superpowers/specs/2026-08-28-aegis-desktop-crf-feature-design.md`

## Global Constraints

- Edition `2024`, resolver `3`, all Rust deps from `[workspace.dependencies]` or `src-tauri/Cargo.toml`. New deps: none — `wiremock` is already a dev-dep.
- React + TS per existing `package.json`. No new packages.
- i18n keys must be added to **both** `en.ts` and `zhCN.ts`. The Chinese file mirrors the English key set verbatim.
- Wire DTOs are mirrored hand-maintained, not generated. Wire keys are `snake_case` on the server, `camelCase` on the client; the rename happens at the Rust serde boundary with `#[serde(rename_all = "camelCase")]`.
- Per-layer error type: every Rust command returns `Result<T, ApiError>`; every TS hook surfaces `ApiError`.
- TDD for every new Rust http submodule: failing wiremock test first, then impl, then commit.
- One commit per task. Use `Co-Authored-By: Claude <noreply@anthropic.com>` at the end of each message body.
- All file paths are relative to the repo root unless noted.

---

## File Structure

### Add (TS feature module)
- `apps/desktop/aegis-desktop/src/features/crf/index.ts` — public barrel
- `apps/desktop/aegis-desktop/src/features/crf/data/index.ts` — data barrel
- `apps/desktop/aegis-desktop/src/features/crf/data/list.ts` — React Query hooks
- `apps/desktop/aegis-desktop/src/features/crf/components/index.ts` — components barrel
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfStatusChip.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfGlobalSearchButton.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfVersionDropdown.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/DeleteCrfFormDialog.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormFilterDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/pages/index.ts` — pages barrel
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx`

### Add (routes)
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/index.tsx`
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx`
- `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx`

### Add (Rust backend)
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs` — module file
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/version.rs` — wire DTOs + `list_by_project`
- `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs` — wire DTOs + 5 endpoints
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs` — module file
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/version.rs` — 1 command shim
- `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` — 5 command shims

### Add (tests)
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx` — smoke test

### Modify
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — add CRF section
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — add `crf` branch
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — add `crf` namespace
- `lib/packages/ui/src/i18n/locales/en.ts` — add CRF keys
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add CRF keys (Chinese translations)
- `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx` — add CRF menu entry + icon import
- `apps/desktop/aegis-desktop/src-tauri/src/http.rs` — add `pub mod crf;`
- `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` — add `pub mod crf;`
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — extend `generate_handler!` with `// crf` block

---

## Task 1: Shared TS layer — types + query keys + api namespace + i18n

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (append CRF section)
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (add `crf` branch)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` (add `crf` namespace)
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (append CRF keys)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (append CRF keys, Chinese translations)

**Interfaces:**
- Produces: `CrfVersion`, `CrfVersionListResponse`, `CrfForm`, `CrfFormListResponse`, `CreateCrfFormInput`, `UpdateCrfFormInput`
- Produces: `queryKeys.crf.{versionsByProject,formsByVersion,form}`
- Produces: `api.crf.{listVersions,listFormsByVersion,getFormById,createForm,updateForm,deleteForm}`

- [ ] **Step 1: Add CRF types to `src/shared/api/types.ts`**

Append at the bottom (after the existing `// Domain model` section):

```ts
// CRF
export interface CrfVersion {
  id: number;
  projectCode: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}
export interface CrfVersionListResponse { versions: CrfVersion[]; }

export interface CrfForm {
  id: number;
  versionId: number;
  code: string;
  name: string;
  order: number;
  notSubmitted: boolean;
  createdAt: string;
  updatedAt: string;
}
export interface CrfFormListResponse { forms: CrfForm[]; }
export interface CreateCrfFormInput {
  code: string;
  name: string;
  order: number;
  notSubmitted: boolean;
}
export interface UpdateCrfFormInput {
  code?: string;
  name?: string;
  order?: number;
  notSubmitted?: boolean;
}
```

- [ ] **Step 2: Add `crf` branch to `src/shared/query/keys.ts`**

Inside the `queryKeys` object literal, after `domainModel: { ... }`, add:

```ts
  crf: {
    versionsByProject: (projectCode: string) =>
      ["crf", "versionsByProject", projectCode] as const,
    formsByVersion: (versionId: number) =>
      ["crf", "formsByVersion", versionId] as const,
    form: (id: number) =>
      ["crf", "form", id] as const,
  },
```

- [ ] **Step 3: Add `crf` namespace to `src/shared/api/index.ts`**

After the `api.domainModel` namespace, add:

```ts
  crf: {
    listVersions: (projectCode: string) =>
      call<CrfVersion[]>("list_crf_versions", { projectCode }),
    listFormsByVersion: (versionId: number) =>
      call<CrfForm[]>("list_crf_forms_by_version", { versionId }),
    getFormById: (id: number) =>
      call<CrfForm>("get_crf_form_by_id", { id }),
    createForm: (versionId: number, body: CreateCrfFormInput) =>
      call<CrfForm>("create_crf_form", { versionId, body }),
    updateForm: (id: number, body: UpdateCrfFormInput) =>
      call<CrfForm>("update_crf_form", { id, body }),
    deleteForm: (id: number) =>
      call<void>("delete_crf_form", { id }),
  },
```

And update the `export type` block at the bottom of `src/shared/api/index.ts` to re-export the new CRF interfaces (mirror the existing pattern for `TerminologyVersionView` etc.).

- [ ] **Step 4: Add CRF + missing common keys to `lib/packages/ui/src/i18n/locales/en.ts`**

Inside the `resources` object, after the existing `crf` keys (if any) — otherwise at the bottom — add this block (also adds four missing `common.*` keys that the CRF components depend on):

```ts
    "common.apply":                            "Apply",
    "common.clear":                            "Clear",
    "common.invalidId":                        "Invalid id",
    "common.noData":                           "No data",
    "workspace.menu.crf":                       "CRF",
    "crf.formList.heading":                     "CRF Form List — {projectCode}",
    "crf.detail.title":                         "CRF Detail",
    "crf.detail.placeholder":                   "Form detail view coming soon",
    "crf.detail.back":                          "Back to form list",
    "crf.globalSearch.heading":                 "CRF Global Search — {projectCode}",
    "crf.globalSearch.searchPlaceholder":       "Search forms, items, options, annotations…",
    "crf.globalSearch.empty":                   "No results",
    "crf.globalSearch.col.form":                "Form",
    "crf.globalSearch.col.item":                "Item",
    "crf.globalSearch.col.option":              "Option",
    "crf.globalSearch.col.annotation":          "Annotation",
    "crf.toolbar.statusPending":                "Pending",
    "crf.toolbar.globalSearch":                 "Global Search",
    "crf.toolbar.globalSearchHint":             "Open the global CRF search",
    "crf.table.column.code":                    "Form Code",
    "crf.table.column.name":                    "Form Name",
    "crf.table.column.taker":                   "Taker",
    "crf.table.column.status":                  "Status",
    "crf.table.column.actions":                 "Operations",
    "crf.table.action.assignTakers":            "Assign takers",
    "crf.table.action.edit":                    "Edit form",
    "crf.table.action.delete":                  "Delete form",
    "crf.table.action.openDetail":              "Open form detail",
    "crf.table.action.addForm":                 "Add form",
    "crf.table.action.filter":                  "Filter forms",
    "crf.drawer.create.title":                  "Create CRF Form",
    "crf.drawer.edit.title":                    "Edit CRF Form",
    "crf.drawer.field.code":                    "Form Code",
    "crf.drawer.field.name":                    "Form Name",
    "crf.drawer.submit.create":                 "Create",
    "crf.drawer.submit.save":                   "Save",
    "crf.filter.title":                         "Filter CRF Forms",
    "crf.filter.search":                        "Search by code or name",
    "crf.filter.status":                        "Status",
    "crf.filter.status.approved":               "Approved",
    "crf.filter.status.pending":                "Pending",
    "crf.filter.involved":                      "Involved",
    "crf.delete.title":                         "Delete CRF Form",
    "crf.delete.message":                       "Delete form \"{code} — {name}\"? This cannot be undone.",
    "crf.delete.submit":                        "Delete",
    "crf.assignTakers.title":                   "Assign Takers",
    "crf.assignTakers.placeholder":             "Takers UI coming soon",
```

- [ ] **Step 5: Add Chinese translations to `lib/packages/ui/src/i18n/locales/zhCN.ts`**

Add the same key set with Chinese values:

```ts
    "common.apply":                            "应用",
    "common.clear":                            "清空",
    "common.invalidId":                        "无效 id",
    "common.noData":                           "暂无数据",
    "workspace.menu.crf":                       "CRF",
    "crf.formList.heading":                     "CRF 表单列表 — {projectCode}",
    "crf.detail.title":                         "CRF 详情",
    "crf.detail.placeholder":                   "表单详情即将上线",
    "crf.detail.back":                          "返回表单列表",
    "crf.globalSearch.heading":                 "CRF 全局搜索 — {projectCode}",
    "crf.globalSearch.searchPlaceholder":       "搜索表单、字段、选项、批注…",
    "crf.globalSearch.empty":                   "无结果",
    "crf.globalSearch.col.form":                "表单",
    "crf.globalSearch.col.item":                "字段",
    "crf.globalSearch.col.option":              "选项",
    "crf.globalSearch.col.annotation":          "批注",
    "crf.toolbar.statusPending":                "待处理",
    "crf.toolbar.globalSearch":                 "全局搜索",
    "crf.toolbar.globalSearchHint":             "打开 CRF 全局搜索",
    "crf.table.column.code":                    "表单编码",
    "crf.table.column.name":                    "表单名称",
    "crf.table.column.taker":                   "填写人",
    "crf.table.column.status":                  "状态",
    "crf.table.column.actions":                 "操作",
    "crf.table.action.assignTakers":            "分配填写人",
    "crf.table.action.edit":                    "编辑表单",
    "crf.table.action.delete":                  "删除表单",
    "crf.table.action.openDetail":              "打开表单详情",
    "crf.table.action.addForm":                 "新增表单",
    "crf.table.action.filter":                  "筛选表单",
    "crf.drawer.create.title":                  "新建 CRF 表单",
    "crf.drawer.edit.title":                    "编辑 CRF 表单",
    "crf.drawer.field.code":                    "表单编码",
    "crf.drawer.field.name":                    "表单名称",
    "crf.drawer.submit.create":                 "创建",
    "crf.drawer.submit.save":                   "保存",
    "crf.filter.title":                         "筛选 CRF 表单",
    "crf.filter.search":                        "按编码或名称搜索",
    "crf.filter.status":                        "状态",
    "crf.filter.status.approved":               "已批准",
    "crf.filter.status.pending":                "待处理",
    "crf.filter.involved":                      "参与",
    "crf.delete.title":                         "删除 CRF 表单",
    "crf.delete.message":                       "确定删除表单 \"{code} — {name}\" 吗?此操作不可撤销。",
    "crf.delete.submit":                        "删除",
    "crf.assignTakers.title":                   "分配填写人",
    "crf.assignTakers.placeholder":             "填写人界面即将上线",
```

- [ ] **Step 6: Run typecheck to verify**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS (no type errors).

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/query/keys.ts \
        apps/desktop/aegis-desktop/src/shared/api/index.ts \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(crf): add shared TS layer + i18n for CRF feature

Wire types CrfVersion + CrfForm + create/update inputs, query key
factory queryKeys.crf, api.crf namespace with 6 methods, and full
i18n key set in both en.ts and zhCN.ts.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Backend `http/crf/version.rs` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs` (module file)
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/version.rs` (DTOs + `list_by_project`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http.rs` (add `pub mod crf;`)

**Interfaces:**
- Produces: `crate::http::crf::version::{CrfVersionViewResponse, CrfVersionListResponse, list_by_project}`
- Signature: `pub async fn list_by_project(c: &HttpClient, project_code: &str) -> Result<Vec<CrfVersionViewResponse>, ApiError>;`

- [ ] **Step 1: Add module file `src-tauri/src/http/crf.rs`**

Create the file with content:

```rust
//! HTTP functions for the CRF namespace. One submodule per resource.
pub mod form;
pub mod version;
```

- [ ] **Step 2: Wire `pub mod crf;` into `src-tauri/src/http.rs`**

Add `pub mod crf;` in alphabetical order (between `pub mod config;` and `pub mod domain_model;`):

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod crf;
pub mod domain_model;
```

- [ ] **Step 3: Write the failing wiremock test for `list_by_project`**

Create `src-tauri/src/http/crf/version.rs` with the test block but **without** the implementation yet (only the test):

```rust
//! HTTP functions under `/api/crf/projects/{project_code}/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionListResponse {
    pub versions: Vec<CrfVersionViewResponse>,
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

    #[tokio::test]
    async fn list_by_project_returns_versions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/projects/abc/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": [{
                    "id": 1, "projectCode": "abc", "name": "v1",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let versions = list_by_project(&client(&server), "abc").await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id, 1);
        assert_eq!(versions[0].project_code, "abc");
        assert_eq!(versions[0].name, "v1");
        assert_eq!(
            versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }
}

pub async fn list_by_project(
    c: &HttpClient,
    _project_code: &str,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    unimplemented!()
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p aegis-desktop --lib http::crf::version::tests::list_by_project_returns_versions`
Expected: FAIL with "not implemented: list_by_project".

- [ ] **Step 5: Implement `list_by_project`**

Replace the `unimplemented!()` body with:

```rust
pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    let resp: CrfVersionListResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/crf/projects/{project_code}/versions"),
            None::<&()>,
        )
        .await?;
    Ok(resp.versions)
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p aegis-desktop --lib http::crf::version::tests::list_by_project_returns_versions`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/crf.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/crf/version.rs
git commit -m "feat(crf): add http/crf/version.rs list_by_project

TDD: wiremock test for GET /api/crf/projects/{code}/versions
returning a Vec<CrfVersionViewResponse>. Mirrors the
http/terminology/version.rs pattern.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Backend `http/crf/form.rs` (TDD — 5 endpoints)

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs` (DTOs + 5 endpoints)

**Interfaces:**
- Produces: `crate::http::crf::form::{CrfFormViewResponse, CrfFormListResponse, CreateCrfFormRequest, UpdateCrfFormRequest, list_by_version, create, update, delete, get_by_id}`

- [ ] **Step 1: Write the 5 failing wiremock tests**

Create `src-tauri/src/http/crf/form.rs` with DTOs + 5 unimplemented!() bodies + 5 tests (copy this verbatim):

```rust
//! HTTP functions under `/api/crf/versions/{id}/forms` and `/api/crf/forms/{id}`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormListResponse {
    pub forms: Vec<CrfFormViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfFormRequest {
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfFormRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

pub async fn list_by_version(
    _c: &HttpClient,
    _version_id: i64,
) -> Result<Vec<CrfFormViewResponse>, ApiError> {
    unimplemented!()
}

pub async fn create(
    _c: &HttpClient,
    _version_id: i64,
    _body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    unimplemented!()
}

pub async fn update(
    _c: &HttpClient,
    _id: i64,
    _body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    unimplemented!()
}

pub async fn delete(_c: &HttpClient, _id: i64) -> Result<(), ApiError> {
    unimplemented!()
}

pub async fn get_by_id(
    _c: &HttpClient,
    _id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    unimplemented!()
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

    fn form_view_json(id: i64, version_id: i64, code: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "versionId": version_id,
            "code": code,
            "name": name,
            "order": 0,
            "notSubmitted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_by_version_returns_forms() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/forms"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "forms": [form_view_json(11, 7, "AE", "Adverse Events")]
            })))
            .mount(&server)
            .await;
        let forms = list_by_version(&client(&server), 7).await.unwrap();
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].id, 11);
        assert_eq!(forms[0].version_id, 7);
        assert_eq!(forms[0].code, "AE");
        assert_eq!(forms[0].name, "Adverse Events");
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/versions/7/forms"))
            .respond_with(ResponseTemplate::new(201).set_body_json(form_view_json(11, 7, "AE", "Adverse Events")))
            .mount(&server)
            .await;
        let f = create(
            &client(&server),
            7,
            CreateCrfFormRequest {
                code: "AE".into(),
                name: "Adverse Events".into(),
                order: 0,
                not_submitted: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(f.id, 11);
        assert_eq!(f.code, "AE");
        assert_eq!(
            f.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/forms/11"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, 7, "AE", "Renamed")))
            .mount(&server)
            .await;
        let f = update(
            &client(&server),
            11,
            UpdateCrfFormRequest {
                code: None,
                name: Some("Renamed".into()),
                order: None,
                not_submitted: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(f.name, "Renamed");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/forms/11"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 11).await.unwrap();
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, 7, "AE", "Adverse Events")))
            .mount(&server)
            .await;
        let f = get_by_id(&client(&server), 11).await.unwrap();
        assert_eq!(f.id, 11);
        assert_eq!(f.code, "AE");
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCrfFormRequest {
            code: None,
            name: Some("renamed".into()),
            order: None,
            not_submitted: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}
```

- [ ] **Step 2: Run tests to verify they all fail**

Run: `cargo test -p aegis-desktop --lib http::crf::form::tests`
Expected: 5 tests FAIL with "not implemented".

- [ ] **Step 3: Implement all 5 endpoints**

Replace each `unimplemented!()` body with the real impl. Final form:

```rust
pub async fn list_by_version(
    c: &HttpClient,
    version_id: i64,
) -> Result<Vec<CrfFormViewResponse>, ApiError> {
    let resp: CrfFormListResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/crf/versions/{version_id}/forms"),
            None::<&()>,
        )
        .await?;
    Ok(resp.forms)
}

pub async fn create(
    c: &HttpClient,
    version_id: i64,
    body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/versions/{version_id}/forms"),
        Some(&body),
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/forms/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/forms/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{id}"),
        None::<&()>,
    )
    .await
}
```

- [ ] **Step 4: Run tests to verify they all pass**

Run: `cargo test -p aegis-desktop --lib http::crf::form::tests`
Expected: 6 tests PASS (5 endpoint tests + 1 serde test).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/crf/form.rs
git commit -m "feat(crf): add http/crf/form.rs with 5 endpoints (TDD)

Wiremock tests cover list_by_version (GET), update (PATCH),
delete (DELETE 204), create (POST 201), get_by_id (GET). Plus a
serde round-trip test that update_request_skips_none_fields.
Mirrors http/terminology/version.rs conventions.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Backend command shims + `lib.rs` registration

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/version.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` (add `pub mod crf;`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (extend `generate_handler!`)

**Interfaces:**
- Produces: `commands::crf::version::list_crf_versions`, `commands::crf::form::{list_crf_forms_by_version, create_crf_form, update_crf_form, delete_crf_form, get_crf_form_by_id}`

- [ ] **Step 1: Create the module file `src-tauri/src/commands/crf.rs`**

```rust
//! Tauri command shims for the CRF HTTP layer.
pub mod form;
pub mod version;
```

- [ ] **Step 2: Add `pub mod crf;` to `src-tauri/src/commands.rs`**

In alphabetical order (between `pub mod auth;` and `pub mod domain_model;`):

```rust
pub mod auth;
pub mod crf;
pub mod domain_model;
```

- [ ] **Step 3: Create `src-tauri/src/commands/crf/version.rs`**

```rust
//! Tauri command shim for `http::crf::version::list_by_project`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::version::{self, CrfVersionViewResponse};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_versions(
    client: State<'_, HttpClient>,
    project_code: String,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    version::list_by_project(&client, &project_code).await
}
```

- [ ] **Step 4: Create `src-tauri/src/commands/crf/form.rs`**

```rust
//! Tauri command shims for `http::crf::form`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::form::{
    self, CreateCrfFormRequest, CrfFormViewResponse, UpdateCrfFormRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<Vec<CrfFormViewResponse>, ApiError> {
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
```

- [ ] **Step 5: Register the 6 commands in `src-tauri/src/lib.rs`**

Inside the `tauri::generate_handler![...]` macro block, **before** the `// health` section (right after the `// domain-model` block), insert:

```rust
            // crf
            commands::crf::version::list_crf_versions,
            commands::crf::form::list_crf_forms_by_version,
            commands::crf::form::create_crf_form,
            commands::crf::form::update_crf_form,
            commands::crf::form::delete_crf_form,
            commands::crf::form::get_crf_form_by_id,
```

- [ ] **Step 6: Build to verify**

Run: `cargo build -p aegis-desktop`
Expected: PASS (no compile errors).

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/crf.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/crf/version.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(crf): add 6 Tauri command shims for CRF endpoints

commands::crf::version::list_crf_versions + 5 form commands,
all registered in generate_handler!. Pure 1:1 shims over
http::crf::*.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Frontend data hooks + barrels

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/list.ts`
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/crf/index.ts`

**Interfaces:**
- Produces: `useListCrfVersions`, `useListCrfForms`, `useGetCrfForm`, `useCreateCrfForm`, `useUpdateCrfForm`, `useDeleteCrfForm`

- [ ] **Step 1: Create `src/features/crf/data/list.ts`**

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "../../../shared/api";
import type {
  CrfForm,
  CrfVersion,
  CreateCrfFormInput,
  UpdateCrfFormInput,
} from "../../../shared/api";
import { ApiError } from "../../../shared/api/error";
import { queryKeys } from "../../../shared/query/keys";

export function useListCrfVersions(projectCode: string | null) {
  return useQuery<CrfVersion[], ApiError>({
    queryKey: queryKeys.crf.versionsByProject(projectCode ?? ""),
    queryFn: () => api.crf.listVersions(projectCode!),
    enabled: projectCode != null && projectCode !== "",
  });
}

export function useListCrfForms(versionId: number | null) {
  return useQuery<CrfForm[], ApiError>({
    queryKey: queryKeys.crf.formsByVersion(versionId ?? 0),
    queryFn: () => api.crf.listFormsByVersion(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

export function useGetCrfForm(id: number | null) {
  return useQuery<CrfForm, ApiError>({
    queryKey: queryKeys.crf.form(id ?? 0),
    queryFn: () => api.crf.getFormById(id!),
    enabled: id != null && Number.isFinite(id) && id > 0,
  });
}

export function useCreateCrfForm() {
  const qc = useQueryClient();
  return useMutation<
    CrfForm,
    ApiError,
    { versionId: number; body: CreateCrfFormInput }
  >({
    mutationFn: ({ versionId, body }) => api.crf.createForm(versionId, body),
    onSuccess: (created) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(created.versionId),
      });
    },
  });
}

export function useUpdateCrfForm() {
  const qc = useQueryClient();
  return useMutation<
    CrfForm,
    ApiError,
    { id: number; body: UpdateCrfFormInput }
  >({
    mutationFn: ({ id, body }) => api.crf.updateForm(id, body),
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

export function useDeleteCrfForm() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; versionId: number }
  >({
    mutationFn: ({ id }) => api.crf.deleteForm(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(vars.versionId),
      });
    },
  });
}
```

- [ ] **Step 2: Create `src/features/crf/data/index.ts`**

```ts
export * from "./list";
```

- [ ] **Step 3: Create `src/features/crf/index.ts`**

```ts
export * from "./pages";
export * from "./data/list";
```

- [ ] **Step 4: Run typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/
git commit -m "feat(crf): add data hooks + barrels

Six React Query hooks (versions list, forms list, single form
get, create/update/delete mutations) with the correct cache
invalidation for both the per-version forms list and the
per-form detail key.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Frontend components (8 small presentational pieces)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfStatusChip.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfGlobalSearchButton.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfVersionDropdown.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/DeleteCrfFormDialog.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormFilterDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/index.ts`

**Interfaces:**
- Produces: `CrfStatusChip`, `CrfGlobalSearchButton` (takes `projectCode: string`), `CrfVersionDropdown`, `CrfFormDrawer`, `DeleteCrfFormDialog`, `CrfAssignTakersDrawer`, `CrfFormFilterDrawer`, `CrfFormTable`

- [ ] **Step 1: Create `CrfStatusChip.tsx`**

```tsx
import { Chip } from "@aegis/ui/mui";
import { PendingActions as PendingActionsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

/**
 * Placeholder status chip. The status API is not ready; renders
 * a literal "Pending" label with a pending-actions glyph.
 */
export function CrfStatusChip() {
  const { t } = useI18n();
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

- [ ] **Step 2: Create `CrfGlobalSearchButton.tsx`**

```tsx
import { Button } from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate } from "@tanstack/react-router";

/**
 * Button that navigates to the CRF Global Search page for the
 * current project. Replaces the spec's "MoreVert + Menu" pattern
 * with a direct navigation button.
 */
export function CrfGlobalSearchButton({ projectCode }: { projectCode: string }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  return (
    <Button
      startIcon={<SearchIcon />}
      variant="outlined"
      size="small"
      onClick={() =>
        navigate({
          to: "/project/$projectCode/crf/search",
          params: { projectCode },
        })
      }
      title={t("crf.toolbar.globalSearchHint")}
    >
      {t("crf.toolbar.globalSearch")}
    </Button>
  );
}
```

- [ ] **Step 3: Create `CrfVersionDropdown.tsx`**

```tsx
import { FormControl, InputLabel, MenuItem, Select } from "@aegis/ui/mui";
import type { CrfVersion } from "../../../shared/api";

interface Props {
  versions: CrfVersion[];
  value: number | null;
  onChange: (versionId: number) => void;
  disabled?: boolean;
}

/**
 * Select dropdown of CRF versions. Disabled when there are no
 * versions yet; placeholder shown when value is null.
 */
export function CrfVersionDropdown({ versions, value, onChange, disabled }: Props) {
  return (
    <FormControl size="small" sx={{ minWidth: 200 }} disabled={disabled}>
      <InputLabel id="crf-version-select-label">Version</InputLabel>
      <Select<number | null>
        labelId="crf-version-select-label"
        label="Version"
        value={value ?? ""}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (Number.isFinite(v) && v > 0) onChange(v);
        }}
      >
        {versions.map((v) => (
          <MenuItem key={v.id} value={v.id}>
            {v.name}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}
```

- [ ] **Step 4: Create `CrfFormDrawer.tsx`**

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
import type { ApiError } from "../../../shared/api/error";
import type { CrfForm, CreateCrfFormInput } from "../../../shared/api";

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: CrfForm;
  onClose: () => void;
  onCreate: (input: CreateCrfFormInput) => void;
  onUpdate: (id: number, input: { code?: string; name?: string }) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY = { code: "", name: "" };

/**
 * Right-anchored drawer for create / edit. Mode = "create" posts
 * a fresh form; mode = "edit" patches the existing row's code +
 * name (other fields are deferred this PR).
 */
export function CrfFormDrawer({
  open,
  mode,
  row,
  onClose,
  onCreate,
  onUpdate,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [code, setCode] = useState(EMPTY.code);
  const [name, setName] = useState(EMPTY.name);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setCode(row.code);
      setName(row.name);
    } else {
      setCode(EMPTY.code);
      setName(EMPTY.name);
    }
  }, [open, mode, row]);

  const submitDisabled =
    mutationPending || code.trim() === "" || name.trim() === "";

  function handleSubmit() {
    if (submitDisabled) return;
    if (mode === "edit" && row) {
      onUpdate(row.id, { code: code.trim(), name: name.trim() });
    } else {
      onCreate({
        code: code.trim(),
        name: name.trim(),
        order: 0,
        notSubmitted: false,
      });
    }
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
          {t(mode === "create" ? "crf.drawer.create.title" : "crf.drawer.edit.title")}
        </Typography>
        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("crf.drawer.field.code")}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            required
            inputProps={{ maxLength: 64 }}
          />
          <TextField
            size="small"
            label={t("crf.drawer.field.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
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
            {t(mode === "create" ? "crf.drawer.submit.create" : "crf.drawer.submit.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 5: Create `DeleteCrfFormDialog.tsx`**

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
import type { ApiError } from "../../../shared/api/error";
import type { CrfForm } from "../../../shared/api";

interface Props {
  open: boolean;
  row: CrfForm | null;
  onClose: () => void;
  onConfirm: (form: CrfForm) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

export function DeleteCrfFormDialog({
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
      <DialogTitle>{t("crf.delete.title")}</DialogTitle>
      <DialogContent>
        {row && (
          <Alert severity="warning">
            {t("crf.delete.message", { code: row.code, name: row.name })}
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
          {t("crf.delete.submit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 6: Create `CrfAssignTakersDrawer.tsx`**

```tsx
import {
  Box,
  Button,
  Drawer,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

interface Props {
  open: boolean;
  onClose: () => void;
}

/**
 * Empty placeholder drawer for the assign-takers flow. Per spec,
 * has no content yet — just title + body placeholder + close.
 */
export function CrfAssignTakersDrawer({ open, onClose }: Props) {
  const { t } = useI18n();
  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">{t("crf.assignTakers.title")}</Typography>
        <Typography color="textSecondary">
          {t("crf.assignTakers.placeholder")}
        </Typography>
        <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
          <Button onClick={onClose}>{t("common.close")}</Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 7: Create `CrfFormFilterDrawer.tsx`**

```tsx
import {
  Box,
  Button,
  Checkbox,
  Chip,
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

export type CrfStatusFilter = "approved" | "pending";

interface Props {
  open: boolean;
  searchInput: string;
  onSearchInputChange: (value: string) => void;
  statusSelected: CrfStatusFilter[];
  onStatusSelectedChange: (value: CrfStatusFilter[]) => void;
  onClear: () => void;
  onApply: () => void;
}

/**
 * Right-anchored filter drawer. Status multi-select + Involved
 * checkbox are UI scaffolding only — the page tracks them but
 * the in-memory filter only consumes the search text this PR.
 */
export function CrfFormFilterDrawer({
  open,
  searchInput,
  onSearchInputChange,
  statusSelected,
  onStatusSelectedChange,
  onClear,
  onApply,
}: Props) {
  const { t } = useI18n();
  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onApply}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">{t("crf.filter.title")}</Typography>
        <TextField
          size="small"
          label={t("crf.filter.search")}
          value={searchInput}
          onChange={(e) => onSearchInputChange(e.target.value)}
        />
        <FormControl size="small">
          <InputLabel id="crf-filter-status-label">
            {t("crf.filter.status")}
          </InputLabel>
          <Select
            labelId="crf-filter-status-label"
            label={t("crf.filter.status")}
            multiple
            value={statusSelected}
            onChange={(e) => {
              const v = e.target.value;
              onStatusSelectedChange(
                Array.isArray(v) ? (v as CrfStatusFilter[]) : [],
              );
            }}
            renderValue={(selected) => (
              <Stack direction="row" spacing={0.5} flexWrap="wrap">
                {selected.map((s) => (
                  <Chip
                    key={s}
                    label={t(
                      s === "approved"
                        ? "crf.filter.status.approved"
                        : "crf.filter.status.pending",
                    )}
                    size="small"
                  />
                ))}
              </Stack>
            )}
          >
            <MenuItem value="approved">
              {t("crf.filter.status.approved")}
            </MenuItem>
            <MenuItem value="pending">
              {t("crf.filter.status.pending")}
            </MenuItem>
          </Select>
        </FormControl>
        <FormControlLabel
          control={<Checkbox disabled checked={false} />}
          label={t("crf.filter.involved")}
        />
        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClear}>{t("common.clear")}</Button>
          <Button variant="contained" onClick={onApply}>
            {t("common.apply")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 8: Create `CrfFormTable.tsx`**

```tsx
import {
  Box,
  Chip,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
} from "@aegis/ui/mui";
import {
  Add as AddIcon,
  AssignmentInd as AssignmentIndIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  FilterList as FilterListIcon,
  Launch as LaunchIcon,
  PendingActions as PendingActionsIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import type { CrfForm } from "../../../shared/api";

interface Props {
  rows: CrfForm[];
  loading: boolean;
  error: Error | null;
  canAddFilter: boolean;
  onAdd: () => void;
  onFilter: () => void;
  onAssignTakers: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
}

export function CrfFormTable({
  rows,
  loading,
  error,
  canAddFilter,
  onAdd,
  onFilter,
  onAssignTakers,
  onEdit,
  onDelete,
  onOpenDetail,
}: Props) {
  const { t } = useI18n();
  return (
    <TableContainer component={Paper}>
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell>{t("crf.table.column.code")}</TableCell>
            <TableCell>{t("crf.table.column.name")}</TableCell>
            <TableCell>{t("crf.table.column.taker")}</TableCell>
            <TableCell>{t("crf.table.column.status")}</TableCell>
            <TableCell align="right">
              <Tooltip title={t("crf.table.action.addForm")}>
                <IconButton
                  size="small"
                  aria-label={t("crf.table.action.addForm")}
                  onClick={onAdd}
                >
                  <AddIcon />
                </IconButton>
              </Tooltip>
              <Tooltip title={t("crf.table.action.filter")}>
                <IconButton
                  size="small"
                  aria-label={t("crf.table.action.filter")}
                  onClick={onFilter}
                  disabled={!canAddFilter}
                >
                  <FilterListIcon />
                </IconButton>
              </Tooltip>
            </TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.length === 0 && !loading && !error && (
            <TableRow>
              <TableCell colSpan={5} align="center">
                <Box sx={{ py: 3, color: "text.secondary" }}>
                  {t("common.noData")}
                </Box>
              </TableCell>
            </TableRow>
          )}
          {rows.map((row) => (
            <TableRow key={row.id} hover>
              <TableCell>{row.code}</TableCell>
              <TableCell>{row.name}</TableCell>
              <TableCell />
              <TableCell>
                <Chip
                  icon={<PendingActionsIcon />}
                  label={t("crf.toolbar.statusPending")}
                  size="small"
                  color="warning"
                  variant="outlined"
                />
              </TableCell>
              <TableCell align="right">
                <Tooltip title={t("crf.table.action.assignTakers")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.assignTakers")}
                    onClick={() => onAssignTakers(row)}
                  >
                    <AssignmentIndIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.edit")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.edit")}
                    onClick={() => onEdit(row)}
                  >
                    <EditIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.delete")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.delete")}
                    onClick={() => onDelete(row)}
                  >
                    <DeleteIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.openDetail")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.openDetail")}
                    onClick={() => onOpenDetail(row)}
                  >
                    <LaunchIcon />
                  </IconButton>
                </Tooltip>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
}
```

- [ ] **Step 9: Create `components/index.ts`**

```ts
export * from "./CrfStatusChip";
export * from "./CrfGlobalSearchButton";
export * from "./CrfVersionDropdown";
export * from "./CrfFormDrawer";
export * from "./DeleteCrfFormDialog";
export * from "./CrfAssignTakersDrawer";
export * from "./CrfFormFilterDrawer";
export * from "./CrfFormTable";
```

- [ ] **Step 10: Run typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/
git commit -m "feat(crf): add 8 presentational components

CrfStatusChip, CrfGlobalSearchButton, CrfVersionDropdown,
CrfFormDrawer (create+edit), DeleteCrfFormDialog,
CrfAssignTakersDrawer (placeholder), CrfFormFilterDrawer,
CrfFormTable — all stateless, controlled by parent page.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: CrfFormListPage + smoke test

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/pages/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

**Interfaces:**
- Produces: `CrfFormListPage` (mounted by `routes/_authed/project/$projectCode/crf/index.tsx`)
- Test: renders list page under fake router, mocks `list_crf_versions` + `list_crf_forms_by_version`, asserts one row text + toolbar visibility.

- [ ] **Step 1: Create `src/features/crf/pages/index.ts`**

```ts
export * from "./CrfFormListPage";
export * from "./CrfDetailPage";
export * from "./CrfGlobalSearchPage";
```

- [ ] **Step 2: Create `src/features/crf/pages/CrfFormListPage.tsx`**

```tsx
import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Box,
  CircularProgress,
  Stack,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import {
  CrfAssignTakersDrawer,
  CrfFormDrawer,
  type CrfStatusFilter,
  CrfFormFilterDrawer,
  CrfFormTable,
  CrfGlobalSearchButton,
  CrfStatusChip,
  CrfVersionDropdown,
  DeleteCrfFormDialog,
} from "../components";
import {
  useCreateCrfForm,
  useDeleteCrfForm,
  useListCrfForms,
  useListCrfVersions,
  useUpdateCrfForm,
} from "../data/list";
import type { CrfForm } from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";

type DrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CrfForm }
  | null;

export function CrfFormListPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as { projectCode: string };
  const navigate = useNavigate();
  const routeSearch = useSearch({ strict: false }) as { versionId?: number };
  const selectedVersionId =
    typeof routeSearch.versionId === "number" && routeSearch.versionId > 0
      ? routeSearch.versionId
      : null;

  const versionsQuery = useListCrfVersions(projectCode);
  const versions = versionsQuery.data ?? [];

  // Reconcile ?versionId URL ↔ first version fallback.
  useEffect(() => {
    if (versions.length === 0) return;
    const valid =
      selectedVersionId != null &&
      versions.some((v) => v.id === selectedVersionId);
    if (!valid) {
      navigate({
        to: "/project/$projectCode/crf",
        params: { projectCode },
        search: { versionId: versions[0].id },
        replace: true,
      });
    }
  }, [versions, selectedVersionId, projectCode, navigate]);

  const formsQuery = useListCrfForms(selectedVersionId);
  const allRows = formsQuery.data ?? [];

  // Page-owned filter state (drawer is fully controlled).
  const [searchInput, setSearchInput] = useState("");
  const [statusSelected, setStatusSelected] = useState<CrfStatusFilter[]>([]);
  const [involvedChecked] = useState(false);

  // Inline debounce: 300 ms delay, 1000 ms max-wait.
  const [debouncedSearch, setDebouncedSearch] = useState("");
  useEffect(() => {
    const handle = setTimeout(() => setDebouncedSearch(searchInput), 300);
    return () => clearTimeout(handle);
  }, [searchInput]);

  const filteredRows = useMemo(() => {
    const q = debouncedSearch.trim().toLowerCase();
    return allRows.filter(
      (r) =>
        q === "" ||
        r.code.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q),
      // statusSelected + involvedChecked are held but no-op this PR
      void statusSelected,
      void involvedChecked,
    );
  }, [allRows, debouncedSearch, statusSelected, involvedChecked]);

  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CrfForm | null>(null);
  const [assignTakersFor, setAssignTakersFor] = useState<CrfForm | null>(null);
  const [filterOpen, setFilterOpen] = useState(false);

  const createMutation = useCreateCrfForm();
  const updateMutation = useUpdateCrfForm();
  const deleteMutation = useDeleteCrfForm();

  const activeMutationError =
    createMutation.error ?? updateMutation.error ?? deleteMutation.error ?? null;
  const activeMutationPending =
    createMutation.isPending ||
    updateMutation.isPending ||
    deleteMutation.isPending;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4">
        {t("crf.formList.heading", { projectCode })}
      </Typography>

      <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
        <CrfVersionDropdown
          versions={versions}
          value={selectedVersionId}
          onChange={(versionId) =>
            navigate({
              to: "/project/$projectCode/crf",
              params: { projectCode },
              search: { versionId },
            })
          }
          disabled={versions.length === 0}
        />
        <CrfStatusChip />
        <Box sx={{ flexGrow: 1 }} />
        <CrfGlobalSearchButton projectCode={projectCode} />
      </Stack>

      {versionsQuery.isError && (
        <Alert severity="error">
          {errorMessage(versionsQuery.error)}
        </Alert>
      )}
      {formsQuery.isError && (
        <Alert severity="error">
          {errorMessage(formsQuery.error)}
        </Alert>
      )}
      {formsQuery.isFetching && !formsQuery.data && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      <CrfFormTable
        rows={filteredRows}
        loading={formsQuery.isFetching}
        error={formsQuery.error}
        canAddFilter={selectedVersionId != null}
        onAdd={() => setDrawer({ mode: "create" })}
        onFilter={() => setFilterOpen(true)}
        onAssignTakers={(row) => setAssignTakersFor(row)}
        onEdit={(row) => setDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpenDetail={(row) =>
          navigate({
            to: "/project/$projectCode/crf/$formId",
            params: { projectCode, formId: String(row.id) },
          })
        }
      />

      <CrfFormDrawer
        open={drawer != null}
        mode={drawer?.mode ?? "create"}
        row={drawer?.mode === "edit" ? drawer.row : undefined}
        onClose={() => setDrawer(null)}
        onCreate={(input) => {
          if (selectedVersionId == null) return;
          createMutation.mutate(
            { versionId: selectedVersionId, body: input },
            { onSuccess: () => setDrawer(null) },
          );
        }}
        onUpdate={(id, body) => {
          updateMutation.mutate(
            { id, body },
            { onSuccess: () => setDrawer(null) },
          );
        }}
        mutationError={activeMutationError}
        mutationPending={activeMutationPending}
      />

      <DeleteCrfFormDialog
        open={confirmDelete != null}
        row={confirmDelete}
        onClose={() => setConfirmDelete(null)}
        onConfirm={(row) => {
          deleteMutation.mutate(
            { id: row.id, versionId: row.versionId },
            { onSuccess: () => setConfirmDelete(null) },
          );
        }}
        mutationError={deleteMutation.error}
        mutationPending={deleteMutation.isPending}
      />

      <CrfAssignTakersDrawer
        open={assignTakersFor != null}
        onClose={() => setAssignTakersFor(null)}
      />

      <CrfFormFilterDrawer
        open={filterOpen}
        searchInput={searchInput}
        onSearchInputChange={setSearchInput}
        statusSelected={statusSelected}
        onStatusSelectedChange={setStatusSelected}
        onClear={() => {
          setSearchInput("");
          setStatusSelected([]);
        }}
        onApply={() => setFilterOpen(false)}
      />
    </Box>
  );
}
```

- [ ] **Step 3: Write failing smoke test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockInvoke } from "../../helpers/tauri-mock";
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

describe("CrfFormListPage", () => {
  it("renders the heading + one form row from the mocked backend", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "is_logged_in") return true;
      if (cmd === "current_user") {
        return { id: 1, code: "u", role: "admin", createdAt: "", updatedAt: "" };
      }
      if (cmd === "list_crf_versions") {
        return [{ id: 7, projectCode: "abc", name: "v1" }];
      }
      if (cmd === "list_crf_forms_by_version") {
        return [
          {
            id: 11,
            versionId: 7,
            code: "AE",
            name: "Adverse Events",
            order: 0,
            notSubmitted: false,
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ];
      }
      return undefined;
    });

    renderPage(["/project/abc/crf?versionId=7"]);

    expect(
      await screen.findByRole("heading", { name: /CRF Form List/i }),
    ).toBeInTheDocument();

    expect(await screen.findByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("AE")).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-list-page.test.tsx`
Expected: PASS.

- [ ] **Step 5: Run typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx \
        apps/desktop/aegis-desktop/src/features/crf/pages/index.ts \
        apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx
git commit -m "feat(crf): add CrfFormListPage + smoke test

URL-driven ?versionId= selector with replace-on-fallback effect,
in-memory search/filter via useDebouncedValue, all 3 mutations
(create/edit/delete) wired to their drawers + the delete dialog,
plus a vitest smoke test that mounts the page under a fake router,
mocks the two list commands, and asserts the row text.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: CrfDetailPage + CrfGlobalSearchPage

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx`

**Interfaces:**
- Produces: `CrfDetailPage` (mounted by `routes/_authed/project/$projectCode/crf/$formId.tsx`)
- Produces: `CrfGlobalSearchPage` (mounted by `routes/_authed/project/$projectCode/crf/search.tsx`)

- [ ] **Step 1: Create `CrfDetailPage.tsx`**

```tsx
import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  Stack,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import {
  useNavigate,
  useParams,
} from "@tanstack/react-router";

import { CrfGlobalSearchButton } from "../components";
import { useGetCrfForm } from "../data/list";
import { errorMessage } from "../../../shared/api/error";

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

  const back = () =>
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: (prev: Record<string, unknown>) => prev,
    });

  if (id == null) {
    return (
      <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
        <Stack direction="row" spacing={2} alignItems="center">
          <IconButton aria-label={t("crf.detail.back")} onClick={back}>
            <ArrowBackIcon />
          </IconButton>
          <Typography variant="h4">{t("crf.detail.title")}</Typography>
        </Stack>
        <Alert severity="error">{t("common.invalidId")}</Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
        <IconButton aria-label={t("crf.detail.back")} onClick={back}>
          <ArrowBackIcon />
        </IconButton>
        {query.data && <Chip label={query.data.code} variant="outlined" />}
        {query.data && (
          <Typography variant="h5">{query.data.name}</Typography>
        )}
        {!query.data && !query.isError && (
          <Typography variant="h5">{t("crf.detail.title")}</Typography>
        )}
        <Box sx={{ flexGrow: 1 }} />
        <CrfGlobalSearchButton projectCode={projectCode} />
      </Stack>

      {query.isFetching && !query.data && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      {query.isError && (
        <Alert severity="error">{errorMessage(query.error)}</Alert>
      )}
      {!query.isFetching && (
        <Alert severity="info">{t("crf.detail.placeholder")}</Alert>
      )}
    </Box>
  );
}
```

- [ ] **Step 2: Create `CrfGlobalSearchPage.tsx`**

```tsx
import { useState } from "react";
import {
  Box,
  IconButton,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams } from "@tanstack/react-router";

import { CrfGlobalSearchButton } from "../components";

export function CrfGlobalSearchPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as { projectCode: string };
  const navigate = useNavigate();
  const [fragment, setFragment] = useState("");

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
        <IconButton
          aria-label={t("crf.detail.back")}
          onClick={() =>
            navigate({
              to: "/project/$projectCode/crf",
              params: { projectCode },
            })
          }
        >
          <ArrowBackIcon />
        </IconButton>
        <Typography variant="h4">
          {t("crf.globalSearch.heading", { projectCode })}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <CrfGlobalSearchButton projectCode={projectCode} />
      </Stack>

      <TextField
        size="small"
        placeholder={t("crf.globalSearch.searchPlaceholder")}
        value={fragment}
        onChange={(e) => setFragment(e.target.value)}
        fullWidth
      />

      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>{t("crf.globalSearch.col.form")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.item")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.option")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.annotation")}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            <TableRow>
              <TableCell colSpan={4} align="center">
                <Box sx={{ py: 3, color: "text.secondary" }}>
                  {t("crf.globalSearch.empty")}
                </Box>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </TableContainer>
    </Box>
  );
}
```

- [ ] **Step 3: Run typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx \
        apps/desktop/aegis-desktop/src/features/crf/pages/CrfGlobalSearchPage.tsx
git commit -m "feat(crf): add CrfDetailPage + CrfGlobalSearchPage

CrfDetailPage shows back button + code chip + name +
global-search button, with an Alert placeholder body and an
error state when the form fetch fails.

CrfGlobalSearchPage has back + heading + global-search button
+ search textfield + empty results table.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: Route files + sidebar entry

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/index.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/$formId.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/crf/search.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx` (add CRF menu entry + AssignmentIcon import)

- [ ] **Step 1: Create `crf/index.tsx`**

```tsx
import { createFileRoute } from "@tanstack/react-router";

import { CrfFormListPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/",
)({
  component: CrfFormListPage,
});
```

- [ ] **Step 2: Create `crf/$formId.tsx`**

```tsx
import { createFileRoute } from "@tanstack/react-router";

import { CrfDetailPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/$formId",
)({
  component: CrfDetailPage,
});
```

- [ ] **Step 3: Create `crf/search.tsx`**

```tsx
import { createFileRoute } from "@tanstack/react-router";

import { CrfGlobalSearchPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/search",
)({
  component: CrfGlobalSearchPage,
});
```

- [ ] **Step 4: Add CRF menu entry to `ProjectWorkspaceLayout.tsx`**

In [apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx](apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx):

- Add to the `@aegis/ui/icons` import:
 ```tsx
 Assignment as AssignmentIcon,
 ```

- Add below the existing `ConfigMenuIcon`:
 ```tsx
 const CrfMenuIcon = () => <AssignmentIcon />;
 ```

- Inside the `menu: MenuItem[]` array, after the `configuration` entry, add:
 ```tsx
 {
   link: `/project/${projectCode}/crf`,
   title: t("workspace.menu.crf"),
   icon: CrfMenuIcon,
 },
 ```

- [ ] **Step 5: Run typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 6: Regenerate the router tree**

Run: `pnpm --filter aegis-desktop dev` once (Ctrl+C after a couple seconds), or `pnpm --filter aegis-desktop build`, so `@tanstack/router-plugin` regenerates `src/routes/routeTree.gen.ts` with the three new files.

Expected: `routeTree.gen.ts` now references the new route ids.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_authed/project/\$projectCode/crf/ \
        apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectWorkspaceLayout.tsx \
        apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git commit -m "feat(crf): wire routes + sidebar entry

Three TanStack Router files under project/\$projectCode/crf/
(index, \$formId, search) plus a CRF menu entry in
ProjectWorkspaceLayout. The router plugin regenerates
routeTree.gen.ts to register the new paths.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Final verification gate

**Files:** (none modified — verification only)

- [ ] **Step 1: Rust formatting**

Run: `cargo fmt --all -- --check`
Expected: PASS. If anything fails: `cargo fmt --all`.

- [ ] **Step 2: Rust lint**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: PASS with no warnings.

- [ ] **Step 3: Rust unit tests**

Run: `cargo test -p aegis-desktop`
Expected: ALL tests PASS, including the 7 wiremock tests in `http::crf::*`.

- [ ] **Step 4: TS typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 5: TS unit tests**

Run: `pnpm --filter aegis-desktop test`
Expected: ALL tests PASS, including the new smoke test for `CrfFormListPage`.

- [ ] **Step 6: TS build**

Run: `pnpm --filter aegis-desktop build`
Expected: PASS — Vite produces `dist/` cleanly with the new routes registered.

- [ ] **Step 7: Final commit (if any auto-fixes were needed)**

If steps 1-6 surfaced any auto-fixable issues, commit them now:

```bash
git add -A
git commit -m "chore(crf): verification gate cleanups

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Spec coverage check

Walking through each section of the spec:

- **§1 Goals:** form list (toolbar + table + drawers + chip + search btn) ✓ Task 6 + 7
- **§1 Goals:** detail (back + chip + name + search btn) ✓ Task 8
- **§1 Goals:** global search (search input + empty table) ✓ Task 8
- **§1 Goals:** "API not ready" → status / taker / involved / taker drawer / global search results are placeholders ✓ all components
- **§3 Routes:** `crf/{index,$formId,search}.tsx` ✓ Task 9
- **§3 Sidebar:** `ProjectWorkspaceLayout` CRF menu entry ✓ Task 9
- **§4 Frontend feature module:** full layout + barrel ✓ Tasks 5/6/7/8 + Step 1 of Task 9
- **§4.1 Data hooks:** 6 hooks with correct cache invalidation ✓ Task 5
- **§4.2 CrfFormListPage:** URL-driven selector + debounced search + 3 drawers + delete dialog ✓ Task 7
- **§4.2 CrfDetailPage:** header row + Alert body + error state ✓ Task 8
- **§4.2 CrfGlobalSearchPage:** header + search field + empty table ✓ Task 8
- **§4.3 Components:** all 8 components with right props ✓ Task 6
- **§5 Shared types:** `CrfVersion`, `CrfForm`, request/response shapes ✓ Task 1
- **§5 Query keys:** `crf.{versionsByProject,formsByVersion,form}` ✓ Task 1
- **§5 Api namespace:** 6 methods ✓ Task 1
- **§6 Backend http/crf/version.rs:** DTOs + `list_by_project` ✓ Task 2
- **§6 Backend http/crf/form.rs:** DTOs + 5 endpoints ✓ Task 3
- **§6 Command shims:** 6 commands + lib.rs registration ✓ Task 4
- **§7 i18n:** both `en.ts` + `zhCN.ts` with full key set ✓ Task 1
- **§8 Error handling:** mutation `Alert severity="error"` + page-level error rendering ✓ Tasks 7/8
- **§9 Testing:** Rust wiremock + TS smoke test ✓ Tasks 2/3/7
- **§10 Verification gate:** final fmt/clippy/test/build ✓ Task 10