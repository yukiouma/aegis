# CRF Detail Page Role Restrictions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gate the form-name hover menu's create entries, the header domain annotation chips, the form-level annotation chips, and the `CrfItemRow` click affordances on a `canEditAnnotations` flag; gate the form / item code chips on a `canOpenEmptyIssueDialog` flag; render the existing pattern (chip `disabled` + tooltip wrapper) consistently across every surface.

**Architecture:** Two flat boolean flags (`canEditAnnotations = isLeader || isMissionDev`, `canOpenEmptyIssueDialog = isLeader || isMissionQc`) derived from the existing `isProjectLeader / isMissionQc / isMissionDev` triple. New `disabled` prop on `AnnotationChip` + Tooltip wrapper. New required `canEditAnnotations` prop on `CrfItemRow` and `CrfAnnotationArea`. Existing dialog components are unchanged — gates live on entry points.

**Tech Stack:** React 19, MUI v6, TanStack Query 5, Vitest + Testing Library. No backend / no Tauri changes. Two i18n keys (en + zhCN).

**Spec:** [`docs/superpowers/specs/2026-09-22-crf-detail-role-restrictions-design.md`](../specs/2026-09-22-crf-detail-role-restrictions-design.md)

## Global Constraints

- i18n keys must be added to BOTH `lib/packages/ui/src/i18n/locales/en.ts` AND `lib/packages/ui/src/i18n/locales/zhCN.ts`. The `satisfies Record<keyof typeof en, string>` constraint at the bottom of `zhCN.ts` enforces sync.
- The two new RBAC flags default to `false` when `isProjectLeader === null` (initial fetch) — matches today's `canCreate` behavior. See [`leader.ts:53-57`](../../apps/desktop/aegis-desktop/src/features/mission/data/leader.ts#L53-L57).
- MUI `Chip disabled` blocks BOTH `onClick` AND `onDelete` — deliberate. QC and task-unrelated users get both gates for free; spec doesn't override that.
- Tooltip-on-disabled wraps follow the existing `<Tooltip><span><Chip/></span></Tooltip>` shape — `disableHoverListener / disableFocusListener / disableTouchListener` all gated on the relevant condition.
- The existing `canCreate / canActOnIssue / canComment` block stays — it governs the dialog's internal RBAC and is independent of these new page-level gates.
- Existing tests in `crf-detail-page.test.tsx` (and any other file calling the changed components) must continue to pass without modification of the test bodies — they mock the page at the level the new flags live at, and `isProjectLeader` defaults to `null` (so `false`) without a `get_project_by_code` mock, which matches the original test setup. (Verify in Task 6.)

---

## File Structure

### Modified files
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` — derive two flags; gate form-name menu items; gate form-code chip; gate header domain-annotation chips; pass `canEditAnnotations` to `CrfAnnotationArea` and `CrfItemRow`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx` — accept `canEditAnnotations` prop; forward to `AnnotationChip`
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx` — accept optional `disabled` prop; wrap chip in `<Tooltip>` host when disabled; gate `onClick`/`onDelete` on `!disabled`
- `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx` — accept `canEditAnnotations` prop; add to `rowBlocked`; extends existing not-submitted / no-domain / label guards
- `lib/packages/ui/src/i18n/locales/en.ts` — add 2 keys
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — add 2 keys (Chinese)
- `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` — append role-coverage `describe` block

### New files
- None.

---

## Task 1: i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:** Two new keys:
- `crf.detail.tooltip.noPermissionEdit`
- `crf.detail.tooltip.noIssueToView`

- [ ] **Step 1: Add keys to `en.ts`**

Insert immediately after the existing `crf.detail.itemRow.disabledWhenNotSubmitted` key (around line 452 in current file):

```ts
  "crf.detail.tooltip.noPermissionEdit":
    "You don't have permission to edit annotations on this form.",
  "crf.detail.tooltip.noIssueToView":
    "There are no issues for this scope yet.",
```

- [ ] **Step 2: Add the same keys to `zhCN.ts`**

Insert immediately after the existing `crf.detail.itemRow.disabledWhenNotSubmitted` key (around line 438 in current file):

```ts
  "crf.detail.tooltip.noPermissionEdit": "您没有权限编辑此表单上的注释。",
  "crf.detail.tooltip.noIssueToView": "此范围内暂无问题。",
```

- [ ] **Step 3: Verify typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter @aegis/ui typecheck
```

Expected: PASS. The `satisfies Record<keyof typeof en, string>` constraint at the bottom of `zhCN.ts` will refuse to compile if a key is missing.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis && git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts && git commit -m "feat(ui): add i18n keys for crf detail role restrictions" -m "Adds crf.detail.tooltip.noPermissionEdit and crf.detail.tooltip.noIssueToView in both en.ts and zhCN.ts. No consumers yet — pure foundation step.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 2: `AnnotationChip` — `disabled` prop + Tooltip wrapper (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx` (already exists per CLAUDE.md)

