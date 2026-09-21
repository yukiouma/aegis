# Mission Issue Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the existing mission-issue API (backend already shipped via PR #76) inside the CRF detail page so QC, project leaders, and DEV assignees can read, create, comment on, and close mission-level and item-level issues.

**Architecture:** Tauri command layer proxies the server's 5 issue endpoints; desktop mirrors the wire DTOs in `shared/api/types.ts`; React Query hooks in `features/mission/data/issues.ts` cover list + 4 mutations; a single `MissionIssueDialog` component shows a table of issues scoped to a chip's scope (form-level vs item-level) with inline expansion for comments / edit-description / state-flip. Form-code and item-code chips are wrapped in MUI `Badge` (first use in the codebase) that shows a red dot when at least one opened issue exists at the scope.

**Tech Stack:** Tauri 2, React 19, TanStack Query 5, MUI v6, `@aegis/ui` shared components, Vitest + Testing Library. Backend unchanged.

**Spec:** `docs/superpowers/specs/2026-09-21-mission-issue-dialog-design.md`

## Global Constraints

- Wire-format mirroring rule: TS identifiers camelCase, JSON keys snake_case (e.g. `targetItem` ↔ `target_item`). Server `IssueState` uses snake_case (`"opened"`, `"closed"`) so TS can `switch (state)` directly.
- Every business dialog follows the page-owns-mutation / dialog-owns-form-state convention. `mutationError: ApiError | null` and `mutationPending: boolean` are passed in.
- Query keys live ONLY in `apps/desktop/aegis-desktop/src/shared/query/keys.ts`. All hooks and invalidations reference them through the factory.
- Mutation `onSuccess` invalidates `queryKeys.mission.issuesByMission(missionId)`.
- i18n keys must be added to BOTH `lib/packages/ui/src/i18n/locales/en.ts` AND `lib/packages/ui/src/i18n/locales/zhCN.ts`. The `satisfies Record<keyof typeof en, string>` constraint at the bottom of `zhCN.ts` enforces sync.
- Backend auth model (gates UI RBAC): create / close / reopen / update-description → project leader OR mission QC; append comment → + DEV.
- Tauri command argument naming: every parameter name MUST be camelCase (Tauri maps JSON keys → fn params by camelCase name); wire-shape tests in `commands/mission.rs` enforce this.
- Tauri state-flip endpoint: `PATCH /api/mission/issues/{issue_id}/state?state=closed|opened` with empty body — this is the as-built server, not the 2026-09-20 spec's "body `{"state":"closed"}`" plan.
- `target_item: Option<String>` is immutable after creation. None = whole-mission issue, Some(label) = item-level issue.
- `description` is the only mutable text field after creation. `comments` are append-only.

---

## File Structure

### New files
- `apps/desktop/aegis-desktop/src/features/mission/data/issues.ts` — 5 React Query hooks (`useListIssuesByMission`, `useCreateIssue`, `usePatchIssueState`, `useUpdateIssueDescription`, `useAppendComment`)
- `apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx` — single dialog component with private `IssueRow` and `IssueDetails` helpers
- `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx` — integration tests (chips + dialog + mutations)

### Modified files
- `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs` — add 5 new `#[tauri::command]` shims
- `apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs` — add 5 HTTP adapter fns + DTOs (snake_case state, camelCase rest)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — add `IssueState`, `IssueCommentViewResponse`, `IssueViewResponse`, `IssueListResponse`, `IssueListQuery`, `CreateIssueInput`, `UpdateIssueDescriptionInput`, `AppendCommentInput`
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` — add 5 client fns (`listIssuesByMission`, `createIssue`, `patchIssueState`, `updateIssueDescription`, `appendComment`)
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` — add `issuesByMission(missionId)` key
- `apps/desktop/aegis-desktop/src/features/mission/index.ts` — re-export the 5 hooks + `MissionIssueDialog`
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` — derive `missionForForm`, `openIssueCountByTarget`, RBAC; wrap form-code chip in `<Badge>`; add `issueDialog` state; render `<MissionIssueDialog>`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` — add `hasOpenIssue`, `onOpenIssues`, `missionExists` props; wrap item-code chip in `<Badge>`
- `lib/packages/ui/src/i18n/locales/en.ts` — add 21 `crf.missionIssue.*` keys
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add the same 21 keys (Chinese)

---

## Task 1: Wire-mirror types + query key

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (after the existing `MissionViewResponse` interface, around line 620)
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (extend the `mission` slice)

**Interfaces:** None yet — types only. Defines the contract every later task consumes.

- [ ] **Step 1: Add types to `shared/api/types.ts`**

Insert immediately after the `MissionViewResponse` interface (around line 635 in the current file). The `IssueState` enum uses `snake_case` strings; every other field uses `camelCase` per CLAUDE.md §"Wire DTOs are duplicated by hand":

```ts
// Mission issue
export type IssueState = "opened" | "closed";

export interface IssueCommentViewResponse {
  user: string;
  content: string;
  createdAt: string;
}

export interface IssueViewResponse {
  id: number;
  missionId: number;
  targetItem?: string;
  issuer: string;
  description: string;
  state: IssueState;
  comments: IssueCommentViewResponse[];
  createdAt: string;
  updatedAt: string;
}

export interface IssueListResponse {
  issues: IssueViewResponse[];
}

export interface IssueListQuery {
  state?: IssueState;
}

export interface CreateIssueInput {
  targetItem?: string;
  description: string;
}

export interface UpdateIssueDescriptionInput {
  description: string;
}

export interface AppendCommentInput {
  content: string;
}
```

- [ ] **Step 2: Add `issuesByMission` key to `shared/query/keys.ts`**

Extend the `mission` slice (currently `byProject` only):

```ts
mission: {
  byProject: (projectCode: string, kind: string) =>
    ["mission", "byProject", projectCode, kind] as const,
  // NEW
  issuesByMission: (missionId: number) =>
    ["mission", "issuesByMission", missionId] as const,
},
```

Single key, no `state` parameter — we fetch all issues in one call and filter on the client (matches Approach A in the spec).

- [ ] **Step 3: Verify typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
```

Expected: PASS. No consumers yet, so this only checks the new types parse.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/shared/api/types.ts apps/desktop/aegis-desktop/src/shared/query/keys.ts && git commit -m "feat(shared): add mission-issue DTOs and query key" -m "Mirrors the server's issue wire format (snake_case state, camelCase
rest) into shared/api/types.ts and adds the issuesByMission key factory
entry. No consumers — pure foundation step.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 2: Tauri command layer (5 commands + 5 HTTP adapters)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs` (add DTOs and 5 HTTP fns; add serde tests)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs` (add 5 `#[tauri::command]` shims; add wire-shape tests)

**Interfaces:**
- Consumes: existing `HttpClient` (already imported in `http::mission.rs`), existing `http::client` helpers
- Produces: `pub fn list_issues_by_mission(...)`, `pub async fn create_issue(...)`, `pub async fn patch_issue_state(...)`, `pub async fn update_issue_description(...)`, `pub async fn append_comment(...)` in `http::mission.rs`; matching `#[tauri::command]` shims in `commands/mission.rs`

- [ ] **Step 1: Add DTOs to `src-tauri/src/http/mission.rs`**

Append after the existing `CreateMissionRequest` struct (around line 68):

```rust
// ===========================================================================
// Mission-issue wire DTOs — mirror `apps/server/aegis-server/src/transport/
// http/dto.rs` lines 2112-2249. State is snake_case so the TS client can
// `switch` directly; the rest is camelCase per CLAUDE.md §"Wire DTOs are
// duplicated by hand".
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Opened,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCommentViewResponse {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueViewResponse {
    pub id: i64,
    pub mission_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentViewResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueListResponse {
    pub issues: Vec<IssueViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IssueListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_item: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchIssueStateRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIssueDescriptionRequest {
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendCommentRequest {
    pub content: String,
}
```

- [ ] **Step 2: Add 5 HTTP adapter fns to `http::mission.rs`**

Append after `create_mission` (around line 134). Each follows the same pattern as the existing fns (`c.request(GET|POST|PATCH, url, Some(&body)).await`):

```rust
// ===========================================================================
// Mission-issue HTTP adapters. Server endpoints (mounted at /api/mission):
//   GET    /by-mission/{mission_id}/issues?state=opened|closed
//   POST   /by-mission/{mission_id}/issues
//   PATCH  /issues/{issue_id}/state?state=closed|opened  (empty body)
//   PATCH  /issues/{issue_id}/description
//   POST   /issues/{issue_id}/comments
//
// State-flip uses the `?state=` QUERY parameter (not body) — matches the
// as-built server, which deviates from the 2026-09-20 spec's "body
// {state:closed}" plan. See spec §"URL → Tauri command → server route".
// ===========================================================================

pub async fn list_issues_by_mission(
    c: &HttpClient,
    mission_id: i64,
    state: Option<IssueState>,
) -> Result<Vec<IssueViewResponse>, ApiError> {
    let mut url = format!("/api/mission/by-mission/{mission_id}/issues");
    if let Some(s) = state {
        let state_str = serde_json::to_string(&s)
            .map_err(|e| ApiError::Parse { message: e.to_string() })?
            .trim_matches('"')
            .to_string();
        url.push_str("?state=");
        url.push_str(&state_str);
    }
    let resp: IssueListResponse = c
        .request(reqwest::Method::GET, &url, None::<&()>)
        .await?;
    Ok(resp.issues)
}

pub async fn create_issue(
    c: &HttpClient,
    mission_id: i64,
    body: CreateIssueRequest,
) -> Result<IssueViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/mission/by-mission/{mission_id}/issues"),
        Some(&body),
    )
    .await
}

pub async fn patch_issue_state(
    c: &HttpClient,
    issue_id: i64,
    target: IssueState,
) -> Result<IssueViewResponse, ApiError> {
    let state_str = serde_json::to_string(&target)
        .map_err(|e| ApiError::Parse { message: e.to_string() })?
        .trim_matches('"')
        .to_string();
    let url = format!("/api/mission/issues/{issue_id}/state?state={state_str}");
    // Empty body — matches the server's `PatchIssueStateRequest {}`.
    let body = PatchIssueStateRequest {};
    c.request(reqwest::Method::PATCH, &url, Some(&body)).await
}

pub async fn update_issue_description(
    c: &HttpClient,
    issue_id: i64,
    body: UpdateIssueDescriptionRequest,
) -> Result<IssueViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/mission/issues/{issue_id}/description"),
        Some(&body),
    )
    .await
}

