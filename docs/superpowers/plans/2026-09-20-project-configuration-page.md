# Project Configuration Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Project Configuration page inside the workspace window with a right-side nav (language + tags on General, leader/worker editing on Members, Filepath placeholder), gated to project leaders, and migrate the desktop's two wire boundaries onto the server's new `ProjectConfiguration { language, tags }` shape.

**Architecture:** Single-page TSX composition behind `ConfigurationSidebar` (right-side nav). Section state is in-page (`useState<"general" | "members" | "filepath">`). Each section has its own per-section Save button driven by a "touched" flag (mirrors the `tagsTouched` pattern from `ProjectDrawer`). Wire migration is mechanical — `ProjectView.tags: Tag[]` becomes `ProjectView.configurations: { language, tags }` on both `src-tauri/src/http/project.rs` and `src/shared/api/types.ts`.

**Tech Stack:** TypeScript + React + TanStack Router + TanStack Query, MUI components from `@aegis/ui/mui` and icons from `@aegis/ui/icons`, vitest + @testing-library for tests, Rust 2024 + serde for the Tauri side.

## Global Constraints

- **Existing patterns to follow:**
  - `useProject(code, { enabled: true })` (in `apps/desktop/aegis-desktop/src/features/project-list/data/projects.ts`) is the auto-enabled read we reuse for the new page.
  - `useUpdateProject()` already invalidates `queryKeys.project.all` and `queryKeys.project.byCode(code)` on success — no need to add a new mutation hook for configuration; just compose the body.
  - `useIsProjectLeader(projectCode)` (in `apps/desktop/aegis-desktop/src/features/mission/data/leader.ts`) returns `boolean | null`; `null` while loading. We re-export a small wrapper hook for the page.
  - `TagEditor` (`apps/desktop/aegis-desktop/src/features/project-list/components/TagEditor.tsx`) is reused as-is in the General section.
  - `useListUsers()` (`apps/desktop/aegis-desktop/src/features/user/data/list.ts`) is the autocomplete source for the workers picker.
  - The `tagsTouched` pattern from `ProjectDrawer` (one-shot `useRef` flag in `TagEditor`) — we extend it with two more flags: `languageTouched` and `membersTouched`.
- **Wire naming:**
  - TypeScript identifiers are camelCase; the wire JSON is snake_case (the rename happens at the serde boundary on the server and at the TS interface on the desktop). The `ProjectLanguage` wire code is `"en"` or `"zh-CN"`, matched by per-variant `#[serde(rename = "...")]` on the Rust side and the string-literal union on the TS side.
  - A present `ProjectConfiguration.language === null` is rendered as "no preference" in the UI.
- **Members scope:** Only the project's main `members` team. `members.leaders` rendered as read-only chips; `members.workers` editable via Autocomplete + chip-with-X. Unblind teams are out of scope.
- **Filepath:** placeholder only — no Save button, no data hook.
- **Save semantics:** the body carries `configurations` only when `languageTouched || tagsTouched`, and `members` only when `membersTouched`. `None` on update means "leave alone" server-side.
- **Tests:** update existing fixtures (`project-drawer`, `project-table`, `project-list-page`, `projects`, `leader`) to use the new shape; add a new suite `test/features/project-workspace/project-configuration-page.test.tsx`.
- **Verification gate** (every task):
  ```bash
  pnpm --filter aegis-desktop typecheck
  pnpm --filter aegis-desktop test
  cargo check -p aegis-desktop
  cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
  ```

## File Structure

### Created (workspace feature)
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationMembersSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationFilepathSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-configuration.ts`
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`

### Modified
- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` — DTOs + bodies + tests
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` + `index.ts` — TS mirror + re-exports
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx`
- `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`
- `apps/desktop/aegis-desktop/src/test/features/project-list/{project-drawer,project-table,project-list-page,projects}.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx`
- `lib/packages/ui/src/i18n/locales/{en,zhCN}.ts` — new configuration keys
- `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx` (rewrite)
- `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts` (re-exports)

---

## Task 1: TS wire mirror — `shared/api/types.ts` + `index.ts`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`

**Interfaces:**
- Produces: `ProjectLanguage = "en" | "zh-CN"`, `ProjectConfiguration { language: ProjectLanguage | null; tags: Tag[] }` consumed by every later task and by the updated `ProjectView`/`UpdateProjectBody`.

- [ ] **Step 1: Update `shared/api/types.ts`**

Replace the existing `Tag` block + `ProjectView` + `CreateProjectInput` + `UpdateProjectBody` with:

```ts
export interface Tag {
  key: string;
  value: string;
}

/** Wire code is one of two well-known strings; mirrors
 *  `src-tauri/src/http/project.rs::ProjectLanguage`. */
export type ProjectLanguage = "en" | "zh-CN";

/** Project configuration payload: optional locale plus the tag list.
 *  Server treats `Some(config)` on update as whole-replace. */
export interface ProjectConfiguration {
  language: ProjectLanguage | null;
  tags: Tag[];
}

export interface ProjectView {
  id: number;
  code: string;
  description: string;
  members: ProjectMembersView;
  unblindMembers: ProjectMembersView;
  configurations: ProjectConfiguration;
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateProjectInput {
  code: string;
  description: string;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  configurations?: ProjectConfiguration;
}

export interface UpdateProjectBody {
  code?: string;
  description?: string;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  configurations?: ProjectConfiguration;
}
```

Drop nothing else — `ProjectMembers`, `ProjectMembersView`, `UserSummary`, `Role`, etc. all stay.

- [ ] **Step 2: Update `shared/api/index.ts` re-exports**

Add `ProjectConfiguration` and `ProjectLanguage` to the `export type { ... }` block (right after the existing `ProjectView` line):

```ts
export type {
  // ...existing entries...
  ProjectConfiguration,
  ProjectLanguage,
  ProjectView,
  // ...
} from "./types";
```

- [ ] **Step 3: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: red — every consumer that referenced `tags` on `ProjectView` or in `UpdateProjectBody` will now fail to compile. That's fine; Task 4 fixes it.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/api/index.ts
git commit -m "wire(desktop): mirror ProjectConfiguration { language, tags }

The TS mirror of the project's wire shape gains:
  - ProjectLanguage = \"en\" | \"zh-CN\"
  - ProjectConfiguration { language, tags }
ProjectView / CreateProjectInput / UpdateProjectBody rename their
top-level tags field to configurations: ProjectConfiguration.
Existing consumers still reference the old shape; they compile
fix in Task 4.

Spec coverage: TS wire mirror per the project-configuration-page
design.

Verification: pnpm --filter aegis-desktop typecheck (intentionally
red until Task 4)"
```

---

## Task 2: Tauri wire — `src-tauri/src/http/project.rs` DTOs + tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`

