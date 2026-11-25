# CRF AnnotationDialog SUPP button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a one-click `SUPP` button to `AnnotationDialog` that drafts `XXXX in SUPPYY` content for item-level annotations and ` in SUPPYY` for non-item owners. Pure client-side content helper — no server or wire change.

**Architecture:** Add a `Stack direction="row"` wrapping the existing `TextField` plus a new outlined `Button` (literal label `SUPP`, no i18n). Add one new prop `ownerItemCode: string | null` to `AnnotationDialog`; the page resolves it from the cached form detail via a new `readOwnerItemCode` helper that mirrors the existing `readOwnerNotSubmitted`. The dialog reuses the existing `selectedDomainName` memo to read the upper-cased domain code from the currently-selected `DomainAnnotation`, so switching the domain-annotation `<Select>` instantly updates what SUPP drafts. Replace semantics; `@`-mention Popover anchors and focus retention are unchanged.

**Tech Stack:** React 19, MUI v9 (re-exported via `@aegis/ui/mui`), Vitest, `@testing-library/react`, React Testing Library `userEvent`-style via `fireEvent` (matches existing `crf-annotation-dialog.test.tsx`).

## Global Constraints

- TypeScript strict; project's `pnpm typecheck` must pass.
- Add no new dependency — both `Stack` (MUI) and the existing test scaffold are already available.
- Button label is the literal string `SUPP` — no i18n key, no tooltip, no translation entry added.
- No server / wire change. No new mutation, no new endpoint, no Rust change.
- All TDD commits must run the new test, see it fail first, then see it pass.
- Follow the codebase commit-message convention: `feat(crf): ...` for new behavior, `test(crf): ...` for test-only commits when separating them. Tests in this plan are committed together with their feature code per step.
- Do not modify `lib/packages/ui/src/i18n/locales/*` — neither `en.ts` nor `zhCN.ts` gets a new key.

---

## File Structure

| File | Responsibility |
|---|---|
| `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx` | Render the new SUPP button; compute `suppDisabled`; handle click → replace `body.content`; destructure `ownerItemCode` prop; drop the `_owner` underscore on the existing owner prop. |
| `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` | Add `readOwnerItemCode` helper next to `readOwnerNotSubmitted`; thread `ownerItemCode` into `<AnnotationDialog />`. |
| `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx` | Add the 8 SUPP test cases listed in the spec's Testing section. |

No file in `lib/packages/ui/**` is touched.

---

## Task 1: SUPP button in `AnnotationDialog` — TDD with 8 test cases

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx:1-20,86-134,290-330`
- Test: `apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx`

**Interfaces:**
- Consumes:
  - `AnnotationOwner` (existing import from `../../../shared/api`)
  - `selectedDomainName` memo already defined in this file at lines 148–153 — upper-cased name of the selected `DomainAnnotation`, or `null`.
- Produces:
  - A new `ownerItemCode: string | null` prop on `AnnotationDialog`.
  - A `data-testid="crf-annotation-dialog-supp"` `<Button>` rendered after the content `TextField`.
  - Replace-mode `handleSuppClick` that updates `body.content` only.

### Sub-task 1.1 — Test 1: SUPP button renders when open & a domain annotation is selected

**Step 1 — write the failing test.** Add at the end of `describe("AnnotationDialog", ...)` in `crf-annotation-dialog.test.tsx`:

```tsx
  // --- SUPP button ---

  it("renders the SUPP button when the dialog is open and a domain annotation is selected", () => {
    mountWithSeed();
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeInTheDocument();
  });
```

**Step 2 — run test to verify it fails.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx -t "renders the SUPP button"
```

Expected: FAIL — `Unable to find an element by: [data-testid="crf-annotation-dialog-supp"]`.

**Step 3 — implement the minimal scaffolding for the testid.** In `AnnotationDialog.tsx`:

1. Add `Stack` to the `@aegis/ui/mui` import list (alphabetical position, after `Select`).
2. Add `ownerItemCode?: string | null` to `Props` (optional + nullable — keep the prop optional so existing call sites pass `undefined` and tests that omit it still type-check).

```tsx
  /**
   * Resolved `CrfItem.code` for the annotation's owner. `null` for
   * non-item owners (form/option/unit) and for item owners whose item
   * is not in the cached form detail. Drives the SUPP button's
   * `<itemCode> in SUPP<domainCode>` draft; `null`/`undefined`
   * disables the button when nothing useful can be drafted.
   */
  ownerItemCode?: string | null;
```