**Interfaces:**
- Consumes: existing `Annotation` type
- Produces: `AnnotationChip` with new optional `disabled?: boolean` prop. When `true`, chip is MUI `disabled` (blocks both onClick/onDelete) and wrapped in a Tooltip with title `t("crf.detail.tooltip.noPermissionEdit")`.

- [ ] **Step 1: Read the existing test file**

Run:

```bash
cd d:/projects/rusty/aegis && cat apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx
```

This pins down the existing test harness (mock providers, render helper) so the new tests slot in cleanly.

- [ ] **Step 2: Append failing tests for the `disabled` prop**

Append a new `describe("AnnotationChip — disabled prop", ...)` block. Use the existing test file's imports / helpers verbatim — add only what's new:

```tsx
describe("AnnotationChip — disabled prop", () => {
  const baseAnnotation = {
    id: 1,
    domainAnnotationId: 50,
    content: "annotation text",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-02T00:00:00Z",
  };

  it("renders the chip unchanged when disabled is omitted or false", () => {
    const { rerender } = render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
      />,
      { wrapper: renderWithQueryClient },
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).not.toHaveClass("Mui-disabled");

    rerender(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
        disabled={false}
      />,
    );
    expect(
      screen.getByText("annotation text").closest(".MuiChip-root")!,
    ).not.toHaveClass("Mui-disabled");
  });

  it("renders with Mui-disabled when disabled is true", () => {
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
        disabled={true}
      />,
      { wrapper: renderWithQueryClient },
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).toHaveClass("Mui-disabled");
    expect(chip).toHaveAttribute("aria-disabled", "true");
  });

  it("does not call onEdit when the disabled chip is clicked", () => {
    const onEdit = vi.fn();
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={onEdit}
        onDelete={() => undefined}
        disabled={true}
      />,
      { wrapper: renderWithQueryClient },
    );
    fireEvent.click(screen.getByText("annotation text"));
    expect(onEdit).not.toHaveBeenCalled();
  });
});
```

(The actual test file may import slightly differently; mirror what already exists. If the existing file doesn't use `renderWithQueryClient` and instead uses a different helper, use that one.)

- [ ] **Step 3: Run the new tests — expect FAIL**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx -t "disabled prop"
```

Expected: FAIL — TypeScript error (`disabled` prop doesn't exist on `AnnotationChip` yet).

- [ ] **Step 4: Implement the `disabled` prop**

Modify `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx`. The full new file:

```tsx
import { Chip, Tooltip } from "@aegis/ui/mui";
import type { ChipProps } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
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
  /**
   * Render the chip as MUI-disabled (blocks both `onClick` and
   * `onDelete`) and wrap it in a Tooltip that explains the user
   * lacks permission. Used by `CrfAnnotationArea` when the current
   * viewer is not allowed to edit annotations on this form. The
   * Tooltip host is required because MUI's disabled chips drop
   * hover events; the same `<Tooltip><span><Chip /></span></Tooltip>`
   * pattern is used elsewhere on the page (cf. `CrfDetailPage.tsx`
   * lines 330-369 and 409-466).
   */
  disabled?: boolean;
}