**Interfaces:**
- Produces: serde-shaped `ProjectLanguage`, `ProjectConfigurationDataRequest`, `ProjectConfigurationViewResponse`. Updated `ProjectViewResponse`, `CreateProjectRequest`, `UpdateProjectRequest` carrying `configurations: Option<ProjectConfigurationDataRequest>` / `configurations: ProjectConfigurationViewResponse`. Consumed by `src-tauri/src/commands/*` (which already defer to `http::project::*`).

- [ ] **Step 1: Update the imports + add new DTOs**

At the top of `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`, after the existing `use` block, add the new DTOs and re-export the matching apis types. The cleanest split keeps all DTOs in this file (matching the existing convention — `TagDataRequest`, `TagViewResponse`, `ProjectMemberDataRequest`, etc. all live here). Add right above the existing `ProjectViewResponse`:

```rust
/// Wire-level mirror of [`apis::project::ProjectLanguage`]. Two variants;
/// per-variant `#[serde(rename = ...)]` so the wire codes are exactly
/// `"en"` and `"zh-CN"` — matching the i18n `Locale` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl From<apis::project::ProjectLanguage> for ProjectLanguage {
    fn from(l: apis::project::ProjectLanguage) -> Self {
        match l {
            apis::project::ProjectLanguage::English => Self::English,
            apis::project::ProjectLanguage::SimplifiedChinese => Self::SimplifiedChinese,
        }
    }
}

impl From<ProjectLanguage> for apis::project::ProjectLanguage {
    fn from(l: ProjectLanguage) -> Self {
        match l {
            ProjectLanguage::English => apis::project::ProjectLanguage::English,
            ProjectLanguage::SimplifiedChinese => apis::project::ProjectLanguage::SimplifiedChinese,
        }
    }
}

/// Wire-level request body for a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationData` field-for-field; absent
/// `language` and empty `tags` are skipped on serialize so a
/// present-but-empty config round-trips as `{}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
}

impl From<ProjectConfigurationDataRequest> for apis::project::ProjectConfigurationData {
    fn from(c: ProjectConfigurationDataRequest) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c
                .tags
                .into_iter()
                .map(|t| apis::project::TagData {
                    key: t.key,
                    value: t.value,
                })
                .collect(),
        }
    }
}

/// Wire-level projection of a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationView`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
}

impl From<apis::project::ProjectConfigurationView> for ProjectConfigurationViewResponse {
    fn from(c: apis::project::ProjectConfigurationView) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c.tags.into_iter().map(Into::into).collect(),
        }
    }
}
```

- [ ] **Step 2: Replace the `tags` field on `ProjectViewResponse`**

Replace the existing `ProjectViewResponse`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub configurations: ProjectConfigurationViewResponse,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 3: Replace `tags` on `CreateProjectRequest` and `UpdateProjectRequest`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}
```

- [ ] **Step 4: Update the existing tests**

In the `tests` module at the bottom of the file:

1. `list_returns_projects` — change the JSON body to include `"configurations": { "language": null, "tags": [{ "key": "Product", "value": "DEMO-001" }] }` and update the assertion:

```rust
#[tokio::test]
async fn list_returns_projects() {
    let server = MockServer::start().await;
    let store = Arc::new(MemoryStore::default());
    store.set_access_token("AT").await.unwrap();
    store.set_refresh_token("RT").await.unwrap();
    server
        .register(
            Mock::given(method("GET"))
                .and(path("/api/project"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({
                        "projects": [{
                            "id": 1, "code": "p", "description": "",
                            "configurations": {
                                "language": "en",
                                "tags": [{ "key": "Product", "value": "DEMO-001" }]
                            },
                            "members": { "leaders": [], "workers": [] },
                            "unblindMembers": { "leaders": [], "workers": [] },
                            "active": true,
                            "createdAt": "2026-01-01T00:00:00Z",
                            "updatedAt": "2026-01-02T00:00:00Z"
                        }]
                    }),
                )),
        )
        .await;
    let c = HttpClient::new(server.uri(), store);
    let projects = list(&c).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].code, "p");
    assert_eq!(
        projects[0].configurations.language,
        Some(ProjectLanguage::English)
    );
    assert_eq!(projects[0].configurations.tags.len(), 1);
    assert_eq!(projects[0].configurations.tags[0].key, "Product");
}
```

2. `update_skips_none_fields` — keep as-is (no `tags` key referenced).

3. Add two new tests:

```rust
#[test]
fn project_language_serializes_to_wire_codes() {
    assert_eq!(
        serde_json::to_string(&ProjectLanguage::English).unwrap(),
        "\"en\""
    );
    assert_eq!(
        serde_json::to_string(&ProjectLanguage::SimplifiedChinese).unwrap(),
        "\"zh-CN\""
    );
}

#[test]
fn project_configuration_data_request_omits_empty() {
    let body = ProjectConfigurationDataRequest::default();
    let j = serde_json::to_string(&body).unwrap();
    assert_eq!(j, "{}");

    let body = ProjectConfigurationDataRequest {
        language: Some(ProjectLanguage::SimplifiedChinese),
        tags: vec![TagDataRequest {
            key: "Product".into(),
            value: "DEMO-001".into(),
        }],
    };
    let j = serde_json::to_string(&body).unwrap();
    assert_eq!(
        j,
        r#"{"language":"zh-CN","tags":[{"key":"Product","value":"DEMO-001"}]}"#
    );
}
```

4. Add a "no `tags` field" assertion to `update_skips_none_fields` confirming `configurations` is also skipped when absent (the test body already exercises this implicitly, but make it explicit):

```rust
#[test]
fn update_skips_none_fields() {
    let body = UpdateProjectRequest {
        active: Some(false),
        ..Default::default()
    };
    let j = serde_json::to_string(&body).unwrap();
    assert_eq!(j, r#"{"active":false}"#);
    // configurations, members, etc. must not appear
    assert!(!j.contains("configurations"));
}
```

- [ ] **Step 5: Verify**

```bash
cargo test -p aegis-desktop --lib http::project
cargo check -p aegis-desktop
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/project.rs
git commit -m "wire(tauri): mirror ProjectConfiguration { language, tags }

The desktop's HTTP client DTOs gain:
  - ProjectLanguage { English, SimplifiedChinese } with
    per-variant serde rename to \"en\" / \"zh-CN\".
  - ProjectConfigurationDataRequest (request body, skips absent
    language / empty tags on serialize).
  - ProjectConfigurationViewResponse (server projection).

ProjectViewResponse, CreateProjectRequest, and UpdateProjectRequest
rename their top-level tags field to configurations: Option<...>
or ProjectConfigurationViewResponse. The Rust unit tests cover the
serde shape end-to-end (wire codes, skip-on-absent, round-trip).

Spec coverage: Tauri wire mirror per the project-configuration-page
design.

Verification: cargo test -p aegis-desktop --lib http::project &&
cargo check -p aegis-desktop && cargo clippy -p aegis-desktop
--all-targets --all-features -- -D warnings"
```

