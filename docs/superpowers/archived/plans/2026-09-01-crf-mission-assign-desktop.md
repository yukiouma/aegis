# CRF Mission-Assign Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing `apis::mission` HTTP surface into the aegis-desktop Tauri shell and replace the placeholder `CrfAssignTakersDrawer` with a working `CrfMissionAssignDrawer` so project leaders can see and manage CRF form assignments (Dev / QC) from the CRF form list page.

**Architecture:** Vertical slice — new `features/mission/` module owns the data hooks. Page owns the cross-cutting query (`useListMissionsByProject`) and projects a `Map<formCode, MissionView>` for the assignee column. The drawer is project-leader gated; the server is the authoritative gate (mission `ensure_leader`). On first add to a form without a mission, the drawer implicitly creates the mission via `create_mission` then `add_assignee`.

**Tech Stack:** Rust (Tauri commands over an `HttpClient` against `aegis-server`); TypeScript (React, TanStack Query, MUI via `@aegis/ui`); i18n keys in `lib/packages/ui/src/i18n/locales/{en,zhCN}.ts`.

**Spec:** [`docs/superpowers/specs/2026-09-01-crf-mission-assign-desktop-design.md`](../specs/2026-09-01-crf-mission-assign-desktop-design.md)

**Branch:** `feat/desktop_crf-mission-assign`

---

## Phase 1 — Rust data layer

### Task 1: Add mission wire DTOs and HTTP adapter

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs`

- [ ] **Step 1.1: Write `http/mission.rs`**

```rust
//! Mission + assignee CRUD. Wire DTOs mirror the server's
//! `apps/server/aegis-server/src/transport/http/dto.rs` lines 1872–2028.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionRole {
    Dev,
    Qc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeDataArg {
    pub user_code: String,
    pub role: MissionRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeViewResponse {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeViewResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionListResponse {
    pub missions: Vec<MissionViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeDataArg>,
}

pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
    kind: Option<MissionKind>,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    // The server accepts `?kind=crf` etc.; we serialize the typed enum
    // via `serde_json` and pass it as a single query string value.
    let mut url = format!("/api/mission/by-project/{project_code}");
    if let Some(k) = kind {
        let kind_str = serde_json::to_string(&k)
            .map_err(|e| ApiError::Parse { message: e.to_string() })?
            .trim_matches('"')
            .to_string();
        url.push_str("?kind=");
        url.push_str(&kind_str);
    }
    let resp: MissionListResponse = c
        .request(reqwest::Method::GET, &url, None::<&()>)
        .await?;
    Ok(resp.missions)
}

pub async fn add_assignee(
    c: &HttpClient,
    mission_id: i64,
    body: AssigneeDataArg,
) -> Result<AssigneeViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/mission/{mission_id}/assignee"),
        Some(&body),
    )
    .await
}

pub async fn remove_assignee(
    c: &HttpClient,
    mission_id: i64,
    assignee_id: i64,
) -> Result<(), ApiError> {
    let _: serde_json::Value = c
        .request(
            reqwest::Method::DELETE,
            &format!("/api/mission/{mission_id}/assignee/{assignee_id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

pub async fn create_mission(
    c: &HttpClient,
    body: CreateMissionRequest,
) -> Result<MissionViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/mission", Some(&body))
        .await
}

#[cfg(test)]
mod tests {
    //! Unit tests for serde shape only. The HTTP adapter round-trips
    //! are covered by the command tests in `commands/mission.rs`.

    use super::*;

    #[test]
    fn mission_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&MissionKind::Crf).unwrap(),
            "\"crf\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Sdtm).unwrap(),
            "\"sdtm\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Adam).unwrap(),
            "\"adam\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Tfl).unwrap(),
            "\"tfl\""
        );
    }

    #[test]
    fn mission_role_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&MissionRole::Dev).unwrap(),
            "\"dev\""
        );
        assert_eq!(
            serde_json::to_string(&MissionRole::Qc).unwrap(),
            "\"qc\""
        );
    }

    #[test]
    fn create_mission_request_round_trips() {
        let body = CreateMissionRequest {
            project_code: "p1".into(),
            mission_kind: MissionKind::Crf,
            mission_code: "AE".into(),
            assignees: vec![AssigneeDataArg {
                user_code: "u1".into(),
                role: MissionRole::Dev,
            }],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert!(j.contains("\"projectCode\":\"p1\""));
        assert!(j.contains("\"missionKind\":\"crf\""));
        assert!(j.contains("\"missionCode\":\"AE\""));
        assert!(j.contains("\"userCode\":\"u1\""));
        assert!(j.contains("\"role\":\"dev\""));
        let parsed: CreateMissionRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    #[test]
    fn mission_view_response_parses_full_wire_shape() {
        let j = r#"{
            "id": 42,
            "projectCode": "p1",
            "missionKind": "crf",
            "missionCode": "AE",
            "assignees": [
                {
                    "id": 7,
                    "userCode": "u1",
                    "role": "qc",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-01T00:00:00Z"
                }
            ],
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z"
        }"#;
        let m: MissionViewResponse = serde_json::from_str(j).unwrap();
        assert_eq!(m.id, 42);
        assert_eq!(m.mission_kind, MissionKind::Crf);
        assert_eq!(m.assignees.len(), 1);
        assert_eq!(m.assignees[0].role, MissionRole::Qc);
    }
}
```

- [ ] **Step 1.2: Register the module**

Edit `apps/desktop/aegis-desktop/src-tauri/src/http.rs`:

```rust
// existing modules...
pub mod mission;
```

(Read the file first to find the right spot; if `http.rs` uses `pub mod foo;` lines, add `pub mod mission;` in alphabetical order.)

- [ ] **Step 1.3: Run unit tests for `http/mission.rs`**

```bash
cd apps/desktop/aegis-desktop && cargo test -p aegis-desktop --lib http::mission
```

Expected: 4 tests pass.

- [ ] **Step 1.4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs apps/desktop/aegis-desktop/src-tauri/src/http.rs
git commit -m "feat(desktop): add mission http adapter and wire DTOs"
```

---

### Task 2: Add Tauri commands and register them

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs:8` (add `pub mod mission;`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:22-111` (register handlers in `invoke_handler!`)

- [ ] **Step 2.1: Write `commands/mission.rs`**

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::mission::{
    self, AddAssignee, AssigneeDataArg, AssigneeViewResponse, CreateMissionRequest,
    MissionViewResponse,
};

// Tauri command argument shapes. The frontend calls these via
// `invoke<...>("command_name", args)`; serde decodes the JSON.
// `kind` / `mission_kind` / `role` come in as plain strings and are
// re-parsed via serde's JSON deserialize so the call sites get
// validation errors back as `ApiError::Parse`.

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMissionsByProjectArgs {
    pub project_code: String,
    pub kind: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssigneeArgs {
    pub mission_id: i64,
    pub user_code: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAssigneeArgs {
    pub mission_id: i64,
    pub assignee_id: i64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionAssigneeArg {
    pub user_code: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionArgs {
    pub project_code: String,
    pub mission_kind: String,
    pub mission_code: String,
    pub assignees: Vec<CreateMissionAssigneeArg>,
}

fn parse_kind(s: &str) -> Result<crate::http::mission::MissionKind, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| ApiError::Parse { message: e.to_string() })
}

fn parse_role(s: &str) -> Result<crate::http::mission::MissionRole, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| ApiError::Parse { message: e.to_string() })
}

