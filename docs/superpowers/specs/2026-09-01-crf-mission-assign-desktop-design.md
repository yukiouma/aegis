# 2026-09-01 — CRF mission-assign desktop surface

> Status: design approved, pending implementation plan.
>
> Scope: build the desktop-side surface for the mission-assign flow on
> the CRF form list page. Wires the existing `apis::mission` port
> (added in `2026-09-01-mission-crate-design.md`) into the Tauri
> shell, and replaces the placeholder `CrfAssignTakersDrawer` with a
> real `CrfMissionAssignDrawer`.

## 1. Goal and non-goals

**Goal.** Surface mission/assignee data on the CRF form list page so
project leaders can see who is assigned to each CRF form (Dev / QC)
and manage those assignments from a right-side drawer. Build the
desktop-side plumbing (Tauri commands, TS API client, query keys,
React Query hooks, feature module) that the mission crate does not
ship.

**Non-goals (this round).**

- SDTM / ADaM / TFL mission-assignment UI. The same hooks will be
  reusable but no UI for those mission kinds is added here.
- Removing the existing hardcoded "Pending" status chip in the table.
  Left as-is.
- Mission CRUD at the project level (mission list / delete-mission
  UI). Only the assign-mission drawer is in scope.
- Live-DB integration tests for the new Tauri commands. Unit / mock
  tests only.
- Optimistic update of the assignee column. Invalidate-and-refetch
  is sufficient for v1.
- Filtering or searching the assignee chips in the cell. Chips wrap
  naturally; no search affordance.
- A Tauri-command exposure of `is_leader(user_code, project_code)`.
  Leader detection stays client-side via `useCurrentUser` +
  `useListProjects`.

## 2. Mapping: mission ↔ CRF form

The mission crate's `Mission.mission_code` is the natural identifier
that ties a mission to a CRF form. Each CRF form has exactly one
corresponding mission when `MissionKind::Crf`. The mapping rule is:

```
mission.mission_code == form.code
```

`CrfForm` (in `apps/desktop/aegis-desktop/src/shared/api/types.ts`)
has no separate `missionCode` field; the linkage is by `code`. The
desktop `useListMissionsByProject(projectCode, "crf")` query returns
all CRF-kind missions for a project; the page builds a
`Map<formCode, MissionView>` in memory and reads from it.

A form with no corresponding mission shows an empty assignee cell
(no chips, an em-dash placeholder). The first assignee added to such
a form implicitly creates the mission.

## 3. Authorization

**Server-side.** Already enforced in `mission::usecase::mission_usecase::ensure_leader`
(see `2026-09-01-mission-crate-design.md`): only the project leader
can `create_mission`, `add_assignee`, or `remove_assignee`. Non-leaders
get a 403 (`MissionApiError::Forbidden`).

**Client-side.** The assign icon in the action column is **hidden
entirely** for non-leaders (decided during brainstorming). The
drawer itself is therefore unreachable for non-leaders via the normal
flow. The server still enforces the rule, so any non-leader who reaches
the drawer through devtools / a race still gets a 403 surfaced as an
inline alert.

**Leader detection.** A new hook `useIsProjectLeader(projectCode)` in
`features/mission/data/leader.ts`:

```ts
export function useIsProjectLeader(projectCode: string) {
  const { data: currentUser } = useCurrentUser();
  const { data: projects } = useListProjects({ enabled: !!projectCode });
  if (!currentUser || !projects) return false;
  const project = projects.find(p => p.code === projectCode);
  if (!project) return false;
  return project.members.leaders.some(u => u.code === currentUser.code);
}
```

Client-side only. No server round-trip. The server remains the
authoritative gate.

## 4. Rust surface (`apps/desktop/aegis-desktop/src-tauri`)

### New files

- `src-tauri/src/commands/mission.rs` — Tauri command handlers.
- `src-tauri/src/http/mission.rs` — transport-only adapter over
  `aegis-server` mission endpoints.