---

## Task 3: Wire consumers — `ProjectDrawer`, `ProjectTable`, `ProjectListPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`

**Interfaces:**
- Consumes: `ProjectConfiguration` + `ProjectLanguage` from Task 1.

- [ ] **Step 1: Update `ProjectDrawer.tsx`**

Inside `ProjectDrawer`, change the imports at the top:

```ts
import {
  type ApiError,
  type CreateProjectInput,
  type ProjectConfiguration,
  type ProjectLanguage,
  type Tag,
  type UpdateProjectBody,
  type UserSummary,
} from "../../../shared/api";
```

Inside the component function, change the tag-related state:

```ts
const [language, setLanguage] = useState<ProjectLanguage | null>(null);
const [tags, setTags] = useState<Tag[]>([]);
const [languageTouched, setLanguageTouched] = useState(false);
const [tagsTouched, setTagsTouched] = useState(false);
```

In the `useEffect` that resets form on `mode === "create"`, add:

```ts
setLanguage(null);
setTags([]);
setLanguageTouched(false);
setTagsTouched(false);
```

(Keep the existing resets for code, description, members, active.)

In the seed-from-fetch effect, replace the `setTags(r.data.tags)` line with:

```ts
setLanguage(r.data.configurations.language);
setTags(r.data.configurations.tags);
setLanguageTouched(false);
setTagsTouched(false);
```

In `onSubmit`, replace the create-mode and edit-mode bodies:

```ts
async function onSubmit() {
  const members = {
    leaders: memberLeaders.map((u) => u.code),
    workers: memberWorkers.map((u) => u.code),
  };
  const unblindMembers = {
    leaders: unblindLeaders.map((u) => u.code),
    workers: unblindWorkers.map((u) => u.code),
  };
  const configurations: ProjectConfiguration = { language, tags };
  try {
    if (mode === "create") {
      const input: CreateProjectInput = {
        code: formCode.trim(),
        description: description.trim(),
        members,
        unblindMembers,
        configurations,
      };
      await create.mutateAsync(input);
    } else if (mode === "edit" && code) {
      const body: UpdateProjectBody = {
        description: description.trim(),
        active,
        members,
        unblindMembers,
        ...(languageTouched || tagsTouched ? { configurations } : {}),
      };
      await update.mutateAsync({ code, body });
    }
    onClose();
  } catch {
    /* error surfaced below via create.error / update.error */
  }
}
```

Add a small `Language select` near the TagEditor. Use the `LanguageDropdown` shape but inline a one-off MUI Select to avoid a new shared component:

```tsx
import { FormControl, InputLabel, MenuItem, Select } from "@aegis/ui/mui";

// inside the Drawer body, right above <TagEditor />:
<FormControl size="small" sx={{ minWidth: 200 }}>
  <InputLabel id="project-language-label">
    {t("project.field.language")}
  </InputLabel>
  <Select<ProjectLanguage | ""> 
    labelId="project-language-label"
    label={t("project.field.language")}
    value={language ?? ""}
    onChange={(e) => {
      const v = e.target.value;
      setLanguage(v === "" ? null : (v as ProjectLanguage));
      if (!languageTouched) setLanguageTouched(true);
    }}
  >
    <MenuItem value="">{t("project.field.language.none")}</MenuItem>
    <MenuItem value="en">{t("language.english")}</MenuItem>
    <MenuItem value="zh-CN">{t("language.simplifiedChinese")}</MenuItem>
  </Select>
</FormControl>
```

Add the new i18n keys to `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`:

```ts
'project.field.language': 'Preferred language',
'project.field.language.none': '(none)',
```

```ts
'project.field.language': '首选语言',
'project.field.language.none': '（无）',
```

(The zh-CN keys are added in the i18n task below; for now put them inline in this task to keep the diff self-contained. The i18n task will then add the page-section keys.)

- [ ] **Step 2: Update `ProjectTable.tsx`**

Replace the two `row.tags.*` reads in the chip rendering with `row.configurations.tags.*`:

```tsx
{row.configurations.tags.map((tag, i) => (
  <Chip
    key={`tag-${i}-${tag.key}-${tag.value}`}
    size="small"
    label={tag.value}
    title={tag.key}
  />
))}
{row.configurations.tags.length === 0 && <span>—</span>}
```

- [ ] **Step 3: Update `ProjectListPage.tsx`**

Replace the `row.tags.some(...)` filter with `row.configurations.tags.some(...)`:

```ts
const inTag = row.configurations.tags.some((tag) =>
  tag.value.toLowerCase().includes(q),
);
```

- [ ] **Step 4: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: still red for the test fixtures in the project-list test files (fixed in Task 4).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx \
        apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx \
        apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx
git commit -m "wire(project-list): render and edit configurations { language, tags }

ProjectDrawer splits the prior tags state into language + tags with
independent touched flags; the create / update bodies now carry
configurations: { language, tags } under the whole-replace
semantics the server expects. A language Select sits above the
existing TagEditor.

ProjectTable and ProjectListPage render and filter on
row.configurations.tags instead of the old top-level tags array.

Spec coverage: consumer-side wire migration per the
project-configuration-page design.