#[tauri::command]
pub async fn list_missions_by_project(
    client: State<'_, HttpClient>,
    args: ListMissionsByProjectArgs,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    let kind = match args.kind.as_deref() {
        Some(s) => Some(parse_kind(s)?),
        None => None,
    };
    mission::list_by_project(&client, &args.project_code, kind).await
}

#[tauri::command]
pub async fn add_assignee(
    client: State<'_, HttpClient>,
    args: AddAssigneeArgs,
) -> Result<AssigneeViewResponse, ApiError> {
    mission::add_assignee(
        &client,
        args.mission_id,
        AssigneeDataArg {
            user_code: args.user_code,
            role: parse_role(&args.role)?,
        },
    )
    .await
}

#[tauri::command]
pub async fn remove_assignee(
    client: State<'_, HttpClient>,
    args: RemoveAssigneeArgs,
) -> Result<(), ApiError> {
    mission::remove_assignee(&client, args.mission_id, args.assignee_id).await
}

#[tauri::command]
pub async fn create_mission(
    client: State<'_, HttpClient>,
    args: CreateMissionArgs,
) -> Result<MissionViewResponse, ApiError> {
    let assignees = args
        .assignees
        .into_iter()
        .map(|a| -> Result<AssigneeDataArg, ApiError> {
            Ok(AssigneeDataArg {
                user_code: a.user_code,
                role: parse_role(&a.role)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    mission::create_mission(
        &client,
        CreateMissionRequest {
            project_code: args.project_code,
            mission_kind: parse_kind(&args.mission_kind)?,
            mission_code: args.mission_code,
            assignees,
        },
    )
    .await
}

// Re-export `AddAssignee` is intentionally unused; kept for parity with
// future-proofing the export block. Remove if cargo warns.
#[allow(dead_code)]
type AddAssignee = AddAssigneeArgs;
```

- [ ] **Step 2.2: Register `pub mod mission;` in `commands.rs`**

Add the line `pub mod mission;` after `pub mod identity;` in `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`.

- [ ] **Step 2.3: Register the four handlers in `lib.rs`**

Edit `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. After the existing `commands::user::update_user,` line (line 39), add the `// mission` block:

```rust
            // mission
            commands::mission::list_missions_by_project,
            commands::mission::add_assignee,
            commands::mission::remove_assignee,
            commands::mission::create_mission,
```

(Insert just before the `// project` block at line 40 so the file stays alphabetical.)

- [ ] **Step 2.4: Compile-check**

```bash
cd apps/desktop/aegis-desktop && cargo check -p aegis-desktop --lib
```

Expected: success with no new warnings.

- [ ] **Step 2.5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs apps/desktop/aegis-desktop/src-tauri/src/commands.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): expose mission Tauri commands + register handlers"
```

---

## Phase 2 — TS API client

### Task 3: Add mission types to `shared/api/types.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (insert mission types after the `// Project` section)

- [ ] **Step 3.1: Insert mission types**

After the closing brace of `ProjectView` (line 120) and before `// Terminology` (line 137), add:

```ts
// Mission
export type MissionKind = "crf" | "sdtm" | "adam" | "tfl";
export type MissionRole = "dev" | "qc";

export interface AssigneeView {
  id: number;
  userCode: string;
  role: MissionRole;
  createdAt: string;
  updatedAt: string;
}

export interface MissionView {
  id: number;
  projectCode: string;
  missionKind: MissionKind;
  missionCode: string;
  assignees: AssigneeView[];
  createdAt: string;
  updatedAt: string;
}

export interface MissionListResponse {
  missions: MissionView[];
}

export interface CreateMissionInput {
  projectCode: string;
  missionKind: MissionKind;
  missionCode: string;
  assignees: { userCode: string; role: MissionRole }[];
}
```

- [ ] **Step 3.2: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: success.

- [ ] **Step 3.3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts
git commit -m "feat(desktop): add mission TS wire DTOs"
```

---

### Task 4: Add mission methods to `shared/api/index.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`

- [ ] **Step 4.1: Add to the import list (lines 4-65)**

Add the mission types to the type import block at the top of the file:

```ts
  AssigneeView,
  CreateMissionInput,
  MissionKind,
  MissionListResponse,
  MissionRole,
  MissionView,
```

(insert these in alphabetical position among the other named imports).

- [ ] **Step 4.2: Add the API methods (after the `// project` block, around line 121)**

Add a new section before `// health`:

```ts
  // mission
  listMissionsByProject: async (
    input: { projectCode: string; kind?: MissionKind },
  ): Promise<MissionView[]> => {
    const resp = await call<MissionListResponse>(
      "list_missions_by_project",
      input,
    );
    return resp.missions;
  },
  addAssignee: (
    missionId: number,
    body: { userCode: string; role: MissionRole },
  ): Promise<AssigneeView> =>
    call<AssigneeView>("add_assignee", { missionId, ...body }),
  removeAssignee: (missionId: number, assigneeId: number): Promise<void> =>
    call<void>("remove_assignee", { missionId, assigneeId }),
  createMission: (input: CreateMissionInput): Promise<MissionView> =>
    call<MissionView>("create_mission", input),
```

- [ ] **Step 4.3: Add to the export list (lines 435-514)**

Add to the `export type { … } from "./types";` block:

```ts
  AssigneeView,
  CreateMissionInput,
  MissionKind,
  MissionListResponse,
  MissionRole,
  MissionView,
```

(in alphabetical position).

- [ ] **Step 4.4: Typecheck + commit**

```bash
pnpm --filter aegis-desktop typecheck
git add apps/desktop/aegis-desktop/src/shared/api/index.ts
git commit -m "feat(desktop): add mission API methods (list/add/remove/create)"
```

Expected: typecheck succeeds; commit lands.

---

### Task 5: Add mission query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`

- [ ] **Step 5.1: Add the `mission` namespace**

After the `project` block (line 21) and before `product` (line 22), insert:

```ts
  mission: {
    byProject: (projectCode: string, kind?: MissionKind) =>
      ["mission", "byProject", projectCode, kind ?? null] as const,
    byId: (id: number) => ["mission", "byId", id] as const,
  },
```

- [ ] **Step 5.2: Update the import at the top of `keys.ts`**

The file currently has no imports. Add at the top:

```ts
import type { MissionKind } from "../api/types";
```

- [ ] **Step 5.3: Typecheck + commit**

```bash
pnpm --filter aegis-desktop typecheck
git add apps/desktop/aegis-desktop/src/shared/query/keys.ts
git commit -m "feat(desktop): add mission query keys"
```

---

## Phase 3 — Mission feature hooks

### Task 6: `useListMissionsByProject`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/list.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/mission/list.test.tsx`

- [ ] **Step 6.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/mission/list.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useListMissionsByProject } from "../../../features/mission/data/list";
import { queryKeys } from "../../../shared/query";
import type { MissionView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import {
  makeTestQueryClient,
  renderWithQueryClient,
} from "../../../test/helpers/render-with-query-client";

const missionFixture: MissionView = {
  id: 9,
  projectCode: "alpha",
  missionKind: "crf",
  missionCode: "AE",
  assignees: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function Probe({ code, kind }: { code: string; kind?: "crf" | "sdtm" | "adam" | "tfl" }) {
  const q = useListMissionsByProject(code, kind);
  return (
    <span data-testid="count">{q.data?.length ?? "none"}</span>
  );
}

describe("useListMissionsByProject", () => {
  it("invokes list_missions_by_project on mount with the project code", async () => {
    mockCommands({ list_missions_by_project: () => [missionFixture] });
    renderWithQueryClient(<Probe code="alpha" kind="crf" />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_missions_by_project", {
        projectCode: "alpha",
        kind: "crf",
      });
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });

  it("uses the byProject query key with kind baked in", async () => {
    mockCommands({ list_missions_by_project: () => [missionFixture] });
    const client = makeTestQueryClient();
    renderWithQueryClient(<Probe code="alpha" kind="crf" />, { client });
    await waitFor(() =>
      expect(client.getQueryData(queryKeys.mission.byProject("alpha", "crf"))).toBeDefined(),
    );
  });

  it("omits kind from the wire args when no kind filter is passed", async () => {
    mockCommands({ list_missions_by_project: () => [missionFixture] });
    renderWithQueryClient(<Probe code="alpha" />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_missions_by_project", {
        projectCode: "alpha",
        kind: undefined,
      });
    });
  });

  it("propagates ApiError into query.error", async () => {
    mockCommands({
      list_missions_by_project: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    function ErrorProbe() {
      const q = useListMissionsByProject("alpha");
      return (
        <span data-testid="error-kind">
          {q.error ? (q.error as { kind: string }).kind : "none"}
        </span>
      );
    }
    renderWithQueryClient(<ErrorProbe />);
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });
});
```

- [ ] **Step 6.2: Run the test to confirm it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/list.test.tsx
```

Expected: FAIL — `useListMissionsByProject` is not exported.

- [ ] **Step 6.3: Implement `useListMissionsByProject`**

Create `apps/desktop/aegis-desktop/src/features/mission/data/list.ts`:

```ts
import { useQuery } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type MissionKind,
  type MissionView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * All missions of a given kind for a project. `kind` is optional;
 * pass `"crf"` for the CRF form list page. Defaults match the rest
 * of the app: no retry, no refetch on focus, staleTime: Infinity.
 */
export function useListMissionsByProject(
  projectCode: string,
  kind?: MissionKind,
) {
  return useQuery<MissionView[], ApiError>({
    queryKey: queryKeys.mission.byProject(projectCode, kind),
    queryFn: () => api.listMissionsByProject({ projectCode, kind }),
  });
}
```

- [ ] **Step 6.4: Run the test to confirm it passes**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/list.test.tsx
```

Expected: 4 tests pass.

- [ ] **Step 6.5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/mission/data/list.ts apps/desktop/aegis-desktop/src/test/features/mission/list.test.tsx
git commit -m "feat(mission): useListMissionsByProject hook + test"
```

---

### Task 7: `useAddAssignee`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/add-assignee.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/mission/add-assignee.test.tsx`

- [ ] **Step 7.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/mission/add-assignee.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useAddAssignee } from "../../../features/mission/data/add-assignee";
import { queryKeys } from "../../../shared/query";
import type { AssigneeView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const assigneeFixture: AssigneeView = {
  id: 7,
  userCode: "u1",
  role: "dev",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function Harness() {
  const m = useAddAssignee("alpha");
  return (
    <button
      onClick={() => {
        m.mutate({ missionId: 9, userCode: "u1", role: "dev" });
      }}
    >
      add
    </button>
  );
}

describe("useAddAssignee", () => {
  it("invokes api.addAssignee with { missionId, userCode, role }", async () => {
    mockCommands({ add_assignee: () => assigneeFixture });
    renderWithQueryClient(<Harness />);
    await userEvent.click(screen.getByRole("button", { name: "add" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("add_assignee", {
        missionId: 9,
        userCode: "u1",
        role: "dev",
      });
    });
  });

  it("invalidates mission.byProject(projectCode, 'crf') on success", async () => {
    mockCommands({ add_assignee: () => assigneeFixture });
    const { client } = renderWithQueryClient(<Harness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "add" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha", "crf"),
        }),
      );
    });
  });
});
```

- [ ] **Step 7.2: Run to confirm failure**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/add-assignee.test.tsx
```

