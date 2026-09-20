# Project Configuration Page Design

## Goal

Replace the placeholder `ProjectConfigurationPage` with a real configuration
surface inside the project workspace window. The page exposes three
configuration modules — **General**, **Members**, and **Filepath** — through a
right-side sidebar nav, edits gated to project leaders (the same authority
that gates the existing mission-assign and CRF-edit affordances), and is
read-only for everyone else.

Alongside the new page, migrate the desktop's two wire boundaries
(`apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` and
`apps/desktop/aegis-desktop/src/shared/api/types.ts`) onto the server's
new `ProjectConfiguration { language, tags }` shape so the configuration
round-trips through `configurations: { language, tags }` instead of the
old top-level `tags` field.

The Filepath module is not implemented in this spec — it ships as a
placeholder.

## Architecture

The page is a single TSX file that composes three section components
behind a right-side `ConfigurationSidebar`. Section navigation is
in-page state; the existing
`apps/desktop/aegis-desktop/src/routes/_authed/project/$projectCode/configuration.tsx`
route file stays a leaf. State management follows the same
`touched → Save → mutate` pattern `ProjectDrawer` already uses for tags
and members, so a section's mutation only fires when the user actually
edited something in that session.

```
        ┌───────────────────────────────────────────┬───────────────────┐
        │  Heading + read-only hint (top bar)      │  Right sidebar    │
        ├───────────────────────────────────────────┤                   │
        │                                           │  General          │
        │  Active section                           │  Members          │
        │  (General | Members | Filepath)           │  Filepath         │
        │                                           │                   │
        │  …form body…                              │                   │
        │                                           │                   │
        │  [Save]                                   │                   │
        └───────────────────────────────────────────┴───────────────────┘
```

Wire-migration touches the Tauri side (the actual HTTP client) and the
frontend TS mirror (1:1 hand-maintained). The two stay in sync because
each shape is duplicated by hand — the rename happens at the serde
boundary on the server side and at the TS interface on the desktop
side. No transform layer.

### Existing components we lean on

| Component / hook | Why |
| --- | --- |
| `useIsProjectLeader(projectCode)` (`features/mission/data/leader.ts`) | Returns `boolean \| null`; `null` = still loading, `false` = not a leader, `true` = is a leader. Used here to gate the Save button and the editable fields. |
| `useProject(code, { enabled: true })` (`features/project-list/data/projects.ts`) | Auto-enabled read so the page has the current `ProjectView` on first mount. Shares the `queryKeys.project.byCode(code)` cache with the leader hook. |
| `useUpdateProject()` (`features/project-list/data/projects.ts`) | Already invalidates `project.all` + `project.byCode(code)` on success. Reused as-is. |
| `useListUsers()` (`features/user/data/list.ts`) | Autocomplete source for the workers picker. |
| `TagEditor` (`features/project-list/components/TagEditor.tsx`) | Pure controlled component, fires `onTouched` once per session. Dropped into the General section as-is. |

### New components

| Component | Responsibility |
| --- | --- |
| `ConfigurationSidebar` | Right-side fixed-width column with three `ListItemButton`s. Calls `onSelect(section)` from the parent. Highlights the active section via the `selected` prop. |
| `ConfigurationGeneralSection` | Language `Select` (English / Simplified Chinese / none) + `TagEditor` + per-section Save button. Tracks `languageTouched` and `tagsTouched` independently. |
| `ConfigurationMembersSection` | Leaders rendered as read-only chips + workers rendered as editable chips (Autocomplete to add, X to remove) + per-section Save button. Tracks `membersTouched`. |
| `ConfigurationFilepathSection` | Centered "Coming soon" placeholder using `workspace.placeholder`. No Save button. |
| `useProjectConfiguration(projectCode)` | Thin wrapper that calls `useProject(code, { enabled: true })` and exposes `data` + `isLeader`. Lives in `features/project-workspace/data/project-configuration.ts`. |

### `ProjectConfigurationPage` composition

