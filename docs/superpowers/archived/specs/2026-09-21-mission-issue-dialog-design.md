# 2026-09-21 — Mission issue dialog (CRF detail page)

## Goal

Surface the existing mission-issue API (created 2026-09-20 in
`feat/mission-issue`) inside the CRF detail page, so QC, project leaders,
and DEV assignees can read, create, comment on, and close mission-level
and item-level issues directly from the page they were flagged against.
The backend already exposes the full surface — this spec covers the
desktop wiring + dialog UI.

## User stories

| Role | Actions |
|---|---|
| **QC user / project leader** | Read / create / update description / close a mission-level or item-level issue, append comments |
| **Dev user** | Read all issues, append comments |

### Triggers

- **Mission-level issues** — click the form-code chip in the CRF detail
  page header.
- **Item-level issues** — click the item-code chip in any row of the
  item list. Each item's chip is its own scope.

### Visible state on chips

When at least one issue in the chip's scope is in the `opened` state,
the chip renders a red dot badge (`<Badge variant="dot" color="error">`).
When all issues are closed (or none exist), the badge is hidden.
This is the FIRST use of MUI's `Badge` in the codebase.

## Architecture

### Files touched

```
apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs   (extend; add 5 commands)
apps/desktop/aegis-desktop/src-tauri/src/http/mission.rs       (extend; add 5 client helpers)
apps/desktop/aegis-desktop/src/shared/api/types.ts             (add issue DTOs)
apps/desktop/aegis-desktop/src/shared/api/index.ts             (add api client fns)
apps/desktop/aegis-desktop/src/shared/query/keys.ts            (add issuesByMission key)
apps/desktop/aegis-desktop/src/features/mission/data/issues.ts                  (NEW)
apps/desktop/aegis-desktop/src/features/mission/components/MissionIssueDialog.tsx  (NEW)
apps/desktop/aegis-desktop/src/features/mission/index.ts       (re-export new hooks + dialog)
apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx     (chip + dialog wiring)
apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx   (chip + props)
lib/packages/ui/src/i18n/locales/en.ts                          (add crf.missionIssue.*)
lib/packages/ui/src/i18n/locales/zhCN.ts                        (add crf.missionIssue.*)
apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx  (NEW)
docs/superpowers/specs/2026-09-20-mission-issue-design.md       (read-only; intent anchor)
```

### Wire-format mirrors

Server (snake_case wire) ↔ desktop (camelCase TS) — added to
`apps/desktop/aegis-desktop/src/shared/api/types.ts`:

```ts
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

export interface IssueListResponse { issues: IssueViewResponse[]; }
export interface IssueListQuery { state?: IssueState; }

export interface CreateIssueInput {
  targetItem?: string;
  description: string;
}

export interface UpdateIssueDescriptionInput { description: string; }
export interface AppendCommentInput { content: string; }
```

Server DTO source: `apps/server/aegis-server/src/transport/http/dto.rs`
lines 2112-2249. Wire-convention comment at lines 2115-2120 documents
that `state` is `snake_case` for direct string `switch`-ing; the rest
follows `camelCase`.

### URL → Tauri command → server route

| New Tauri command | Server endpoint | Server handler |
|---|---|---|
| `list_issues_by_mission` | `GET /api/mission/by-mission/{mission_id}/issues` | `mission::handlers::list_issues_by_mission` |
| `create_issue` | `POST /api/mission/by-mission/{mission_id}/issues` | `mission::handlers::create_issue` |
| `patch_issue_state` | `PATCH /api/mission/issues/{issue_id}/state?state=closed\|opened` | `mission::handlers::patch_issue_state` |
| `update_issue_description` | `PATCH /api/mission/issues/{issue_id}/description` | `mission::handlers::update_issue_description` |
| `append_comment` | `POST /api/mission/issues/{issue_id}/comments` | `mission::handlers::append_comment` |

The state-flip URL uses the `?state=` **query** parameter (not body) —
matches the as-built server, which deviates from the 2026-09-20 spec's
"body `{"state":"closed"}`" plan. Body is the empty marker
`PatchIssueStateRequest {}`.

### Query keys

Extend `apps/desktop/aegis-desktop/src/shared/query/keys.ts`:

