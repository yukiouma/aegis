# 2026-09-22 — CRF detail page role-based UI restrictions

## Goal

Gate the create / update affordances and the mission-issue dialog on the
CRF detail page according to the current user's relationship to the
project and the active mission. Today the page already derives
`isProjectLeader`, `isMissionQc`, and `isMissionDev` (see
[`CrfDetailPage.tsx:268-270`](../../apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx#L268-L270)),
but uses them only for the mission-issue dialog's create / act-on-issue /
comment flags. This spec extends that RBAC surface so:

- Users outside the mission (no leader / QC / DEV seat) cannot edit
  domain annotations, annotations, or open the mission-issue dialog when
  there are no issues.
- Mission QC users cannot edit annotations either (their role is
  reviewer, not author) but can still open the mission-issue dialog.
- Mission DEV users can keep editing annotations but cannot open the
  empty-issue dialog.
- Project leaders remain unrestricted.

The motivation is role hygiene — today a non-assignee who lands on the
page can still edit annotations they have no business touching, and a
DEV without context can churn through empty issue dialogs that hold no
information for them.

## User stories

| Role | Can edit annotations / domain annotations | Can open empty-issue dialog |
|---|---|---|
| **Project leader** | yes | yes |
| **Mission QC** | no | yes |
| **Mission DEV** | yes | no |
| **Other project member** | no | no |

"Empty-issue dialog" means the dialog whose scope (form or item) has
zero mission issues. Once at least one issue exists in that scope, every
role in the table above can open the dialog.

## Architecture

### Files touched

```
apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx           (page-level RBAC wiring)
apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx  (pass-through prop)
apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx     (new `disabled` prop + Tooltip)
apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx         (gate the row-level create affordance)
lib/packages/ui/src/i18n/locales/en.ts                                       (add 2 keys)
lib/packages/ui/src/i18n/locales/zhCN.ts                                     (add 2 keys, mirrored)
apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx     (add role coverage)
```

### Derived flags

Add to `CrfDetailPage.tsx` next to the existing
`canCreate / canActOnIssue / canComment` block (~line 268):

```ts
const isLeader = isProjectLeader === true;
const canEditAnnotations = isLeader || isMissionDev;
const canOpenEmptyIssueDialog = isLeader || isMissionQc;
```

Three roles with mutually-exclusive precedence — but expressed as two
flat flags because the QC role is the only one that's uniquely
permitted to open the empty-issue dialog while being denied edit
permission. Encoding both as flags makes the menu / chip gate sites
read as one-liners.

`canEditAnnotations` is `false` for QC and task-unrelated users
(everyone except leader and DEV). `canOpenEmptyIssueDialog` is `false`
for DEV and task-unrelated users (everyone except leader and QC).

When `isProjectLeader === null` (initial fetch, per
[`leader.ts:53-57`](../../apps/desktop/aegis-desktop/src/features/mission/data/leader.ts#L53-L57)),
both flags default to `false` — same conservative default as today's
`canCreate`.

## Surface-by-surface effects

| Surface | Today | With this spec |
|---|---|---|
| Form-name hover menu: `New domain` | gated by `notSubmitted` + leader flag | **also** gated by `canEditAnnotations`; if false → render disabled + tooltip |
| Form-name hover menu: `New annotation` | gated by `notSubmitted` + `noDomainAnnotations` | **also** gated by `canEditAnnotations`; same disabled + tooltip |
| Header domain annotation chips (`<Chip onClick={edit} onDelete={delete}>`) | always clickable | when `!canEditAnnotations` → `disabled` + tooltip |
| Form-level annotation chips (`CrfAnnotationArea`) | always clickable (edit + delete) | when `!canEditAnnotations` → forward `disabled` to every `AnnotationChip` |
| Item / option / unit Typography (`CrfItemRow`) | gated by `rowBlocked` (form-not-submitted / item-not-submitted / no-domain / label) | `rowBlocked` extended with `!canEditAnnotations` |
| Form code chip | gated by `!missionExists` | when `openIssueCount === 0 && !canOpenEmptyIssueDialog` → `disabled` + tooltip |
| Item code chips (one per row) | gated by `!missionExists` | when `openIssueCount === 0 && !canOpenEmptyIssueDialog` → `disabled` + tooltip |

Delete affordances on the header domain annotation chip and on the
annotation chips are NOT restricted by role — delete is unchanged so
that QC can still clean up annotation noise (no spec mention of
blocking it). The disabled chip's MUI behavior blocks BOTH `onClick` and
`onDelete` when `disabled` is true — for QC / task-unrelated, this
disables delete too. That's a deliberate consequence: a QC user who
cannot edit a row should not be deleting it either. (See *Decisions*
below for the trade-off discussion.)

## Component changes

### `CrfDetailPage.tsx`

Add `canEditAnnotations` and `canOpenEmptyIssueDialog` to the existing
derived block.

Pass `canEditAnnotations` to:
- `<CrfAnnotationArea … />` (new prop, forward to `AnnotationChip`).
- `<CrfItemRow … />` (new prop, combined with the existing `rowBlocked`
  guard in `CrfItemRow.tsx`).

For the form-name hover menu's two MenuItems, gate the `disabled` flag
on `!canEditAnnotations` and update the Tooltip wrapper's
`title`/`disableHoverListener` triple accordingly. Same pattern as the
existing `form?.notSubmitted` gate at lines 409-434.

For the form-code chip and item-code chips, gate `disabled` on the
conjunction `!missionExists || (openIssueCount === 0 && !canOpenEmptyIssueDialog)`
and update the Tooltip wrapper to show the new "no issues to view"
tooltip when the new branch is hit.

The existing `canCreate / canActOnIssue / canComment` block stays — it
governs the dialog's internal RBAC and is independent of these new
page-level gates.

### `CrfAnnotationArea.tsx`

New prop `canEditAnnotations: boolean`. Forwarded to each
`<AnnotationChip disabled={!canEditAnnotations} />`. Default
`true` would break the type — we'll make it required so every callsite
opts in.

### `AnnotationChip.tsx`

New optional prop `disabled?: boolean`. When truthy:
- `<Chip disabled ...>` (MUI's `disabled` blocks both `onClick` and
  `onDelete` — see *Decisions* below).
- Wrap the chip in a `<Tooltip>` with title
  `t("crf.detail.tooltip.noPermissionEdit")` (new i18n key).
  `disableHoverListener / disableFocusListener / disableTouchListener`
  all gated on `!disabled` so the tooltip only appears when relevant.

When `disabled` is unset or `false`, the chip is unchanged.

### `CrfItemRow.tsx`

New prop `canEditAnnotations: boolean` (required). The existing
`rowBlocked` line becomes:

```ts
const rowBlocked =
  formNotSubmitted || itemNotSubmitted || noDomainAnnotations ||
  isLabel || !canEditAnnotations;
```

`createFor`'s short-circuit then picks up the new branch for free, and
`clickableSx` falls through to `undefined` (no pointer cursor, no hover
underline) — same affordance drop as today's not-submitted branches.

### Dialogs — UNCHANGED

`AnnotationDialog`, `DomainAnnotationDialog`, `MissionIssueDialog` —
no prop changes. The gates live on the entry points (chips, menu
items, code chips), so by the time a dialog would mount the user has
already been authorised. This matches how today's
`openCreateAnnotation` short-circuits on `noDomainAnnotations` /
`form?.notSubmitted` without touching the dialog component.

## i18n

Two new keys, in both `en.ts` and `zhCN.ts`:

| Key | en | zhCN |
|---|---|---|
| `crf.detail.tooltip.noPermissionEdit` | You don't have permission to edit annotations on this form. | 您没有权限编辑此表单上的注释。 |
| `crf.detail.tooltip.noIssueToView` | There are no issues for this scope yet. | 此范围内暂无问题。 |

Mirroring is enforced by the
`satisfies Record<keyof typeof en, string>` constraint at the bottom of
`zhCN.ts` — adding the key to `en.ts` first and copying to `zhCN.ts`
keeps the typechecker honest.

## Decisions

### Disable over read-only for the chips

The user picked the recommended option: disable the chips and add a
tooltip, rather than render the dialog in a read-only mode. Read-only
mode would force `AnnotationDialog` and `DomainAnnotationDialog` to
accept a `readOnly` prop, threading it through every TextField, the
Select, the Checkbox, the Not-submit action, and the submit button. The
chip-disabled path is consistent with the page's existing pattern (the
menu items are already disabled-with-tooltip for not-submitted / no-
domain-annotations) and keeps the dialog components single-purpose.

### MUI `Chip disabled` blocks both `onClick` and `onDelete`

MUI's outlined Chip, when `disabled`, refuses click events on both the
chip body and the delete icon. This is the right behaviour for QC and
task-unrelated users — if they can't edit the row they shouldn't be
able to delete it either. The spec doesn't call out delete-affordance
gating, but in practice "no edit" implies "no destructive action". If
delete needs to stay available for some future role, we'll re-evaluate.

### DEV keeps annotation-edit permission

The spec explicitly restricts DEV users only on the empty-issue dialog.
DEV's job is to author annotations, so the create / update flows stay
open. The chip disable and the form-name menu hide are scoped to
`canEditAnnotations`, which is true for DEV.

### Mutually-exclusive precedence vs. flat flags

The page already encodes role as three independent booleans
(`isProjectLeader === true`, `isMissionQc`, `isMissionDev`). We
deliberately don't add a fourth boolean like `isTaskUnrelated` — both
new flags (`canEditAnnotations`, `canOpenEmptyIssueDialog`) fall out of
the existing trio as `isLeader || X`. This keeps the chip / menu gate
sites readable and avoids a flag explosion if a fifth role appears.

### Tooltip-on-disabled wraps

Each disabled chip / menu item needs a Tooltip wrapper because MUI's
`disabled` suppresses hover events. The existing page already does this
with `<Tooltip><span><MenuItem disabled /></span></Tooltip>` at lines
409-434 and `<Tooltip><span><Chip disabled /></span></Tooltip>` at
lines 330-369. The new `AnnotationChip` disabled + Tooltip wrapper
follows the same shape; the `<span>` host lives inside the chip
component so callers don't have to remember.

## Edge cases

- **`isProjectLeader === null` (initial)** — both new flags are `false`,
  chips render disabled with the no-permission tooltip. Matches today's
  `canCreate` behaviour.
- **Race: leader status flips after first render** — flags recompute on
  every render via the same `useMemo` chain the existing flags use, so
  the chip un-disables the moment the leader check resolves.
- **Mission exists but is empty (no QC, no DEV)** — every visitor except
  leader becomes "task unrelated". Both flags are `false`. Chips render
  disabled. Matches existing `canCreate === false` semantics.
- **`form.notSubmitted === true`** — orthogonal to the role flags; the
  existing `notSubmitted` gates still win. A leader sees a disabled chip
  for not-submitted reasons, not for permission reasons — and the
  existing tooltip `crf.detail.menu.disabledWhenNotSubmitted` wins
  (we don't double-stamp tooltips).
- **`noDomainAnnotations === true`** — `New annotation` is disabled
  with `crf.detail.menu.disabledWhenNoDomainAnnotations` (existing key).
  When `canEditAnnotations` is also `false`, we keep the existing
  not-submitted / no-domain tooltip messages (they're the more
  informative reasons). Role is the secondary fallback message.
- **Pre-existing `openIssueCountByTarget` cache** — already
  `staleTime: 0` on the issues query (`useListIssuesByMission` uses the
  default `staleTime: Infinity` per CLAUDE.md; but mission-issue spec
  kept it at `0` for fresh issue counts). The new gate reads the same
  cache, so a freshly-created issue flips the chip from disabled to
  enabled the moment the query refetches.

## Testing

Test file:
`apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`.
New `describe("CrfDetailPage — role-based restrictions")` block. Each
case mocks `current_user` and `get_project_by_code` to drive the
leader / QC / DEV booleans.

| Case | Setup | Assertion |
|---|---|---|
| Project leader — full access | `get_project_by_code` returns a project where the user is a leader; `list_missions_by_project` returns one mission; `list_issues_by_mission` returns `[]` | Form code chip is **enabled** despite zero issues. Item code chip is **enabled**. Annotation chips are clickable. Form-name menu items are **enabled**. |
| Mission QC — annotations disabled, empty-issue dialog still open | `get_project_by_code` returns a project with the user not in leaders; `list_missions_by_project` returns one mission where the user is `qc`; `list_issues_by_mission` returns `[]` | Form code chip is **enabled** (QC can open empty-issue). Annotation chips render with `Mui-disabled`. Form-name menu items render with `Mui-disabled`. Item / option / unit Typography do **not** carry `cursor: pointer`. |
| Mission DEV — annotations enabled, empty-issue dialog blocked | `get_project_by_code` returns a project with the user not in leaders; `list_missions_by_project` returns one mission where the user is `dev`; `list_issues_by_mission` returns `[]` | Form code chip is `Mui-disabled` with the new no-issue tooltip. Item code chips likewise. Annotation chips are **clickable**. Form-name menu items are **enabled**. |
| Mission DEV — issue dialog opens when at least one issue exists | Same as DEV above; `list_issues_by_mission` returns `[openedMissionIssue]` | Form code chip is **enabled**. Item code chip with `openIssueCount > 0` is **enabled**. |
| Task unrelated (no role) — strictest | `get_project_by_code` returns a project with the user not in leaders; `list_missions_by_project` returns one mission where the user is **not** in `assignees`; `list_issues_by_mission` returns `[]` | Form / item code chips are `Mui-disabled`. Annotation chips are `Mui-disabled`. Form-name menu items are `Mui-disabled`. Item / option / unit Typography do not carry `cursor: pointer`. |
| Tooltip text | QC, item-chip, no issue | `<Tooltip>` host around the item chip carries the `noIssueToView` title. |

The existing `crf-detail-page.test.tsx` fixture already mocks
`list_missions_by_project` and `list_issues_by_mission`. We'll extend
`mockCommands` in the new cases to also mock `get_project_by_code` (so
`useIsProjectLeader` resolves), and a separate fixture for the
"task-unrelated" case where the user isn't in the mission.

## Non-goals (this round)

- Server-side RBAC. The backend already enforces
  `MissionUsecase::ensure_leader` for assignment changes and lets any
  authenticated user create / read issues. We're not touching that
  surface.
- Gating the `DeleteAnnotationDialog` and `DeleteDomainAnnotationDialog`
  entry points. Per the spec, only create / update flows are gated;
  delete is currently triggered by the same chip that the spec wants
  disabled, so delete is transitively gated. We are NOT adding a
  separate role check inside the delete confirmation dialogs.
- Hiding the form-name hover menu entirely for QC / task-unrelated.
  The menu still opens on form-name click — only the two create
  entries are disabled. The Edit / Delete entry points on existing
  domain annotation chips are the way to interact with the data
  (which is also blocked for those roles).
- Per-row RBAC bypass for project leader override. Leader is a
  flat global flag today, not a per-row one.
- A unified `<RoleGate>` abstraction. The two flag names are short and
  the gate sites are obvious; abstracting now would obscure more than
  it clarifies.

## Spec history

- **`docs/superpowers/specs/2026-09-21-mission-issue-dialog-design.md`**
  — introduced `isMissionQc / isMissionDev / isProjectLeader` on the
  same page; this spec extends the same RBAC surface to the
  annotation / domain-annotation flows and to the chip-level
  issue-dialog gate.
- **`docs/superpowers/specs/2026-08-29-aegis-desktop-crf-detail-page-design.md`**
  — original CrfDetailPage design; this spec layers on top of its
  RBAC story.