Verification: pnpm --filter aegis-desktop typecheck (tests in Task 4)"
```

---

## Task 4: Test fixture updates — `project-list/*` and `mission/leader`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-table.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/projects.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx`

**Interfaces:**
- Consumes: `ProjectConfiguration` from Task 1.

- [ ] **Step 1: Add a shared `makeProject` test helper**

Create `apps/desktop/aegis-desktop/src/test/helpers/project-fixture.ts`:

```ts
import type { ProjectConfiguration, ProjectView } from "../../shared/api";

export function makeProject(overrides: Partial<ProjectView> = {}): ProjectView {
  const baseConfig: ProjectConfiguration = { language: null, tags: [] };
  return {
    id: 1,
    code: "alpha",
    description: "Alpha description",
    members: { leaders: [], workers: [] },
    unblindMembers: { leaders: [], workers: [] },
    configurations: baseConfig,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

export function withTags(
  config: ProjectConfiguration,
  tags: { key: string; value: string }[],
): ProjectConfiguration {
  return { ...config, tags };
}
```

(Each test file that needs a project fixture will use `makeProject({ ... })` going forward.)

- [ ] **Step 2: Update `project-drawer.test.tsx`**

1. Drop the existing `projectFixture` const.
2. Import `makeProject`.
3. Replace each `projectFixture` / `{ ...projectFixture, tags: [...] }` usage:
   - `create_project: () => projectFixture,` → `create_project: () => makeProject(),`
   - `get_project_by_code: () => projectFixture,` → `get_project_by_code: () => makeProject(),`
   - `get_project_by_code: () => ({ ...projectFixture, tags: [{ key: "Product", value: "DEMO-001" }] }),` → `get_project_by_code: () => makeProject({ configurations: { language: null, tags: [{ key: "Product", value: "DEMO-001" }] } }),`
4. Update the test assertions:
   - `expect(invoke).toHaveBeenCalledWith("create_project", expect.objectContaining({ tags: [{ key: "Product", value: "DEMO-007" }] }))` → `expect.objectContaining({ configurations: { language: null, tags: [{ key: "Product", value: "DEMO-007" }] } })`
   - The "submit omits tags from the body when user did NOT touch" test: replace `expect(body).not.toHaveProperty("tags")` with `expect(body).not.toHaveProperty("configurations")`.
   - The "edit-mode submit sends the new tags array" test: replace `expect(body.tags).toEqual([...])` with `expect(body.configurations).toEqual({ language: null, tags: [{ key: "Product", value: "DEMO-002" }] })`.
5. The "create-mode submit includes the assembled tags array" test: assert against `configurations.tags` similarly.

- [ ] **Step 3: Update `project-table.test.tsx`**

Locate every fixture that sets `tags: [...]` directly on a `ProjectView` and rewrite it to `configurations: { language: null, tags: [...] }`. Update every `row.tags.*` assertion to `row.configurations.tags.*`.

- [ ] **Step 4: Update `project-list-page.test.tsx` and `projects.test.tsx`**

Same mechanical fix: rewrite fixtures + assertions to use `configurations.tags` and (if any test constructs an `UpdateProjectBody` directly) `configurations`.

- [ ] **Step 5: Update `leader.test.tsx`**

Replace the `projectAliceLeader` fixture's `tags: []` line with `configurations: { language: null, tags: [] }`. No other changes.

- [ ] **Step 6: Verify**

```bash
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/helpers/project-fixture.ts \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-table.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/projects.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx
git commit -m "tests(wire): update fixtures for configurations { language, tags }

Every fixture that built a ProjectView / UpdateProjectBody with a
top-level tags: [...] now uses configurations: { language: null,
tags: [...] }. A small makeProject / withTags helper in
test/helpers/project-fixture.ts centralises the shape so future
tests don't drift.

Spec coverage: wire-migration test fixtures per the
project-configuration-page design.

Verification: pnpm --filter aegis-desktop test &&
pnpm --filter aegis-desktop typecheck"
```

---

## Task 5: i18n — add the configuration section keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Produces: keys `project.configuration.*` and `project.field.language` consumed by the page + sections in Tasks 7–11.

- [ ] **Step 1: Add the en keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, append the following keys right after the existing `'project.action.save'` line (keep the trailing comma):

```ts
  'project.field.language': 'Preferred language',
  'project.field.language.none': '(none)',
  'project.configuration.heading': 'Configuration — {projectCode}',
  'project.configuration.section.general': 'General',
  'project.configuration.section.members': 'Members',
  'project.configuration.section.filepath': 'Filepath',
  'project.configuration.readOnly': 'Read-only — project leaders only',
  'project.configuration.saveFailed': 'Failed to save: {message}',
  'project.configuration.general.language': 'Preferred language',
  'project.configuration.general.language.none': '(no preference)',
  'project.configuration.general.tags': 'Tags',
  'project.configuration.members.leadersHeading': 'Leaders (read-only)',
  'project.configuration.members.workersHeading': 'Workers',
  'project.configuration.members.add': 'Add worker',
  'project.configuration.members.empty': 'No workers yet',
  'project.configuration.filepath.placeholder': 'Coming soon',
```

- [ ] **Step 2: Add the zh-CN keys**

Same set, translated, in `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```ts
  'project.field.language': '首选语言',
  'project.field.language.none': '（无）',
  'project.configuration.heading': '配置 — {projectCode}',
  'project.configuration.section.general': '通用',
  'project.configuration.section.members': '成员',
  'project.configuration.section.filepath': '文件路径',
  'project.configuration.readOnly': '只读 — 仅项目负责人可编辑',
  'project.configuration.saveFailed': '保存失败：{message}',
  'project.configuration.general.language': '首选语言',
  'project.configuration.general.language.none': '（未指定）',
  'project.configuration.general.tags': '标签',
  'project.configuration.members.leadersHeading': '负责人（只读）',
  'project.configuration.members.workersHeading': '项目成员',
  'project.configuration.members.add': '新增成员',
  'project.configuration.members.empty': '暂无成员',
  'project.configuration.filepath.placeholder': '敬请期待',
```

- [ ] **Step 3: Verify**

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
```

Expected: green. Both `en` and `zhCN` are exhaustively keyed on `TranslationKey = keyof typeof en`, so a typo in `zhCN` will fail compilation.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "i18n: add project-configuration section + preferred-language keys

New keys cover the configuration page sidebar (General / Members /
Filepath), the read-only hint, the save error, the language Select
inside General, and the workers list inside Members.

Both en and zh-CN are added — the typed TranslationKey union on en
enforces parity at compile time, so a missing zh-CN entry fails
the package typecheck.

Spec coverage: configuration page i18n keys per the
project-configuration-page design.

Verification: pnpm --filter @aegis/ui typecheck &&
pnpm --filter @aegis/ui test"
```

---

## Task 6: Data hook — `useProjectConfiguration`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-configuration.ts`

**Interfaces:**
- Consumes: `useCurrentUser` from `features/auth`, `useProject(code, { enabled: true })` from `features/project-list/data/projects`.
- Produces: `useProjectConfiguration(projectCode): { data, isLeader, isLoading, isError, error }` consumed by the page in Task 11.

- [ ] **Step 1: Create the file**

```ts
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { api, type ApiError, type ProjectView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
import { useCurrentUser } from "../../auth";

/**
 * Configuration page data hook. Auto-fetches the project (the page is
 * long-lived, unlike the edit drawer that uses manual-trigger
 * `useProject`) and joins it with the current user to derive the
 * leader flag. The leader flag returns `null` while either query is
 * still loading — the page uses that to suppress Save buttons until
 * the truth is in.
 *
 * Re-implements the leader check (rather than calling
 * `useIsProjectLeader` from `features/mission`) because we need the
 * project data alongside the leader flag, and `useIsProjectLeader`
 * doesn't expose `data`. Both end up sharing the same
 * `queryKeys.project.byCode(code)` cache, so a second observer
 * refetching under our feet (e.g. the assign-mission drawer
 * mounting later) doesn't shake the leader flag.
 */