export function AnnotationChip({
  annotation,
  colorIndex,
  onEdit,
  onDelete,
  disabled = false,
}: Props) {
  const { t } = useI18n();
  // Default `false` keeps every existing call site working unchanged.
  const chip = (
    <Chip
      label={annotation.content}
      color={annotationColor(colorIndex)}
      onClick={disabled ? undefined : onEdit}
      onDelete={disabled ? undefined : onDelete}
      size="small"
      variant="outlined"
      disabled={disabled}
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

  // The Tooltip host only when disabled — otherwise the chip stays
  // a single Chip element and the existing tests' `.closest("button")`
  // lookups keep working.
  if (!disabled) return chip;

  return (
    <Tooltip
      title={t("crf.detail.tooltip.noPermissionEdit")}
      disableHoverListener={false}
      disableFocusListener={false}
      disableTouchListener={false}
    >
      <span>{chip}</span>
    </Tooltip>
  );
}
```

- [ ] **Step 5: Run the new tests — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx
```

Expected: PASS — all tests pass (existing tests still work because the prop defaults to `false`).

- [ ] **Step 6: Verify typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
```

Expected: PASS — the `disabled?:` prop is optional with a default, so every existing callsite is unchanged.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationChip.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx && git commit -m "feat(crf): AnnotationChip disabled prop + tooltip" -m "Adds an optional disabled prop that flips the chip to MUI disabled
(blocks onClick + onDelete) and wraps it in a Tooltip with the new
noPermissionEdit key. Defaults to false so every existing call site is
unchanged.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 3: `CrfAnnotationArea` — `canEditAnnotations` pass-through

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` (forward the prop with a temporary default of `true` so typecheck doesn't break in this task — the value is wired in Task 5)

**Interfaces:**
- Consumes: existing `Props` (`annotations`, `colorByDomainAnnotationId`, `onEdit`, `onDelete`)
- Produces: extended `Props` adding required `canEditAnnotations: boolean`; each rendered `AnnotationChip` receives `disabled={!canEditAnnotations}`

- [ ] **Step 1: Read the existing test file (if any)**

Run:

```bash
cd d:/projects/rusty/aegis && ls apps/desktop/aegis-desktop/src/test/features/crf/ | grep -i "annotation-area\|annotationArea"
```

(No existing file is expected for this component — it's a small internal helper. Skip to Step 2.)

- [ ] **Step 2: Append failing tests for `CrfAnnotationArea`**

Append to `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx` (keep all chip-related coverage in one file). Reuse the existing `renderWithQueryClient` helper:

```tsx
import { CrfAnnotationArea } from "../../../features/crf/components/CrfAnnotationArea";

describe("CrfAnnotationArea — canEditAnnotations", () => {
  const baseAnnotation = {
    id: 1,
    domainAnnotationId: 50,
    content: "annotation text",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-02T00:00:00Z",
  };

  it("renders chips enabled when canEditAnnotations is true", () => {
    render(
      <CrfAnnotationArea
        annotations={[baseAnnotation]}
        colorByDomainAnnotationId={new Map([[50, 0]])}
        canEditAnnotations={true}
        onEdit={() => undefined}
        onDelete={() => undefined}
      />,
      { wrapper: renderWithQueryClient },
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).not.toHaveClass("Mui-disabled");
  });

  it("renders chips disabled when canEditAnnotations is false", () => {
    render(
      <CrfAnnotationArea
        annotations={[baseAnnotation]}
        colorByDomainAnnotationId={new Map([[50, 0]])}
        canEditAnnotations={false}
        onEdit={() => undefined}
        onDelete={() => undefined}
      />,
      { wrapper: renderWithQueryClient },
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).toHaveClass("Mui-disabled");
    expect(chip).toHaveAttribute("aria-disabled", "true");
  });

  it("does not call onEdit when a disabled chip is clicked", () => {
    const onEdit = vi.fn();
    render(
      <CrfAnnotationArea
        annotations={[baseAnnotation]}
        colorByDomainAnnotationId={new Map([[50, 0]])}
        canEditAnnotations={false}
        onEdit={onEdit}
        onDelete={() => undefined}
      />,
      { wrapper: renderWithQueryClient },
    );
    fireEvent.click(screen.getByText("annotation text"));
    expect(onEdit).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 3: Run the new tests — expect FAIL**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx -t "canEditAnnotations"
```

Expected: FAIL — TypeScript error (`canEditAnnotations` is not in `Props`).

- [ ] **Step 4: Implement the pass-through**

Replace `apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx` with:

```tsx
import { Box, Stack } from "@aegis/ui/mui";

import type { Annotation } from "../../../shared/api";
import { AnnotationChip } from "./AnnotationChip";

interface Props {
  annotations: Annotation[];
  colorByDomainAnnotationId: Map<number, number>;
  /**
   * Whether the current viewer is allowed to edit annotations on
   * this form. When `false`, every chip is rendered with
   * `disabled={true}` so MUI blocks `onClick` and `onDelete` — see
   * `AnnotationChip` for the Tooltip wrapper. The page derives this
   * from the same RBAC flags used by the form-name hover menu.
   */
  canEditAnnotations: boolean;
  onEdit: (annotation: Annotation) => void;
  onDelete: (annotation: Annotation) => void;
}

/**
 * Renders the form-level annotation chips. The list lives directly
 * under the header, above the item rows.
 */
export function CrfAnnotationArea({
  annotations,
  colorByDomainAnnotationId,
  canEditAnnotations,
  onEdit,
  onDelete,
}: Props) {
  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1, height: "20px" }}>
      {annotations.length === 0 ? null : <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
        {annotations.map((a) => (
          <AnnotationChip
            key={a.id}
            annotation={a}
            colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
            disabled={!canEditAnnotations}
            onEdit={() => onEdit(a)}
            onDelete={() => onDelete(a)}
          />
        ))}
      </Stack>}

    </Box>
  );
}
```

- [ ] **Step 5: Forward the prop from `CrfDetailPage` with a temporary default**

In `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` line ~519, the `<CrfAnnotationArea ...>` invocation needs `canEditAnnotations`. Add a placeholder `true` literal — Task 5 replaces it with the real derived flag:

```tsx
{/* Form-level annotation chips */}
{detail && (
  <CrfAnnotationArea
    annotations={sortByDomainAnnotationOrder(
      detail.formAnnotations,
      colorByDomainAnnotationId,
    )}
    colorByDomainAnnotationId={colorByDomainAnnotationId}
    canEditAnnotations={true}
    onEdit={(a) => openEditAnnotation(a, { kind: "form", id })}
    onDelete={(a) => setConfirmDeleteAnnotation(a)}
  />
)}
```

- [ ] **Step 6: Run the new tests — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-chip.test.tsx
```

Expected: PASS — both the existing tests and the new `canEditAnnotations` tests pass.

- [ ] **Step 7: Verify no regressions in the CrfDetailPage suite**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: PASS — the placeholder `true` keeps the page's existing tests green.

- [ ] **Step 8: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/components/CrfAnnotationArea.tsx apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-chip.test.tsx && git commit -m "feat(crf): CrfAnnotationArea canEditAnnotations pass-through" -m "Adds a required canEditAnnotations prop that flips every chip to
disabled via the new AnnotationChip prop. Page wires a placeholder
true; Task 5 wires the real RBAC-derived value.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 4: `CrfItemRow` — `canEditAnnotations` blocks row-level create affordance

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` (forward the prop with a temporary default of `true`)

**Interfaces:**
- Consumes: existing `Props`
- Produces: extended `Props` adding required `canEditAnnotations: boolean`; `rowBlocked` extends to include `!canEditAnnotations`; `createFor` and `clickableSx` pick up the new branch automatically

- [ ] **Step 1: Read the existing test file**

Run:

```bash
cd d:/projects/rusty/aegis && cat apps/desktop/aegis-desktop/src/test/features/crf/crf-item-row.test.tsx
```

(Or just the first 60 lines if it's long — we only need the harness shape.)

- [ ] **Step 2: Append failing tests for `CrfItemRow` `canEditAnnotations`**

Append to `crf-item-row.test.tsx` (use the existing render helper / `renderWithQueryClient`):

```tsx
describe("CrfItemRow — canEditAnnotations", () => {
  const itemDetail = {
    item: {
      id: 21,
      formId: 11,
      code: "AETERM",
      name: "Term",
      kind: "text",
      order: 0,
      notSubmitted: false,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-02T00:00:00Z",
    },
    options: [],
    units: [],
    annotations: [],
  };

  it("drops the pointer cursor on item name when canEditAnnotations is false", () => {
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        canEditAnnotations={false}
        onCreateAnnotation={() => undefined}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        openIssueCount={0}
        onOpenIssues={() => undefined}
        missionExists={true}
      />,
      { wrapper: renderWithQueryClient },
    );
    const item = screen.getByTestId("crf-item-name-21");
    expect(item).not.toHaveStyle({ cursor: "pointer" });
  });

  it("does not call onCreateAnnotation when item name is clicked and canEditAnnotations is false", () => {
    const onCreateAnnotation = vi.fn();
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        canEditAnnotations={false}
        onCreateAnnotation={onCreateAnnotation}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        openIssueCount={0}
        onOpenIssues={() => undefined}
        missionExists={true}
      />,
      { wrapper: renderWithQueryClient },
    );
    fireEvent.click(screen.getByTestId("crf-item-name-21"));
    expect(onCreateAnnotation).not.toHaveBeenCalled();
  });

  it("keeps the pointer cursor and click handler when canEditAnnotations is true", () => {
    const onCreateAnnotation = vi.fn();
    render(
      <CrfItemRow
        itemDetail={itemDetail}
        colorByDomainAnnotationId={new Map()}
        canEditAnnotations={true}
        onCreateAnnotation={onCreateAnnotation}
        onEditAnnotation={() => undefined}
        onDeleteAnnotation={() => undefined}
        onClearNotSubmitted={() => undefined}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        openIssueCount={0}
        onOpenIssues={() => undefined}
        missionExists={true}
      />,
      { wrapper: renderWithQueryClient },
    );
    fireEvent.click(screen.getByTestId("crf-item-name-21"));
    expect(onCreateAnnotation).toHaveBeenCalledWith({ kind: "item", id: 21 });
  });
});
```

- [ ] **Step 3: Run the new tests — expect FAIL**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-item-row.test.tsx -t "canEditAnnotations"
```

Expected: FAIL — TypeScript error (`canEditAnnotations` is not in `Props`).

- [ ] **Step 4: Implement the `canEditAnnotations` block**

Modify `apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx`. The diff is small:

a) Extend the `Props` interface. Insert after the existing `missionExists` prop (around line 76):

```ts
  /**
   * Whether the current viewer is allowed to create / update
   * annotations on this form. When `false`, every row-level
   * create-annotation entry point (item name, option value, unit
   * value) is gated: the pointer cursor / hover underline drop and
   * the click is a no-op. Derived by the page from the same RBAC
   * flags used by the form-name hover menu.
   */
  canEditAnnotations: boolean;
```

b) Add `canEditAnnotations` to the destructured props at the top of the function (around line 78-91).

c) Extend `rowBlocked`. Replace the existing line:

```ts
  const rowBlocked =
    formNotSubmitted || itemNotSubmitted || noDomainAnnotations || isLabel;
```

with:

```ts
  // Collapse the row-level "no new annotations" guards into one.
  // When any of these is true every create-annotation entry point on
  // this row must short-circuit — the form cascade has wiped every
  // annotation, the item cascade has wiped this row's annotations,
  // there is no domain annotation to assign a new annotation to,
  // the item is a static label, or the current viewer doesn't have
  // permission to edit annotations on this form (QC / task-unrelated).
  const rowBlocked =
    formNotSubmitted || itemNotSubmitted || noDomainAnnotations ||
    isLabel || !canEditAnnotations;
```

`createFor`'s short-circuit (`if (rowBlocked) return;`) and the existing
`clickableSx` conditional (`rowBlocked ? undefined : { cursor: "pointer", ... }`)
pick up the new branch for free.

- [ ] **Step 5: Forward the prop from `CrfDetailPage` with a temporary default**

In `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` line ~537, the `<CrfItemRow ...>` invocation. Add `canEditAnnotations={true}` to the prop block. Task 5 replaces this with the real RBAC value:

```tsx
<CrfItemRow
  key={itemDetail.item.id}
  itemDetail={...}
  colorByDomainAnnotationId={colorByDomainAnnotationId}
  canEditAnnotations={true}
  onCreateAnnotation={openCreateAnnotation}
  onEditAnnotation={...}
  onDeleteAnnotation={...}
  onClearNotSubmitted={...}
  formNotSubmitted={...}
  itemNotSubmitted={...}
  noDomainAnnotations={noDomainAnnotations}
  openIssueCount={...}
  onOpenIssues={...}
  missionExists={...}
/>
```

- [ ] **Step 6: Run the new tests — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-item-row.test.tsx
```

Expected: PASS — existing tests still work (placeholder `true` keeps the default behaviour), new tests pass.

- [ ] **Step 7: Verify no regressions in the CrfDetailPage suite**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-item-row.test.tsx && git commit -m "feat(crf): CrfItemRow canEditAnnotations guard" -m "Adds a required canEditAnnotations prop that extends the existing
rowBlocked guard. When false, the item/option/unit Typography drops
its pointer cursor and the create-annotation click is a no-op, matching
the existing affordance-drop pattern for the not-submitted branches.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 5: `CrfDetailPage` — wire RBAC flags + gate menu / chips

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`

**Interfaces:**
- Consumes: existing `isProjectLeader`, `isMissionQc`, `isMissionDev` derivations
- Produces: two new flags (`canEditAnnotations`, `canOpenEmptyIssueDialog`); gated form-name menu items; gated form-code chip; gated header domain-annotation chips; replacement of the placeholder `true` from Tasks 3 + 4 with the real values

- [ ] **Step 1: Add the two new flags**

In `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`, the existing derivation block is around line 262-270:

```tsx
  const isMissionQc = !!formMission?.assignees.some(
    (a) => a.userCode === currentUser?.code && a.role === "qc",
  );
  const isMissionDev = !!formMission?.assignees.some(
    (a) => a.userCode === currentUser?.code && a.role === "dev",
  );
  const canCreate = isProjectLeader === true || isMissionQc;
  const canActOnIssue = isProjectLeader === true || isMissionQc;
  const canComment = canActOnIssue || isMissionDev;
```

Add two new lines (right after `canComment`):

```tsx
  // Two flat RBAC flags that drive the page-level gates. `canEditAnnotations`
  // controls the form-name hover menu's create entries, the header
  // domain-annotation chips, the form-level annotation chips (via
  // CrfAnnotationArea), and the CrfItemRow click affordances.
  // `canOpenEmptyIssueDialog` controls the form / item code chip's
  // `disabled` when its scope has zero opened issues — the spec lets
  // leader and QC through, blocks DEV and task-unrelated. Both default
  // to `false` when `isProjectLeader === null` (initial fetch) —
  // matches today's `canCreate` behavior.
  const canEditAnnotations = isProjectLeader === true || isMissionDev;
  const canOpenEmptyIssueDialog = isProjectLeader === true || isMissionQc;
```

- [ ] **Step 2: Replace the placeholder in `<CrfAnnotationArea>`**

In the JSX around line ~519, change:

```tsx
canEditAnnotations={true}
```

to:

```tsx
canEditAnnotations={canEditAnnotations}
```

- [ ] **Step 3: Replace the placeholder in `<CrfItemRow>`**

In the JSX around line ~537, change:

```tsx
canEditAnnotations={true}
```

to:

```tsx
canEditAnnotations={canEditAnnotations}
```

- [ ] **Step 4: Gate the form-name hover menu's two MenuItems**

Replace the existing `<Tooltip>` block around line ~409-466 (the two `MenuItem`s for "New domain" and "New annotation") with:

```tsx
<MenuList>
  <Tooltip
    title={
      form?.notSubmitted
        ? t("crf.detail.menu.disabledWhenNotSubmitted")
        : !canEditAnnotations
          ? t("crf.detail.tooltip.noPermissionEdit")
          : ""
    }
    disableHoverListener={
      !!form?.notSubmitted || !canEditAnnotations
    }
    disableFocusListener={
      !!form?.notSubmitted || !canEditAnnotations
    }
    disableTouchListener={
      !!form?.notSubmitted || !canEditAnnotations
    }
  >
    <span>
      <MenuItem
        disabled={Boolean(form?.notSubmitted) || !canEditAnnotations}
        onClick={() => {
          setFormNameMenuAnchor(null);
          setDomainDialog({ mode: "create" });
        }}
      >
        {t("crf.detail.menu.newDomain")}
      </MenuItem>
    </span>
  </Tooltip>
  <Tooltip
    title={
      form?.notSubmitted
        ? t("crf.detail.menu.disabledWhenNotSubmitted")
        : noDomainAnnotations
          ? t("crf.detail.menu.disabledWhenNoDomainAnnotations")
          : !canEditAnnotations
            ? t("crf.detail.tooltip.noPermissionEdit")
            : ""
    }
    disableHoverListener={
      !!form?.notSubmitted || noDomainAnnotations || !canEditAnnotations
    }
    disableFocusListener={
      !!form?.notSubmitted || noDomainAnnotations || !canEditAnnotations
    }
    disableTouchListener={
      !!form?.notSubmitted || noDomainAnnotations || !canEditAnnotations
    }
  >
    <span>
      <MenuItem
        disabled={
          Boolean(form?.notSubmitted) ||
          noDomainAnnotations ||
          !canEditAnnotations
        }
        onClick={() => {
          setFormNameMenuAnchor(null);
          openCreateAnnotation({ kind: "form", id });
        }}
      >
        {t("crf.detail.menu.newAnnotation")}
      </MenuItem>
    </span>
  </Tooltip>
</MenuList>
```

The not-submitted and no-domain tooltips win over the no-permission
one — they're more informative when both apply.

- [ ] **Step 5: Gate the header domain-annotation chips**

Replace the existing domain-annotation chip `<Chip>` block around line ~475-491 with:

```tsx
<Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
  {detail.domainAnnotations.map((d, i) => {
    const chip = (
      <Chip
        key={d.id}
        label={t("crf.detail.domainChip.label", {
          name: d.name,
          description: d.description,
        })}
        color={annotationColor(i)}
        onClick={
          canEditAnnotations ? () => setDomainDialog({ mode: "edit", row: d }) : undefined
        }
        onDelete={
          canEditAnnotations ? () => setConfirmDeleteDomain(d) : undefined
        }
        size="small"
        data-testid={`domain-annotation-chip-${d.id}`}
        variant="outlined"
        disabled={!canEditAnnotations}
      />
    );
    if (canEditAnnotations) return chip;
    return (
      <Tooltip
        key={d.id}
        title={t("crf.detail.tooltip.noPermissionEdit")}
      >
        <span>{chip}</span>
      </Tooltip>
    );
  })}
</Stack>
```

The Tooltip host is required only when the chip is disabled (MUI's
disabled chips drop hover events). The wrapper shape matches the
existing pattern at lines 330-369 in the same file.

- [ ] **Step 6: Gate the form-code chip on empty-issue + permission**

Replace the existing form-code chip `<Chip>` block around line ~330-369 with:

```tsx
{form?.code && (
  <Tooltip
    title={
      !formMission
        ? t("crf.missionIssue.tooltip.noMission")
        : openIssueCountByTarget.get(null) === 0 &&
            !canOpenEmptyIssueDialog
          ? t("crf.detail.tooltip.noIssueToView")
          : ""
    }
    disableHoverListener={
      Boolean(formMission) &&
      !(openIssueCountByTarget.get(null) === 0 && !canOpenEmptyIssueDialog)
    }
    disableFocusListener={
      Boolean(formMission) &&
      !(openIssueCountByTarget.get(null) === 0 && !canOpenEmptyIssueDialog)
    }
    disableTouchListener={
      Boolean(formMission) &&
      !(openIssueCountByTarget.get(null) === 0 && !canOpenEmptyIssueDialog)
    }
  >
    <span>
      <Badge
        color="error"
        badgeContent={openIssueCountByTarget.get(null) ?? 0}
        invisible={!formMission}
        overlap="circular"
      >
        <Chip
          sx={{ minWidth: 70 }}
          size="small"
          label={form.code}
          variant="outlined"
          disabled={
            !formMission ||
            (openIssueCountByTarget.get(null) === 0 &&
              !canOpenEmptyIssueDialog)
          }
          onClick={() =>
            formMission &&
            setIssueDialog({
              scope: { kind: "form" },
              missionId: formMission.id,
            })
          }
          // Stable anchor for `?focus=form-<id>` from the global
          // search page. Sits next to the form-name Typography so
          // scrolling here lands the user on the form header.
          data-testid={`crf-form-${id}`}
        />
      </Badge>
    </span>
  </Tooltip>
)}
```

The no-mission tooltip wins over the no-issues tooltip — when no
mission exists, "no issues to view" is incorrect. Otherwise the chip
gates on `!canOpenEmptyIssueDialog` whenever its scope has zero opened
issues. When `canOpenEmptyIssueDialog === true` (leader / QC), the
chip stays enabled regardless of count.

- [ ] **Step 7: Gate the item code chips**

The item chip is rendered inside `CrfItemRow`. We have two options:

**Option A (chosen):** Add an `canOpenEmptyIssueDialog` prop to `CrfItemRow` (mirrors the `canEditAnnotations` pattern from Task 4), and a corresponding `disabled` derivation in `CrfItemRow` that combines `!missionExists`, `openIssueCount === 0 && !canOpenEmptyIssueDialog`. Forward the prop from the page.

**Option B (rejected):** Push the empty-issue gate entirely into `CrfItemRow` by computing the RBAC there. Rejected — the page already holds the flag, and the existing convention is for the page to compute role flags and the row to consume them.

Go with Option A:

a) Extend `CrfItemRow`'s `Props` (after the existing `missionExists` prop around line 75):

```ts
  /**
   * Whether the current viewer is allowed to open the mission-issue
   * dialog when the scope has zero opened issues. When `false` AND
   * `openIssueCount === 0`, the item code chip is disabled with the
   * no-issues-to-view tooltip. The page derives this from the same
   * RBAC flags used by the form-name hover menu.
   */
  canOpenEmptyIssueDialog: boolean;
```

b) Add `canOpenEmptyIssueDialog` to the destructured props.

c) Wrap the existing item-code chip's `<Chip>` (around line 165-173) in a `<Tooltip>` host when `openIssueCount === 0 && !canOpenEmptyIssueDialog` (and `missionExists`), and update the chip's `disabled` accordingly. The exact pattern mirrors Task 5 Step 6.

The full updated item-chip block:

```tsx
{!isLabel && (() => {
  const noIssues = openIssueCount === 0 && !canOpenEmptyIssueDialog;
  const chip = (
    <Chip
      sx={{ width: 92 }}
      label={item.code}
      variant="outlined"
      size="small"
      onClick={onOpenIssues}
      disabled={!missionExists || noIssues}
      data-testid={`crf-item-code-${item.id}`}
    />
  );
  if (!missionExists) {
    return (
      <Tooltip
        title={t("crf.missionIssue.tooltip.noMission")}
        disableHoverListener={false}
        disableFocusListener={false}
        disableTouchListener={false}
      >
        <span>{chip}</span>
      </Tooltip>
    );
  }
  if (noIssues) {
    return (
      <Tooltip
        title={t("crf.detail.tooltip.noIssueToView")}
        disableHoverListener={false}
        disableFocusListener={false}
        disableTouchListener={false}
      >
        <span>{chip}</span>
      </Tooltip>
    );
  }
  return chip;
})()}
```

The current chip is wrapped in `<Tooltip>` for the no-mission case at
lines 149-176 of `CrfItemRow.tsx` — preserve that and add the new
no-issues case before the un-tooltipped return path.

d) Forward the prop from `CrfDetailPage`:

```tsx
<CrfItemRow
  ...
  canOpenEmptyIssueDialog={canOpenEmptyIssueDialog}
/>
```

- [ ] **Step 8: Run all CrfDetailPage-related tests — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx src/test/features/crf/crf-item-row.test.tsx src/test/features/crf/crf-annotation-chip.test.tsx
```

Expected: PASS — every existing test continues to pass because:
- `isProjectLeader === null` when `get_project_by_code` is not mocked → `canEditAnnotations === false` and `canOpenEmptyIssueDialog === false`. But existing tests that exercise create / edit flows usually do so via direct clicks that bypass the chip-disabled check (`fireEvent.click` on a `Mui-disabled` chip still fires onClick in jsdom — only the visual click is suppressed). Verify by reading the affected tests below in Task 6; if any test relies on chip disable to short-circuit, we'll add a `get_project_by_code` mock.

- [ ] **Step 9: Verify typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx apps/desktop/aegis-desktop/src/features/crf/components/CrfItemRow.tsx && git commit -m "feat(crf): wire RBAC flags into detail page + item row" -m "Derives canEditAnnotations and canOpenEmptyIssueDialog from the
existing isProjectLeader / isMissionQc / isMissionDev triple. Gates
the form-name hover menu's two MenuItems, the header domain-annotation
chips, the form-code chip, and the item-code chips (via a new
canOpenEmptyIssueDialog prop on CrfItemRow).

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 6: Role-coverage tests (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx`

**Interfaces:** Covers the spec §"Testing" table.

- [ ] **Step 1: Read the existing test file (already done in pre-planning)**

The file at `apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx` already has:
- `fakeUser` (with `code: "u"`)
- `fakeForm` (`id: 11`, `code: "AE"`)
- `fakeDetail` (form id 11, item id 21)
- `fakeMission` (id 10, `missionCode: "AE"`, single assignee `{ userCode: "u", role: "qc" }`)
- `mockCommands` helper

To test other roles, we'll add new fixtures in this task.

- [ ] **Step 2: Append fixtures and the role-coverage describe block**

Append to `crf-detail-page.test.tsx`:

```tsx
// =========================================================================
// Role-based restriction coverage. Each case drives the leader / QC / DEV
// booleans by mocking get_project_by_code AND the project's leaders list
// (so useIsProjectLeader resolves), then mocks
// list_missions_by_project / list_issues_by_mission for the chip state.
// =========================================================================

const leaderProject = {
  members: {
    leaders: [{ code: "u", name: "U", role: "admin", active: true }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "u", name: "U", role: "admin", active: true }],
    workers: [],
  },
};

const nonLeaderProject = {
  members: {
    leaders: [{ code: "other", name: "Other", role: "admin", active: true }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "other", name: "Other", role: "admin", active: true }],
    workers: [],
  },
};

function missionForUser(role: "qc" | "dev" | "other") {
  return {
    ...fakeMission,
    assignees: role === "other" ? [] : [{ ...fakeMission.assignees[0], role }],
  };
}

describe("CrfDetailPage — role-based restrictions", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("project leader: form chip enabled with zero issues; menu and chips enabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => leaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    // Leader exempt: form chip stays enabled even though zero issues.
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");

    // Menu: both MenuItems enabled.
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find((el) => el.textContent?.trim() === "New domain");
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).not.toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).not.toHaveAttribute("aria-disabled", "true");
  });

  it("mission QC: annotations disabled but form chip is enabled (empty-issue dialog allowed)", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");

    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find((el) => el.textContent?.trim() === "New domain");
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");

    // Item / option / unit Typography drop pointer cursor.
    const item = await screen.findByTestId("crf-item-name-21");
    expect(item).not.toHaveStyle({ cursor: "pointer" });

    // Annotation chip is disabled.
    const ann = await screen.findByText("item-level note");
    expect(ann.closest(".MuiChip-root")).toHaveClass("Mui-disabled");
  });

  it("mission DEV: form chip disabled when zero issues; menu stays enabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("dev")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).toHaveAttribute("aria-disabled", "true");

    // Menu still enabled (DEV can edit annotations).
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find((el) => el.textContent?.trim() === "New domain");
    expect(newDomain).not.toHaveAttribute("aria-disabled", "true");
  });

  it("mission DEV: form chip becomes enabled once an issue exists", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("dev")],
      list_issues_by_mission: () => [openedMissionIssue],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");
  });

  it("task unrelated (no role): strictest — chips and menu all disabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("other")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).toHaveAttribute("aria-disabled", "true");

    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find((el) => el.textContent?.trim() === "New domain");
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");

    const item = await screen.findByTestId("crf-item-name-21");
    expect(item).not.toHaveStyle({ cursor: "pointer" });

    const ann = await screen.findByText("item-level note");
    expect(ann.closest(".MuiChip-root")).toHaveClass("Mui-disabled");
  });
});
```

- [ ] **Step 3: Run the role-coverage tests — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx -t "role-based restrictions"
```