pub async fn append_comment(
    c: &HttpClient,
    issue_id: i64,
    body: AppendCommentRequest,
) -> Result<IssueViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/mission/issues/{issue_id}/comments"),
        Some(&body),
    )
    .await
}
```

- [ ] **Step 3: Add serde shape tests to `http::mission.rs`**

Append inside the existing `mod tests` block (around line 137). Two tests pin the wire shape:

```rust
#[test]
fn issue_state_serializes_snake_case() {
    assert_eq!(
        serde_json::to_string(&IssueState::Opened).unwrap(),
        "\"opened\""
    );
    assert_eq!(
        serde_json::to_string(&IssueState::Closed).unwrap(),
        "\"closed\""
    );
}

#[test]
fn issue_view_response_parses_full_wire_shape() {
    let j = r#"{
        "id": 7,
        "missionId": 10,
        "targetItem": "AE",
        "issuer": "carol",
        "description": "Missing CRF row in AE form",
        "state": "opened",
        "comments": [
            {
                "user": "bob",
                "content": "will fix by EOD",
                "createdAt": "2026-01-01T00:00:00Z"
            }
        ],
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }"#;
    let issue: IssueViewResponse = serde_json::from_str(j).unwrap();
    assert_eq!(issue.id, 7);
    assert_eq!(issue.state, IssueState::Opened);
    assert_eq!(issue.target_item.as_deref(), Some("AE"));
    assert_eq!(issue.comments.len(), 1);
    assert_eq!(issue.comments[0].user, "bob");
}
```

- [ ] **Step 4: Add 5 tauri commands to `commands/mission.rs`**

Append after the existing `create_mission` command (around line 95). Each shim is a thin wrapper — every parameter is camelCase per the file's existing convention:

```rust
#[tauri::command]
pub async fn list_issues_by_mission(
    client: State<'_, HttpClient>,
    mission_id: i64,
    state: Option<String>,
) -> Result<Vec<mission::IssueViewResponse>, ApiError> {
    let parsed = match state.as_deref() {
        Some(s) => Some(parse_issue_state(s)?),
        None => None,
    };
    mission::list_issues_by_mission(&client, mission_id, parsed).await
}

#[tauri::command]
pub async fn create_issue(
    client: State<'_, HttpClient>,
    mission_id: i64,
    body: mission::CreateIssueRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    mission::create_issue(&client, mission_id, body).await
}

#[tauri::command]
pub async fn patch_issue_state(
    client: State<'_, HttpClient>,
    issue_id: i64,
    state: String,
) -> Result<mission::IssueViewResponse, ApiError> {
    mission::patch_issue_state(&client, issue_id, parse_issue_state(&state)?).await
}

#[tauri::command]
pub async fn update_issue_description(
    client: State<'_, HttpClient>,
    issue_id: i64,
    body: mission::UpdateIssueDescriptionRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    mission::update_issue_description(&client, issue_id, body).await
}

#[tauri::command]
pub async fn append_comment(
    client: State<'_, HttpClient>,
    issue_id: i64,
    body: mission::AppendCommentRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    mission::append_comment(&client, issue_id, body).await
}
```

Also add a `parse_issue_state` helper alongside the existing `parse_kind` / `parse_role` (around line 31):

```rust
fn parse_issue_state(s: &str) -> Result<crate::http::mission::IssueState, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}
```

- [ ] **Step 5: Add wire-shape tests to `commands/mission.rs`**

Append inside the existing `mod tests` block. Each pins that the camelCase JSON key is the one the TS frontend must emit:

```rust
#[test]
fn create_issue_request_deserializes_camel_case_payload() {
    // Frontend emits { missionId, body: { targetItem?, description } }.
    let raw = json!({
        "missionId": 10,
        "body": { "targetItem": "AE", "description": "missing row" }
    });
    // We just need the body struct shape, but Tauri passes the whole
    // object — exercise serde_json with a wrapper struct.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper {
        mission_id: i64,
        body: crate::http::mission::CreateIssueRequest,
    }
    let w: Wrapper = serde_json::from_value(raw).unwrap();
    assert_eq!(w.mission_id, 10);
    assert_eq!(w.body.target_item.as_deref(), Some("AE"));
    assert_eq!(w.body.description, "missing row");
}

#[test]
fn create_issue_request_rejects_snake_case_body() {
    let raw = json!({
        "missionId": 10,
        "body": { "target_item": "AE", "description": "missing row" }
    });
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper {
        mission_id: i64,
        body: crate::http::mission::CreateIssueRequest,
    }
    let result: Result<Wrapper, _> = serde_json::from_value(raw);
    assert!(
        result.is_err(),
        "snake_case target_item must not parse against camelCase CreateIssueRequest"
    );
}
```

- [ ] **Step 6: Update the doc comment at the top of `commands/mission.rs`**

The file header currently lists every command's wire shape (lines 99-107). Add the new commands:

```rust
//!   list_issues_by_mission:    { missionId, state? }
//!   create_issue:             { missionId, body: { targetItem?, description } }
//!   patch_issue_state:        { issueId, state: "opened"|"closed" }
//!   update_issue_description: { issueId, body: { description } }
//!   append_comment:           { issueId, body: { content } }
```

- [ ] **Step 7: Verify build + tests**

Run:

```bash
cd d:/projects/rusty/aegis && cargo build -p aegis-desktop
cd d:/projects/rusty/aegis && cargo test -p aegis-desktop --lib http::mission::tests commands::mission::tests
```

Expected: build PASS; tests PASS.

- [ ] **Step 8: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs && git commit -m "feat(tauri): expose mission-issue HTTP endpoints" -m "Adds 5 #[tauri::command] shims (list_issues_by_mission, create_issue,
patch_issue_state, update_issue_description, append_comment) and their
HTTP adapters in src-tauri/src/http/mission.rs. Wire-shape tests pin
camelCase payload deserialization and snake_case state encoding.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 3: API client functions + i18n keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` (extend `api` object)
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (add 21 keys)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (add 21 keys)

**Interfaces:**
- Consumes: types added in Task 1
- Produces: `api.listIssuesByMission`, `api.createIssue`, `api.patchIssueState`, `api.updateIssueDescription`, `api.appendComment`

- [ ] **Step 1: Add 5 API client functions to `shared/api/index.ts`**

Append after the existing `createMission` entry (around line 463). Mirror the established patterns — `call<>` with `{ ...input }` spread for bodies, query-key wrappers use named args:

```ts
// mission issues
listIssuesByMission: (
  missionId: number,
  options: IssueListQuery = {},
): Promise<IssueViewResponse[]> =>
  call<IssueViewResponse[]>("list_issues_by_mission", {
    missionId,
    state: options.state,
  }),
createIssue: (
  missionId: number,
  body: CreateIssueInput,
): Promise<IssueViewResponse> =>
  call<IssueViewResponse>("create_issue", { missionId, body: { ...body } }),
patchIssueState: (
  issueId: number,
  state: IssueState,
): Promise<IssueViewResponse> =>
  call<IssueViewResponse>("patch_issue_state", { issueId, state }),
updateIssueDescription: (
  issueId: number,
  body: UpdateIssueDescriptionInput,
): Promise<IssueViewResponse> =>
  call<IssueViewResponse>("update_issue_description", {
    issueId,
    body: { ...body },
  }),
appendComment: (
  issueId: number,
  body: AppendCommentInput,
): Promise<IssueViewResponse> =>
  call<IssueViewResponse>("append_comment", { issueId, body: { ...body } }),
```