```ts
mission: {
  byProject: (projectCode: string, kind: string) =>
    ["mission", "byProject", projectCode, kind] as const,
  // NEW
  issuesByMission: (missionId: number) =>
    ["mission", "issuesByMission", missionId] as const,
},
```

Single key — no `state` parameter — because we fetch all issues in one
call and filter client-side. All 4 mutations invalidate this key on
success.

### Data hooks (`features/mission/data/issues.ts`)

Mirror the existing `features/mission/data/missions.ts` style:

- `useListIssuesByMission(missionId | null)` — `useQuery<IssueViewResponse[]>`,
  `enabled: missionId != null`, `staleTime: 0`.
- `useCreateIssue()` — mutation; `onSuccess` invalidates
  `queryKeys.mission.issuesByMission(missionId)`.
- `usePatchIssueState()` — same invalidation.
- `useUpdateIssueDescription()` — same invalidation.
- `useAppendComment()` — same invalidation.

All re-exported from `features/mission/index.ts`.

## Page state (`CrfDetailPage`)

```ts
type IssueScope =
  | { kind: "form" }
  | { kind: "item"; itemId: number; itemCode: string };

type IssueDialogState = {
  scope: IssueScope;
  missionId: number;
} | null;

const [issueDialog, setIssueDialog] = useState<IssueDialogState>(null);
```

Derived state:

```ts
const missionForForm = useMemo(() => {
  const list = useListMissionsByProject(projectCode, "crf").data ?? [];
  return list.find((m) => m.missionCode === form?.code) ?? null;
}, [form?.code, list]);

const issuesQuery = useListIssuesByMission(missionForForm?.id ?? null);

const openIssueCountByTarget = useMemo(() => {
  const map = new Map<string | null, number>();
  for (const i of issuesQuery.data ?? []) {
    if (i.state !== "opened") continue;
    map.set(i.targetItem ?? null, (map.get(i.targetItem ?? null) ?? 0) + 1);
  }
  return map;
}, [issuesQuery.data]);

const isProjectLeader = useIsProjectLeader(projectCode);   // boolean | null
const currentUser = useCurrentUser().data;

const isMissionQc  = missionForForm?.assignees.some(
  (a) => a.userCode === currentUser?.code && a.role === "qc") ?? false;
const isMissionDev = missionForForm?.assignees.some(
  (a) => a.userCode === currentUser?.code && a.role === "dev") ?? false;

const canCreate     = isProjectLeader === true || isMissionQc;
const canActOnIssue = isProjectLeader === true || isMissionQc;
const canComment    = canActOnIssue || isMissionDev;
```

When `isProjectLeader === null` (initial fetch), all three are
`false`. The dot badge is purely data-driven — no RBAC check on it.

## Chip + Badge

### Form-code chip (`CrfDetailPage.tsx`, lines 268-279)

Replace the current `<Chip>` with:

```tsx
{form?.code && (
  <Tooltip
    title={missionForForm ? "" : t("crf.missionIssue.tooltip.noMission")}
    disableHoverListener={Boolean(missionForForm)}
    disableFocusListener={Boolean(missionForForm)}
    disableTouchListener={Boolean(missionForForm)}
  >
    <span>
      <Badge
        variant="dot"
        color="error"
        invisible={
          !missionForForm ||
          (openIssueCountByTarget.get(null) ?? 0) === 0
        }
        overlap="circular"
      >
        <Chip
          sx={{ minWidth: 70 }}
          size="small"
          label={form.code}
          variant="outlined"
          disabled={!missionForForm}
          onClick={() => missionForForm &&
            setIssueDialog({ scope: { kind: "form" }, missionId: missionForForm.id })
          }
          data-testid={`crf-form-${id}`}
        />
      </Badge>
    </span>
  </Tooltip>
)}
```

The `<span>` wrapper inside `<Tooltip>` is required for the same reason
as the existing `Tooltip > span > MenuItem` pattern at lines 329-344.

### Item-code chip (`CrfItemRow.tsx`, lines 124-131)

New props on `Props`:

```ts
interface Props {
  // … existing
  hasOpenIssue: boolean;
  onOpenIssues: () => void;
  missionExists: boolean;
}
```