Expected: PASS — all 5 role-coverage tests pass.

- [ ] **Step 4: Run the full CrfDetailPage suite — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: PASS — all existing tests + the 5 new ones.

If any existing test fails because the new flags default to `false`
when `get_project_by_code` is not mocked, add `get_project_by_code: () => leaderProject` to its `mockCommands` call. The fix is purely additive — the existing tests' harness doesn't know about RBAC, and a leader-projection mock keeps the assertions they make (clicks, dialog opens) working.

- [ ] **Step 5: Run the full test suite — expect PASS**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop test
cd d:/projects/rusty/aegis && pnpm --filter @aegis/ui test
```

Expected: PASS — no regressions.

- [ ] **Step 6: Verify lint / format / typecheck**

Run:

```bash
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop typecheck
cd d:/projects/rusty/aegis && pnpm --filter @aegis/ui typecheck
cd d:/projects/rusty/aegis && pnpm --filter aegis-desktop exec vitest run --reporter=default
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis && git add apps/desktop/aegis-desktop/src/test/features/crf/crf-detail-page.test.tsx && git commit -m "test(crf): role-based restriction coverage for CrfDetailPage" -m "Adds 5 cases covering leader / QC / DEV-with-no-issues /
DEV-with-issues / task-unrelated. Pins the chip aria-disabled state
and the menu-item Mui-disabled class. Mocks get_project_by_code to
drive useIsProjectLeader.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage** — every requirement in `2026-09-22-crf-detail-role-restrictions-design.md` is mapped:

| Spec section | Task(s) |
|---|---|
| Derived flags (`canEditAnnotations`, `canOpenEmptyIssueDialog`) | Task 5 Step 1 |
| Form-name hover menu gating | Task 5 Step 4 |
| Header domain-annotation chips gating | Task 5 Step 5 |
| Form-level annotation chips (`CrfAnnotationArea`) | Tasks 2 + 3 + 5 Step 2 |
| `CrfItemRow` create affordance drop | Tasks 4 + 5 Step 3 |
| Form-code chip empty-issue gate | Task 5 Step 6 |
| Item-code chips empty-issue gate | Task 5 Step 7 |
| i18n keys (en + zhCN) | Task 1 |
| Tooltip-on-disabled wraps | Tasks 2 + 5 Steps 4/5/6/7 |
| Default to `false` when `isProjectLeader === null` | Task 5 Step 1 (matches today's `canCreate`) |
| Spec §"Testing" 5 role-coverage cases | Task 6 |
| Spec §"Non-goals" (no backend changes, no `RoleGate` abstraction) | Honoured — no Rust / Tauri / shared/api changes |

**Placeholder scan** — none. Every step has concrete code.

**Type consistency** — `canEditAnnotations` is named the same in `CrfAnnotationArea`, `CrfItemRow`, and `CrfDetailPage`. `canOpenEmptyIssueDialog` only exists on `CrfDetailPage` and `CrfItemRow` (Task 5 Step 7). The two new i18n keys are referenced verbatim across all tasks.

**Open uncertainty** — Task 6 Step 4 notes that some existing tests may need a `get_project_by_code` mock added. The fix is mechanical and explained in-place.