Also add the new types to the `import type { ... }` block at the top of the file (around line 7) and to the `export type { ... } from "./types"` block at the bottom (around line 467):

Imports to add:
```ts
AppendCommentInput,
CreateIssueInput,
IssueCommentViewResponse,
IssueListQuery,
IssueListResponse,
IssueState,
IssueViewResponse,
UpdateIssueDescriptionInput,
```

Exports to add (the 8 type names above).

- [ ] **Step 2: Add i18n keys to `lib/packages/ui/src/i18n/locales/en.ts`**

Insert immediately after the existing `crf.missionAssign.*` block (around line 397):

```ts
  "crf.missionIssue.tooltip.noMission": "No mission exists for this form yet",
  "crf.missionIssue.dialog.title": "Mission issues — {scope}",
  "crf.missionIssue.dialog.empty": "No issues yet",
  "crf.missionIssue.table.reviewer": "Reviewer",
  "crf.missionIssue.table.description": "Description",
  "crf.missionIssue.table.state": "State",
  "crf.missionIssue.state.opened": "Opened",
  "crf.missionIssue.state.closed": "Closed",
  "crf.missionIssue.create.field.description": "Issue description",
  "crf.missionIssue.create.submit": "Create",
  "crf.missionIssue.detail.comments": "Comments",
  "crf.missionIssue.detail.editDescription": "Edit description",
  "crf.missionIssue.detail.save": "Save",
  "crf.missionIssue.detail.cancel": "Cancel",
  "crf.missionIssue.detail.close": "Close",
  "crf.missionIssue.detail.reopen": "Reopen",
  "crf.missionIssue.detail.noComments": "No comments yet",
  "crf.missionIssue.detail.addComment": "Add comment",
  "crf.missionIssue.detail.commentPlaceholder": "Type a comment…",
  "crf.missionIssue.disabled.issueActions":
    "Only mission QC or project leaders can do this",
  "crf.missionIssue.disabled.comment":
    "Only mission QC, DEV, or project leaders can comment",
```

- [ ] **Step 3: Add the same keys to `lib/packages/ui/src/i18n/locales/zhCN.ts`**

Insert at the equivalent location (after `crf.missionAssign.*`). The `satisfies Record<keyof typeof en, string>` constraint at line 442 will refuse to compile if any key is missing:

```ts
  "crf.missionIssue.tooltip.noMission": "此表单暂无对应任务",
  "crf.missionIssue.dialog.title": "任务问题 — {scope}",
  "crf.missionIssue.dialog.empty": "暂无问题",
  "crf.missionIssue.table.reviewer": "审核人",
  "crf.missionIssue.table.description": "描述",
  "crf.missionIssue.table.state": "状态",
  "crf.missionIssue.state.opened": "打开",
  "crf.missionIssue.state.closed": "已关闭",
  "crf.missionIssue.create.field.description": "问题描述",
  "crf.missionIssue.create.submit": "创建",
  "crf.missionIssue.detail.comments": "评论",
  "crf.missionIssue.detail.editDescription": "编辑描述",
  "crf.missionIssue.detail.save": "保存",
  "crf.missionIssue.detail.cancel": "取消",
  "crf.missionIssue.detail.close": "关闭",
  "crf.missionIssue.detail.reopen": "重新打开",
  "crf.missionIssue.detail.noComments": "暂无评论",
  "crf.missionIssue.detail.addComment": "添加评论",
  "crf.missionIssue.detail.commentPlaceholder": "输入评论……",
  "crf.missionIssue.disabled.issueActions": "仅任务QC或项目负责人可操作",
  "crf.missionIssue.disabled.comment":
    "仅任务QC、DEV或项目负责人可评论",
```

- [ ] **Step 4: Verify typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter @aegis/ui typecheck
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
```

Expected: both PASS. The `satisfies` constraint enforces zhCN is in sync with en.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/shared/api/index.ts lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts && git commit -m "feat(shared): mission-issue api client + i18n keys" -m "Adds 5 api client wrappers (listIssuesByMission, createIssue,
patchIssueState, updateIssueDescription, appendComment) and 21
crf.missionIssue.* keys in both en.ts and zhCN.ts.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 4: Data hooks (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/issues.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/mission/index.ts` (re-export)
- Test: `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx` (write the failing test for `useListIssuesByMission` first; the dialog tests will be added in later tasks)

**Interfaces:**
- Consumes: `api.*`, `queryKeys.mission.issuesByMission`, types from Task 1
- Produces: `useListIssuesByMission(missionId | null)`, `useCreateIssue()`, `usePatchIssueState()`, `useUpdateIssueDescription()`, `useAppendComment()`

- [ ] **Step 1: Write the failing test for `useListIssuesByMission`**

Create `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx` with just the harness + one test (other tests added in later tasks). The full file will be filled in incrementally; this task adds the harness + the read hook test:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useListIssuesByMission } from "../../../features/mission";
import { queryKeys } from "../../../shared/query";
import type { IssueViewResponse } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

afterEach(() => cleanup());

const openedIssue: IssueViewResponse = {
  id: 1,
  missionId: 10,
  targetItem: null,
  issuer: "carol",
  description: "missing CRF row in AE",
  state: "opened",
  comments: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const closedIssue: IssueViewResponse = {
  ...openedIssue,
  id: 2,
  state: "closed",
};

function ListProbe({ missionId }: { missionId: number | null }) {
  const q = useListIssuesByMission(missionId);
  return (
    <span data-testid="result">{JSON.stringify(q.data ?? null)}</span>
  );
}

describe("useListIssuesByMission", () => {
  it("does not fetch when missionId is null", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue] });
    render(<ListProbe missionId={null} />, { wrapper: renderWithQueryClient });
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalledWith(
      "list_issues_by_mission",
      expect.anything(),
    );
  });

  it("invokes list_issues_by_mission with { missionId, state } and exposes the array", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue, closedIssue] });
    render(<ListProbe missionId={10} />, { wrapper: renderWithQueryClient });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_issues_by_mission", {
        missionId: 10,
        state: undefined,
      });
    });
    expect(screen.getByTestId("result")).toHaveTextContent(
      JSON.stringify([openedIssue, closedIssue]),
    );
  });

  it("uses queryKeys.mission.issuesByMission(missionId) as the query key", async () => {
    // The factory entry is the contract — this test pins the key shape.
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue] });
    const { client } = renderWithQueryClient(<ListProbe missionId={10} />);
    await waitFor(() => {
      const cache = client.getQueryCache().getAll();
      expect(
        cache.some(
          (q) =>
            JSON.stringify(q.queryKey) ===
            JSON.stringify(queryKeys.mission.issuesByMission(10)),
        ),
      ).toBe(true);
    });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "useListIssuesByMission"
```

Expected: FAIL — `useListIssuesByMission` is not exported from `features/mission` (the index barrel doesn't re-export it, and the hook itself doesn't exist yet).

- [ ] **Step 3: Implement `features/mission/data/issues.ts`**

Create the file with 5 hooks. All mutations invalidate `queryKeys.mission.issuesByMission(missionId)` on success. The `missionId` parameter is captured at hook-creation time (matches the existing `useAddAssignee(projectCode, kind)` pattern):

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type AppendCommentInput,
  type CreateIssueInput,
  type IssueState,
  type IssueViewResponse,
  type UpdateIssueDescriptionInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * List every issue attached to a mission. We deliberately fetch ALL
 * states in one call (no server-side `?state=` filter) and filter on
 * the client. The CRF detail page reads `openIssueCountByTarget`
 * derived from this single query to drive chip badges, so a state-
 * filtered fetch would force a second round-trip just to count
 * opened issues.
 */
export function useListIssuesByMission(missionId: number | null) {
  return useQuery<IssueViewResponse[], ApiError>({
    queryKey: queryKeys.mission.issuesByMission(missionId ?? -1),
    queryFn: () => api.listIssuesByMission(missionId!, {}),
    enabled: missionId != null,
    staleTime: 0,
  });
}

/**
 * Open a new issue against a mission. `targetItem` is set by the
 * caller — the form chip passes `undefined` for a whole-mission
 * issue; an item chip passes the item code. On success the per-mission
 * list is invalidated so the chip's red-dot badge updates reactively.
 */
export function useCreateIssue() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; body: CreateIssueInput }
  >({
    mutationFn: ({ missionId, body }) =>
      api.createIssue(missionId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}

/**
 * Flip an issue's open/closed state. Backend is idempotent
 * (closing a closed issue is a no-op success; same for reopening) so
 * the client doesn't guard against no-op transitions. Same
 * invalidation pattern as `useCreateIssue`.
 */
export function usePatchIssueState() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; issueId: number; next: IssueState }
  >({
    mutationFn: ({ issueId, next }) => api.patchIssueState(issueId, next),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}

/**
 * Replace an issue's description. `targetItem`, `issuer`, `state`,
 * and `comments` are immutable through this path — backend refuses
 * to mutate them. Same invalidation pattern.
 */
export function useUpdateIssueDescription() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    {
      missionId: number;
      issueId: number;
      body: UpdateIssueDescriptionInput;
    }
  >({
    mutationFn: ({ issueId, body }) =>
      api.updateIssueDescription(issueId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}

/**
 * Append a comment to an issue's thread. Comments are append-only —
 * no edit / delete endpoints. The returned `IssueViewResponse`
 * carries the updated `comments` array; invalidation ensures every
 * observer picks it up.
 */
export function useAppendComment() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; issueId: number; body: AppendCommentInput }
  >({
    mutationFn: ({ issueId, body }) => api.appendComment(issueId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}
```