### New commands

| Command | Input | Output |
|---|---|---|
| `list_missions_by_project` | `{ projectCode: string, kind?: string }` | `MissionListResponse` |
| `add_assignee` | `{ missionId: i64, userCode: string, role: string }` | `AssigneeViewResponse` |
| `remove_assignee` | `{ missionId: i64, assigneeId: i64 }` | `()` (204 No Content) |
| `create_mission` | `{ projectCode, missionKind, missionCode, assignees[] }` | `MissionViewResponse` |

All commands accept an injected `State<HttpClient>` and translate the
wire DTO through the `http/mission.rs` adapter. The Rust DTOs in
`http/dto.rs` mirror `apps/server/aegis-server/src/transport/http/dto.rs`
lines 1872–2028 (`MissionKind`, `MissionRole`, `AssigneeDataArg`,
`AssigneeViewResponse`, `MissionViewResponse`, `MissionListResponse`).

### Registration

- Add `pub mod mission;` to `src-tauri/src/commands/mod.rs`.
- Register the four commands in the Tauri builder (likely
  `src-tauri/src/lib.rs` or `src-tauri/src/main.rs`, alongside the
  existing project / user commands).

## 5. TS API client (`shared/api`)

### Types added to `shared/api/types.ts`

```ts
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

These are hand-maintained wire mirrors; the rename from
`snake_case` (server JSON) to `camelCase` (TS identifier) happens at
the serde boundary in the Tauri command.

### Methods added to `shared/api/index.ts`

```ts
listMissionsByProject: (input: { projectCode: string; kind?: MissionKind }): Promise<MissionView[]> =>
  call<MissionListResponse>("list_missions_by_project", input).then(r => r.missions),

addAssignee: (missionId: number, body: { userCode: string; role: MissionRole }): Promise<AssigneeView> =>
  call<AssigneeView>("add_assignee", { missionId, ...body }),

removeAssignee: (missionId: number, assigneeId: number): Promise<void> =>
  call<void>("remove_assignee", { missionId, assigneeId }),

createMission: (input: CreateMissionInput): Promise<MissionView> =>
  call<MissionView>("create_mission", input),
```

### Query keys (`shared/query/keys.ts`)

Add a `mission` namespace:

```ts
mission: {
  byProject: (projectCode: string, kind?: MissionKind) =>
    ["mission", "byProject", projectCode, kind ?? null] as const,
  byId: (id: number) => ["mission", "byId", id] as const,
},
```

## 6. Feature module (`features/mission`)

New folder mirroring `features/user` / `features/project-list`:

```
features/mission/
  data/
    list.ts              # useListMissionsByProject
    add-assignee.ts      # useAddAssignee
    remove-assignee.ts   # useRemoveAssignee
    create-mission.ts    # useCreateMission
    leader.ts            # useIsProjectLeader
  index.ts               # re-exports