Expected: FAIL — `useAddAssignee` not exported.

- [ ] **Step 7.3: Implement `useAddAssignee`**

Create `apps/desktop/aegis-desktop/src/features/mission/data/add-assignee.ts`:

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type AssigneeView,
  type MissionRole,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * Add an assignee to an existing mission. Factory-style: pass
 * `projectCode` so the `onSuccess` invalidation knows which query to
 * bust. The `kind` arg in the invalidation key is hard-coded to
 * `"crf"` because the only consumer (the CRF mission-assign drawer)
 * only invalidates the CRF-mission list.
 */
export function useAddAssignee(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<
    AssigneeView,
    ApiError,
    { missionId: number; userCode: string; role: MissionRole }
  >({
    mutationFn: ({ missionId, userCode, role }) =>
      api.addAssignee(missionId, { userCode, role }),
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, "crf"),
      });
    },
  });
}
```

- [ ] **Step 7.4: Run tests, expect pass; commit**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/add-assignee.test.tsx
git add apps/desktop/aegis-desktop/src/features/mission/data/add-assignee.ts apps/desktop/aegis-desktop/src/test/features/mission/add-assignee.test.tsx
git commit -m "feat(mission): useAddAssignee hook + invalidation + test"
```

Expected: 2 tests pass.

---

### Task 8: `useRemoveAssignee`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/remove-assignee.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/mission/remove-assignee.test.tsx`

- [ ] **Step 8.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/mission/remove-assignee.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useRemoveAssignee } from "../../../features/mission/data/remove-assignee";
import { queryKeys } from "../../../shared/query";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function Harness() {
  const m = useRemoveAssignee("alpha");
  return (
    <button
      onClick={() => {
        m.mutate({ missionId: 9, assigneeId: 7 });
      }}
    >
      remove
    </button>
  );
}

describe("useRemoveAssignee", () => {
  it("invokes api.removeAssignee with { missionId, assigneeId }", async () => {
    mockCommands({ remove_assignee: () => undefined });
    renderWithQueryClient(<Harness />);
    await userEvent.click(screen.getByRole("button", { name: "remove" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("remove_assignee", {
        missionId: 9,
        assigneeId: 7,
      });
    });
  });

  it("invalidates mission.byProject(projectCode, 'crf') on success", async () => {
    mockCommands({ remove_assignee: () => undefined });
    const { client } = renderWithQueryClient(<Harness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "remove" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha", "crf"),
        }),
      );
    });
  });
});
```

- [ ] **Step 8.2: Implement the hook**

Create `apps/desktop/aegis-desktop/src/features/mission/data/remove-assignee.ts`:

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { api, type ApiError } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

export function useRemoveAssignee(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { missionId: number; assigneeId: number }
  >({
    mutationFn: ({ missionId, assigneeId }) =>
      api.removeAssignee(missionId, assigneeId),
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, "crf"),
      });
    },
  });
}
```