3. Destructure `ownerItemCode` from `Props` (default `null`).
4. Add the SUPP button next to the content `TextField` (line 318–330 region). Wrap the existing `<TextField>` in a `<Stack direction="row" spacing={1} alignItems="center">` and add:

```tsx
            <Button
              size="small"
              variant="outlined"
              onClick={() => undefined}
              data-testid="crf-annotation-dialog-supp"
            >
              SUPP
            </Button>
```

(No `disabled` prop yet — sub-tasks 1.2 and 1.3 add the gating rules so test 2 / test 3 can fail first and drive the rule in.)

5. Add `sx={{ flexGrow: 1 }}` to the `<TextField>` props so it fills the row width.

**Step 4 — run test to verify it passes.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx -t "renders the SUPP button"
```

Expected: PASS.

**Step 5 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx
git commit -m "feat(crf): render SUPP button scaffolding in AnnotationDialog"
```

---

### Sub-task 1.2 — Test 2: SUPP disabled when no domain annotation is selected

**Step 1 — write the failing test.** Append to `describe("AnnotationDialog", ...)`:

```tsx
  it("disables the SUPP button when no domain annotation is selected", () => {
    // availableDomainAnnotations has three entries, but in create
    // mode the dialog defaults `body.domainAnnotationId` to
    // `availableDomainAnnotations[0].id`, so simulate "no selection"
    // by passing an empty list.
    mountWithSeed({ availableDomainAnnotations: [] });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeDisabled();
  });
```

**Step 2 — run test to verify it fails.** With sub-task 1.1's scaffolding, the SUPP button is unconditionally enabled. The test asserts `toBeDisabled()`, so it must FAIL — the button is, in fact, enabled. Failure message: `expected ... to be disabled`.

**Step 3 — implement the first half of `suppDisabled`.** In `AnnotationDialog.tsx`, define inside the function body (after the `selectedDomainName` memo at line 153):

```tsx
  const suppDisabled = !selectedDomainName;
```

Then bind `disabled={suppDisabled}` on the SUPP button (replace the missing `disabled` prop).

**Step 4 — run test to verify it passes.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx -t "disables the SUPP button when no domain annotation is selected"
```

Expected: PASS.

**Step 5 — fold into the next sub-task's commit.** Keep the in-progress `AnnotationDialog.tsx` diff (new prop + new button + half of `suppDisabled`) in the working tree; sub-task 1.3 extends `suppDisabled` and commits once with both halves logical. Avoid noise.

---

### Sub-task 1.3 — Test 3: SUPP disabled for item owner when `ownerItemCode` is `null`

**Step 1 — write the failing test.** Append:

```tsx
  it("disables the SUPP button for an item owner with no resolved item code", () => {
    mountWithSeed({ owner: { kind: "item", id: 99 }, ownerItemCode: null });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeDisabled();
  });
```

**Step 2 — run test to verify it fails.** Currently `suppDisabled` only checks `!selectedDomainName`, so this test will FAIL because the button is enabled.

**Step 3 — extend `suppDisabled`.** In `AnnotationDialog.tsx`, define inside the function body:

```tsx
  const suppDisabled =
    !selectedDomainName ||
    (owner.kind === "item" && !ownerItemCode);
```

Then bind `disabled={suppDisabled}` on the SUPP button.

**Step 4 — run test to verify it passes.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx -t "disables the SUPP button for an item owner"
```

Expected: PASS.

**Step 5 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx
git commit -m "feat(crf): gate SUPP button on domain annotation + item code"
```

---

### Sub-task 1.4 — Test 4: SUPP enabled for item owner with item code

This sub-task exists as a sanity test rather than a TDD driver — the `suppDisabled` rule from 1.3 already covers the enabled case.

**Step 1 — write the test.** Append:

```tsx
  it("enables the SUPP button for an item owner with a resolved item code", () => {
    mountWithSeed({ owner: { kind: "item", id: 99 }, ownerItemCode: "LBCLSIG" });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).not.toBeDisabled();
  });