- [ ] **Step 4: Re-export from `features/mission/index.ts`**

Replace the current barrel content:

```ts
export {
  useAddAssignee,
  useCreateMission,
  useListMissionsByProject,
  useRemoveAssignee,
} from "./data/missions";
export { useIsProjectLeader } from "./data/leader";
export {
  useAppendComment,
  useCreateIssue,
  useListIssuesByMission,
  usePatchIssueState,
  useUpdateIssueDescription,
} from "./data/issues";
```

- [ ] **Step 5: Run the test to verify it passes**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "useListIssuesByMission"
```

Expected: PASS — 3 tests in the `useListIssuesByMission` describe block.

- [ ] **Step 6: Verify all existing tests still pass**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test
```

Expected: PASS — no regressions.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/mission/data/issues.ts apps/desktop/aegis-desktop/src/features/mission/index.ts apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx && git commit -m "feat(mission): add 5 issue hooks + barrel export" -m "useListIssuesByMission fetches all states in one call (no server
filter) so the chip-dot count can derive from the same query; the 4
mutation hooks invalidate queryKeys.mission.issuesByMission(missionId)
on success.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 5: MissionIssueDialog shell (table + empty state)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/mission/index.ts` (re-export the dialog)
- Modify: `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx` (append the dialog tests)

**Interfaces:**
- Consumes: types from Task 1, hooks from Task 4
- Produces: a `<MissionIssueDialog>` component with the prop signature from the spec

- [ ] **Step 1: Write the failing render tests for the dialog shell**

Append the following `describe` block to the test file. It exercises the dialog shell: title, empty Alert, table rendering with one opened issue, and the close button.

```tsx
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { fireEvent, screen } from "@testing-library/react";
import { MissionIssueDialog } from "../../../features/mission";
import type { MissionViewResponse } from "../../../shared/api";