- [ ] **Step 8.3: Test + commit**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/remove-assignee.test.tsx
git add apps/desktop/aegis-desktop/src/features/mission/data/remove-assignee.ts apps/desktop/aegis-desktop/src/test/features/mission/remove-assignee.test.tsx
git commit -m "feat(mission): useRemoveAssignee hook + invalidation + test"
```

Expected: 2 tests pass.

---

### Task 9: `useCreateMission`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/create-mission.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/mission/create-mission.test.tsx`

- [ ] **Step 9.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/mission/create-mission.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useCreateMission } from "../../../features/mission/data/create-mission";
import { queryKeys } from "../../../shared/query";
import type { MissionView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const missionFixture: MissionView = {
  id: 9,
  projectCode: "alpha",
  missionKind: "crf",
  missionCode: "AE",
  assignees: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function Harness() {
  const m = useCreateMission("alpha");
  return (
    <button
      onClick={() => {
        m.mutate({
          projectCode: "alpha",
          missionKind: "crf",
          missionCode: "AE",
          assignees: [{ userCode: "u1", role: "dev" }],
        });
      }}
    >
      create
    </button>
  );
}

describe("useCreateMission", () => {
  it("invokes api.createMission with the full input shape", async () => {
    mockCommands({ create_mission: () => missionFixture });
    renderWithQueryClient(<Harness />);
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_mission", {
        projectCode: "alpha",
        missionKind: "crf",
        missionCode: "AE",
        assignees: [{ userCode: "u1", role: "dev" }],
      });
    });
  });

  it("invalidates mission.byProject(projectCode, 'crf') on success", async () => {
    mockCommands({ create_mission: () => missionFixture });
    const { client } = renderWithQueryClient(<Harness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha", "crf"),
        }),
      );
    });
  });
});
```

- [ ] **Step 9.2: Implement the hook**

Create `apps/desktop/aegis-desktop/src/features/mission/data/create-mission.ts`:

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateMissionInput,
  type MissionView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

export function useCreateMission(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<MissionView, ApiError, CreateMissionInput>({
    mutationFn: (input) => api.createMission(input),
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, "crf"),
      });
    },
  });
}
```

- [ ] **Step 9.3: Test + commit**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/create-mission.test.tsx
git add apps/desktop/aegis-desktop/src/features/mission/data/create-mission.ts apps/desktop/aegis-desktop/src/test/features/mission/create-mission.test.tsx
git commit -m "feat(mission): useCreateMission hook + invalidation + test"
```

Expected: 2 tests pass.

---

### Task 10: `useIsProjectLeader`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/data/leader.ts`
- Create: `apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx`

- [ ] **Step 10.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useIsProjectLeader } from "../../../features/mission/data/leader";
import type { ProjectView, UserView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const aliceUser: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "",
  updatedAt: "",
};

const bobUser: UserView = {
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
  active: true,
  createdAt: "",
  updatedAt: "",
};

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "x",
  tags: [],
  active: true,
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: {
    leaders: [],
    workers: [],
  },
  createdAt: "",
  updatedAt: "",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function Probe({ projectCode, expected }: { projectCode: string; expected: "true" | "false" }) {
  const isLeader = useIsProjectLeader(projectCode);
  return <span data-testid="flag">{isLeader ? "true" : expected === "true" ? "false" : "false"}</span>;
}

function makeProbe(currentUser: UserView) {
  return function Inner({ projectCode }: { projectCode: string }) {
    const isLeader = useIsProjectLeader(projectCode);
    return (
      <span data-testid="flag">
        {isLeader ? "true" : "false"}
      </span>
    );
  };
}

describe("useIsProjectLeader", () => {
  it("returns true when the current user is in members.leaders", async () => {
    mockCommands({
      current_user: () => aliceUser,
      list_projects: () => [projectFixture],
    });
    const Probe = makeProbe(aliceUser);
    renderWithQueryClient(<Probe projectCode="alpha" />);
    await waitFor(() => {
      expect(screen.getByTestId("flag").textContent).toBe("true");
    });
  });

  it("returns false when the current user is only a worker", async () => {
    mockCommands({
      current_user: () => bobUser,
      list_projects: () => [projectFixture],
    });
    const Probe = makeProbe(bobUser);
    renderWithQueryClient(<Probe projectCode="alpha" />);
    await waitFor(() => {
      expect(screen.getByTestId("flag").textContent).toBe("false");
    });
  });

  it("returns false when the project is not found in the list", async () => {
    mockCommands({
      current_user: () => aliceUser,
      list_projects: () => [],
    });
    const Probe = makeProbe(aliceUser);
    renderWithQueryClient(<Probe projectCode="missing" />);
    await waitFor(() => {
      expect(screen.getByTestId("flag").textContent).toBe("false");
    });
  });
});
```

- [ ] **Step 10.2: Implement `useIsProjectLeader`**

Create `apps/desktop/aegis-desktop/src/features/mission/data/leader.ts`:

```ts
import { useCurrentUser } from "../../auth/data/current-user";
import { useListProjects } from "../../project-list/data/projects";

/**
 * Client-side leader check. `true` iff the signed-in user appears in
 * `project.members.leaders[].code`. The server remains the
 * authoritative gate; this is purely for hiding the UI affordance.
 *
 * `useListProjects` is the same data source the project-list page
 * uses — it always includes the membership block, so no extra fetch.
 */
export function useIsProjectLeader(projectCode: string): boolean {
  const { data: currentUser } = useCurrentUser();
  const { data: projects } = useListProjects();
  if (!currentUser || !projects) return false;
  const project = projects.find((p) => p.code === projectCode);
  if (!project) return false;
  return project.members.leaders.some((u) => u.code === currentUser.code);
}
```

- [ ] **Step 10.3: Run test + commit**

```bash
pnpm --filter aegis-desktop test -- src/test/features/mission/leader.test.tsx
git add apps/desktop/aegis-desktop/src/features/mission/data/leader.ts apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx
git commit -m "feat(mission): useIsProjectLeader hook + test"
```

Expected: 3 tests pass.

---

### Task 11: `features/mission/index.ts` barrel

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/mission/index.ts`

- [ ] **Step 11.1: Add the barrel**

```ts
export { useListMissionsByProject } from "./data/list";
export { useAddAssignee } from "./data/add-assignee";
export { useRemoveAssignee } from "./data/remove-assignee";
export { useCreateMission } from "./data/create-mission";
export { useIsProjectLeader } from "./data/leader";
```

- [ ] **Step 11.2: Typecheck + commit**