```

**Step 2 — run test.** Expected to PASS on first run since the `(owner.kind === 'item' && !ownerItemCode)` rule already permits a non-empty `ownerItemCode`.

**Step 3 — if it fails, the gate rule from sub-task 1.3 regressed. Stop and fix that before continuing.**

**Step 4 — fold the test into the next sub-task's commit (1.5 introduces the click handler — natural pairing).**

---

### Sub-task 1.5 — Tests 5 & 6: Click drafts the right string (form-level + item-level)

**Step 1 — write the failing tests.** Append:

```tsx
  it("drafts ' in SUPPXX' for a form-level owner (AE)", () => {
    const { onSubmit } = mountWithSeed({ owner: { kind: "form", id: 11 } });
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(" in SUPPAE");
    // onSubmit must NOT have been triggered — the click only sets body.
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("drafts '<itemCode> in SUPPXX' for an item-level owner (LBCLSIG/LB)", () => {
    const { onSubmit } = mountWithSeed({
      owner: { kind: "item", id: 99 },
      ownerItemCode: "LBCLSIG",
      // Switch the picked domain annotation to "LB" (matches
      // domainAnnotations[1] which the test seed provides).
    });
    // The dialog defaults to domainAnnotations[0] (AE). Switch it.
    fireEvent.mouseDown(screen.getByRole("combobox"));
    // The default is "AE" (id=50) for the form-level owner case above.
    // Here we override by changing the rendered Select — pick "VS" instead
    // to test a non-default domain.
    fireEvent.click(screen.getByRole("option", { name: "VS" }));
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(
      "LBCLSIG in SUPPVS",
    );
    expect(onSubmit).not.toHaveBeenCalled();
  });
```

**Step 2 — run tests to verify they fail.** The button currently does nothing on click, so the content field stays empty.

**Step 3 — implement `handleSuppClick`.** In `AnnotationDialog.tsx`, near the other handlers (around line 192–218):

```tsx
  function handleSuppClick() {
    if (suppDisabled) return;
    const domainCode = selectedDomainName!;
    const next =
      owner.kind === "item" && ownerItemCode
        ? `${ownerItemCode} in SUPP${domainCode}`
        : ` in SUPP${domainCode}`;
    setBody((b) => ({ ...b, content: next }));
    setMentionRange(null);
    setAnchorEl(null);
    queueMicrotask(() => {
      inputRef.current?.setSelectionRange(next.length, next.length);
    });
  }
```

Drop the underscore on the existing `owner: _owner` destructure (line 90) so `owner` is in scope for `handleSuppClick`. Bind it on the SUPP button:

```tsx
              onClick={handleSuppClick}
```

**Step 4 — run tests to verify they pass.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx -t "drafts"
```

Expected: both PASS.

**Step 5 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx
git commit -m "feat(crf): SUPP click drafts '<itemCode> in SUPP<domainCode>' content"
```

---

### Sub-task 1.6 — Test 7: Existing content is replaced (not appended)

**Step 1 — write the failing test.** Append:

```tsx
  it("replaces existing content in the field (does not append)", () => {
    mountWithSeed({ owner: { kind: "form", id: 11 } });
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "pre-existing note" } });
    expect(content).toHaveValue("pre-existing note");
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(content).toHaveValue(" in SUPPAE");
  });
```

**Step 2 — run test to verify it passes.** The implementation from 1.5 already does `setBody((b) => ({ ...b, content: next }))`, which replaces. This passes on first run; if it ever fails, the test catches a regression in 1.5.

**Step 3 — no fix needed.**

**Step 4 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx
git commit -m "test(crf): SUPP replaces existing content (replace semantics)"
```

---

### Sub-task 1.7 — Test 8: Changing domain-annotation Select re-evaluates SUPP

**Step 1 — write the failing test.** Append:

```tsx
  it("re-evaluates the SUPP draft when the domain annotation changes", () => {
    mountWithSeed({ owner: { kind: "form", id: 11 } });
    // Default picked domain annotation is AE — click SUPP first.
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(" in SUPPAE");
    // Switch the picked domain annotation to VS.
    fireEvent.mouseDown(screen.getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "VS" }));
    // Click SUPP again — domain code now reads VS.
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(" in SUPPVS");
  });
```

**Step 2 — run test to verify it passes.** `selectedDomainName` is already a memo over `body.domainAnnotationId` (lines 148–153). The handler re-reads it on every click. This passes on first run.

**Step 3 — if the test fails because the content gets " in SUPPAE in SUPPVS" instead of being replaced, verify 1.6 is still correct — the handler does `setBody((b) => ({ ...b, content: next }))`, so replacement is structural.**

**Step 4 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx
git commit -m "test(crf): SUPP re-drafts when the domain annotation changes"
```

---

### Sub-task 1.8 — Final dialog test sweep

**Step 1 — run the full dialog file to make sure nothing regressed.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx
```

Expected: all tests pass (existing + 8 new SUPP tests).