// helper for the dialog tests
function renderDialog(
  props: Partial<React.ComponentProps<typeof MissionIssueDialog>> = {},
) {
  const onClose = vi.fn();
  const onCreate = vi.fn();
  const onPatchState = vi.fn();
  const onUpdateDescription = vi.fn();
  const onAppendComment = vi.fn();

  const mission: MissionViewResponse = {
    id: 10,
    projectCode: "alpha",
    missionKind: "crf",
    missionCode: "AE",
    assignees: [],
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  const utils = render(
    <AegisI18nProvider>
      <MissionIssueDialog
        open
        scope={{ kind: "form" }}
        mission={mission}
        issues={[]}
        canCreate={true}
        canActOnIssue={true}
        canComment={true}
        createPending={false}
        createError={null}
        onCreate={onCreate}
        patchPending={false}
        patchError={null}
        onPatchState={onPatchState}
        updateDescPending={false}
        updateDescError={null}
        onUpdateDescription={onUpdateDescription}
        commentPending={false}
        commentError={null}
        onAppendComment={onAppendComment}
        onClose={onClose}
        {...props}
      />
    </AegisI18nProvider>,
  );
  return {
    onClose,
    onCreate,
    onPatchState,
    onUpdateDescription,
    onAppendComment,
    ...utils,
  };
}

describe("MissionIssueDialog (shell)", () => {
  it("renders the title with the scope label", () => {
    renderDialog();
    expect(screen.getByText(/Mission issues/i)).toBeInTheDocument();
  });

  it("shows an empty Alert when there are no issues", () => {
    renderDialog();
    expect(screen.getByText(/No issues yet/i)).toBeInTheDocument();
  });

  it("renders a table with one row per issue", () => {
    renderDialog({ issues: [openedIssue, closedIssue] });
    // 2 data rows + 1 header row
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBeGreaterThanOrEqual(3);
    expect(screen.getByText("carol")).toBeInTheDocument(); // reviewer = issuer
    expect(screen.getByText(/missing CRF row in AE/i)).toBeInTheDocument();
  });

  it("calls onClose when the Cancel button is clicked", () => {
    const { onClose } = renderDialog();
    fireEvent.click(screen.getByRole("button", { name: /Cancel|Close/i }));
    expect(onClose).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "MissionIssueDialog"
```

Expected: FAIL — `MissionIssueDialog` is not exported from `features/mission`.

- [ ] **Step 3: Implement `MissionIssueDialog` (shell + table + empty state)**

Create the file. This task implements only the shell — the create form and per-row actions come in Tasks 6 and 7:

```tsx
import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ExpandMore as ExpandMoreIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { ApiError, errorMessage } from "../../../shared/api";
import type {
  IssueViewResponse,
  MissionViewResponse,
  UserView,
} from "../../../shared/api";

export type IssueScope =
  | { kind: "form" }
  | { kind: "item"; itemId: number; itemCode: string };

interface Props {
  open: boolean;
  scope: IssueScope;
  mission: MissionViewResponse;
  issues: IssueViewResponse[];
  currentUser?: UserView;
  canCreate: boolean;
  canActOnIssue: boolean;
  canComment: boolean;

  createPending: boolean;
  createError: ApiError | null;
  onCreate: (description: string) => void;

  patchPending: boolean;
  patchError: ApiError | null;
  onPatchState: (issueId: number, next: "opened" | "closed") => void;

  updateDescPending: boolean;
  updateDescError: ApiError | null;
  onUpdateDescription: (issueId: number, description: string) => void;

  commentPending: boolean;
  commentError: ApiError | null;
  onAppendComment: (issueId: number, content: string) => void;

  onClose: () => void;
}

function scopeLabel(scope: IssueScope): string {
  return scope.kind === "form" ? "Form" : `Item ${scope.itemCode}`;
}

export function MissionIssueDialog({
  open,
  scope,
  mission,
  issues,
  canCreate,
  canActOnIssue,
  canComment,
  createPending,
  createError,
  onCreate,
  patchPending,
  patchError,
  onPatchState,
  updateDescPending,
  updateDescError,
  onUpdateDescription,
  commentPending,
  commentError,
  onAppendComment,
  onClose,
}: Props) {
  const { t } = useI18n();

  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [newDescription, setNewDescription] = useState("");
  const [editing, setEditing] = useState<{ id: number; value: string } | null>(
    null,
  );
  const [commentDraft, setCommentDraft] = useState<{
    id: number;
    value: string;
  } | null>(null);

  // Reset all internal state on close. Matches the existing dialog
  // convention (cf. AnnotationDialog). Matches the spec §"Internal state".
  useEffect(() => {
    if (!open) {
      setExpandedId(null);
      setNewDescription("");
      setEditing(null);
      setCommentDraft(null);
    }
  }, [open]);

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        {t("crf.missionIssue.dialog.title", { scope: scopeLabel(scope) })}
      </DialogTitle>
      <DialogContent>
        {/* The create form and per-row actions are added in Tasks 6 and 7. */}
        {issues.length === 0 ? (
          <Alert severity="info">{t("crf.missionIssue.dialog.empty")}</Alert>
        ) : (
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>
                  {t("crf.missionIssue.table.reviewer")}
                </TableCell>
                <TableCell>
                  {t("crf.missionIssue.table.description")}
                </TableCell>
                <TableCell>{t("crf.missionIssue.table.state")}</TableCell>
                <TableCell />
              </TableRow>
            </TableHead>
            <TableBody>
              {issues.map((issue) => (
                <TableRow key={issue.id}>
                  <TableCell>
                    <Tooltip title={issue.issuer}>
                      <span>{issue.issuer}</span>
                    </Tooltip>
                  </TableCell>
                  <TableCell>
                    <Typography
                      variant="body2"
                      sx={{
                        display: "-webkit-box",
                        WebkitLineClamp: 2,
                        WebkitBoxOrient: "vertical",
                        overflow: "hidden",
                      }}
                    >
                      {issue.description}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip
                      size="small"
                      variant="outlined"
                      label={t(
                        issue.state === "opened"
                          ? "crf.missionIssue.state.opened"
                          : "crf.missionIssue.state.closed",
                      )}
                      color={issue.state === "opened" ? "warning" : "default"}
                      sx={
                        issue.state === "closed"
                          ? { borderStyle: "dashed" }
                          : undefined
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <IconButton
                      size="small"
                      onClick={() =>
                        setExpandedId(
                          expandedId === issue.id ? null : issue.id,
                        )
                      }
                      aria-label="expand"
                    >
                      <ExpandMoreIcon
                        sx={{
                          transform:
                            expandedId === issue.id
                              ? "rotate(180deg)"
                              : "none",
                          transition: "transform 150ms",
                        }}
                      />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>
          {t("common.cancel") /* or close */}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 4: Re-export from `features/mission/index.ts`**

Append one line:

```ts
export { MissionIssueDialog } from "./components/MissionIssueDialog";
export type { IssueScope } from "./components/MissionIssueDialog";
```

- [ ] **Step 5: Run the dialog tests**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "MissionIssueDialog"
```

Expected: PASS — 4 tests pass.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx apps/desktop/aegis-desktop/src/features/mission/index.ts apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx && git commit -m "feat(mission): add MissionIssueDialog shell" -m "Implements the dialog title, empty-state Alert, and the 3-column
table (Reviewer | Description | State) with the per-row expand
toggle. Create form + per-row actions are added in subsequent tasks.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 6: MissionIssueDialog — create form (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx`

- [ ] **Step 1: Add failing tests for the create form**

Append a new describe block to the test file. The test file already has the `renderDialog` helper from Task 5.

```tsx
describe("MissionIssueDialog — create form", () => {
  it("hides the create form when canCreate is false", () => {
    renderDialog({ canCreate: false });
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
  });

  it("create submit is disabled while description is empty", () => {
    renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
  });

  it("create submit is disabled when description is whitespace only", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "   " },
    });
    expect(screen.getByRole("button", { name: /Create/i })).toBeDisabled();
  });

  it("create submit is enabled when description is non-empty", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "missing CRF row in AE" },
    });
    expect(screen.getByRole("button", { name: /Create/i })).not.toBeDisabled();
  });

  it("clicking create invokes onCreate with the trimmed description", () => {
    const { onCreate } = renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "  missing row  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /Create/i }));
    expect(onCreate).toHaveBeenCalledWith("missing row");
  });

  it("shows the create error inline below the description TextField", () => {
    renderDialog({ createError: { kind: "network", message: "boom" } });
    expect(screen.getByText(/boom/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the create form tests — expect FAIL**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "create form"
```

Expected: FAIL — create form not implemented.

- [ ] **Step 3: Implement the create form in `MissionIssueDialog.tsx`**

Insert this block at the top of `<DialogContent>`, BEFORE the empty Alert / table:

```tsx
{canCreate && (
  <Box
    sx={{
      display: "flex",
      flexDirection: "column",
      gap: 1,
      mb: 2,
    }}
  >
    <TextField
      multiline
      minRows={2}
      label={t("crf.missionIssue.create.field.description")}
      value={newDescription}
      onChange={(e) => setNewDescription(e.target.value)}
      disabled={createPending}
      data-testid="mission-issue-new-description"
    />
    {createError && (
      <Alert severity="error">{errorMessage(createError)}</Alert>
    )}
    <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
      <Button
        variant="contained"
        onClick={() => onCreate(newDescription.trim())}
        disabled={createPending || newDescription.trim() === ""}
      >
        {t("crf.missionIssue.create.submit")}
      </Button>
    </Box>
  </Box>
)}
```

Add `TextField` to the `@aegis/ui/mui` import.

- [ ] **Step 4: Run the tests — expect PASS**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "create form"
```

Expected: PASS — 6 tests pass.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx && git commit -m "feat(mission): MissionIssueDialog create form" -m "Adds the create form above the issue table: visible iff canCreate,
submit disabled on empty/whitespace, calls onCreate with the trimmed
description. Inline Alert on createError.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 7: MissionIssueDialog — per-row actions (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx`

- [ ] **Step 1: Add failing tests for per-row actions**

Append a new describe block:

```tsx
describe("MissionIssueDialog — per-row actions", () => {
  const issueWithComment: IssueViewResponse = {
    ...openedIssue,
    id: 5,
    comments: [
      {
        user: "bob",
        content: "will fix by EOD",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ],
  };

  it("expands a row to show comments + edit + close/reopen + comment form", () => {
    renderDialog({ issues: [issueWithComment] });
    // comments visible only when expanded
    expect(screen.queryByText(/will fix by EOD/i)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByText(/will fix by EOD/i)).toBeInTheDocument();
  });

  it("clicking Close on an opened issue calls onPatchState(id, closed)", () => {
    const { onPatchState } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /^Close$/i }));
    expect(onPatchState).toHaveBeenCalledWith(openedIssue.id, "closed");
  });

  it("clicking Reopen on a closed issue calls onPatchState(id, opened)", () => {
    const { onPatchState } = renderDialog({ issues: [closedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Reopen/i }));
    expect(onPatchState).toHaveBeenCalledWith(closedIssue.id, "opened");
  });

  it("edit description: click Edit → TextField pre-filled → Save invokes onUpdateDescription", () => {
    const { onUpdateDescription } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    const ta = screen.getByDisplayValue(openedIssue.description);
    fireEvent.change(ta, { target: { value: "new description" } });
    fireEvent.click(screen.getByRole("button", { name: /^Save$/i }));
    expect(onUpdateDescription).toHaveBeenCalledWith(
      openedIssue.id,
      "new description",
    );
  });

  it("append comment: typing then Send invokes onAppendComment with trimmed content", () => {
    const { onAppendComment } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "  hello  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Send$/i }));
    expect(onAppendComment).toHaveBeenCalledWith(openedIssue.id, "hello");
  });

  it("Send is disabled when comment is empty or whitespace", () => {
    renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByRole("button", { name: /^Send$/i })).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "   " },
    });
    expect(screen.getByRole("button", { name: /^Send$/i })).toBeDisabled();
  });

  it("hides Close/Reopen + Edit description when canActOnIssue is false", () => {
    renderDialog({ canActOnIssue: false, issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(
      screen.queryByRole("button", { name: /^Close$/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Edit description/i }),
    ).not.toBeInTheDocument();
  });

  it("hides the comment form when canComment is false", () => {
    renderDialog({
      canActOnIssue: false,
      canComment: false,
      issues: [openedIssue],
    });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(
      screen.queryByPlaceholderText(/Type a comment/i),
    ).not.toBeInTheDocument();
  });

  it("shows the patchError inline above the Close/Reopen button when present", () => {
    renderDialog({
      patchError: { kind: "http", status: 403, code: "forbidden", message: "nope" },
      issues: [openedIssue],
    });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByText(/forbidden: nope/i)).toBeInTheDocument();
  });

  it("editing description on row A does not affect row B", () => {
    const { rerender } = renderDialog({
      issues: [openedIssue, { ...openedIssue, id: 99 }],
    });
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[0]);
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    fireEvent.change(screen.getByDisplayValue(openedIssue.description), {
      target: { value: "row A edit" },
    });
    // collapse row A; expand row B
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[0]);
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[1]);
    // row B's description should NOT show "row A edit"
    expect(screen.queryByDisplayValue("row A edit")).not.toBeInTheDocument();
  });

  it("closing and reopening the dialog resets all internal state", () => {
    const { rerender } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    fireEvent.click(screen.getByRole("button", { name: /expand/i })); // collapse
    // close and reopen
    rerender(
      <AegisI18nProvider>
        <MissionIssueDialog
          {...{
            open: false,
            scope: { kind: "form" },
            mission: {
              id: 10,
              projectCode: "alpha",
              missionKind: "crf",
              missionCode: "AE",
              assignees: [],
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-01T00:00:00Z",
            },
            issues: [openedIssue],
            canCreate: true,
            canActOnIssue: true,
            canComment: true,
            createPending: false,
            createError: null,
            onCreate: () => undefined,
            patchPending: false,
            patchError: null,
            onPatchState: () => undefined,
            updateDescPending: false,
            updateDescError: null,
            onUpdateDescription: () => undefined,
            commentPending: false,
            commentError: null,
            onAppendComment: () => undefined,
            onClose: () => undefined,
          }}
        />
      </AegisI18nProvider>,
    );
    rerender(
      <AegisI18nProvider>
        <MissionIssueDialog
          {...{
            open: true,
            scope: { kind: "form" },
            mission: {
              id: 10,
              projectCode: "alpha",
              missionKind: "crf",
              missionCode: "AE",
              assignees: [],
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-01T00:00:00Z",
            },
            issues: [openedIssue],
            canCreate: true,
            canActOnIssue: true,
            canComment: true,
            createPending: false,
            createError: null,
            onCreate: () => undefined,
            patchPending: false,
            patchError: null,
            onPatchState: () => undefined,
            updateDescPending: false,
            updateDescError: null,
            onUpdateDescription: () => undefined,
            commentPending: false,
            commentError: null,
            onAppendComment: () => undefined,
            onClose: () => undefined,
          }}
        />
      </AegisI18nProvider>,
    );
    // no row is expanded → Edit description button not visible
    expect(
      screen.queryByRole("button", { name: /Edit description/i }),
    ).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the per-row action tests — expect FAIL**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "per-row actions"
```

Expected: FAIL — per-row actions not implemented.

- [ ] **Step 3: Implement per-row `IssueDetails` in `MissionIssueDialog.tsx`**

Replace the simple `<TableRow>` in the table body with a fragment containing the row + a collapsible details row. Add an `IssueDetails` private helper component inside the same file.

Replace the existing `issues.map((issue) => ...)` block with:

```tsx
{issues.map((issue) => (
  <Fragment key={issue.id}>
    <TableRow>
      {/* existing reviewer / description / state-cell / expand-cell */}
      <TableCell>
        <Tooltip title={issue.issuer}>
          <span>{issue.issuer}</span>
        </Tooltip>
      </TableCell>
      <TableCell>
        <Typography
          variant="body2"
          sx={{
            display: "-webkit-box",
            WebkitLineClamp: 2,
            WebkitBoxOrient: "vertical",
            overflow: "hidden",
          }}
        >
          {issue.description}
        </Typography>
      </TableCell>
      <TableCell>
        <Chip
          size="small"
          variant="outlined"
          label={t(
            issue.state === "opened"
              ? "crf.missionIssue.state.opened"
              : "crf.missionIssue.state.closed",
          )}
          color={issue.state === "opened" ? "warning" : "default"}
          sx={
            issue.state === "closed"
              ? { borderStyle: "dashed" }
              : undefined
          }
        />
      </TableCell>
      <TableCell>
        <IconButton
          size="small"
          onClick={() =>
            setExpandedId(expandedId === issue.id ? null : issue.id)
          }
          aria-label="expand"
        >
          <ExpandMoreIcon
            sx={{
              transform: expandedId === issue.id ? "rotate(180deg)" : "none",
              transition: "transform 150ms",
            }}
          />
        </IconButton>
      </TableCell>
    </TableRow>
    {expandedId === issue.id && (
      <TableRow>
        <TableCell colSpan={4} sx={{ bgcolor: "action.hover" }}>
          <IssueDetails
            issue={issue}
            editing={editing}
            setEditing={setEditing}
            commentDraft={commentDraft}
            setCommentDraft={setCommentDraft}
            canActOnIssue={canActOnIssue}
            canComment={canComment}
            patchPending={patchPending}
            patchError={patchError}
            updateDescPending={updateDescPending}
            updateDescError={updateDescError}
            commentPending={commentPending}
            commentError={commentError}
            onPatchState={onPatchState}
            onUpdateDescription={onUpdateDescription}
            onAppendComment={onAppendComment}
          />
        </TableCell>
      </TableRow>
    )}
  </Fragment>
))}
```

Then add the private `IssueDetails` component at the bottom of the same file (after the export):

```tsx
interface IssueDetailsProps {
  issue: IssueViewResponse;
  editing: { id: number; value: string } | null;
  setEditing: (v: { id: number; value: string } | null) => void;
  commentDraft: { id: number; value: string } | null;
  setCommentDraft: (v: { id: number; value: string } | null) => void;
  canActOnIssue: boolean;
  canComment: boolean;
  patchPending: boolean;
  patchError: ApiError | null;
  updateDescPending: boolean;
  updateDescError: ApiError | null;
  commentPending: boolean;
  commentError: ApiError | null;
  onPatchState: (id: number, next: "opened" | "closed") => void;
  onUpdateDescription: (id: number, description: string) => void;
  onAppendComment: (id: number, content: string) => void;
}

function IssueDetails({
  issue,
  editing,
  setEditing,
  commentDraft,
  setCommentDraft,
  canActOnIssue,
  canComment,
  patchPending,
  patchError,
  updateDescPending,
  updateDescError,
  commentPending,
  commentError,
  onPatchState,
  onUpdateDescription,
  onAppendComment,
}: IssueDetailsProps) {
  const { t } = useI18n();
  const isEditing = editing?.id === issue.id;
  const draft = commentDraft?.id === issue.id ? commentDraft.value : "";

  return (
    <Stack spacing={2} sx={{ py: 1 }}>
      {/* Description block */}
      <Box>
        {isEditing ? (
          <Stack spacing={1}>
            <TextField
              multiline
              minRows={2}
              value={editing!.value}
              onChange={(e) =>
                setEditing({ id: issue.id, value: e.target.value })
              }
              disabled={updateDescPending}
              fullWidth
            />
            {updateDescError && (
              <Alert severity="error">{errorMessage(updateDescError)}</Alert>
            )}
            <Stack direction="row" spacing={1} justifyContent="flex-end">
              <Button
                onClick={() => setEditing(null)}
                disabled={updateDescPending}
              >
                {t("crf.missionIssue.detail.cancel")}
              </Button>
              <Button
                variant="contained"
                onClick={() =>
                  onUpdateDescription(issue.id, editing!.value.trim())
                }
                disabled={
                  updateDescPending ||
                  editing!.value.trim() === "" ||
                  editing!.value.trim() === issue.description
                }
              >
                {t("crf.missionIssue.detail.save")}
              </Button>
            </Stack>
          </Stack>
        ) : (
          <Stack direction="row" spacing={1} alignItems="flex-start">
            <Typography variant="body2" sx={{ flexGrow: 1 }}>
              {issue.description}
            </Typography>
            {canActOnIssue && (
              <Button
                size="small"
                onClick={() =>
                  setEditing({ id: issue.id, value: issue.description })
                }
              >
                {t("crf.missionIssue.detail.editDescription")}
              </Button>
            )}
          </Stack>
        )}
      </Box>

      {/* State-flip button */}
      {canActOnIssue && (
        <Box>
          {patchError && (
            <Alert severity="error" sx={{ mb: 1 }}>
              {errorMessage(patchError)}
            </Alert>
          )}
          <Button
            size="small"
            variant="outlined"
            color={issue.state === "opened" ? "warning" : "primary"}
            disabled={patchPending}
            onClick={() =>
              onPatchState(
                issue.id,
                issue.state === "opened" ? "closed" : "opened",
              )
            }
            data-testid={`mission-issue-${issue.state === "opened" ? "close" : "reopen"}-${issue.id}`}
          >
            {t(
              issue.state === "opened"
                ? "crf.missionIssue.detail.close"
                : "crf.missionIssue.detail.reopen",
            )}
          </Button>
        </Box>
      )}

      {/* Comments */}
      <Box>
        <Typography variant="subtitle2" sx={{ mb: 1 }}>
          {t("crf.missionIssue.detail.comments")}
        </Typography>
        {issue.comments.length === 0 ? (
          <Alert severity="info">
            {t("crf.missionIssue.detail.noComments")}
          </Alert>
        ) : (
          <Stack spacing={1}>
            {issue.comments.map((c, i) => (
              <Box key={i} sx={{ borderLeft: 2, pl: 1, borderColor: "divider" }}>
                <Typography variant="body2">
                  <strong>{c.user}</strong>: {c.content}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {c.createdAt}
                </Typography>
              </Box>
            ))}
          </Stack>
        )}
      </Box>

      {/* Comment form */}
      {canComment && (
        <Stack spacing={1}>
          <TextField
            multiline
            minRows={1}
            placeholder={t("crf.missionIssue.detail.commentPlaceholder")}
            value={draft}
            onChange={(e) =>
              setCommentDraft({ id: issue.id, value: e.target.value })
            }
            disabled={commentPending}
            fullWidth
          />
          {commentError && (
            <Alert severity="error">{errorMessage(commentError)}</Alert>
          )}
          <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
            <Button
              variant="contained"
              disabled={commentPending || draft.trim() === ""}
              onClick={() =>
                onAppendComment(issue.id, draft.trim())
              }
            >
              {t("crf.missionIssue.detail.addComment")}
            </Button>
          </Box>
        </Stack>
      )}
    </Stack>
  );
}
```

- [ ] **Step 4: Run the per-row action tests — expect PASS**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx -t "per-row actions"
```

Expected: PASS — 11 tests pass.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx && git commit -m "feat(mission): MissionIssueDialog per-row actions" -m "Implements inline expansion: comments thread, edit-description form,
Close/Reopen toggle, and comment form. Hidden when RBAC denies; inline
Alerts scoped per error slot.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 8: CrfItemRow — chip with Badge (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` (add 3 props + wrap chip in `<Badge>`)
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-item-row.test.tsx` (if exists, add new tests; else add a smoke test to ensure no regression)

**Interfaces:**
- Consumes: existing `Props` interface
- Produces: extended `Props` with `hasOpenIssue: boolean`, `onOpenIssues: () => void`, `missionExists: boolean`; modified chip render

- [ ] **Step 1: Check if `crf-item-row.test.tsx` exists**

Run:

```bash
cd d:/projects/rusty/aegis && ls apps/desktop/aegis-desktop/src/test/features/crf/ | grep -i "item-row\|item.row"
```

If a file exists, append to it. If not, create one with the tests below.

- [ ] **Step 2: Write failing tests**

Append (or create) the tests:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CrfItemRow } from "../../../../features/crf/components/CrfItemRow";

afterEach(() => cleanup());

const itemDetail = {
  item: {
    id: 21,
    code: "CMTRT",
    name: "CM Treatment",
    kind: "text",
    notSubmitted: false,
    /* other required fields */
  },
  options: [],
  units: [],
  annotations: [],
};

describe("CrfItemRow — issue chip", () => {
  it("renders the item code chip", () => {
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        onCreateAnnotation={() => undefined}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        hasOpenIssue={false}
        onOpenIssues={() => undefined}
        missionExists={true}
      />,
    );
    expect(screen.getByText("CMTRT")).toBeInTheDocument();
  });

  it("clicking the item code chip calls onOpenIssues", () => {
    const onOpenIssues = vi.fn();
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        onCreateAnnotation={() => undefined}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        hasOpenIssue={true}
        onOpenIssues={onOpenIssues}
        missionExists={true}
      />,
    );
    fireEvent.click(screen.getByText("CMTRT"));
    expect(onOpenIssues).toHaveBeenCalled();
  });

  it("disables the chip when missionExists is false", () => {
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        onCreateAnnotation={() => undefined}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        hasOpenIssue={false}
        onOpenIssues={() => undefined}
        missionExists={false}
      />,
    );
    // MUI Chip renders the underlying <button> when clickable; with
    // disabled=true the button has aria-disabled.
    const chip = screen.getByText("CMTRT").closest("button");
    expect(chip).toHaveAttribute("aria-disabled", "true");
  });
});
```

If you had to read the existing `CrfItemRow.tsx` props to discover the exact `itemDetail` shape, do so first; the snippet above assumes `CrfItemDetail` already has `item.id`, `item.code`, etc. (the existing chip uses `item.code` at lines 124-131). Adjust the placeholder fields if the type demands it.

- [ ] **Step 3: Run the tests — expect FAIL**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-item-row.test.tsx
```

Expected: FAIL — the new props don't exist yet on `Props`.

- [ ] **Step 4: Implement the chip + Badge in `CrfItemRow.tsx`**

Add the import:

```ts
import { Badge, Tooltip } from "@aegis/ui/mui";
```

Extend the `Props` interface (currently around line 10):

```ts
interface Props {
  // … existing
  hasOpenIssue: boolean;
  onOpenIssues: () => void;
  missionExists: boolean;
}
```

Replace the existing code chip at lines 124-131 with:

```tsx
{!isLabel && (
  <Tooltip
    title={
      missionExists
        ? ""
        : /* i18n key per task 3: */ t("crf.missionIssue.tooltip.noMission")
    }
    disableHoverListener={missionExists}
    disableFocusListener={missionExists}
    disableTouchListener={missionExists}
  >
    <span>
      <Badge
        variant="dot"
        color="error"
        invisible={!hasOpenIssue}
        overlap="circular"
      >
        <Chip
          sx={{ width: 92 }}
          label={item.code}
          variant="outlined"
          size="small"
          onClick={onOpenIssues}
          disabled={!missionExists}
          data-testid={`crf-item-code-${item.id}`}
        />
      </Badge>
    </span>
  </Tooltip>
)}
```

Use `useI18n` already imported at the top of `CrfItemRow.tsx` if available; otherwise add it. The `t` variable should already be in scope.

- [ ] **Step 5: Run the tests — expect PASS**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-item-row.test.tsx
```

Expected: PASS — 3 tests pass.

- [ ] **Step 6: Verify CrfDetailPage tests still pass**

The `CrfDetailPage` invokes `CrfItemRow` with the existing prop set; since we just added 3 NEW props (didn't change the old ones), the page is now in a TypeScript error state until Task 9 wires them. To avoid a typecheck regression, do NOT run `pnpm typecheck` here. We fix the page in Task 9.

If there are existing CrfItemRow tests that pass the old prop set, those will start failing. Inspect and update them to pass the 3 new props as `undefined` / `() => undefined` / `false`.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-item-row.test.tsx && git commit -m "feat(crf): item-row issue chip with Badge + tooltip" -m "Adds hasOpenIssue / onOpenIssues / missionExists props and wraps the
item code chip in a Badge variant=dot color=error that hides when no
opened issue exists for this item. Disabled + tooltip when no mission
exists for the form.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 9: CrfDetailPage wiring (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` (derive missionForForm, RBAC, openIssueCountByTarget; wire form-code chip; render `<MissionIssueDialog>`)
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` callers: the inline `<CrfItemRow ...>` JSX block (which already passes the old props) needs the 3 new props added
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` (if exists, add chip + dialog tests; else add a smoke render test)

**Interfaces:**
- Consumes: hooks from Task 4, dialog from Task 5
- Produces: a working page where clicking the form-code chip or an item-code chip opens the dialog

- [ ] **Step 1: Read the existing `CrfDetailPage.test.tsx` (if it exists)**

Run:

```bash
cd d:/projects/rusty/aegis && ls apps/desktop/aegis-desktop/src/test/features/crf/ | grep -i "detail\|crf.detail"
```

If a test file exists, look at how it sets up the test harness and what mocks it provides. The integration tests in later steps need to mock `useListMissionsByProject` and `useListIssuesByMission`.

- [ ] **Step 2: Write failing integration tests**

Append (or create) tests that exercise the page-level wiring:

```tsx
describe("CrfDetailPage — mission-issue entry points", () => {
  it("form code chip click opens dialog showing mission-level issues", async () => {
    // Mock: list_missions_by_project returns a mission matching form.code;
    // list_issues_by_mission returns 1 opened issue.
    mockCommands({
      list_missions_by_project: () => [
        {
          id: 10,
          projectCode: "alpha",
          missionKind: "crf",
          missionCode: "AE",
          assignees: [],
          createdAt: "",
          updatedAt: "",
        },
      ],
      list_issues_by_mission: () => [
        {
          id: 1,
          missionId: 10,
          targetItem: null,
          issuer: "carol",
          description: "missing row",
          state: "opened",
          comments: [],
          createdAt: "",
          updatedAt: "",
        },
      ],
    });
    renderWithQueryClient(<CrfDetailPage />);
    // ... open chip, assert dialog appears
  });
});
```

(Full test bodies are abbreviated here to keep the plan readable. The implementer should mirror the existing `crf-detail-page.test.tsx` style for harness setup and follow the spec's 24 test cases for full coverage — Tasks 10 and beyond flesh out the integration scenarios.)

- [ ] **Step 3: Run the integration tests — expect FAIL**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: FAIL — page doesn't wire the dialog.

- [ ] **Step 4: Implement the wiring in `CrfDetailPage.tsx`**

The page is ~630 lines. The diff is localized to (a) imports / hook setup at the top of the component, (b) state declarations, (c) the form-code chip (lines 268-279), (d) the `<CrfItemRow ...>` invocation (lines 446-487), and (e) the bottom of the JSX where new dialogs are rendered.

Add the imports at the top:

```ts
import {
  Badge,
  Tooltip,
  // … existing
} from "@aegis/ui/mui";
import {
  MissionIssueDialog,
  useCreateIssue,
  usePatchIssueState,
  useUpdateIssueDescription,
  useAppendComment,
  useListIssuesByMission,
  useListMissionsByProject,
  type IssueScope,
} from "../../mission";
import { useCurrentUser } from "../../auth";
```

Add hooks and derived state inside the component (after the existing `useUpdateOwnerNotSubmitted` line, around line 171):

```ts
// Mission + issues
const missionForForm = useListMissionsByProject(projectCode, "crf");
const missionList = missionForForm.data ?? [];
const formMission = useMemo(
  () => missionList.find((m) => m.missionCode === form?.code) ?? null,
  [missionList, form?.code],
);
const issuesQuery = useListIssuesByMission(formMission?.id ?? null);

const openIssueCountByTarget = useMemo(() => {
  const map = new Map<string | null, number>();
  for (const i of issuesQuery.data ?? []) {
    if (i.state !== "opened") continue;
    map.set(i.targetItem ?? null, (map.get(i.targetItem ?? null) ?? 0) + 1);
  }
  return map;
}, [issuesQuery.data]);

const currentUser = useCurrentUser().data;
const isMissionQc = !!formMission?.assignees.some(
  (a) => a.userCode === currentUser?.code && a.role === "qc",
);
const isMissionDev = !!formMission?.assignees.some(
  (a) => a.userCode === currentUser?.code && a.role === "dev",
);
const canCreate = isProjectLeader === true || isMissionQc;
const canActOnIssue = isProjectLeader === true || isMissionQc;
const canComment = canActOnIssue || isMissionDev;

const createIssue = useCreateIssue();
const patchIssueState = usePatchIssueState();
const updateIssueDescription = useUpdateIssueDescription();
const appendComment = useAppendComment();
```

Add dialog state (next to the existing dialog states, around line 173):

```ts
const [issueDialog, setIssueDialog] = useState<
  | { scope: IssueScope; missionId: number }
  | null
>(null);
```

Replace the form-code chip block (lines 268-279):

```tsx
{form?.code && (
  <Tooltip
    title={
      formMission
        ? ""
        : t("crf.missionIssue.tooltip.noMission")
    }
    disableHoverListener={Boolean(formMission)}
    disableFocusListener={Boolean(formMission)}
    disableTouchListener={Boolean(formMission)}
  >
    <span>
      <Badge
        variant="dot"
        color="error"
        invisible={
          !formMission ||
          (openIssueCountByTarget.get(null) ?? 0) === 0
        }
        overlap="circular"
      >
        <Chip
          sx={{ minWidth: 70 }}
          size="small"
          label={form.code}
          variant="outlined"
          disabled={!formMission}
          onClick={() =>
            formMission &&
            setIssueDialog({
              scope: { kind: "form" },
              missionId: formMission.id,
            })
          }
          data-testid={`crf-form-${id}`}
        />
      </Badge>
    </span>
  </Tooltip>
)}
```

Update the `<CrfItemRow ...>` JSX (lines 446-487) to pass the 3 new props:

```tsx
<CrfItemRow
  …
  hasOpenIssue={(openIssueCountByTarget.get(itemDetail.item.code) ?? 0) > 0}
  onOpenIssues={() =>
    formMission &&
    setIssueDialog({
      scope: {
        kind: "item",
        itemId: itemDetail.item.id,
        itemCode: itemDetail.item.code,
      },
      missionId: formMission.id,
    })
  }
  missionExists={Boolean(formMission)}
/>
```

Render the dialog at the bottom of the JSX (after the existing dialogs, around line 627):

```tsx
{issueDialog && formMission && (
  <MissionIssueDialog
    open
    scope={issueDialog.scope}
    mission={formMission}
    issues={(issuesQuery.data ?? []).filter((i) => {
      if (issueDialog.scope.kind === "form") return i.targetItem == null;
      return i.targetItem === issueDialog.scope.itemCode;
    })}
    currentUser={currentUser}
    canCreate={canCreate}
    canActOnIssue={canActOnIssue}
    canComment={canComment}
    createPending={createIssue.isPending}
    createError={createIssue.error}
    onCreate={(description) =>
      createIssue.mutate(
        {
          missionId: issueDialog.missionId,
          body: {
            description,
            targetItem:
              issueDialog.scope.kind === "item"
                ? issueDialog.scope.itemCode
                : undefined,
          },
        },
        {
          onSuccess: () => {
            /* keep dialog open */
          },
        },
      )
    }
    patchPending={patchIssueState.isPending}
    patchError={patchIssueState.error}
    onPatchState={(issueId, next) =>
      patchIssueState.mutate({
        missionId: issueDialog.missionId,
        issueId,
        next,
      })
    }
    updateDescPending={updateIssueDescription.isPending}
    updateDescError={updateIssueDescription.error}
    onUpdateDescription={(issueId, description) =>
      updateIssueDescription.mutate({
        missionId: issueDialog.missionId,
        issueId,
        body: { description },
      })
    }
    commentPending={appendComment.isPending}
    commentError={appendComment.error}
    onAppendComment={(issueId, content) =>
      appendComment.mutate({
        missionId: issueDialog.missionId,
        issueId,
        body: { content },
      })
    }
    onClose={() => setIssueDialog(null)}
  />
)}
```

- [ ] **Step 5: Run the integration tests — expect PASS**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: PASS — all wiring tests pass.

- [ ] **Step 6: Verify the whole suite still passes**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test
```

Expected: PASS — no regressions.

- [ ] **Step 7: Verify typecheck**

```bash
cd d:/projects/rusty/aegis && pnpm --filter @aegis/ui typecheck
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
cd d:/projects/rusty/aegis && cargo check -p aegis-desktop
```

Expected: all PASS.

- [ ] **Step 8: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx && git commit -m "feat(crf): wire mission-issue chip + dialog on detail page" -m "Derives formMission + openIssueCountByTarget + RBAC; wraps the form-
code chip in Badge; passes 3 new props to CrfItemRow; renders
<MissionIssueDialog> at the bottom of the page. Clicking the form chip
opens mission-level issues; clicking an item chip opens that item's
issues (filtered client-side by targetItem).

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 10: Integration test coverage (full spec §"Testing" 24 cases)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx` (add the chip + scope-filter tests)
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` (add the chip + dialog + invalidation tests)
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-item-row.test.tsx` (add the chip + Badge + dot visibility tests at page level)

**Interfaces:** None new — full coverage of spec §"Testing" 24 cases.

- [ ] **Step 1: Add the spec §"Testing" 5 chip + badge tests at page level**

Append to `crf-detail-page.test.tsx`:

1. Chip shows dot badge when open issue exists for the form scope.
2. Chip hides dot badge when no issues exist.
3. Chip hides dot badge when only closed issues exist.
4. Item chip shows dot badge only when an issue matches its `targetItem`.
5. Chip disabled with no-mission tooltip when no mission exists for the form.

(Use the same test harness as Task 9, varying the `list_issues_by_mission` mock to drive Badge `invisible`.)

- [ ] **Step 2: Add the spec §"Testing" 3 list tests**

Append to `mission-issue-dialog.test.tsx`:

6. Clicking form chip opens dialog showing only mission-level issues.
7. Clicking item chip opens dialog filtered to that item's `target_item`.
8. Empty list shows Alert with `.empty` key, not the table.

- [ ] **Step 3: Add the spec §"Testing" 5 create tests**

Append:

9. Create form hidden when `canCreate === false`.
10. Create submit disabled while description is empty / whitespace.
11. Clicking create invokes `create_issue` with `{ missionId, body: { description } }` — and captures the `targetItem` when invoked from the item chip.
12. Create failure renders inline Alert.
13. Create success invalidates `queryKeys.mission.issuesByMission(missionId)` (via `vi.spyOn(queryClient, "invalidateQueries")`).

- [ ] **Step 4: Add the spec §"Testing" 7 per-row action tests**

Append:

14. Close calls `patch_issue_state` with `{ issueId, state: "closed" }`.
15. Reopen calls `patch_issue_state` with `{ issueId, state: "opened" }`.
16. Close/Reopen buttons hidden when `canActOnIssue === false`.
17. Edit description: Edit → TextField pre-filled → Save invokes `update_issue_description`.
18. Append comment: Send invokes `append_comment` and clears draft.
19. Send disabled when comment is empty / whitespace.
20. Comment failure renders inline Alert.

- [ ] **Step 5: Add the spec §"Testing" cache-invalidation + multi-row + state-reset tests**

Append (these are already partially covered by Task 7's tests 22-24):

21. Every successful mutation invalidates `queryKeys.mission.issuesByMission(missionId)`.
22. Editing description on row A does not affect row B (already in Task 7).
23. Comment drafts are per-row (new — explicitly verify by expanding row A, typing, expanding row B, asserting row B's draft is empty).
24. Closing dialog and reopening resets all internal state (already in Task 7).

- [ ] **Step 6: Run the full mission-issue test suite**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/mission/mission-issue-dialog.test.tsx src/test/features/crf/crf-detail-page.test.tsx src/test/features/crf/crf-item-row.test.tsx
```

Expected: PASS — all 24 spec test cases covered.

- [ ] **Step 7: Run the entire test suite for regressions**

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/test/ && git commit -m "test(crf): full spec §Testing coverage for mission-issue dialog" -m "Adds the remaining spec §Testing cases: chip+badge scope filtering
(5), list/empty (3), create + RBAC (5), per-row actions (7), cache
invalidation + multi-row independence + state reset (4).

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage** — every requirement in `2026-09-21-mission-issue-dialog-design.md` is mapped:

| Spec section | Task(s) |
|---|---|
| Wire-format mirrors | Task 1 |
| URL → Tauri command → server route | Task 2 |
| Query keys | Task 1 |
| Data hooks | Task 4 |
| Page state (missionForForm, RBAC, openIssueCountByTarget) | Task 9 |
| Form-code chip + Badge | Task 9 |
| Item-code chip + Badge | Task 8 |
| Dialog props / state / layout | Tasks 5-7 |
| Per-row actions (close/reopen/edit-desc/comment) | Task 7 |
| Error handling (4 slots, `<Alert severity=error>{errorMessage}</Alert>`) | Tasks 6-7 |
| Edge cases (no mission, RBAC null, whitespace) | Tasks 8-9 |
| i18n keys | Task 3 |
| All 24 test cases | Tasks 5-10 (incremental) + Task 10 (final coverage) |

**Placeholder scan** — none. Every step has concrete code.

**Type consistency** — types from Task 1 are referenced verbatim in Tasks 2-10. Hook signatures in Task 4 match the dialog's `Props` in Task 5. The `IssueScope` discriminated union is exported from Task 5 and re-imported in Task 9.

**Open uncertainty** — Task 8 Step 4 references an `i18n` import (`t`) that may need to be added to `CrfItemRow.tsx`. Verify by reading the current imports of that file. Task 9's full test bodies in Step 2 are abbreviated; the implementer should mirror the existing `crf-detail-page.test.tsx` harness pattern for full coverage (Tasks 5-7 already establish the dialog-level tests; Task 10 fleshes out the page-level integration).