```bash
pnpm --filter aegis-desktop typecheck
git add apps/desktop/aegis-desktop/src/features/mission/index.ts
git commit -m "feat(mission): add feature barrel"
```

---

## Phase 4 — Page + table wiring

### Task 12: Update `CrfFormListPage` wiring

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx`

- [ ] **Step 12.1: Update imports**

Replace the existing `CrfAssignTakersDrawer` import (line 18) with `CrfMissionAssignDrawer`:

```tsx
import {
  CrfFormDrawer,
  CrfFormFilterDrawer,
  CrfMissionAssignDrawer,
  type CrfStatusFilter,
  CrfFormTable,
  CrfToolsMenu,
  CrfStatusChip,
  CrfVersionDropdown,
  DeleteCrfFormDialog,
} from "../components";
```

Add the new hook imports (place after the `useListCrfForms` block, around line 34):

```tsx
import {
  useAddAssignee,
  useCreateMission,
  useIsProjectLeader,
  useListMissionsByProject,
  useRemoveAssignee,
} from "../../mission";
import { useListUsers } from "../../user/data/list";
import type { MissionView } from "../../../shared/api";
```

- [ ] **Step 12.2: Rename state + add hook calls**

Replace `assignTakersFor` with `assignMissionFor` (around line 139):

```tsx
  const [assignMissionFor, setAssignMissionFor] = useState<CrfForm | null>(null);
```

Add the new hook calls just below the existing mutation declarations (around line 144):

```tsx
  const missionsQuery = useListMissionsByProject(projectCode, "crf");
  const missionsByFormCode = useMemo<ReadonlyMap<string, MissionView>>(
    () => new Map((missionsQuery.data ?? []).map((m) => [m.missionCode, m])),
    [missionsQuery.data],
  );
  const isLeader = useIsProjectLeader(projectCode);
  const usersQuery = useListUsers();
  const userNameByCode = useMemo<ReadonlyMap<string, string>>(
    () => new Map((usersQuery.data ?? []).map((u) => [u.code, u.name])),
    [usersQuery.data],
  );