**Step 2 — typecheck.**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: 0 errors. `ownerItemCode?: string | null` accepts `undefined` so no caller is forced to update yet.

---

## Task 2: Wire `ownerItemCode` from `CrfDetailPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx:81-103,691-757`

**Interfaces:**
- Produces: `readOwnerItemCode(detail, owner)` returns `string | null`. Used as: `ownerItemCode={readOwnerItemCode(detail, annotationDialog ? annotationDialog.owner : { kind: "form", id })}` on the `<AnnotationDialog />` element.

### Sub-task 2.1 — Add the helper

**Step 1 — drop the helper in next to `readOwnerNotSubmitted`.** In `CrfDetailPage.tsx`, immediately after the existing `readOwnerNotSubmitted` function (around line 103), add:

```tsx
/**
 * Look up the `CrfItem.code` for an item-level annotation owner so
 * the AnnotationDialog's SUPP button can draft `<code> in SUPP<xx>`
 * content. Returns `null` for non-item owners and for item owners
 * whose item isn't in the cached form detail — `null` is the same
 * "nothing useful to draft" signal the dialog already treats as
 * "disable the button".
 */
function readOwnerItemCode(
  detail: CrfFormDetail | undefined,
  owner: AnnotationOwner,
): string | null {
  if (!detail || owner.kind !== "item") return null;
  const found = detail.items.find((i) => i.item.id === owner.id);
  return found ? found.item.code : null;
}
```

**Step 2 — typecheck.**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: 0 errors.

**Step 3 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx
git commit -m "feat(crf): add readOwnerItemCode helper for SUPP button"
```

---

### Sub-task 2.2 — Thread the prop into `<AnnotationDialog />`

**Step 1 — extend the JSX.** In `CrfDetailPage.tsx`, locate the `<AnnotationDialog ... />` element (line 691–757). Add a new prop right after `ownerNotSubmitted={...}`:

```tsx
        ownerItemCode={readOwnerItemCode(
          detail,
          annotationDialog
            ? annotationDialog.owner
            : { kind: "form", id },
        )}
```

**Step 2 — typecheck.**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: 0 errors.

**Step 3 — run the dialog tests again to ensure the harness still passes.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-annotation-dialog.test.tsx
```

Expected: still all pass. (The dialog is unit-tested without the page, so this is a sanity check that the dialog prop interface hasn't regressed.)

**Step 4 — run the existing page tests.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```

Expected: all pass. The page-level test harness mocks `useCrfFormDetail`; if it provides `detail.items`, our new helper resolves to a string; if not, it resolves to `null`, and the SUPP button stays disabled — neither changes the existing test outcomes.

**Step 5 — commit.**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx
git commit -m "feat(crf): thread ownerItemCode from CrfDetailPage into AnnotationDialog"
```

---

## Task 3: Whole-feature verification

**Files:** None modified — verification only.

**Step 1 — typecheck the desktop app.**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: 0 errors.

**Step 2 — run the full crf test folder.**

```bash
pnpm --filter aegis-desktop test -- src/test/features/crf
```

Expected: all tests pass (existing + 8 new SUPP tests across both files).

**Step 3 — if any test fails, do NOT skip past it — read the failure, identify the bug, fix it, commit the fix with a `fix(crf):` prefix, re-run the failing test, then re-run the full sweep.**

**Step 4 — manual smoke (optional, only if you have a desktop dev shell available).** Open the desktop app, navigate to a CRF form with at least one domain annotation, open the annotation dialog for an item, click SUPP, confirm the content becomes `<ITEMCODE> in SUPP<DOMAINCODE>`. Repeat for a form-level owner (or an option) and confirm ` in SUPP<DOMAINCODE>`.

**Step 5 — final review.** `git log --oneline` should now show, on top of the prior merge:

```
954ee82 docs(spec): SUPP quick-draft button in CRF AnnotationDialog
...  feat(crf): thread ownerItemCode from CrfDetailPage into AnnotationDialog
...  feat(crf): add readOwnerItemCode helper for SUPP button
...  test(crf): SUPP re-drafts when the domain annotation changes
...  test(crf): SUPP replaces existing content (replace semantics)
...  feat(crf): SUPP click drafts '<itemCode> in SUPP<domainCode>' content
...  feat(crf): gate SUPP button on domain annotation + item code
...  feat(crf): render SUPP button scaffolding in AnnotationDialog
```

All 8 test cases and the page wiring landed across bite-sized commits.