```tsx
function ProjectConfigurationPage() {
  const { projectCode } = useParams(...);
  const { data, isLeader, isLoading } = useProjectConfiguration(projectCode);
  const [section, setSection] = useState<Section>("general");

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Box sx={{ flexGrow: 1, p: 4 }}>
        <Heading />
        <ReadOnlyHint visible={!isLeader && !isLoading} />
        {section === "general" && <ConfigurationGeneralSection ... />}
        {section === "members" && <ConfigurationMembersSection ... />}
        {section === "filepath" && <ConfigurationFilepathSection />}
      </Box>
      <ConfigurationSidebar value={section} onChange={setSection} />
    </Box>
  );
}
```

The leader gate is shared via a `readonly` prop so each section can
disable its own controls; the leader gate stays the single source of
truth in the page, the sections consume it.

## Data flow

### Read path

```
ProjectConfigurationPage
  └── useProjectConfiguration(projectCode)
        ├── useCurrentUser()                                  // from features/auth
        ├── useProject(code, { enabled: true })              // auto-fetch
        └── derive isLeader = user in (members.leaders | unblindMembers.leaders)
```

`useProjectConfiguration` is a 30-line hook that lives in
`features/project-workspace/data/project-configuration.ts` and
re-exports the three values the page needs (`data`, `isLeader`,
`isLoading`). It uses the same `useIsProjectLeader` semantics
(`null` while loading, `false` when not a leader, `true` when a leader).

### Write path

```
ConfigurationGeneralSection
  ├── user changes language or tag row
  ├── local state updates + languageTouched / tagsTouched flip
  ├── user clicks Save
  └── update.mutateAsync({ code, body: { configurations: { language, tags } } })
        └── onSuccess: queryKeys.project.byCode(code) + .all invalidate
             (already wired inside useUpdateProject)

ConfigurationMembersSection
  ├── user adds/removes worker
  ├── local state updates + membersTouched flip
  ├── user clicks Save
  └── update.mutateAsync({ code, body: { members: { leaders: [unchanged], workers: [edited] } } })
```

The body only carries `configurations` / `members` when at least one
touched flag is true. `members.leaders` is read-only in the UI but
always re-sent back unchanged so a leader change by another path is
preserved across the save.

## Wire shape (post-migration)

```ts
// shared/api/types.ts
export type ProjectLanguage = "en" | "zh-CN";

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
  configurations: ProjectConfiguration;     // was: tags: Tag[]
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateProjectInput {
  code: string;
  description: string;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  configurations?: ProjectConfiguration;    // was: tags?: Tag[]
}

export interface UpdateProjectBody {
  code?: string;
  description?: string;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  configurations?: ProjectConfiguration;    // was: tags?: Tag[]
}
```

```rust
// src-tauri/src/http/project.rs (excerpt)
#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ProjectLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub configurations: ProjectConfigurationViewResponse,   // was: tags
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,  // was: tags
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    // ...unchanged...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,  // was: tags
}
```

The serde rename `kebab-case` produces wire codes `"en"` and `"zh-CN"`,
which match the i18n `Locale` values exactly.

## Member-scope decision

The user's wording ("leader can not be modified, read only" + "members
can be add and remove") maps cleanly to the project's main
`members` team:

- `members.leaders` → rendered as read-only chips, never sent as a UI
  edit. The save body includes the **unchanged** leader list so a leader
  change made through another surface survives.
- `members.workers` → editable: Autocomplete to add, chip with X to
  remove.

Unblind teams (`unblindMembers.leaders`, `unblindMembers.workers`) are
out of scope for this spec. The user can add them in a follow-up.

## Save semantics (mirrors `ProjectDrawer`)

`ProjectDrawer` already implements a "touched" pattern for tags: a
local `tagsTouched` flag flips on the first user interaction, and the
`UpdateProjectBody` carries `tags` only when that flag is true. We
adopt the same pattern verbatim, extended to two more flags:

| Section | Flags | Body field |
| --- | --- | --- |
| General | `languageTouched`, `tagsTouched` | `configurations: { language, tags }` (sent when either is true) |
| Members | `membersTouched` | `members: { leaders: unchanged, workers: edited }` (sent when true) |
| Filepath | — | — |

`Save` is disabled when (a) `readonly`, (b) no flag is true, or
(c) a mutation is in flight. A small `Alert` renders the mutation
error when present.

## i18n keys

Added to `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`:

```
project.configuration.heading
project.configuration.section.general
project.configuration.section.members
project.configuration.section.filepath
project.configuration.readOnly
project.configuration.saveFailed
project.configuration.general.language
project.configuration.general.language.none
project.configuration.general.tags
project.configuration.members.leadersHeading
project.configuration.members.workersHeading
project.configuration.members.add
project.configuration.members.empty
project.configuration.filepath.placeholder
```

The English and Simplified Chinese values follow the existing
convention (sentence-case English, end-with-`。` where appropriate in
zh-CN; tags use `key/value` labels inherited from the existing
`project.field.tags.*` keys).

## Tests

#### Wire-migration test updates (existing files)

Every fixture that constructs a `ProjectView` or an `UpdateProjectBody`
with a top-level `tags: [...]` gets rewritten to
`configurations: { language: null, tags: [...] }`. Affected files:

- `src/test/features/project-list/project-drawer.test.tsx`
- `src/test/features/project-list/project-table.test.tsx`
- `src/test/features/project-list/project-list-page.test.tsx`
- `src/test/features/project-list/projects.test.tsx`
- `src/test/features/mission/leader.test.tsx`

`ProjectDrawer` keeps the same "tagsTouched" semantics — only the field
name moves from `tags` to `configurations.tags`.

#### New tests

`src/test/features/project-workspace/project-configuration-page.test.tsx`:

| Case | Expectation |
| --- | --- |
| Renders the heading + project code | `<h4>Configuration — {code}</h4>` present |
| Renders three sidebar nav items | All three labels present; first (`General`) is selected by default |
| Clicking a sidebar item switches the section | Members click shows the workers picker; Filepath click shows the placeholder |
| Non-leader sees read-only mode | Language Select is `disabled`, TagEditor controls are disabled, no Save button rendered |
| Leader without changes sees a disabled Save button | Save button rendered but disabled |
| Leader changes language → Save enables → submit | Click Save → `invoke("update_project", { code, body: { configurations: { language, tags } } })` |
| Leader adds a worker → Save enables → submit | Click Save → `invoke("update_project", { code, body: { members: { leaders: unchanged, workers: [...with new] } } })` |
| Mutation error renders an Alert | `screen.getByRole("alert")` carries the error message |

Test fixtures follow the leader.test.tsx style: `mockCommands({...})`
with `get_project_by_code` and `current_user` returning the test
project and user.

## Out of scope

- Filepath module (placeholder only)
- Unblind-members editing on this page
- Sub-route layout (`/configuration/general` etc.)
- A "Save all" button spanning sections — each section saves
  independently so a partial edit doesn't roll back unrelated changes

## File structure

### Created

- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationSidebar.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationMembersSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationFilepathSection.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-configuration.ts`
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`
- `docs/superpowers/specs/2026-09-20-aegis-desktop-project-configuration-page-design.md` (this file)

### Modified

- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` — DTOs and bodies
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` + `index.ts` — TS mirror
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx`
- `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`
- `apps/desktop/aegis-desktop/src/test/features/project-list/{project-drawer,project-table,project-list-page,projects}.test.tsx`
- `apps/desktop/aegis-desktop/src/test/features/mission/leader.test.tsx`
- `lib/packages/ui/src/i18n/locales/{en,zhCN}.ts`
- `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectConfigurationPage.tsx`
- `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`

## Commits

12 commits in dependency order. Each one leaves the verification gate
green.

1. **wire(types)** — TS mirror (types + index)
2. **wire(tauri)** — Tauri DTOs + bodies
3. **wire(project-list)** — `ProjectDrawer` + `ProjectTable` + `ProjectListPage`
4. **tests(wire)** — update existing fixtures
5. **i18n** — add the new configuration keys
6. **feature(data)** — `useProjectConfiguration`
7. **feature(sidebar)** — `ConfigurationSidebar`
8. **feature(general)** — `ConfigurationGeneralSection`
9. **feature(members)** — `ConfigurationMembersSection`
10. **feature(filepath)** — `ConfigurationFilepathSection`
11. **feature(page)** — `ProjectConfigurationPage` composes + leader gate
12. **feature(tests)** — `project-configuration-page.test.tsx`

## Verification gate

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
cargo check -p aegis-desktop
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```

`pnpm test` runs the vitest suite (wire-migration fixture updates +
the new page suite). `cargo check` covers the Tauri DTO shape.
`typecheck` covers both wire boundaries + the new components.