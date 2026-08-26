# Auto-seed Variable Descriptions by Available Language Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When `VariableEditDrawer` opens in create mode, pre-seed the `descriptions` field with one row per language in the parent's `availableLanguages` list.

**Architecture:** Add an optional `availableLanguages: string[]` prop to `VariableEditDrawer`. The parent `SdtmDomainDetail` passes its already-computed `availableLanguages`. The drawer's existing create-mode `useEffect` branch seeds `descriptions` with `{ lang: <code>, details: { label: "" } }` for each language. Edit mode and submit semantics are unchanged.

**Tech Stack:** React 18, Material UI, Vitest, @testing-library/react.

## File Structure

| File | Role | Change |
|---|---|---|
| `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx` | Drawer that owns description row state. | Add optional prop; use it in the create-mode `useEffect` to seed descriptions. |
| `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx` | Detail page that already computes `availableLanguages`. | Pass the list to the drawer at the call site. |
| `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx` | Existing component test file. | Extend the `renderDrawer` helper to accept `availableLanguages`; add 6 new cases. |

## Global Constraints

- TypeScript strict — every new prop needs an explicit type. Use `availableLanguages?: string[]`.
- Test framework: Vitest + @testing-library/react. Pattern matches existing `variable-edit-drawer.test.tsx`.
- i18n: no new strings — existing labels work.
- Submit-time filter `descriptions.filter((d) => d.lang.trim() !== "")` stays unchanged.
- `useEffect` deps stay `[open, mode, row]` — do not add `availableLanguages` to the dep array.
- Commits end with `Co-Authored-By: Claude <noreply@anthropic.com>`.

---

### Task 1: Add `availableLanguages` prop + seed descriptions in `VariableEditDrawer`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx`

**Interfaces (this task produces):**
- `VariableEditDrawerProps` gains `availableLanguages?: string[]`.
- When `open && mode === "create"`, the `descriptions` state is `availableLanguages.map((lang) => ({ lang, details: { label: "" } }))` (or `[]` if `availableLanguages` is `undefined`/empty).

- [ ] **Step 1: Extend the `renderDrawer` helper to accept `availableLanguages`**

In `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx`, update the helper signature and pass the new prop through:

```tsx
function renderDrawer(props: {
  open: boolean;
  mode: "create" | "edit";
  row?: SdtmVariableView;
  domainId?: number;
  initialSequence?: number;
  availableLanguages?: string[];
  onClose?: () => void;
  onCreate?: (i: CreateSdtmVariableInput) => void;
  onUpdate?: (id: number, b: UpdateSdtmVariableInput) => void;
  mutationError?: unknown;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <VariableEditDrawer
          open={props.open}
          mode={props.mode}
          row={props.row}
          domainId={props.domainId ?? 5}
          initialSequence={props.initialSequence ?? 3}
          availableLanguages={props.availableLanguages}
          onClose={props.onClose ?? vi.fn()}
          onCreate={props.onCreate ?? vi.fn()}
          onUpdate={props.onUpdate ?? vi.fn()}
          canMutate={true}
          mutationError={(props.mutationError ?? null) as never}
          mutationPending={false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}
```

- [ ] **Step 2: Add the six failing tests inside the existing `describe("VariableEditDrawer", ...)` block**

Append after the "renders mutation error inline" test:

```tsx
it("seeds one row per language in create mode", () => {
  renderDrawer({
    open: true,
    mode: "create",
    availableLanguages: ["en", "zh-CN"],
  });
  const langInputs = screen.getAllByRole("textbox", { name: /language/i });
  expect(langInputs).toHaveLength(2);
  expect((langInputs[0] as HTMLInputElement).value).toBe("en");
  expect((langInputs[1] as HTMLInputElement).value).toBe("zh-CN");
  const labelInputs = screen.getAllByRole("textbox", { name: /^label$/i });
  expect(labelInputs.length).toBeGreaterThan(0);
  for (const input of labelInputs) {
    expect((input as HTMLInputElement).value).toBe("");
  }
});

it("seeds no rows when availableLanguages is empty", () => {
  renderDrawer({
    open: true,
    mode: "create",
    availableLanguages: [],
  });
  expect(screen.queryAllByRole("textbox", { name: /language/i })).toHaveLength(0);
});

it("seeds a single row when there is one language", () => {
  renderDrawer({
    open: true,
    mode: "create",
    availableLanguages: ["en"],
  });
  const langInputs = screen.getAllByRole("textbox", { name: /language/i });
  expect(langInputs).toHaveLength(1);
  expect((langInputs[0] as HTMLInputElement).value).toBe("en");
});

it("loads row.descriptions in edit mode and ignores availableLanguages", () => {
  const editRow: SdtmVariableView = {
    ...sample,
    descriptions: [
      { lang: "ja", details: { label: "J-Label" } },
      { lang: "fr", details: { label: "F-Label" } },
    ],
  };
  renderDrawer({
    open: true,
    mode: "edit",
    row: editRow,
    availableLanguages: ["en"],
  });
  const langInputs = screen.getAllByRole("textbox", { name: /language/i });
  expect(langInputs).toHaveLength(2);
  expect((langInputs[0] as HTMLInputElement).value).toBe("ja");
  expect((langInputs[1] as HTMLInputElement).value).toBe("fr");
});

it("re-seeds descriptions when reopening create mode after closing", async () => {
  const { rerender } = renderDrawer({
    open: true,
    mode: "create",
    availableLanguages: ["en", "zh-CN"],
  });
  expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(2);

  const removeButtons = screen.getAllByRole("button", {
    name: /remove-description/i,
  });
  await userEvent.click(removeButtons[0]);
  expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(1);

  // Close then reopen with the same availableLanguages.
  rerender(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <VariableEditDrawer
          open={false}
          mode="create"
          row={undefined}
          domainId={5}
          initialSequence={3}
          availableLanguages={["en", "zh-CN"]}
          onClose={vi.fn()}
          onCreate={vi.fn()}
          onUpdate={vi.fn()}
          canMutate={true}
          mutationError={null}
          mutationPending={false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
  rerender(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <VariableEditDrawer
          open={true}
          mode="create"
          row={undefined}
          domainId={5}
          initialSequence={3}
          availableLanguages={["en", "zh-CN"]}
          onClose={vi.fn()}
          onCreate={vi.fn()}
          onUpdate={vi.fn()}
          canMutate={true}
          mutationError={null}
          mutationPending={false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );

  expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(2);
});

it("includes auto-seeded rows in the submitted descriptions (create mode)", async () => {
  const onCreate = vi.fn();
  renderDrawer({
    open: true,
    mode: "create",
    availableLanguages: ["en", "zh-CN"],
    onCreate,
  });
  await userEvent.type(
    screen.getByRole("textbox", { name: /name/i }),
    "AETOX",
  );
  await userEvent.click(screen.getByRole("button", { name: /create/i }));

  const submitted = onCreate.mock.calls[0][0] as CreateSdtmVariableInput;
  expect(submitted.descriptions).toEqual([
    { lang: "en", details: { label: "" } },
    { lang: "zh-CN", details: { label: "" } },
  ]);
});
```

- [ ] **Step 3: Run the new tests to confirm they fail**

Run:
```
pnpm vitest run src/test/features/domain-model/variable-edit-drawer.test.tsx --reporter=basic --pool=forks --poolOptions.forks.singleFork=true
```