Replace the existing `<Chip>`:

```tsx
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
```

`hasOpenIssue` is `openIssueCountByTarget.get(item.code) > 0` from the
page. `onOpenIssues` calls `setIssueDialog({ scope: { kind: "item", itemId: item.id, itemCode: item.code }, missionId })`.

## Dialog (`MissionIssueDialog`)

### Props

```ts
interface Props {
  open: boolean;
  scope: IssueScope;
  mission: MissionViewResponse;
  issues: IssueViewResponse[];            // page pre-filters to scope
  currentUser: UserView | undefined;
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
```

### Internal state

```ts
const [expandedId, setExpandedId] = useState<number | null>(null);
const [newDescription, setNewDescription] = useState("");
const [editing, setEditing] = useState<{ id: number; value: string } | null>(null);
const [commentDraft, setCommentDraft] = useState<{ id: number; value: string } | null>(null);

useEffect(() => {
  if (!open) {
    setExpandedId(null);
    setNewDescription("");
    setEditing(null);
    setCommentDraft(null);
  }
}, [open]);
```

### Layout

```
<Dialog maxWidth="md" fullWidth open={open} onClose={onClose}>
  <DialogTitle>{t("crf.missionIssue.dialog.title", { scope: scopeLabel(scope) })}</DialogTitle>
  <DialogContent>
    {canCreate && (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1, mb: 2 }}>
        <TextField
          multiline minRows={2}
          label={t("crf.missionIssue.create.field.description")}
          value={newDescription}
          onChange={(e) => setNewDescription(e.target.value)}
          disabled={createPending}
        />
        {createError && <Alert severity="error">{errorMessage(createError)}</Alert>}
        <Button
          onClick={() => onCreate(newDescription.trim())}
          disabled={createPending || newDescription.trim() === ""}
        >
          {t("crf.missionIssue.create.submit")}
        </Button>
      </Box>
    )}
    {issues.length === 0 ? (
      <Alert severity="info">{t("crf.missionIssue.dialog.empty")}</Alert>
    ) : (
      <Table>
        <TableHead>
          <TableRow>
            <TableCell>{t("crf.missionIssue.table.reviewer")}</TableCell>
            <TableCell>{t("crf.missionIssue.table.description")}</TableCell>
            <TableCell>{t("crf.missionIssue.table.state")}</TableCell>
            <TableCell />   {/* expand toggle */}
          </TableRow>
        </TableHead>
        <TableBody>
          {issues.map((issue) => (
            <Fragment key={issue.id}>
              <IssueRow
                issue={issue}
                expanded={expandedId === issue.id}
                onToggle={() => setExpandedId(expandedId === issue.id ? null : issue.id)}
              />
              {expandedId === issue.id && (
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
              )}
            </Fragment>
          ))}
        </TableBody>
      </Table>
    )}
  </DialogContent>
  <DialogActions>
    <Button onClick={onClose}>{t("common.close")}</Button>
  </DialogActions>
</Dialog>
```

`IssueRow` and `IssueDetails` are private components in the same file
(small enough to inline). `scopeLabel(scope)` returns the title scope
text — e.g. `"Form AE"` for `{ kind: "form" }`, `"Item CMTRT"` for
`{ kind: "item", itemCode: "CMTRT" }`.

### Per-row component details

`IssueRow` shows:

- **Reviewer cell** — the `issuer` string (we have no name lookup on the
  desktop; the issuer's `user_code` is displayed verbatim, with the full
  description as a Tooltip).
- **Description cell** — truncated to 2 lines + Tooltip with full text.
- **State cell** — `<Chip color={issue.state === "opened" ? "warning" : "default"} variant="outlined">`
  with the localised state label. Closed-state chip uses `borderStyle: "dashed"` to
  match the codebase's "archived" convention (cf. `CrfFormTable.tsx` lines
  118-139 and `AnnotationChip.tsx` line 35-53).
- **Expand cell** — `<IconButton>` with `<ExpandMore>` (rotated 180° when
  expanded).

`IssueDetails` shows (only when the row is expanded):