```

All three mutation hooks invalidate
`queryKeys.mission.byProject(projectCode, "crf")` on success. The
invalidating project code is captured via the closure of the hook
factory (the call site passes `projectCode`).

`useListMissionsByProject(projectCode, "crf")` returns
`useQuery<MissionView[], ApiError>` keyed on
`queryKeys.mission.byProject(projectCode, "crf")`. Default
`QueryClient` config (no retry, no refetch) applies; per-query
overrides live in the hook file if needed.

## 7. Table column (`CrfFormTable.tsx`)

### Header rename

| Old key | New key | en | zhCN |
|---|---|---|---|
| `crf.table.column.taker` | `crf.table.column.assignee` | `Taker` → `Assignee` | `填写人` → `负责人` |
| `crf.table.action.assignTakers` | `crf.table.action.assignMission` | `Assign takers` → `Assign mission` | `分配填写人` → `分配任务` |

### Body cell

The previously-empty `taker` cell renders assignee chips:

```tsx
<TableCell sx={{ maxWidth: 280 }}>
  <Stack direction="row" spacing={0.5} sx={{ flexWrap: "wrap", gap: 0.5 }}>
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
```

`userNameByCode` is a `Map<string, string>` built by the page from
`useListUsers()` — a single query, single source of truth. The fallback
to `userCode` covers the brief window before `useListUsers` resolves.

The `borderStyle: dashed` pattern matches the existing convention at
`features/crf/components/AnnotationChip.tsx:43-48`: `variant="outlined"`
already supplies border-color and 1px width, so only `borderStyle` is
overridden to preserve theming.

### Action column — leader gate

```tsx
{isLeader && (
  <Tooltip title={t("crf.table.action.assignMission")}>
    <IconButton onClick={() => onAssignMission(row)}>
      <AssignmentIndIcon />
    </IconButton>
  </Tooltip>
)}
```

Non-leaders see no assign icon at all.

## 8. Drawer (`CrfMissionAssignDrawer.tsx`)

### File

New: `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`.
Deleted: `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`.

### Props

```ts
interface CrfMissionAssignDrawerProps {
  open: boolean;
  onClose: () => void;
  projectCode: string;
  form: CrfForm | null;
}
```

The drawer is only rendered when `form !== null` (parent gates it).

### Layout

Right-anchored drawer, `width: 480`, mirroring `ProjectDrawer` style:

1. **Header**: title + subtitle (`Form: {{formCode}}`).
2. **Current assignees list**: each row shows user-name chip
   (dashed for QC), role chip (`Dev` / `QC`), and a remove icon
   button. Empty state: `"No assignees yet."`.
3. **Add form**: `Autocomplete<UserSummary>` (user) + `Select` (role)
   + Add button. User dropdown shows project members minus already-
   assigned users (per the "hide already-assigned users" decision).
4. **Footer**: Close button.

### Data hooks used by the drawer

| Hook | Purpose |
|---|---|
| `useListMissionsByProject(projectCode, "crf")` | Look up this form's mission |
| `useListUsers()` | Resolve `userCode` → display name |
| `useProject(projectCode)` (existing pattern at `features/project-list/data/projects.ts:34-47`) | Get project members for the user dropdown |
| `useCreateMission()` | Implicit mission creation on first add |
| `useAddAssignee()` | Add assignee mutation |
| `useRemoveAssignee()` | Remove assignee mutation |

### Implicit mission-creation flow

`handleAdd(userCode, role)` does the following:

1. If `mission` is undefined (no mission yet for this form): call
   `useCreateMission` with
   `{ projectCode, missionKind: "crf", missionCode: form.code,
      assignees: [{ userCode, role }] }`.
   Capture the returned `MissionView`.
2. Otherwise: call `useAddAssignee(mission.id, { userCode, role })`.
3. On success: clear `selectedUser` + `selectedRole`, invalidate
   `queryKeys.mission.byProject(projectCode, "crf")`.
4. Drawer stays open so the leader can add more assignees
   (e.g., a Dev and a QC).

The Add button is disabled while any of `useCreateMission`,
`useAddAssignee`, or `useRemoveAssignee` is `isPending`.

### `availableMembers` derivation

```ts
const allMembers = useMemo(() => {
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
  () => new Set(assignees.map(a => a.userCode)),
  [assignees],
);

const availableMembers = useMemo(
  () => allMembers.filter(u => !assignedCodes.has(u.code)),
  [allMembers, assignedCodes],
);
```

Union of leaders + workers across blind and unblind membership, deduped
by `userCode`, minus already-assigned users.

### i18n strings added

`crf.missionAssign.title` — "Assign Mission" / "分配任务"
`crf.missionAssign.subtitle` — "Form: {{formCode}}" / "表单: {{formCode}}"
`crf.missionAssign.currentAssignees` — "Current Assignees" / "当前负责人"
`crf.missionAssign.addAssignee` — "Add Assignee" / "添加负责人"
`crf.missionAssign.user` — "User" / "用户"
`crf.missionAssign.role.dev` — "Dev" / "开发"
`crf.missionAssign.role.qc` — "QC" / "质控"
`crf.missionAssign.empty` — "No assignees yet." / "暂无负责人"
`crf.missionAssign.remove` — "Remove assignee" / "删除负责人"

## 9. Page wiring (`CrfFormListPage.tsx`)

```ts
const { data: missions = [] } = useListMissionsByProject(projectCode, "crf");
const missionsByFormCode = useMemo(
  () => new Map(missions.map(m => [m.missionCode, m])),
  [missions],
);
const isLeader = useIsProjectLeader(projectCode);
const { data: users = [] } = useListUsers();
const userNameByCode = useMemo(
  () => new Map(users.map(u => [u.code, u.name])),
  [users],
);
```

Both `missionsByFormCode` + `userNameByCode` + `isLeader` are passed
down to `CrfFormTable` as new props:

```ts
interface CrfFormTableProps {
  // ... existing
  missionsByFormCode: Map<string, MissionView>;
  userNameByCode: Map<string, string>;
  isLeader: boolean;
  onAssignMission: (row: CrfForm) => void;  // renamed from onAssignTakers
}
```

Page-level state renames `assignTakersFor: CrfForm | null` →
`assignMissionFor: CrfForm | null`. The drawer reference in JSX
becomes `<CrfMissionAssignDrawer ... />`.

The `components/index.ts:5` re-export switches from
`CrfAssignTakersDrawer` to `CrfMissionAssignDrawer`.

## 10. Edge cases

| # | Case | Handling |
|---|---|---|
| 1 | Mission exists, assignees empty | Empty-state message; Add button enabled when user + role picked |
| 2 | Form has no mission yet | First add calls `useCreateMission` with the picked user as the first assignee |
| 3 | Project lookup 404 mid-flow | `availableMembers = []`; Add button stays disabled |
| 4 | `useCreateMission` 409 (race with another tab) | Alert with server message; user can retry — invalidation surfaces the existing mission |
| 5 | `useAddAssignee` 409 (duplicate assignee) | Should not occur in practice (dropdown filters); if it does, alert with server message |
| 6 | `useRemoveAssignee` fails (404 because removed elsewhere) | Alert with server message; invalidation re-converges |
| 7 | Non-leader reaches drawer (race) | Server returns 403; alert surfaces the error |
| 8 | Rapid Add clicks | Add button disabled while `mutation.isPending` |
| 9 | Drawer closes mid-mutation | Mutation lifecycle continues; success invalidation still fires |
| 10 | `useListUsers()` not loaded | Chips fall back to `userCode` until names arrive |
| 11 | Project has zero members | `availableMembers = []`; Add button disabled |
| 12 | Form's `code` matches no mission | Treated as "no mission yet" (case 2) |
| 13 | Mixed-kind missions in cache | The `?kind=crf` filter on the query key filters at the server; defensive but not load-bearing |

## 11. Tests

### Rust (Tauri command tests)

`apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs` test module:

- `list_missions_by_project_returns_missions` — happy path
- `list_missions_by_project_with_kind_filter` — `?kind=crf`
- `add_assignee_returns_assignee_view` — happy path
- `remove_assignee_returns_204` — happy path
- `create_mission_returns_mission_view` — happy path
- One error-propagation test per command

### TS (hook tests)

Each `features/mission/data/*.ts` has a sibling `*.test.ts`:

- `list.test.ts` — `useListMissionsByProject` calls `api.listMissionsByProject` with right args + query key
- `add-assignee.test.ts` — successful mutation invalidates `mission.byProject(projectCode, "crf")`
- `remove-assignee.test.ts` — same invalidation
- `create-mission.test.ts` — same invalidation
- `leader.test.ts` — `useIsProjectLeader` returns true when `currentUser.code` is in `project.members.leaders`, false otherwise

### Component tests

`features/crf/components/crf-form-table.test.tsx` updates:

- Comment at line 170: `cells[3] = taker` → `cells[3] = assignee`
- Header text assertion updated to `"Assignee"`
- New test: `isLeader={false}` → assign icon not rendered
- New test: `isLeader={true}` + mission with assignees → chips render with right labels + `borderStyle: dashed` for qc

`features/crf/components/CrfMissionAssignDrawer.test.tsx` (new):

- Empty state when no mission exists
- Lists current assignees with chips + remove icons
- Add button disabled until user picked
- Add button calls `useCreateMission` when no mission, `useAddAssignee` when mission exists
- Removing an assignee calls `useRemoveAssignee`

## 12. File summary

**New (15 files):**

- `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs`
- `apps/desktop/aegis-desktop/src/features/mission/data/list.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/add-assignee.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/remove-assignee.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/create-mission.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/leader.ts`
- `apps/desktop/aegis-desktop/src/features/mission/index.ts`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.test.tsx`
- `apps/desktop/aegis-desktop/src/features/mission/data/list.test.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/add-assignee.test.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/remove-assignee.test.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/create-mission.test.ts`
- `apps/desktop/aegis-desktop/src/features/mission/data/leader.test.ts`

**Modified (10 files):**

- `apps/desktop/aegis-desktop/src-tauri/src/commands/mod.rs` (register `mission` module)
- `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs` (add mission DTOs)
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` or `main.rs` (register commands in builder)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` (add mission types)
- `apps/desktop/aegis-desktop/src/shared/api/index.ts` (add mission wrappers)
- `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (add mission keys)
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfFormListPage.tsx` (wire hooks + state rename)
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx` (assignee cell + leader gate)
- `apps/desktop/aegis-desktop/src/features/crf/components/index.ts` (re-export rename)
- `apps/desktop/aegis-desktop/src/features/crf/components/crf-form-table.test.tsx` (comment + assertion updates + new cases)
- `lib/packages/ui/src/i18n/locales/en.ts` (rename + new keys)
- `lib/packages/ui/src/i18n/locales/zhCN.ts` (rename + new keys)

**Deleted (1 file):**

- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAssignTakersDrawer.tsx`

## 13. Implementation order

1. Rust DTOs in `http/dto.rs`
2. `http/mission.rs` transport adapter
3. `commands/mission.rs` Tauri commands + registration + tests
4. TS types in `shared/api/types.ts`
5. TS wrappers in `shared/api/index.ts`
6. Query keys in `shared/query/keys.ts`
7. `features/mission/data/list.ts` + test
8. `features/mission/data/add-assignee.ts` + test
9. `features/mission/data/remove-assignee.ts` + test
10. `features/mission/data/create-mission.ts` + test
11. `features/mission/data/leader.ts` + test
12. `features/mission/index.ts` re-export
13. `CrfFormListPage.tsx` wiring
14. `CrfFormTable.tsx` assignee cell + leader gate
15. `crf-form-table.test.tsx` updates + new cases
16. i18n column/action rename in en.ts + zhCN.ts
17. `CrfMissionAssignDrawer.tsx` + test
18. Delete `CrfAssignTakersDrawer.tsx`
19. Add `crf.missionAssign.*` strings to en.ts + zhCN.ts

Each phase ends with `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, `pnpm --filter aegis-desktop typecheck`, `pnpm --filter aegis-desktop test`.

## 14. Out of scope reminder

(Repeated from §1 for emphasis at the end of the document.)

- SDTM / ADaM / TFL mission-assignment UI.
- Removing the "Pending" status chip.
- Mission CRUD at the project level.
- Live-DB integration tests.
- Optimistic updates.
- Cell-level search / filter on assignee chips.
- A server-side `is_leader` Tauri command.