Expected: the 6 new tests FAIL. The existing 3 tests should still pass. (Failure mode for the first 3 is "expected 2 elements, found 0" — descriptions state is `[]` because `availableLanguages` is unused. Failure mode for the 4th is the same — edit mode already worked but `langInputs` counts may shift if the test mocks don't include labels. Failure modes for 5/6 mirror the same.)

- [ ] **Step 4: Add `availableLanguages` to the drawer's props interface**

In `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx`, add one line to `VariableEditDrawerProps`:

```ts
export interface VariableEditDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  row?: SdtmVariableView;
  domainId: number;
  initialSequence?: number;
  availableLanguages?: string[];
  onClose: () => void;
  onCreate: (input: CreateSdtmVariableInput) => void;
  onUpdate: (id: number, body: UpdateSdtmVariableInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}
```

- [ ] **Step 5: Destructure `availableLanguages` in the function signature**

In the same file, update the parameter list:

```ts
export function VariableEditDrawer({
  open,
  mode,
  row,
  domainId,
  initialSequence,
  availableLanguages,
  onClose,
  onCreate,
  onUpdate,
  canMutate,
  mutationError,
  mutationPending,
}: VariableEditDrawerProps) {
```

- [ ] **Step 6: Update the create-mode branch of the existing `useEffect` to seed from `availableLanguages`**

In the same `useEffect`, replace the `setDescriptions([])` line with the seeded version. The full create-mode branch becomes:

```ts
} else if (mode === "create") {
  setName("");
  setVariableControlled("");
  setVariableType("Character");
  setVariableCore("Req");
  setVariableRole(null);
  const langs = availableLanguages ?? [];
  setDescriptions(
    langs.map((lang) => ({ lang, details: { label: "" } })),
  );
}
```

Do not change the `useEffect` dependency array — it stays `[open, mode, row]`. Adding `availableLanguages` would cause the seed to overwrite the user's manual edits mid-flight.

- [ ] **Step 7: Run the tests to confirm they pass**

Run the same vitest command as Step 3.

Expected: all 9 tests in `variable-edit-drawer.test.tsx` pass (3 pre-existing + 6 new).

- [ ] **Step 8: Commit**

```bash
cd "D:/projects/rusty/aegis"
git add \
  apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx \
  apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx
git commit -m "feat(domain-model): auto-seed VariableEditDrawer descriptions by available language

When the drawer opens in create mode, seed descriptions with one row per
language passed via the new optional availableLanguages prop. Edit mode
and submit-time filtering stay unchanged.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Pass `availableLanguages` from `SdtmDomainDetail`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx`

**Interfaces (this task consumes from Task 1):**
- `VariableEditDrawer` now accepts `availableLanguages?: string[]`.

- [ ] **Step 1: Locate the `VariableEditDrawer` JSX**

In `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx`, find the `<VariableEditDrawer` block (around line 256).

- [ ] **Step 2: Add the `availableLanguages` prop**

Add one prop to the existing JSX so the drawer receives the parent's already-computed list:

```tsx
<VariableEditDrawer
  open={variableDrawer !== null}
  mode={variableDrawer?.mode ?? "create"}
  row={variableDrawer?.mode === "edit" ? variableDrawer.row : undefined}
  domainId={domainId}
  initialSequence={initialSequence}
  availableLanguages={availableLanguages}
  onClose={() => setVariableDrawer(null)}
  onCreate={(input: CreateSdtmVariableInput) =>
    createVariable.mutate(input, {
      onSuccess: () => setVariableDrawer(null),
    })
  }
  onUpdate={(id: number, body: UpdateSdtmVariableInput) =>
    updateVariable.mutate(
      { id, body },
      { onSuccess: () => setVariableDrawer(null) },
    )
  }
  canMutate={canMutate}
  mutationError={createVariable.error ?? updateVariable.error ?? null}
  mutationPending={
    createVariable.isPending || updateVariable.isPending
  }
/>
```

- [ ] **Step 3: Run typecheck**

Run:
```
cd "D:/projects/rusty/aegis/apps/desktop/aegis-desktop"
pnpm typecheck
```

Expected: clean exit, no errors.

- [ ] **Step 4: Commit**

```bash
cd "D:/projects/rusty/aegis"
git add apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx
git commit -m "feat(domain-model): pass availableLanguages to VariableEditDrawer

Wire the parent's already-computed availableLanguages list into the
drawer so the create-mode seed picks it up.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Final verification

**Files:** none modified in this task — runs verification only.

- [ ] **Step 1: Run the focused test files**

Run:
```
cd "D:/projects/rusty/aegis/apps/desktop/aegis-desktop"
NODE_OPTIONS="--max-old-space-size=4096" pnpm vitest run \
  src/test/features/domain-model/variable-edit-drawer.test.tsx \
  src/test/features/domain-model/sdtm-domain-detail.test.tsx \
  src/test/shared/api.test.ts \
  --reporter=basic --pool=forks --poolOptions.forks.singleFork=true
```

Expected: all three test files pass.

- [ ] **Step 2: Run typecheck across the desktop app**

Run:
```
cd "D:/projects/rusty/aegis/apps/desktop/aegis-desktop"
pnpm typecheck
```

Expected: clean exit, no errors.

- [ ] **Step 3: Confirm `git status` is clean**

Run:
```
cd "D:/projects/rusty/aegis"
git status
```

Expected: nothing to commit, working tree clean. (Task 1 and Task 2 each committed their own scope.)