- **Description** — read-only display by default; clicking the "Edit
  description" button (visible iff `canActOnIssue`) swaps it for a
  `<TextField multiline>` pre-filled with `issue.description`, with
  Save/Cancel buttons. Save is `disabled={updateDescPending || editing.value.trim() === "" || editing.value.trim() === issue.description}`.
  Cancel resets `editing` to `null`.
- **State flip button** — visible iff `canActOnIssue`. Single toggle:
  - `opened` → `Close`, color `warning`, calls `onPatchState(issue.id, "closed")`.
  - `closed` → `Reopen`, color `primary`, calls `onPatchState(issue.id, "opened")`.
  - When `!canActOnIssue`, the button is hidden (not just disabled).
- **Comments** — `<List>` of all `issue.comments`, each row showing
  `user`, `content`, and a formatted `createdAt`. When empty, an `<Alert
  severity="info">` with `t("crf.missionIssue.detail.noComments")` is shown.
- **Comment form** — visible iff `canComment`. `<TextField multiline>` +
  Send button, Send `disabled={commentPending || (commentDraft?.value.trim() ?? "") === ""}`.
- **Patch / update-desc / comment errors** — each rendered as
  `<Alert severity="error">{errorMessage(...)}</Alert>` scoped to its
  respective section.

## Error handling

| Slot | Source | Where rendered |
|---|---|---|
| `createError` | `useCreateIssue().error` | Below create-description TextField |
| `updateDescError` | `useUpdateIssueDescription().error` | Below description edit TextField |
| `patchError` | `usePatchIssueState().error` | Above Close/Reopen button |
| `commentError` | `useAppendComment().error` | Below comment TextField |

All errors render as `<Alert severity="error">{errorMessage(error)}</Alert>`,
matching `AnnotationDialog.tsx:195-199` and `DomainAnnotationDialog.tsx:136-140`.

Errors persist until the next mutation on the same hook (the next
`mutate()` call replaces `mutation.error`) or the dialog reopens with a
fresh query. We do NOT manually reset errors on close — matches the
existing `AnnotationDialog` behavior.

Server `MissionApiError` → UX mapping:

| Server error | HTTP | UX |
|---|---|---|
| `Validation(...)` (empty description / empty content) | 400 | Submit is client-side disabled when empty, so this is the server safety net. Alert renders `validation_failed: ...`. |
| `NotFound` / `IssueNotFound` / `MissionNotFoundForIssue(id)` | 404 | After invalidation, the issue row vanishes. No special Alert. |
| `Forbidden { user_code, project_code }` | 403 | Typically unreachable because controls are disabled by RBAC; if reached (stale state), Alert shows. |
| `Repository(...)` | 500 | Generic Alert; user retries or contacts admin. |

401 handling: identical to every other dialog — Tauri transport layer's
auth-refresh middleware retries once; on failure the dialog fills the
slot with `refreshFailed`, the global `useCurrentUser` query invalidates,
and the user is bounced to login.

## Edge cases

- **No mission for form** — chip is `disabled` with the `noMission`
  tooltip. Dialog can't open. Badge renders (with `invisible={true}`).
- **`isProjectLeader === null` (initial)** — chip is `disabled` (the
  Tooltip wraps an enabled Chip; the Chip itself is `disabled`). All
  action gating is `false` until the leader check resolves to `true`.
- **Description / comment unchanged** — Save / Send button stays
  disabled.
- **Whitespace-only description / comment** — submit disabled; backend
  would 400 with `DomainError::EmptyIssueDescription` / `EmptyCommentContent`.
- **Closing an issue with comments** — backend allows; comments are
  append-only on closed issues.
- **`useListMissionsByProject` loading** — chip `disabled` with no
  tooltip (loading implicit).
- **Issues query errors out** — chip Badge is `invisible` (no count),
  but chip stays enabled so the user can click and see the dialog's
  inline error Alert.
- **`form.notSubmitted === true`** — issue chip NOT additionally
  disabled. Issues can be tracked against an un-submitted form.

## i18n keys

Added to `lib/packages/ui/src/i18n/locales/en.ts` AND
`lib/packages/ui/src/i18n/locales/zhCN.ts`. The TypeScript
`satisfies Record<keyof typeof en, string>` constraint at the bottom of
`zhCN.ts` enforces both files stay in sync.