export function useProjectConfiguration(projectCode: string) {
  const currentUser = useCurrentUser();
  const projectQuery = useQuery<ProjectView, ApiError>({
    queryKey: queryKeys.project.byCode(projectCode),
    queryFn: () => api.getProjectByCode(projectCode),
    enabled: projectCode !== null && projectCode !== "",
    staleTime: 0,
  });

  const isLoading = currentUser.isLoading || projectQuery.isLoading;
  const isError = currentUser.isError || projectQuery.isError;
  const error = currentUser.error ?? projectQuery.error ?? null;

  const isLeader = useMemo<boolean | null>(() => {
    if (isLoading) return null;
    if (!currentUser.data || !projectQuery.data) return null;
    const myCode = currentUser.data.code;
    const project = projectQuery.data;
    return (
      project.members.leaders.some((u) => u.code === myCode) ||
      project.unblindMembers.leaders.some((u) => u.code === myCode)
    );
  }, [isLoading, currentUser.data, projectQuery.data]);

  return {
    data: projectQuery.data,
    isLeader,
    isLoading,
    isError,
    error,
  };
}
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green (the file isn't imported anywhere yet, but `typecheck` will compile-check it).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/data/project-configuration.ts
git commit -m "feat(data): add useProjectConfiguration hook

Auto-enabled get_project_by_code + current_user joined into a
single { data, isLeader, isLoading, isError, error } return.
isLeader is null while loading; re-derives on every render via a
small useMemo.

Spec coverage: configuration page data layer per the
project-configuration-page design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 7: Right sidebar — `ConfigurationSidebar`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx`

**Interfaces:**
- Consumes: nothing (pure UI).
- Produces: `ConfigurationSidebar` consumed by the page in Task 11.

- [ ] **Step 1: Create the file**

```tsx
import {
  Box,
  Divider,
  List,
  ListItemButton,
  ListItemText,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export type ConfigurationSection = "general" | "members" | "filepath";

export interface ConfigurationSidebarProps {
  value: ConfigurationSection;
  onChange: (next: ConfigurationSection) => void;
  width?: number;
}

const ORDER: ConfigurationSection[] = ["general", "members", "filepath"];

/**
 * Right-side nav for the configuration page. Fixed-width column with
 * a vertical List of three ListItemButtons. The active item gets the
 * MUI `selected` treatment (background + primary tint) so the user
 * always knows which section is on screen.
 *
 * No routing here — selection is in-page state, driven by `value` /
 * `onChange`. The parent owns the state so it can also rehydrate on
 * back-navigation or external deep-links later without restructuring
 * this component.
 */
export function ConfigurationSidebar({
  value,
  onChange,
  width = 220,
}: ConfigurationSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      component="nav"
      aria-label="configuration-section"
      sx={{
        width,
        flexShrink: 0,
        borderLeft: 1,
        borderColor: "divider",
        bgcolor: "background.paper",
      }}
    >
      <Divider />
      <List dense disablePadding>
        {ORDER.map((section) => (
          <ListItemButton
            key={section}
            selected={section === value}
            onClick={() => onChange(section)}
            data-testid={`config-section-${section}`}
          >
            <ListItemText
              primary={t(`project.configuration.section.${section}` as const)}
            />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx
git commit -m "feat(sidebar): ConfigurationSidebar right-side nav

Fixed-width right column with a vertical List of three
ListItemButtons (General / Members / Filepath). Selected item
gets MUI's selected treatment. Pure controlled component —
value / onChange are the parent's state.

Spec coverage: configuration sidebar per the
project-configuration-page design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 8: General section — `ConfigurationGeneralSection`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`

**Interfaces:**
- Consumes: `Tag` from `shared/api`, `useUpdateProject` from `features/project-list/data/projects`, `TagEditor` from `features/project-list/components/TagEditor`, `errorMessage` from `shared/api/error`.
- Produces: `ConfigurationGeneralSection` consumed by the page in Task 11. Props: `projectCode: string; readonly: boolean; initial: ProjectConfiguration`.

- [ ] **Step 1: Create the file**

```tsx
import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import {
  type ApiError,
  type ProjectConfiguration,
  type ProjectLanguage,
  type Tag,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useUpdateProject } from "../../project-list";
import { TagEditor } from "../../project-list/components/TagEditor";

export interface ConfigurationGeneralSectionProps {
  projectCode: string;
  readonly: boolean;
  initial: ProjectConfiguration;
}

/**
 * General section: language Select + TagEditor + per-section Save.
 * Mirrors `ProjectDrawer`'s "touched" semantics — `languageTouched`
 * and `tagsTouched` flip independently on the user's first
 * interaction, and the body carries `configurations` only when at
 * least one is true. Touched flags survive across renders but
 * reset on every re-mount, so a re-mount with new `initial` data
 * (e.g. after a successful save) starts a fresh session.
 *
 * `readonly` short-circuits the Save button entirely: a non-leader
 * sees the controls but can't edit them, so there's nothing to save.
 */
export function ConfigurationGeneralSection({
  projectCode,
  readonly,
  initial,
}: ConfigurationGeneralSectionProps) {
  const { t } = useI18n();
  const update = useUpdateProject();

  const [language, setLanguage] = useState<ProjectLanguage | null>(
    initial.language,
  );
  const [tags, setTags] = useState<Tag[]>(initial.tags);
  const languageTouchedRef = useRef(false);
  const tagsTouchedRef = useRef(false);

  // Reseed the local state whenever a fresh `initial` arrives
  // (e.g. after `useUpdateProject` invalidates the project cache
  // and the parent's hook re-fetches).
  const lastInitialRef = useRef(initial);
  useEffect(() => {
    if (lastInitialRef.current === initial) return;
    lastInitialRef.current = initial;
    setLanguage(initial.language);
    setTags(initial.tags);
    languageTouchedRef.current = false;
    tagsTouchedRef.current = false;
  }, [initial]);

  const languageTouched = languageTouchedRef.current;
  const tagsTouched = tagsTouchedRef.current;
  const dirty = languageTouched || tagsTouched;
  const submitDisabled = readonly || !dirty || update.isPending;

  async function onSave() {
    const configurations: ProjectConfiguration = { language, tags };
    await update.mutateAsync({
      code: projectCode,
      body: { configurations },
    });
    // The mutation invalidates the project cache; the parent's
    // `useProjectConfiguration` will refetch and our `lastInitialRef`
    // effect will reseed the local state, clearing the touched flags.
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
      <FormControl size="small" sx={{ maxWidth: 320 }} disabled={readonly}>
        <InputLabel id="config-language-label">
          {t("project.configuration.general.language")}
        </InputLabel>
        <Select<ProjectLanguage | "">
          labelId="config-language-label"
          label={t("project.configuration.general.language")}
          value={language ?? ""}
          onChange={(e) => {
            const v = e.target.value;
            setLanguage(v === "" ? null : (v as ProjectLanguage));
            languageTouchedRef.current = true;
          }}
          inputProps={{
            "data-testid": "config-language-input",
          }}
        >
          <MenuItem value="">
            {t("project.configuration.general.language.none")}
          </MenuItem>
          <MenuItem value="en">{t("language.english")}</MenuItem>
          <MenuItem value="zh-CN">
            {t("language.simplifiedChinese")}
          </MenuItem>
        </Select>
      </FormControl>

      <Box>
        <InputLabel sx={{ mb: 1 }}>
          {t("project.configuration.general.tags")}
        </InputLabel>
        <TagEditor
          value={tags}
          onChange={setTags}
          onTouched={() => {
            tagsTouchedRef.current = true;
          }}
        />
      </Box>

      {!readonly && (
        <Stack direction="row" spacing={1} sx={{ justifyContent: "flex-end" }}>
          {update.error && (
            <Alert severity="error" sx={{ flexGrow: 1 }}>
              {t("project.configuration.saveFailed", {
                message: errorMessage(update.error as ApiError),
              })}
            </Alert>
          )}
          <Button
            variant="contained"
            disabled={submitDisabled}
            onClick={() => void onSave()}
            data-testid="config-general-save"
          >
            {t("common.save")}
          </Button>
        </Stack>
      )}
    </Box>
  );
}
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx
git commit -m "feat(general): ConfigurationGeneralSection with Save

General section = language Select + TagEditor + per-section Save.
languageTouched / tagsTouched flip independently on first
interaction; Save fires update_project with
{ configurations: { language, tags } } when at least one flag is
true. Reseeds when parent's `initial` changes (post-save refetch).
Read-only mode hides Save.

Spec coverage: general section per the project-configuration-page
design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 9: Members section — `ConfigurationMembersSection`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationMembersSection.tsx`

**Interfaces:**
- Consumes: `useListUsers` from `features/user`, `useUpdateProject` from `features/project-list`, `ProjectMembersView` from `shared/api`.
- Produces: `ConfigurationMembersSection` consumed by the page in Task 11. Props: `projectCode: string; readonly: boolean; initial: ProjectMembersView`.

- [ ] **Step 1: Create the file**

```tsx
import { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { Close } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import {
  type ApiError,
  type ProjectMembersView,
  type UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useUpdateProject } from "../../project-list";
import { useListUsers } from "../../user";

export interface ConfigurationMembersSectionProps {
  projectCode: string;
  readonly: boolean;
  initial: ProjectMembersView;
}

/**
 * Members section: leaders rendered as read-only chips; workers
 * editable via Autocomplete + chip-with-X. Per-section Save mirrors
 * the General section: a single `membersTouched` flag flips on
 * first interaction, the body carries `members` only when touched.
 *
 * `members.leaders` is always re-sent back unchanged so a leader
 * change made through another surface (the project-list drawer,
 * etc.) survives this save.
 */
export function ConfigurationMembersSection({
  projectCode,
  readonly,
  initial,
}: ConfigurationMembersSectionProps) {
  const { t } = useI18n();
  const update = useUpdateProject();
  const users = useListUsers();

  const [leaders, setLeaders] = useState<UserSummary[]>(initial.leaders);
  const [workers, setWorkers] = useState<UserSummary[]>(initial.workers);
  const membersTouchedRef = useRef(false);

  const lastInitialRef = useRef(initial);
  useEffect(() => {
    if (lastInitialRef.current === initial) return;
    lastInitialRef.current = initial;
    setLeaders(initial.leaders);
    setWorkers(initial.workers);
    membersTouchedRef.current = false;
  }, [initial]);

  const touched = membersTouchedRef.current;
  const submitDisabled = readonly || !touched || update.isPending;

  // The autocomplete dropdown should exclude people already on the
  // team so the user can't pick the same code twice (duplicates are
  // rejected by the server anyway, but excluding them client-side
  // keeps the UX tidy).
  const usersExceptTeam = useMemo<UserSummary[]>(() => {
    const teamCodes = new Set([
      ...leaders.map((u) => u.code),
      ...workers.map((u) => u.code),
    ]);
    return (users.data ?? []).filter((u) => !teamCodes.has(u.code));
  }, [users.data, leaders, workers]);

  async function onSave() {
    await update.mutateAsync({
      code: projectCode,
      body: {
        members: {
          leaders: leaders.map((u) => u.code),
          workers: workers.map((u) => u.code),
        },
      },
    });
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
      <Box>
        <Typography variant="subtitle2" gutterBottom>
          {t("project.configuration.members.leadersHeading")}
        </Typography>
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ flexWrap: "wrap", gap: 0.5 }}
          data-testid="config-leaders"
        >
          {leaders.map((u) => (
            <Chip key={u.code} variant="outlined" label={`${u.code} — ${u.name}`} />
          ))}
          {leaders.length === 0 && (
            <Typography color="text.secondary">—</Typography>
          )}
        </Stack>
      </Box>

      <Box>
        <Typography variant="subtitle2" gutterBottom>
          {t("project.configuration.members.workersHeading")}
        </Typography>
        {!readonly && (
          <Autocomplete<UserSummary, true>
            multiple
            options={usersExceptTeam}
            getOptionLabel={(u) => `${u.code} — ${u.name}`}
            onChange={(_e, value) => {
              setWorkers(value);
              membersTouchedRef.current = true;
            }}
            renderInput={(params) => (
              <TextField
                {...params}
                size="small"
                placeholder={t("project.configuration.members.add")}
                inputProps={{
                  ...params.inputProps,
                  "data-testid": "config-workers-input",
                }}
              />
            )}
            sx={{ mb: 1 }}
          />
        )}
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ flexWrap: "wrap", gap: 0.5 }}
          data-testid="config-workers"
        >
          {workers.map((u) =>
            readonly ? (
              <Chip
                key={u.code}
                variant="filled"
                label={`${u.code} — ${u.name}`}
              />
            ) : (
              <Chip
                key={u.code}
                variant="filled"
                label={`${u.code} — ${u.name}`}
                onDelete={() => {
                  setWorkers((prev) =>
                    prev.filter((w) => w.code !== u.code),
                  );
                  membersTouchedRef.current = true;
                }}
                deleteIcon={<Close />}
              />
            ),
          )}
          {workers.length === 0 && (
            <Typography color="text.secondary">
              {t("project.configuration.members.empty")}
            </Typography>
          )}
        </Stack>
      </Box>

      {!readonly && (
        <Stack direction="row" spacing={1} sx={{ justifyContent: "flex-end" }}>
          {update.error && (
            <Alert severity="error" sx={{ flexGrow: 1 }}>
              {t("project.configuration.saveFailed", {
                message: errorMessage(update.error as ApiError),
              })}
            </Alert>
          )}
          <Button
            variant="contained"
            disabled={submitDisabled}
            onClick={() => void onSave()}
            data-testid="config-members-save"
          >
            {t("common.save")}
          </Button>
        </Stack>
      )}
    </Box>
  );
}
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationMembersSection.tsx
git commit -m "feat(members): ConfigurationMembersSection with Save

Members section = leaders (read-only chips) + workers (Autocomplete
+ chip-with-X when not readonly). Single membersTouched flag flips
on first add/remove; Save fires update_project with
{ members: { leaders, workers } } (leaders always re-sent
unchanged so external leader edits survive).

Spec coverage: members section per the project-configuration-page
design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 10: Filepath section — `ConfigurationFilepathSection`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationFilepathSection.tsx`

**Interfaces:**
- Consumes: nothing (pure UI).
- Produces: `ConfigurationFilepathSection` consumed by the page in Task 11. No props.

- [ ] **Step 1: Create the file**

```tsx
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

/**
 * Filepath section. Placeholder only — out of scope for this spec.
 * The shape mirrors the General / Members sections so the page
 * composition stays uniform: a centered "coming soon" message in a
 * single Box.
 */
export function ConfigurationFilepathSection() {
  const { t } = useI18n();
  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 1,
        py: 8,
      }}
      data-testid="config-filepath-placeholder"
    >
      <Typography variant="h6">
        {t("project.configuration.section.filepath")}
      </Typography>
      <Typography color="text.secondary">
        {t("project.configuration.filepath.placeholder")}
      </Typography>
    </Box>
  );
}
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationFilepathSection.tsx
git commit -m "feat(filepath): ConfigurationFilepathSection placeholder

