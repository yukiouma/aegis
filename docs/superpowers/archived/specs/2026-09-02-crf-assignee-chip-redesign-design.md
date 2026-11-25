# 2026-09-02 — CRF assignee chip redesign

> Status: design approved, pending implementation.
>
> Scope: rework the assignee chip on the CRF form-list table and the
> assignee list inside the mission-assign drawer so each row carries
> the user's display name as the primary label and uses the chip's
> outline + border style as the role signal. Also drop the user code
> from the drawer's user dropdown and switch the per-row remove
> button to a delete icon styled with `color="error"`.

## 1. Goal and non-goals

**Goal.**

1. The CRF form-list table's `AssigneeChip` shows ONLY the assignee's
   display name. The role is communicated by the chip's border style
   (dev = solid, qc = dashed) — both chips are outlined.
2. The mission-assign drawer's per-assignee row is split into three
   visible elements:
   - a `Typography` rendering the resolved display name,
   - a small role `Chip` whose outline + border style matches the
     table chip (dev solid, qc dashed),
   - a `DeleteIcon` `IconButton` with `color="error"` for removal.
3. The drawer's user-dropdown `Autocomplete` drops `code —` from
   `getOptionLabel`; only `name` is shown. The matcher stays on
   `code`.

**Non-goals.**

- The `useUserNameMap` hook is unchanged. Same fallback semantics.
- The role i18n keys (`crf.missionAssign.roleQc`,
  `crf.missionAssign.roleDev`) are unchanged.
- No new components. The `Chip` from `@aegis/ui/mui` is reused in both
  places; only its `variant` and `sx` change.
- No new tests beyond updating the two existing assertions whose
  regex matched the now-removed `· role` suffix.

## 2. CRF form-table — `AssigneeChip`

File: `apps/desktop/aegis-desktop/src/features/crf/components/CrfFormTable.tsx`

Current chip (lines 116–140):

- `label={\`${name} · ${t(roleQc|roleDev)}\`}`
- `variant={isQc ? "outlined" : "filled"}`
- `sx={isQc ? { borderStyle: "dashed" } : undefined}`

New chip:

- `label={name}` — drop the role text and the `·` separator.
- `variant="outlined"` for both roles — solid color, no fill.
- `sx={isQc ? { borderStyle: "dashed" } : undefined}` — keep the
  dashed border for qc; remove the `sx` for dev (default = solid).

`AssigneeChip`'s prop signature is unchanged: `{ role: "dev" | "qc";
name: string }`. The call site at line 178 already passes the
resolved name and the role; no call-site change is needed.

The internal comment in `AssigneeChip` (lines 125–127) is updated to
reflect that the border style — not the text — is now the role
signal.

## 3. Mission-assign drawer — three sub-edits

File: `apps/desktop/aegis-desktop/src/features/crf/components/CrfMissionAssignDrawer.tsx`

### 3.1 User dropdown (Autocomplete, lines 240–256)

- `getOptionLabel={(opt) => opt.name}` — drop `\`${opt.code} — \``.
- `isOptionEqualToValue` stays on `code` (the wire key). Dropping
  the code from the label does not change the value/match logic.
- The `TextField`'s `label` prop (`t("crf.missionAssign.field.user")`)
  stays — that's a field label, not a row.

### 3.2 Assignees list — per-row shape (lines 198–237)

Replace the single `Chip` with a three-element row inside the
existing flex container:

```tsx
<Box
  key={a.id}
  sx={{ display: "flex", alignItems: "center", gap: 1 }}
>
  <Typography sx={{ flexGrow: 1 }}>
    {resolveName(a.userCode)}
  </Typography>
  <Chip
    size="small"
    label={a.role === "qc"
      ? t("crf.missionAssign.roleQc")
      : t("crf.missionAssign.roleDev")}
    variant="outlined"
    color="primary"
    sx={a.role === "qc" ? { borderStyle: "dashed" } : undefined}
  />
  <IconButton
    size="small"
    color="error"
    aria-label={t("crf.missionAssign.remove")}
    onClick={() => handleRemove(a.id)}
  >
    <DeleteIcon fontSize="small" />
  </IconButton>
</Box>
```

Notes:

- The outer `<Stack spacing={1}>` (line 200) is unchanged.
- The previous `justifyContent: "space-between"` on the row was used
  to push the close-icon to the far right while the chip stayed on
  the left. With three elements (`Typography` with `flexGrow: 1`,
  `Chip`, `IconButton`), `space-between` is no longer needed — the
  `Typography`'s `flexGrow: 1` does the same job.
- `DeleteIcon` is imported from `@aegis/ui/icons` alongside the
  existing `CloseIcon`. The `CloseIcon` import stays because the
  drawer header still uses it.
- The role chip's `size="small"` matches the table's chip size and
  keeps the row visually compact.

### 3.3 Drawer's header close button (lines 180–187)

Unchanged. The `CloseIcon` import remains.

## 4. Tests

File: `apps/desktop/aegis-desktop/src/test/features/crf/crf-form-list-page.test.tsx`

The two assertions added in the previous turn match the
`${name} · ${role}` shape. Both loosen:

- `it("renders the assignee's display name …")` — `screen.getByText(/^Carol Lin · /)`
  → `screen.getByText("Carol Lin")` (now the chip's full label).
- `it("falls back to userCode …")` — `screen.findByText(/^carol · /)`
  → `screen.findByText("carol")`.

No new tests are added. The pre-existing `renders the heading + one
form row` test continues to fail for the same unrelated reason; no
change there.

`useUserNameMap.test.tsx` is unaffected — those tests assert the
resolver function, not the rendered chip.

## 5. Migration / rollout

Single PR, no schema migration. The visual change is contained to two
component files plus two assertion updates in one test file. No new
i18n keys, no new components, no API surface change.

## 6. Out-of-scope follow-ups (not in this PR)

- Disambiguating same-name users in the dropdown (no `code` shown).
  Could surface a tooltip on the option or fall back to `code` in
  the `getOptionLabel` when names collide. Deferred.
- Surfacing the role in a tooltip on the table's name-only chip for
  accessibility (screen readers). Deferred.