```
crf.missionIssue.tooltip.noMission
crf.missionIssue.dialog.title            # uses {scope}
crf.missionIssue.dialog.empty
crf.missionIssue.table.reviewer
crf.missionIssue.table.description
crf.missionIssue.table.state
crf.missionIssue.state.opened
crf.missionIssue.state.closed
crf.missionIssue.create.field.description
crf.missionIssue.create.submit
crf.missionIssue.detail.comments
crf.missionIssue.detail.editDescription
crf.missionIssue.detail.save
crf.missionIssue.detail.cancel
crf.missionIssue.detail.close
crf.missionIssue.detail.reopen
crf.missionIssue.detail.noComments
crf.missionIssue.detail.addComment
crf.missionIssue.detail.commentPlaceholder
crf.missionIssue.disabled.issueActions  # "Only mission QC or project leaders can do this"
crf.missionIssue.disabled.comment       # "Only mission QC, DEV or project leaders can comment"
```

## Testing

Test file:
`apps/desktop/aegis-desktop/src/test/features/mission/mission-issue-dialog.test.tsx`.

Uses existing helpers from `src/test/helpers/tauri-mock.ts`
(`mockCommands`, `httpError`).

### Chip + badge rendering (5 cases)

1. Chip shows dot badge when open issue exists for the form scope.
2. Chip hides dot badge when no issues exist.
3. Chip hides dot badge when only closed issues exist.
4. Item chip shows dot badge only when an issue matches its `targetItem`.
5. Chip disabled with no-mission tooltip when no mission exists for the
   form.

### Dialog — list + per-row details (3 cases)

6. Clicking form chip opens dialog showing only mission-level issues
   (`targetItem === null`).
7. Clicking item chip opens dialog filtered to that item's `target_item`.
8. Empty list shows Alert with `.empty` key, not the table.

### Create (5 cases)

9. Create form hidden when `canCreate === false`.
10. Create submit disabled while description is empty / whitespace.
11. Clicking create invokes `create_issue` tauri command with correct
    args (`{ missionId, body: { description } }`).
12. Create failure renders inline Alert and keeps form open.
13. Create success invalidates `issuesByMission` and clears the textarea.

### Per-row actions (7 cases)

14. Close button calls `patch_issue_state` with `?state=closed` query,
    empty body.
15. Reopen button calls `patch_issue_state` with `?state=opened`.
16. Close/Reopen buttons hidden when `canActOnIssue === false` but expand
    toggle still works.
17. Edit description: Edit → TextField pre-filled → Save invokes
    `update_issue_description`.
18. Append comment: Send invokes `append_comment` and clears the draft.
19. Send disabled when comment is empty / whitespace.
20. Comment failure renders inline Alert under the comment TextField.

### Cache invalidation (1 case)

21. Every successful mutation invalidates
    `queryKeys.mission.issuesByMission(missionId)`. Spy on
    `queryClient.invalidateQueries`.

### Multi-row independence (2 cases)

22. Editing description on row A does not affect description TextField on
    row B.
23. Comment drafts are per-row.

### State reset on close (1 case)

24. Closing dialog and reopening resets all internal state (no rows
    expanded, edit mode off, comment draft empty, `newDescription`
    empty).

### What we DON'T test

- Tauri command registration (covered by `src-tauri` unit tests).
- RBAC enforcement on the backend (covered by
  `lib/crates/mission` integration tests).
- Live-DB mission-issue integration tests (covered by 2026-09-20 work).

## Non-goals (this round)

- Editing the issue `target_item` after creation (backend does not
  support it).
- Editing or deleting individual comments (backend does not support it;
  comments are append-only).
- Bulk-create / batch operations on issues.
- Notification fan-out when an issue is opened or commented on.
- Search / full-text indexing of `description` or `comments`.
- An audit log of state transitions (close/reopen history).
- Reassigning an issue to a different reviewer — the wire has no
  reviewer concept; "Reviewer" in the table is the `issuer` field per
  this spec.
- Per-row reviewer assignment from the project leader panel — out of
  scope of the dialog.

## Spec history

- **`docs/superpowers/specs/2026-09-20-mission-issue-design.md`** —
  the backend mission-issue design (approved, merged via PR #76). This
  spec covers the desktop-side consumption of that backend work.