Centered heading + 'Coming soon' message. No data hooks, no Save
button — out of scope for this spec.

Spec coverage: filepath placeholder per the
project-configuration-page design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 11: Page composition — `ProjectConfigurationPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`

**Interfaces:**
- Consumes: `useProjectConfiguration` from Task 6, `ConfigurationSidebar` from Task 7, three sections from Tasks 8–10.

- [ ] **Step 1: Update the page**

```tsx
import { useState } from "react";
import { Alert, Box, CircularProgress, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

import { useProjectConfiguration } from "../data/project-configuration";
import {
  ConfigurationFilepathSection,
  ConfigurationGeneralSection,
  ConfigurationMembersSection,
  ConfigurationSidebar,
  type ConfigurationSection,
} from "../components/ConfigurationSidebar";

/**
 * Project configuration page. Right-side nav picks between
 * General / Members / Filepath. Project leaders can edit; everyone
 * else sees the controls in read-only mode (no Save buttons).
 *
 * The leader flag comes from `useProjectConfiguration`: `null`
 * while either the user or the project query is still loading
 * (so we don't briefly flash the read-only banner). Once it
 * settles to `false`, the read-only banner appears and every
 * section's Save button is suppressed by the `readonly` prop.
 */
export function ProjectConfigurationPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const [section, setSection] = useState<ConfigurationSection>("general");
  const { data, isLeader, isLoading, isError, error } =
    useProjectConfiguration(projectCode);

  if (isLoading) {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          minHeight: "50vh",
        }}
        data-testid="config-loading"
      >
        <CircularProgress />
      </Box>
    );
  }

  if (isError || !data) {
    return (
      <Box sx={{ p: 4 }}>
        <Alert severity="error">
          {t("project.loadFailed", {
            message: error?.message ?? "",
          })}
        </Alert>
      </Box>
    );
  }

  const readonly = isLeader !== true;

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Box
        component="main"
        sx={{ flexGrow: 1, p: 4, display: "flex", flexDirection: "column", gap: 3 }}
      >
        <Typography variant="h4" gutterBottom>
          {t("project.configuration.heading", { projectCode })}
        </Typography>
        {readonly && (
          <Alert severity="info" data-testid="config-readonly">
            {t("project.configuration.readOnly")}
          </Alert>
        )}

        {section === "general" && (
          <ConfigurationGeneralSection
            projectCode={projectCode}
            readonly={readonly}
            initial={data.configurations}
          />
        )}
        {section === "members" && (
          <ConfigurationMembersSection
            projectCode={projectCode}
            readonly={readonly}
            initial={data.members}
          />
        )}
        {section === "filepath" && <ConfigurationFilepathSection />}
      </Box>
      <ConfigurationSidebar value={section} onChange={setSection} />
    </Box>
  );
}
```