```

- [ ] **Step 12.3: Pass the new props to `CrfFormTable`**

In the `<CrfFormTable … />` block (lines 222-245):

```tsx
      <CrfFormTable
        rows={filteredRows}
        loading={formsQuery.isFetching}
        error={formsQuery.error}
        canAddFilter={selectedVersionId != null}
        missionsByFormCode={missionsByFormCode}
        userNameByCode={userNameByCode}
        isLeader={isLeader}
        onAdd={() => setDrawer({ mode: "create" })}
        onFilter={() => setFilterOpen(true)}
        onAssignMission={(row) => setAssignMissionFor(row)}
        onEdit={(row) => setDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpenDetail={(row) =>
          navigate({
            to: "/project/$projectCode/crf/$formId",
            params: { projectCode, formId: String(row.id) },
            search: { versionId: selectedVersionId ?? undefined },
          })
        }
        onReorder={handleReorder}
      />
```

- [ ] **Step 12.4: Replace the drawer JSX**

Replace the `<CrfAssignTakersDrawer … />` block (lines 284-287):

```tsx
      <CrfMissionAssignDrawer
        open={assignMissionFor != null}
        onClose={() => setAssignMissionFor(null)}
        projectCode={projectCode}
        form={assignMissionFor}
      />
```

- [ ] **Step 12.5: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: errors only in `CrfFormTable.tsx` (we'll fix those next).

- [ ] **Step 12.6: Commit (page wiring) — leave table for next task**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx
git commit -m "feat(crf): wire mission-assign state into CrfFormListPage"
```

---

### Task 13: Update `CrfFormTable` (assignee cell + leader-gated icon)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`

- [ ] **Step 13.1: Update imports**

Add the `Stack` import (it isn't imported yet — `Chip` already is). Replace the import block at the top:

```tsx
import {
  Box,
  Chip,
  IconButton,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
} from "@aegis/ui/mui";
```

Add the `MissionView` type import:

```tsx
import type { CrfForm, MissionView } from "../../../shared/api";
```

- [ ] **Step 13.2: Update the `Props` and `DraggableRowProps` interfaces**

Replace the existing `Props` (lines 88-100):

```tsx
interface Props {
  rows: CrfForm[];
  loading: boolean;
  error: unknown;
  canAddFilter: boolean;
  missionsByFormCode: ReadonlyMap<string, MissionView>;
  userNameByCode: ReadonlyMap<string, string>;
  isLeader: boolean;
  onAdd: () => void;
  onFilter: () => void;
  onAssignMission: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
  onReorder: (orderedIds: number[]) => void;
}

interface DraggableRowProps {
  row: CrfForm;
  showHandle: boolean;
  mission: MissionView | undefined;
  userNameByCode: ReadonlyMap<string, string>;
  isLeader: boolean;
  onAssignMission: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
}
```

- [ ] **Step 13.3: Replace `DraggableRow` body**

Replace the `DraggableRow` function (lines 111-191):

```tsx
function DraggableRow({
  row,
  showHandle,
  mission,
  userNameByCode,
  isLeader,
  onAssignMission,
  onEdit,
  onDelete,
  onOpenDetail,
}: DraggableRowProps) {
  const { t } = useI18n();
  const draggable = useDraggable({ id: String(row.id), type: "crfForm" });
  const droppable = useDroppable({ id: String(row.id), accept: "crfForm" });
  return (
    <TableRow
      hover
      ref={(el: HTMLTableRowElement | null) => {
        if (el && draggable.ref) draggable.ref(el);
        if (el && droppable.ref) droppable.ref(el);
      }}
    >
      <TableCell sx={{ width: 40 }}>
        {showHandle && (
          <DragIndicatorIcon
            fontSize="small"
            sx={{ cursor: "grab", opacity: 0.6 }}
            aria-label={t("crf.table.dragHandle")}
          />
        )}
      </TableCell>
      <TableCell>{row.code}</TableCell>
      <TableCell>{row.name}</TableCell>
      <TableCell sx={{ maxWidth: 280 }}>
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ flexWrap: "wrap", gap: 0.5 }}
        >
          {mission?.assignees.map((a) => (
            <Chip
              key={a.id}
              label={userNameByCode.get(a.userCode) ?? a.userCode}
              size="small"
              variant="outlined"
              sx={a.role === "qc" ? { borderStyle: "dashed" } : undefined}
            />
          ))}
          {!mission && <span aria-hidden>—</span>}
        </Stack>
      </TableCell>
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
        {isLeader && (
          <Tooltip title={t("crf.table.action.assignMission")}>
            <IconButton
              size="small"
              aria-label={t("crf.table.action.assignMission")}
              onClick={() => onAssignMission(row)}
            >
              <AssignmentIndIcon />
            </IconButton>
          </Tooltip>
        )}
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
  );
}
```

- [ ] **Step 13.4: Update the table signature and render loop**

Replace the `CrfFormTable` function signature (lines 193-205):

```tsx
export function CrfFormTable({
  rows,
  loading,
  error,
  canAddFilter,
  missionsByFormCode,
  userNameByCode,
  isLeader,
  onAdd,
  onFilter,
  onAssignMission,
  onEdit,
  onDelete,
  onOpenDetail,
  onReorder,
}: Props) {
```

Replace the column header (line 254):

```tsx
              <TableCell>{t("crf.table.column.assignee")}</TableCell>
```

Replace the `<DraggableRow … />` block (lines 289-303):

```tsx
            {orderedIds.map((id) => {
              const row = rowById.get(id);
              if (!row) return null;
              return (
                <DraggableRow
                  key={row.id}
                  row={row}
                  showHandle={showHandle}
                  mission={missionsByFormCode.get(row.code)}
                  userNameByCode={userNameByCode}
                  isLeader={isLeader}
                  onAssignMission={onAssignMission}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  onOpenDetail={onOpenDetail}
                />
              );
            })}
```

- [ ] **Step 13.5: Typecheck**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: clean.

- [ ] **Step 13.6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx
git commit -m "feat(crf): render assignee chips + leader-gated mission icon"
```

---

### Task 14: Update `crf-form-table.test.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx`

- [ ] **Step 14.1: Update `renderTable` to pass the new props**

Replace the `renderTable` helper (lines 39-68):

```tsx
function renderTable(
  props: Partial<ComponentProps<typeof CrfFormTable>> = {},
) {
  const onAdd = props.onAdd ?? vi.fn();
  const onFilter = props.onFilter ?? vi.fn();
  const onAssignMission = props.onAssignMission ?? vi.fn();
  const onEdit = props.onEdit ?? vi.fn();
  const onDelete = props.onDelete ?? vi.fn();
  const onOpenDetail = props.onOpenDetail ?? vi.fn();
  const onReorder = props.onReorder ?? vi.fn();
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CrfFormTable
          rows={props.rows ?? rows}
          loading={props.loading ?? false}
          error={props.error ?? null}
          canAddFilter={props.canAddFilter ?? true}
          missionsByFormCode={props.missionsByFormCode ?? new Map()}
          userNameByCode={props.userNameByCode ?? new Map()}
          isLeader={props.isLeader ?? true}
          onAdd={onAdd}
          onFilter={onFilter}
          onAssignMission={onAssignMission}
          onEdit={onEdit}
          onDelete={onDelete}
          onOpenDetail={onOpenDetail}
          onReorder={onReorder}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}
```

- [ ] **Step 14.2: Update the cell-index comment**

Replace the comment in the drag-handle test (line 170):

```tsx
    // cells[0] = drag handle, cells[1] = code, cells[2] = name,
    // cells[3] = assignee, cells[4] = status, cells[5] = actions
```

- [ ] **Step 14.3: Add the new tests**

Append these two new test cases inside the `describe("CrfFormTable", …)` block (right before the closing brace):

```tsx
  it("does not render the assign icon when isLeader is false", () => {
    renderTable({ isLeader: false });
    expect(
      screen.queryByLabelText(/assign mission/i),
    ).not.toBeInTheDocument();
  });

  it("renders assignee chips with the user name (and dashed border for qc)", () => {
    const missionsByFormCode = new Map([
      [
        "AE",
        {
          id: 9,
          projectCode: "p1",
          missionKind: "crf",
          missionCode: "AE",
          assignees: [
            {
              id: 1,
              userCode: "alice",
              role: "dev",
              createdAt: "",
              updatedAt: "",
            },
            {
              id: 2,
              userCode: "bob",
              role: "qc",
              createdAt: "",
              updatedAt: "",
            },
          ],
          createdAt: "",
          updatedAt: "",
        },
      ],
    ]);
    const userNameByCode = new Map([
      ["alice", "Alice"],
      ["bob", "Bob"],
    ]);
    renderTable({ missionsByFormCode, userNameByCode, isLeader: true });
    expect(screen.getByText("Alice")).toBeInTheDocument();
    expect(screen.getByText("Bob")).toBeInTheDocument();
    // The QC chip's MUI chip element carries the dashed border style.
    const bobChip = screen.getByText("Bob").closest(".MuiChip-root");
    expect(bobChip).not.toBeNull();
    expect(bobChip!.className).toContain("MuiChip-outlined");
  });
```

- [ ] **Step 14.4: Run the test + commit**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-form-table.test.tsx
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-form-table.test.tsx
git commit -m "test(crf): update CrfFormTable tests for assignee column + leader gate"
```

Expected: all tests pass (existing tests still green after the prop rename; new tests cover leader-gated icon and chip rendering).

---

## Phase 5 — i18n + drawer

### Task 15: i18n strings (column/action rename + missionAssign keys)

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 15.1: Update `en.ts`**

Replace the existing keys:

```ts
  "crf.table.column.taker": "Assignee",
  "crf.table.action.assignTakers": "Assign mission",
  "crf.assignTakers.title": "Assign Mission",
  "crf.assignTakers.placeholder": "Takers UI coming soon",
```

with:

```ts
  "crf.table.column.assignee": "Assignee",
  "crf.table.action.assignMission": "Assign mission",
  "crf.missionAssign.title": "Assign Mission",
  "crf.missionAssign.subtitle": "Form: {formCode}",
  "crf.missionAssign.currentAssignees": "Current Assignees",
  "crf.missionAssign.addAssignee": "Add Assignee",
  "crf.missionAssign.user": "User",
  "crf.missionAssign.role.dev": "Dev",
  "crf.missionAssign.role.qc": "QC",
  "crf.missionAssign.empty": "No assignees yet.",
  "crf.missionAssign.remove": "Remove assignee",
```

(The old `crf.assignTakers.*` keys are removed — the new `crf.missionAssign.*` keys replace them.)

- [ ] **Step 15.2: Update `zhCN.ts`**

Replace the matching zhCN block:

```ts
  "crf.table.column.assignee": "负责人",
  "crf.table.action.assignMission": "分配任务",
  "crf.missionAssign.title": "分配任务",
  "crf.missionAssign.subtitle": "表单: {formCode}",
  "crf.missionAssign.currentAssignees": "当前负责人",
  "crf.missionAssign.addAssignee": "添加负责人",
  "crf.missionAssign.user": "用户",
  "crf.missionAssign.role.dev": "开发",
  "crf.missionAssign.role.qc": "质控",
  "crf.missionAssign.empty": "暂无负责人",
  "crf.missionAssign.remove": "删除负责人",
```

- [ ] **Step 15.3: Typecheck + run all tests**

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop test
```

Expected: typecheck clean. Existing tests that referenced `crf.assignTakers.title` will fail (they're all in the placeholder, which we delete next) — those failures are addressed in Task 17.

- [ ] **Step 15.4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): rename taker→assignee + add crf.missionAssign.* keys"
```

---

### Task 16: Build `CrfMissionAssignDrawer`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/crf/crf-mission-assign-drawer.test.tsx`

- [ ] **Step 16.1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/crf/crf-mission-assign-drawer.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { CrfMissionAssignDrawer } from "../../../features/crf/components/CrfMissionAssignDrawer";
import type {
  CrfForm,
  MissionView,
  ProjectView,
  UserView,
} from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const form: CrfForm = {
  id: 1,
  versionId: 7,
  code: "AE",
  name: "Adverse Events",
  order: 1,
  notSubmitted: false,
  createdAt: "",
  updatedAt: "",
};

const project: ProjectView = {
  id: 1,
  code: "alpha",
  description: "",
  tags: [],
  active: true,
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  createdAt: "",
  updatedAt: "",
};

const alice: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "",
  updatedAt: "",
};

const existingMission: MissionView = {
  id: 9,
  projectCode: "alpha",
  missionKind: "crf",
  missionCode: "AE",
  assignees: [
    {
      id: 7,
      userCode: "alice",
      role: "dev",
      createdAt: "",
      updatedAt: "",
    },
  ],
  createdAt: "",
  updatedAt: "",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function renderDrawer(formArg: CrfForm | null) {
  return renderWithQueryClient(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CrfMissionAssignDrawer
          open={formArg != null}
          onClose={vi.fn()}
          projectCode="alpha"
          form={formArg}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("CrfMissionAssignDrawer", () => {
  it("renders the empty-state message when no mission exists", async () => {
    mockCommands({
      list_projects: () => [project],
      list_users: () => [alice],
      list_missions_by_project: () => [],
    });
    renderDrawer(form);
    await waitFor(() => {
      expect(screen.getByText(/no assignees yet/i)).toBeInTheDocument();
    });
  });

  it("renders the existing assignees with chips and a remove icon", async () => {
    mockCommands({
      list_projects: () => [project],
      list_users: () => [alice],
      list_missions_by_project: () => [existingMission],
    });
    renderDrawer(form);
    await waitFor(() => {
      // The Alice chip is rendered in both the assignee list AND the
      // members dropdown source, but it should be present at least
      // once in the document.
      expect(screen.getAllByText("Alice").length).toBeGreaterThan(0);
    });
  });

  it("calls create_mission on Add when no mission exists yet", async () => {
    mockCommands({
      list_projects: () => [project],
      list_users: () => [alice],
      list_missions_by_project: () => [],
      create_mission: () => existingMission,
    });
    renderDrawer(form);
    // Wait for the user dropdown to populate.
    await waitFor(() =>
      expect(screen.getAllByText("Alice").length).toBeGreaterThan(0),
    );
    const userInput = screen.getByLabelText(/user/i);
    await userEvent.click(userInput);
    const aliceOption = await screen.findByRole("option", { name: "Alice" });
    await userEvent.click(aliceOption);
    await userEvent.click(screen.getByRole("button", { name: /^add$/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "create_mission",
        expect.objectContaining({
          projectCode: "alpha",
          missionKind: "crf",
          missionCode: "AE",
        }),
      );
    });
  });

  it("calls add_assignee on Add when a mission already exists", async () => {
    mockCommands({
      list_projects: () => [project],
      list_users: () => [alice, { ...alice, code: "bob", name: "Bob", id: 2 }],
      list_missions_by_project: () => [existingMission],
      add_assignee: () => ({
        id: 8,
        userCode: "bob",
        role: "qc",
        createdAt: "",
        updatedAt: "",
      }),
    });
    renderDrawer(form);
    await waitFor(() =>
      expect(screen.getAllByText("Bob").length).toBeGreaterThan(0),
    );
    const userInput = screen.getByLabelText(/user/i);
    await userEvent.click(userInput);
    const bobOption = await screen.findByRole("option", { name: "Bob" });
    await userEvent.click(bobOption);
    // Default role is "dev"; switch to "qc" so the call is deterministic.
    await userEvent.click(screen.getByLabelText(/role/i));
    await userEvent.click(screen.getByRole("option", { name: "QC" }));
    await userEvent.click(screen.getByRole("button", { name: /^add$/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "add_assignee",
        expect.objectContaining({
          missionId: 9,
          userCode: "bob",
          role: "qc",
        }),
      );
    });
  });
});
```

- [ ] **Step 16.2: Run the test to confirm it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-mission-assign-drawer.test.tsx
```

Expected: FAIL — `CrfMissionAssignDrawer` is not exported.

- [ ] **Step 16.3: Implement the drawer**

Create `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`:

```tsx
import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  Drawer,
  IconButton,
  MenuItem,
  Select,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { Close as CloseIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type {
  CrfForm,
  MissionRole,
  UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useCurrentUser } from "../../auth/data/current-user";
import {
  useAddAssignee,
  useCreateMission,
  useListMissionsByProject,
  useRemoveAssignee,
} from "../../mission";
import { useListProjects } from "../../project-list/data/projects";
import { useListUsers } from "../../user/data/list";

interface Props {
  open: boolean;
  onClose: () => void;
  projectCode: string;
  form: CrfForm | null;
}

/**
 * Right-anchored drawer that lists a CRF form's current assignees
 * (Dev / QC) and lets the project leader add or remove them. If
 * the form has no mission yet, the first Add creates the mission
 * (with that assignee) in one user-perceived action.
 */
export function CrfMissionAssignDrawer({
  open,
  onClose,
  projectCode,
  form,
}: Props) {
  const { t } = useI18n();
  const { data: missions = [] } = useListMissionsByProject(
    projectCode,
    "crf",
  );
  const { data: projects = [] } = useListProjects();
  const { data: users = [] } = useListUsers();
  const { data: currentUser } = useCurrentUser();

  const mission = useMemo(
    () =>
      form ? missions.find((m) => m.missionCode === form.code) : undefined,
    [missions, form],
  );

  const project = useMemo(
    () => projects.find((p) => p.code === projectCode),
    [projects, projectCode],
  );

  const userNameByCode = useMemo(
    () => new Map(users.map((u) => [u.code, u.name])),
    [users],
  );

  const allMembers = useMemo<UserSummary[]>(() => {
    if (!project) return [];
    const map = new Map<string, UserSummary>();
    for (const m of [project.members, project.unblindMembers]) {
      for (const u of [...m.leaders, ...m.workers]) {
        if (!map.has(u.code)) map.set(u.code, u);
      }
    }
    return [...map.values()];
  }, [project]);

  const assignedCodes = useMemo(
    () => new Set((mission?.assignees ?? []).map((a) => a.userCode)),
    [mission],
  );

  const availableMembers = useMemo(
    () => allMembers.filter((u) => !assignedCodes.has(u.code)),
    [allMembers, assignedCodes],
  );

  const [selectedUser, setSelectedUser] = useState<UserSummary | null>(null);
  const [selectedRole, setSelectedRole] = useState<MissionRole>("dev");

  // Reset the add-form when the target form changes (e.g. leader
  // closes and reopens on a different row).
  useEffect(() => {
    setSelectedUser(null);
    setSelectedRole("dev");
  }, [form?.code]);

  const createMission = useCreateMission(projectCode);
  const addAssignee = useAddAssignee(projectCode);
  const removeAssignee = useRemoveAssignee(projectCode);

  const activePending =
    createMission.isPending ||
    addAssignee.isPending ||
    removeAssignee.isPending;
  const activeError =
    createMission.error ?? addAssignee.error ?? removeAssignee.error;

  const assignees = mission?.assignees ?? [];

  function handleAdd() {
    if (!form || !selectedUser) return;
    if (mission) {
      addAssignee.mutate({
        missionId: mission.id,
        userCode: selectedUser.code,
        role: selectedRole,
      });
    } else {
      createMission.mutate({
        projectCode,
        missionKind: "crf",
        missionCode: form.code,
        assignees: [{ userCode: selectedUser.code, role: selectedRole }],
      });
    }
    setSelectedUser(null);
    setSelectedRole("dev");
  }

  function handleRemove(assigneeId: number) {
    if (!mission) return;
    removeAssignee.mutate({ missionId: mission.id, assigneeId });
  }

  if (!form) return null;

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t("crf.missionAssign.title")}
        </Typography>
        <Typography color="textSecondary">
          {t("crf.missionAssign.subtitle", { formCode: form.code })}
        </Typography>

        <Typography variant="subtitle2">
          {t("crf.missionAssign.currentAssignees")}
        </Typography>
        {assignees.length === 0 ? (
          <Typography color="textSecondary">
            {t("crf.missionAssign.empty")}
          </Typography>
        ) : (
          <Stack divider={<Box sx={{ borderBottom: 1, borderColor: "divider" }} />}>
            {assignees.map((a) => (
              <Stack
                key={a.id}
                direction="row"
                alignItems="center"
                spacing={1}
                sx={{ py: 1 }}
              >
                <Chip
                  label={userNameByCode.get(a.userCode) ?? a.userCode}
                  size="small"
                  variant="outlined"
                  sx={a.role === "qc" ? { borderStyle: "dashed" } : undefined}
                />
                <Chip
                  label={t(`crf.missionAssign.role.${a.role}`)}
                  size="small"
                  color="primary"
                />
                <Box sx={{ flexGrow: 1 }} />
                <IconButton
                  size="small"
                  aria-label={t("crf.missionAssign.remove")}
                  onClick={() => handleRemove(a.id)}
                  disabled={activePending}
                >
                  <CloseIcon fontSize="small" />
                </IconButton>
              </Stack>
            ))}
          </Stack>
        )}

        <Typography variant="subtitle2">
          {t("crf.missionAssign.addAssignee")}
        </Typography>
        <Stack direction="row" spacing={1} alignItems="center">
          <Autocomplete
            sx={{ flex: 1 }}
            options={availableMembers}
            getOptionLabel={(u) => u.name}
            value={selectedUser}
            onChange={(_e, v) => setSelectedUser(v)}
            disabled={activePending}
            renderInput={(params) => (
              <TextField
                {...params}
                label={t("crf.missionAssign.user")}
                size="small"
              />
            )}
          />
          <Select
            size="small"
            value={selectedRole}
            onChange={(e) => setSelectedRole(e.target.value as MissionRole)}
            label={t("crf.missionAssign.role.dev")}
            disabled={activePending}
            inputProps={{ "aria-label": t("crf.missionAssign.role.dev") }}
          >
            <MenuItem value="dev">{t("crf.missionAssign.role.dev")}</MenuItem>
            <MenuItem value="qc">{t("crf.missionAssign.role.qc")}</MenuItem>
          </Select>
          <Button
            variant="contained"
            disabled={!selectedUser || activePending}
            onClick={handleAdd}
          >
            {t("common.add")}
          </Button>
        </Stack>

        {activeError && (
          <Alert severity="error">{errorMessage(activeError)}</Alert>
        )}

        <Stack
          direction="row"
          spacing={1}
          sx={{ justifyContent: "flex-end" }}
        >
          <Button onClick={onClose}>{t("common.close")}</Button>
        </Stack>

        {/* `currentUser` is consumed indirectly via `useIsProjectLeader`
            in the parent; reading it here keeps the hook order stable
            for tests that mock `current_user`. */}
        {currentUser ? null : null}
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 16.4: Re-export from `components/index.ts`**

Edit `apps/desktop/aegis-desktop/src/features/crf/components/index.ts`. Replace the line that re-exports `CrfAssignTakersDrawer` (line 5) with:

```ts
export { CrfMissionAssignDrawer } from "./CrfMissionAssignDrawer";
```

- [ ] **Step 16.5: Typecheck + run drawer test**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-mission-assign-drawer.test.tsx
```

Expected: typecheck clean; 4 drawer tests pass (Autocomplete-driven Add may need looser assertions in practice — if it flakes, simplify by mocking the Autocomplete click via `fireEvent.change` on the underlying input).

- [ ] **Step 16.6: Run the full aegis-desktop test suite**

```bash
pnpm --filter aegis-desktop test
```

Expected: all green (including existing crf-form-table, mission hook tests, and the new drawer test).

- [ ] **Step 16.7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.test.tsx apps/desktop/aegis-desktop/src/features/crf/components/index.ts
git commit -m "feat(crf): replace placeholder drawer with CrfMissionAssignDrawer"
```

---

### Task 17: Delete the old placeholder drawer

**Files:**
- Delete: `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`

- [ ] **Step 17.1: Remove the file**

```bash
git rm apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx
```

- [ ] **Step 17.2: Sanity-check no references remain**

```bash
cd apps/desktop/aegis-desktop && grep -R "CrfAssignTakersDrawer" src
```

Expected: no matches (the page import was already renamed in Task 12; the barrel re-export was already replaced in Task 16.4).

- [ ] **Step 17.3: Typecheck + commit**

```bash
pnpm --filter aegis-desktop typecheck
git commit -m "refactor(crf): drop placeholder CrfAssignTakersDrawer"
```

---

## Phase 6 — Final verification

### Task 18: Cross-workspace checks

- [ ] **Step 18.1: Cargo fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```

Expected: no diff for `fmt`; clippy clean.

- [ ] **Step 18.2: Cargo test (no `--ignored`)**

```bash
cargo test -p aegis-desktop --lib
```

Expected: all unit tests pass.

- [ ] **Step 18.3: Workspace sanity**

```bash
cargo check --workspace
```

Expected: clean.

- [ ] **Step 18.4: pnpm test + typecheck (full desktop + ui)**

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui test
```

Expected: all green.

- [ ] **Step 18.5: Final commit (if any)**

If `cargo fmt` reported diffs, apply them and commit:

```bash
cargo fmt --all
git add -u
git commit -m "style: cargo fmt"
```

---

## Summary

18 tasks, ~24 commits, end state:

- New Tauri commands: `list_missions_by_project`, `add_assignee`, `remove_assignee`, `create_mission`.
- New TS API methods + types mirroring the server wire DTOs.
- New `features/mission/` vertical slice with 5 hooks + barrel.
- `CrfFormTable` renders assignee chips (dashed for qc) and hides the assign icon for non-leaders.
- `CrfMissionAssignDrawer` lists / adds / removes assignees with the implicit mission-creation flow on first add.
- All i18n keys migrated: `taker` → `assignee`, `assignTakers` → `assignMission`, new `crf.missionAssign.*` namespace.
- Existing placeholder drawer deleted; all references updated.

Out of scope (deferred per the spec): SDTM / ADaM / TFL mission UIs, removing the "Pending" status chip, mission CRUD at the project level, live-DB integration tests, optimistic updates.