(Import `ConfigurationFilepathSection`, `ConfigurationGeneralSection`, `ConfigurationMembersSection`, `ConfigurationSidebar`, `ConfigurationSection` from the new components. The single barrel re-export below makes the import paths cleaner.)

- [ ] **Step 2: Update the barrel `features/project-workspace/index.ts`**

```ts
// Public API of the project-workspace feature.
export { useProjectConfiguration } from "./data/project-configuration";
export {
  ConfigurationFilepathSection,
  ConfigurationGeneralSection,
  ConfigurationMembersSection,
  ConfigurationSidebar,
  type ConfigurationSection,
} from "./components/ConfigurationSidebar";
```

- [ ] **Step 3: Verify**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx \
        apps/desktop/aegis-desktop/src/features/project-workspace/index.ts
git commit -m "feat(page): ProjectConfigurationPage composes sidebar + sections

Right-column nav + main column with heading + read-only banner +
the active section. Leader flag drives a `readonly` prop threaded
through each section. Loading + error paths render
CircularProgress / Alert. Barrel index re-exports the new pieces
so callers can `import { ... } from features/project-workspace`.

Spec coverage: page composition per the project-configuration-page
design.

Verification: pnpm --filter aegis-desktop typecheck"
```

---

## Task 12: Tests — `project-configuration-page.test.tsx`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`

**Interfaces:**
- Consumes: `makeProject` from `test/helpers/project-fixture`, `renderWithFullRouter` from `test/helpers/file-route-utils`.

- [ ] **Step 1: Create the test file**

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import type { ProjectView, UserView } from "../../../shared/api";
import { makeProject } from "../../helpers/project-fixture";

const leader: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const other: UserView = {
  id: 2,
  code: "carol",
  name: "Carol",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = makeProject({
  code: "alpha",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "bob", name: "Bob" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  configurations: {
    language: "en",
    tags: [{ key: "Product", value: "DEMO-001" }],
  },
});

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() {
      return 0;
    },
  });
});
afterEach(() => cleanup());

function renderPage(initialEntry = "/project/alpha/configuration") {
  return renderWithFullRouter({
    initialEntries: [initialEntry],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("ProjectConfigurationPage — leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
    });
  });

  it("renders the heading + project code", async () => {
    await renderPage();
    expect(
      await screen.findByRole("heading", { name: /Configuration — alpha/i }),
    ).toBeInTheDocument();
  });

  it("renders the three-section sidebar with General selected", async () => {
    await renderPage();
    const general = await screen.findByTestId("config-section-general");
    const members = await screen.findByTestId("config-section-members");
    const filepath = await screen.findByTestId("config-section-filepath");
    expect(general).toHaveAttribute("aria-selected", "true");
    expect(members).toHaveAttribute("aria-selected", "false");
    expect(filepath).toHaveAttribute("aria-selected", "false");
  });

  it("General section shows the seeded language + tag values", async () => {
    await renderPage();
    expect(
      await screen.findByDisplayValue("English"),
    ).toBeInTheDocument();
    expect(
      (await screen.findAllByDisplayValue("DEMO-001")).length,
    ).toBeGreaterThan(0);
  });

  it("does NOT render the read-only banner for a project leader", async () => {
    await renderPage();
    await screen.findByTestId("config-section-general");
    expect(screen.queryByTestId("config-readonly")).not.toBeInTheDocument();
  });

  it("Save button starts disabled and enables after a change", async () => {
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    // Switch language to Simplified Chinese.
    await userEvent.click(screen.getByLabelText(/preferred language/i));
    await userEvent.click(screen.getByRole("option", { name: /simplified chinese/i }));

    await waitFor(() =>
      expect(save).not.toBeDisabled(),
    );
  });

  it("clicking Save fires update_project with configurations { language, tags }", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      update_project: () => projectFixture,
    });
    await renderPage();

    // Change the language to Simplified Chinese.
    await userEvent.click(screen.getByLabelText(/preferred language/i));
    await userEvent.click(screen.getByRole("option", { name: /simplified chinese/i }));

    const save = await screen.findByTestId("config-general-save");
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              language: "zh-CN",
              tags: [{ key: "Product", value: "DEMO-001" }],
            }),
          }),
        }),
      ),
    );
  });

  it("clicking the Members sidebar item shows the Members section", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-members"));
    expect(await screen.findByTestId("config-leaders")).toBeInTheDocument();
    expect(await screen.findByTestId("config-workers")).toBeInTheDocument();
  });

  it("adding a worker enables Save and submit carries the new code", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_users: () => [leader, other, { ...other, code: "dave", name: "Dave" }],
      update_project: () => projectFixture,
    });
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-members"));

    // Add Dave to the workers.
    const input = await screen.findByTestId("config-workers-input");
    await userEvent.click(input);
    await userEvent.click(screen.getByRole("option", { name: /dave/i }));

    const save = await screen.findByTestId("config-members-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            members: expect.objectContaining({
              leaders: ["alice"],
              workers: expect.arrayContaining(["bob", "dave"]),
            }),
          }),
        }),
      ),
    );
  });
});

describe("ProjectConfigurationPage — non-leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => other,
      get_project_by_code: () => projectFixture,
    });
  });

  it("renders the read-only banner and hides Save buttons", async () => {
    await renderPage();
    expect(await screen.findByTestId("config-readonly")).toBeInTheDocument();
    expect(screen.queryByTestId("config-general-save")).not.toBeInTheDocument();
    expect(screen.queryByTestId("config-members-save")).not.toBeInTheDocument();
  });

  it("language Select is disabled for non-leaders", async () => {
    await renderPage();
    const select = await screen.findByLabelText(/preferred language/i);
    expect(select).toBeDisabled();
  });

  it("workers Autocomplete is hidden for non-leaders", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-members"));
    expect(
      screen.queryByTestId("config-workers-input"),
    ).not.toBeInTheDocument();
    // Workers themselves still render as plain chips.
    expect(await screen.findByTestId("config-workers")).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — filepath placeholder", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
    });
  });

  it("clicking Filepath shows the placeholder", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-filepath"));
    expect(
      await screen.findByTestId("config-filepath-placeholder"),
    ).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — error path", () => {
  it("renders an Alert when get_project_by_code fails", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => {
        throw {
          kind: "http",
          status: 500,
          code: "server",
          message: "boom",
        };
      },
    });
    await renderPage();
    expect(await screen.findByRole("alert")).toHaveTextContent(/server/i);
  });
});
```

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx
git commit -m "feat(tests): ProjectConfigurationPage suite

Covers: heading + sidebar, language seeded from the wire, save
disabled + enables on change + fires update_project with the new
language, members section add-worker flow, non-leader read-only
banner + hidden save buttons + disabled Select, filepath
placeholder, error path.

Spec coverage: end-to-end tests per the project-configuration-page
design.

Verification: pnpm --filter aegis-desktop test -- src/test
features/project-workspace/project-configuration-page.test.tsx"
```

---

## Self-Review

**1. Spec coverage**
- Wire mirror (TS) → Task 1.
- Wire mirror (Tauri) + tests → Task 2.
- Wire consumers (`ProjectDrawer`, `ProjectTable`, `ProjectListPage`) → Task 3.
- Test fixture updates → Task 4.
- i18n keys → Task 5.
- Data hook `useProjectConfiguration` → Task 6.
- Right sidebar → Task 7.
- General section (language + tags + save) → Task 8.
- Members section (leaders read-only + workers editable + save) → Task 9.
- Filepath placeholder → Task 10.
- Page composition + leader gate → Task 11.
- End-to-end tests → Task 12.

**2. Placeholder scan** — no "TBD" / "TODO" / "implement later" anywhere. Every step shows concrete code or commands.

**3. Type consistency** — the `ProjectConfiguration` shape is identical across Tasks 1, 2, 3, 4, 8, 9. The `readonly` prop is consistent across sections and the page. The "touched" pattern is uniform across General + Members sections. The `useUpdateProject` mutation hook is reused in both sections without divergence. The `makeProject` test helper in Task 4 is reused in Task 12's tests.

**4. Task ordering** — wire first (1–4), i18n (5), then feature build bottom-up (6 data → 7 sidebar → 8–10 sections → 11 page → 12 tests). Each task compiles independently given the previous ones (Task 3 leaves typecheck red on purpose, fixed in Task 4).

Plan complete and saved to `docs/superpowers/plans/2026-09-20-project-configuration-page.md`.