# SDTM Domain Create Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a `canMutate` user click a `+` icon in the `SdtmDomainList` table's operation-column header to open a drawer that creates a new SDTM domain for the currently selected version, with description rows added per language via an "Add description" button.

**Architecture:** Reuse the existing `create_sdtm_domain` Tauri command. Extend `DomainEditDrawer` with a `mode: "create" | "edit"` prop (mirroring `VariableEditDrawer`'s pattern). Add a new `useCreateSdtmDomain` hook + `api.createSdtmDomain` wrapper. Add a `+` `IconButton` in `DomainTable`'s operation-column header that calls `onCreate`.

**Tech Stack:** React 18, TanStack Query, MUI, `@aegis/ui` icons + i18n, Vitest + Testing Library, Tauri v2 (already wired).

## Global Constraints

- Spec lives at `docs/superpowers/specs/2026-08-26-sdtm-domain-create-design.md`.
- Wire DTOs: `CreateSdtmDomainRequest` on Rust uses snake_case; TS uses `CreateSdtmDomainInput` with camelCase (`versionId`, etc.). The Tauri command parameter is named `input`, so the wrapper must pass `{ input: { ...input } }` (same pattern as `createSdtmVariable`).
- `DomainEditDrawer` callers MUST continue to work without changes in edit mode. Default `mode` = `"edit"`. Existing edit-mode call sites pass `mode="edit"` explicitly.
- `canMutate = role === "admin" || role === "root"` (same as existing `SdtmDomainList`).
- The `+` header button MUST only render when `canMutate && onCreate` is provided.
- After successful create, close the drawer and stay on the list (no navigation).
- Run `pnpm --filter aegis-desktop typecheck` and the affected vitest files after each task.

## File Structure

| File | Action | Responsibility |
| --- | --- | --- |
| `apps/desktop/aegis-desktop/src/shared/api/types.ts` | modify | Add `CreateSdtmDomainInput`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts` | modify | Add `api.createSdtmDomain`, export `CreateSdtmDomainInput`. |
| `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts` | modify | Add `useCreateSdtmDomain` hook. |
| `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx` | modify | Add `mode`, `versionId`, `onCreate` props; branch reset + submit by mode. |
| `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx` | modify | Add `onCreate` prop; render `+` `IconButton` in header when `canMutate && onCreate`. |
| `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx` | modify | Wire drawer state, mutations, and handlers. |
| `lib/packages/ui/src/i18n/locales/en.ts` | modify | Add `domainModel.sdtm.create.title` + `.tooltip`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | modify | Add zh-CN strings. |
| `apps/desktop/aegis-desktop/src/test/features/domain-model/data/list.test.tsx` | modify | Add `useCreateSdtmDomain` test. |
| `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-edit-drawer.test.tsx` | modify | Add create-mode tests. |
| `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-table.test.tsx` | modify | Add `+` header button tests. |
| `apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-list.test.tsx` | modify | Add end-to-end create flow test. |

---

## Task 1: Add `CreateSdtmDomainInput` type

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts:381-386` (after `UpdateSdtmDomainInput`)

**Interfaces:**
- Consumes: existing `DomainCategory`, `SdtmDomainDescription`.
- Produces: `CreateSdtmDomainInput { versionId: number; name: string; category: DomainCategory; descriptions: SdtmDomainDescription[] }`.

- [ ] **Step 1: Add the interface**

Append immediately after the `UpdateSdtmDomainInput` block (currently ending at line 385 with `descriptions?: SdtmDomainDescription[];`):

```ts
export interface CreateSdtmDomainInput {
  versionId: number;
  name: string;
  category: DomainCategory;
  descriptions: SdtmDomainDescription[];
}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS — the new type is unused so far, but compiles.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts
git commit -m "feat(domain-model): add CreateSdtmDomainInput type"
```

---

## Task 2: Add `api.createSdtmDomain` wrapper + re-export

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts:5-40` (imports) and `:251` (after `deleteSdtmDomain` in the domain-model section)

**Interfaces:**
- Consumes: `CreateSdtmDomainInput` from Task 1, existing `api` object, `call` helper.
- Produces: `api.createSdtmDomain(input: CreateSdtmDomainInput): Promise<SdtmDomainView>`. Re-exports `CreateSdtmDomainInput` from the file's named-export list.

- [ ] **Step 1: Add the import**

In the top-of-file type import block (the `import type { … } from "./types";` line, currently around line 5), add `CreateSdtmDomainInput` to the list. Final order should be alphabetical; place it next to `CreateSdtmVariableInput`:

```ts
import type {
  CodeItemListQuery,
  …
  CreateProjectInput,
  CreateSdtmDomainInput,
  CreateSdtmVariableInput,
  …
} from "./types";
```

- [ ] **Step 2: Add the wrapper**

After the `deleteSdtmDomain` entry (line 212 in current file), insert:

```ts
createSdtmDomain: (
  input: CreateSdtmDomainInput,
): Promise<SdtmDomainView> =>
  call<SdtmDomainView>("create_sdtm_domain", { input: { ...input } }),
```

- [ ] **Step 3: Add to the type-only re-export block**

In the `export type { … }` block near the bottom (currently around line 254), add `CreateSdtmDomainInput` alongside the other `Create*` exports, alphabetically:

```ts
export type {
  …
  CreateSdtmDomainInput,
  CreateSdtmVariableInput,
  …
} from "./types";
```

- [ ] **Step 4: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/index.ts
git commit -m "feat(domain-model): add api.createSdtmDomain wrapper"
```

---

## Task 3: Add `useCreateSdtmDomain` hook

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts:1-14` (imports) and `:106-115` (after `useDeleteSdtmVariable`)

**Interfaces:**
- Consumes: `api.createSdtmDomain` from Task 2, `CreateSdtmDomainInput` from Task 1, `useQueryClient`, `useMutation`.
- Produces: `useCreateSdtmDomain(): UseMutationResult<SdtmDomainView, ApiError, CreateSdtmDomainInput>` that on success invalidates `["domainModel", "sdtmDomains", created.versionId]`.

- [ ] **Step 1: Add the import**

In the type import block at the top of `list.ts`, add `CreateSdtmDomainInput`:

```ts
import {
  api,
  type ApiError,
  type CreateSdtmDomainInput,
  type CreateSdtmVariableInput,
  type SdtmDomainView,
  …
} from "../../../shared/api";
```

- [ ] **Step 2: Add the hook**

After `useDeleteSdtmVariable` (currently the last hook in the file), append:

```ts
export function useCreateSdtmDomain() {
  const qc = useQueryClient();
  return useMutation<SdtmDomainView, ApiError, CreateSdtmDomainInput>({
    mutationFn: (input) => api.createSdtmDomain(input),
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmDomains", created.versionId],
      });
    },
  });
}
```

- [ ] **Step 3: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts
git commit -m "feat(domain-model): add useCreateSdtmDomain hook"
```

---

## Task 4: Add i18n strings

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts:262-263` (after `domainModel.sdtm.detail.saveFailed`)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts:243` (find the equivalent line)

**Interfaces:**
- Consumes: existing `domainModel.sdtm.*` keys.
- Produces: two new keys — `domainModel.sdtm.create.title` and `domainModel.sdtm.create.tooltip` — in both locales.

- [ ] **Step 1: Add English strings**

After the line `"domainModel.sdtm.detail.saveFailed": "Save failed: {message}",` in `en.ts`, insert:

```ts
"domainModel.sdtm.create.title": "Create domain",
"domainModel.sdtm.create.tooltip": "Create domain",
```

- [ ] **Step 2: Add zh-CN strings**

Find the equivalent zh-CN block (search for the line that mirrors `domainModel.sdtm.detail.saveFailed`) and insert immediately after it:

```ts
"domainModel.sdtm.create.title": "新建域",
"domainModel.sdtm.create.tooltip": "新建域",
```

- [ ] **Step 3: Typecheck the UI package**

Run: `pnpm --filter @aegis/ui typecheck`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add SDTM domain create strings (en + zh-CN)"
```

---

## Task 5: Add the `useCreateSdtmDomain` data-hook test

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/domain-model/data/list.test.tsx:19` (imports), `:75-78` (Probe body), `:103-110` (`beforeEach` mockCommands), and the `describe` block at `:127`

**Interfaces:**
- Consumes: `useCreateSdtmDomain` from Task 3.
- Produces: one new vitest case `"useCreateSdtmDomain calls the API"` asserting `mockInvoke` is called with `"create_sdtm_domain"` and `{ input: { … } }` containing the supplied fields.

- [ ] **Step 1: Import the hook**

In the import block at the top of `list.test.tsx`, add `useCreateSdtmDomain` alphabetically (after `useCreateSdtmVariable`):

```ts
import {
  useCreateSdtmDomain,
  useCreateSdtmVariable,
  …
} from "../../../../features/domain-model/data/list";
```

- [ ] **Step 2: Wire the hook into the Probe**

Inside the `Probe` function body, after the existing `const deleteVar = useDeleteSdtmVariable();` line, add:

```ts
const createDomain = useCreateSdtmDomain();
```

Add a button to trigger it (alphabetically placed, after the `create-var` button):

```tsx
<button
  data-testid="create-domain"
  onClick={() =>
    createDomain.mutate({
      versionId: props.versionId,
      name: "AENEW",
      category: "Events",
      descriptions: [
        { lang: "en", details: { description: "d", structure: "s" } },
      ],
    })
  }
>
  create-domain
</button>
```

- [ ] **Step 3: Mock the API**

In `beforeEach`'s `mockCommands({…})` call, add a `create_sdtm_domain` entry that returns a stub matching what `useCreateSdtmDomain`'s `onSuccess` expects (`created.versionId`):

```ts
create_sdtm_domain: () => ({ ...sampleDomain, id: 99, name: "AENEW" }),
```

- [ ] **Step 4: Add the test case**

Inside the existing `describe("domain-model data hooks", () => { … })`, after the `useCreateSdtmVariable` test, add:

```ts
it("useCreateSdtmDomain calls the API", async () => {
  renderProbe(1);
  screen.getByTestId("create-domain").click();
  await waitFor(() =>
    expect(mockInvoke).toHaveBeenCalledWith(
      "create_sdtm_domain",
      expect.objectContaining({
        input: expect.objectContaining({
          versionId: 1,
          name: "AENEW",
          category: "Events",
          descriptions: [
            { lang: "en", details: { description: "d", structure: "s" } },
          ],
        }),
      }),
    ),
  );
});
```

- [ ] **Step 5: Run the data-hook test**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/data/list.test.tsx`
Expected: PASS — all existing cases plus the new one.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/domain-model/data/list.test.tsx
git commit -m "test(domain-model): cover useCreateSdtmDomain"
```

---

## Task 6: Add `mode` + `onCreate` to `DomainEditDrawer`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx:1-58` (props + import block) and `:66-103` (reset effect + `handleSubmit`)

**Interfaces:**
- Consumes: `CreateSdtmDomainInput` from Task 1, existing `SdtmDomainView`, `UpdateSdtmDomainInput`, `ApiError`.
- Produces: extended props:
  ```ts
  interface DomainEditDrawerProps {
    open: boolean;
    row: SdtmDomainView;
    mode?: "create" | "edit"; // default "edit"
    versionId?: number;       // required when mode === "create"
    onClose: () => void;
    onUpdate: (id: number, body: UpdateSdtmDomainInput) => void;
    onCreate?: (input: CreateSdtmDomainInput) => void;
    canMutate: boolean;
    mutationError: ApiError | null;
    mutationPending: boolean;
  }
  ```

- [ ] **Step 1: Update the props interface**

Replace the existing `DomainEditDrawerProps` block (lines 28–36) with:

```ts
export interface DomainEditDrawerProps {
  open: boolean;
  row: SdtmDomainView;
  mode?: "create" | "edit";
  versionId?: number;
  onClose: () => void;
  onUpdate: (id: number, body: UpdateSdtmDomainInput) => void;
  onCreate?: (input: CreateSdtmDomainInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}
```

- [ ] **Step 2: Add the type import**

In the type import block at the top of the file, add `CreateSdtmDomainInput` (keep alphabetical order next to `ApiError`):

```ts
import type {
  ApiError,
  CreateSdtmDomainInput,
  DomainCategory,
  SdtmDomainDescription,
  SdtmDomainView,
  UpdateSdtmDomainInput,
} from "../../../shared/api";
```

- [ ] **Step 3: Destructure the new props**

In the function signature, extend the destructured args (currently `mode`/`versionId`/`onCreate` are missing):

```ts
export function DomainEditDrawer({
  open,
  row,
  mode = "edit",
  versionId,
  onClose,
  onUpdate,
  onCreate,
  canMutate,
  mutationError,
  mutationPending,
}: DomainEditDrawerProps) {
```

- [ ] **Step 4: Branch the reset effect**

Replace the existing `useEffect` (lines 66–73) with:

```ts
useEffect(() => {
  if (!open) return;
  if (mode === "create") {
    setName("");
    setCategory("Special Purpose");
    setDescriptions(EMPTY_DESCRIPTIONS);
  } else {
    setName(row.name);
    setCategory(row.category);
    setDescriptions(
      row.descriptions.length ? [...row.descriptions] : EMPTY_DESCRIPTIONS,
    );
  }
}, [open, mode, row]);
```

- [ ] **Step 5: Branch `handleSubmit`**

Replace the existing `handleSubmit` (lines 93–103) with:

```ts
function handleSubmit() {
  if (!canMutate) return;
  const trimmed = name.trim();
  if (trimmed === "") return;
  if (mode === "create") {
    if (versionId == null || onCreate == null) return;
    onCreate({
      versionId,
      name: trimmed,
      category,
      descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
    });
    return;
  }
  const body: UpdateSdtmDomainInput = {
    name: trimmed,
    category,
    descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
  };
  onUpdate(row.id, body);
}
```

- [ ] **Step 6: Branch the title and submit label**

Just before the `return (` at the bottom, add:

```ts
const title =
  mode === "create"
    ? t("domainModel.sdtm.create.title")
    : t("domainModel.sdtm.detail.editTitle");
const submitLabel =
  mode === "create" ? t("common.create") : t("common.save");
```

Then, in the JSX, replace the two existing references:

```tsx
<Typography variant="h6">{title}</Typography>
```

…and the submit button's text:

```tsx
<Button
  variant="contained"
  onClick={handleSubmit}
  disabled={
    !canMutate ||
    name.trim() === "" ||
    (mode === "create" && versionId == null) ||
    mutationPending
  }
>
  {submitLabel}
</Button>
```

- [ ] **Step 7: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx
git commit -m "feat(domain-model): add create mode to DomainEditDrawer"
```

---

## Task 7: Add `+` header button to `DomainTable`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx:14-23` (props + imports) and `:95-103` (header row)

**Interfaces:**
- Consumes: `onCreate: () => void` from the page; existing `canMutate`.
- Produces: an optional `onCreate` prop. When `canMutate && onCreate`, the empty header cell becomes a right-aligned cell with a `+` `IconButton` wrapped in a `Tooltip` (`t("domainModel.sdtm.create.tooltip")`). Otherwise the cell is hidden (`null`).

- [ ] **Step 1: Add the `onCreate` prop to the props interface**

After the `onNavigate?: ...` field (line 39), add:

```ts
onCreate?: () => void;
```

- [ ] **Step 2: Update the imports**

In the `@aegis/ui/icons` import, add `Add as AddIcon` (keep alphabetical):

```ts
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  OpenInNew as OpenInNewIcon,
} from "@aegis/ui/icons";
```

- [ ] **Step 3: Add `onCreate` to the destructured props**

In the function signature, add `onCreate` to the destructured args (after `onNavigate`).

- [ ] **Step 4: Render the header button**

Replace the empty `<TableCell />` inside the header `TableRow` (line 101) with:

```tsx
<TableCell align="right">
  {canMutate && onCreate && (
    <Tooltip title={t("domainModel.sdtm.create.tooltip")}>
      <IconButton
        size="small"
        aria-label={t("domainModel.sdtm.create.tooltip")}
        onClick={onCreate}
      >
        <AddIcon fontSize="small" />
      </IconButton>
    </Tooltip>
  )}
</TableCell>
```

- [ ] **Step 5: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx
git commit -m "feat(domain-model): add + header button to DomainTable"
```

---

## Task 8: Wire the drawer into `SdtmDomainList`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx:1-20` (imports) and `:35-36` (state) and `:127-130` (mutation hooks) and `:165-200` (render body)

**Interfaces:**
- Consumes: `useCreateSdtmDomain` from Task 3, `useUpdateSdtmDomain` (already exists), extended `DomainEditDrawer` from Task 6, extended `DomainTable` from Task 7.
- Produces: a `domainDrawer` discriminated union state (`null | { mode: "edit"; row: SdtmDomainView } | { mode: "create" }`), wires `onCreate={() => setDomainDrawer({ mode: "create" })}` into `DomainTable`, renders `<DomainEditDrawer>` below `DeleteDomainDialog`, with success closing the drawer.

- [ ] **Step 1: Update imports**

In the component imports, extend:
- data hook imports — add `useUpdateSdtmDomain`:
  ```ts
  import {
    useCreateSdtmDomain,
    useDeleteSdtmDomain,
    useListSdtmDomains,
    useListSdtmVersions,
    useUpdateSdtmDomain,
  } from "../data";
  ```
- shared api type import — add `CreateSdtmDomainInput`:
  ```ts
  import type {
    CreateSdtmDomainInput,
    SdtmDomainView,
  } from "../../../shared/api";
  ```

- [ ] **Step 2: Add the drawer state and mutation hooks**

Below the existing `const [confirmDelete, setConfirmDelete] = useState<SdtmDomainView | null>(null);` line, add:

```ts
type DomainDrawerState =
  | { mode: "edit"; row: SdtmDomainView }
  | { mode: "create" }
  | null;
const [domainDrawer, setDomainDrawer] = useState<DomainDrawerState>(null);
```

Below the existing `const deleteDomain = useDeleteSdtmDomain();` line, add:

```ts
const updateDomain = useUpdateSdtmDomain();
const createDomain = useCreateSdtmDomain();
```

- [ ] **Step 3: Pass `onCreate` to `DomainTable`**

In the `<DomainTable … />` JSX, add `onCreate={() => setDomainDrawer({ mode: "create" })}`:

```tsx
<DomainTable
  rows={filteredRows}
  loading={domainsQuery.isLoading}
  error={domainsQuery.error}
  canMutate={canMutate}
  selectedLang={selectedLang}
  onRetry={() => domainsQuery.refetch()}
  onCreate={() => setDomainDrawer({ mode: "create" })}
  onDelete={(row) => setConfirmDelete(row)}
  onNavigate={(row) =>
    navigate({
      to: "/domain-model/sdtm/$domainId",
      params: { domainId: String(row.id) },
      search: { lang: selectedLang ?? undefined },
    })
  }
  emptyMessage={
    trimmedFragment
      ? t("domainModel.sdtm.noMatches")
      : t("domainModel.sdtm.empty")
  }
/>
```

- [ ] **Step 4: Render `<DomainEditDrawer>` after `DeleteDomainDialog`**

After the `</DeleteDomainDialog>` block (currently lines 190–201), insert:

```tsx
{domainDrawer?.mode === "edit" && (
  <DomainEditDrawer
    open
    mode="edit"
    row={domainDrawer.row}
    onClose={() => setDomainDrawer(null)}
    onUpdate={(_id, body) =>
      updateDomain.mutate(
        { id: domainDrawer.row.id, body },
        { onSuccess: () => setDomainDrawer(null) },
      )
    }
    canMutate={canMutate}
    mutationError={updateDomain.error ?? null}
    mutationPending={updateDomain.isPending}
  />
)}

{domainDrawer?.mode === "create" && (
  <DomainEditDrawer
    open
    mode="create"
    row={{} as SdtmDomainView}
    versionId={selectedVersionId ?? undefined}
    onClose={() => setDomainDrawer(null)}
    onCreate={(input: CreateSdtmDomainInput) =>
      createDomain.mutate(input, {
        onSuccess: () => setDomainDrawer(null),
      })
    }
    canMutate={canMutate}
    mutationError={createDomain.error ?? null}
    mutationPending={createDomain.isPending}
  />
)}
```

Note: in create mode the drawer ignores `row`; we still pass a typed cast to satisfy the prop signature. The `useEffect` reset always overwrites it because `mode === "create"` short-circuits the row branch before any field is read.

- [ ] **Step 5: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx
git commit -m "feat(domain-model): wire create-domain drawer into SdtmDomainList"
```

---

## Task 9: Add `DomainTable` header button tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-table.test.tsx:31-45` (`renderTable` helper) and `:47` (`describe` block)

**Interfaces:**
- Consumes: `onCreate?: () => void` from Task 7.
- Produces: three new vitest cases:
  1. Header `+` button is visible when `canMutate=true` AND `onCreate` is provided.
  2. Header `+` button is hidden when `canMutate=false`.
  3. Header `+` button is hidden when `onCreate` is not provided.
  4. Clicking the `+` button calls `onCreate`.

- [ ] **Step 1: Extend `renderTable` typing**

The existing helper (`renderTable(props: Partial<React.ComponentProps<typeof DomainTable>> = {})`) already accepts any prop, so no change is needed.

- [ ] **Step 2: Add the four tests**

Inside the existing `describe("DomainTable", …)`, append:

```ts
it("renders the create header button when canMutate=true and onCreate is provided", () => {
  renderTable({ canMutate: true, onCreate: vi.fn() });
  expect(
    screen.getByRole("button", { name: /create domain/i }),
  ).toBeInTheDocument();
});

it("hides the create header button when canMutate=false", () => {
  renderTable({ canMutate: false, onCreate: vi.fn() });
  expect(
    screen.queryByRole("button", { name: /create domain/i }),
  ).not.toBeInTheDocument();
});

it("hides the create header button when onCreate is not provided", () => {
  renderTable({ canMutate: true });
  expect(
    screen.queryByRole("button", { name: /create domain/i }),
  ).not.toBeInTheDocument();
});

it("calls onCreate when the header create button is clicked", async () => {
  const onCreate = vi.fn();
  renderTable({ canMutate: true, onCreate });
  await userEvent.click(
    screen.getByRole("button", { name: /create domain/i }),
  );
  expect(onCreate).toHaveBeenCalledOnce();
});
```

- [ ] **Step 3: Run the table tests**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/domain-table.test.tsx`
Expected: PASS — all existing cases plus the four new ones.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/domain-model/domain-table.test.tsx
git commit -m "test(domain-model): cover DomainTable create header button"
```

---

## Task 10: Add `DomainEditDrawer` create-mode tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/domain-model/domain-edit-drawer.test.tsx:29-50` (`renderDrawer` helper) and `:52-78` (`describe` block)

**Interfaces:**
- Consumes: extended `DomainEditDrawer` props from Task 6.
- Produces: three new vitest cases:
  1. Create mode renders the title `Create domain` and a submit button labelled `Create`.
  2. Submitting in create mode calls `onCreate` with the expected input shape.
  3. Submit is disabled when `versionId` is `undefined` in create mode.

- [ ] **Step 1: Extend `renderDrawer` to accept the new props**

Replace the existing helper signature/body with:

```ts
function renderDrawer(props: {
  mode?: "create" | "edit";
  row?: SdtmDomainView;
  versionId?: number;
  onUpdate?: (id: number, b: UpdateSdtmDomainInput) => void;
  onCreate?: (input: CreateSdtmDomainInput) => void;
  pending?: boolean;
  error?: unknown;
}) {
  const row = props.row ?? sample;
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DomainEditDrawer
          open={true}
          mode={props.mode ?? "edit"}
          row={row}
          versionId={props.versionId}
          onClose={vi.fn()}
          onUpdate={props.onUpdate ?? vi.fn()}
          onCreate={props.onCreate ?? vi.fn()}
          canMutate={true}
          mutationError={(props.error ?? null) as never}
          mutationPending={props.pending ?? false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}
```

Also update the import block at the top of the file to add `CreateSdtmDomainInput`:

```ts
import type {
  CreateSdtmDomainInput,
  SdtmDomainView,
  UpdateSdtmDomainInput,
} from "../../../shared/api";
```

- [ ] **Step 2: Add the three tests**

Inside the existing `describe("DomainEditDrawer", …)`, append:

```ts
it("renders the Create title and submit label in create mode", () => {
  renderDrawer({ mode: "create", versionId: 5 });
  expect(screen.getByText("Create domain")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /^Create$/ })).toBeInTheDocument();
});

it("submits the new domain via onCreate in create mode", async () => {
  const onCreate = vi.fn();
  renderDrawer({ mode: "create", versionId: 5, onCreate });
  const nameInput = screen.getByRole("textbox", { name: /^code$/i });
  await userEvent.type(nameInput, "AE");
  await userEvent.click(screen.getByRole("button", { name: /^Create$/ }));
  expect(onCreate).toHaveBeenCalledOnce();
  expect(onCreate).toHaveBeenCalledWith({
    versionId: 5,
    name: "AE",
    category: "Special Purpose",
    descriptions: [],
  });
});

it("disables the create submit button when versionId is undefined", () => {
  renderDrawer({ mode: "create" });
  expect(screen.getByRole("button", { name: /^Create$/ })).toBeDisabled();
});
```

- [ ] **Step 3: Run the drawer tests**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/domain-edit-drawer.test.tsx`
Expected: PASS — all existing cases plus the three new ones.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/domain-model/domain-edit-drawer.test.tsx
git commit -m "test(domain-model): cover DomainEditDrawer create mode"
```

---

## Task 11: Add end-to-end create flow test on `SdtmDomainList`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-list.test.tsx:66-85` (`beforeEach` mockCommands) and `:102-149` (`describe` block)

**Interfaces:**
- Consumes: `SdtmDomainList` (post-Task 8), `api.createSdtmDomain` wrapper (Task 2).
- Produces: one new vitest case that opens the drawer from the `+` button, types a name, clicks the "Add description" button, fills a description row, submits, and asserts the row appears in the table.

- [ ] **Step 1: Mock `create_sdtmDomain` in `beforeEach`**

In the `mockCommands({…})` block, add:

```ts
create_sdtm_domain: () => ({
  id: 99,
  versionId: 1,
  name: "ZZ",
  category: "Findings",
  descriptions: [
    { lang: "en", details: { description: "ZZ created", structure: "One per ZZ" } },
  ],
  createdAt: "",
  updatedAt: "",
}),
```

Also extend the existing `list_sdtm_domains_by_version` so it can be re-invoked after create and include the new row. Replace the existing handler with:

```ts
list_sdtm_domains_by_version: () => ({
  domains: [
    …domains,
    {
      id: 99,
      versionId: 1,
      name: "ZZ",
      category: "Findings",
      descriptions: [
        { lang: "en", details: { description: "ZZ created", structure: "One per ZZ" } },
      ],
      createdAt: "",
      updatedAt: "",
    },
  ],
}),
```

(Use the spread syntax the file already supports; if the current version of the test inlines the array, copy the existing two `domains` entries and append the new one.)

- [ ] **Step 2: Add the end-to-end test**

Inside the existing `describe("SdtmDomainList", …)`, append:

```ts
it("opens the create drawer from the header + button and creates a new domain", async () => {
  renderPage();
  const createBtn = await screen.findByRole("button", { name: /create domain/i });
  await userEvent.click(createBtn);
  const nameInput = await screen.findByRole("textbox", { name: /^code$/i });
  await userEvent.type(nameInput, "ZZ");
  await userEvent.click(screen.getByRole("button", { name: /add description/i }));
  const descInput = await screen.findByRole("textbox", { name: /^description$/i });
  await userEvent.type(descInput, "ZZ created");
  const structInput = screen.getByRole("textbox", { name: /^structure$/i });
  await userEvent.type(structInput, "One per ZZ");
  await userEvent.click(screen.getByRole("button", { name: /^Create$/ }));
  expect(await screen.findByText("ZZ created")).toBeInTheDocument();
  expect(screen.getByText("One per ZZ")).toBeInTheDocument();
});
```

- [ ] **Step 3: Run the list-page test**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/sdtm-domain-list.test.tsx`
Expected: PASS — all existing cases plus the new one.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-list.test.tsx
git commit -m "test(domain-model): cover SdtmDomainList end-to-end create flow"
```

---

## Task 12: Full verification

**Files:** none modified.

- [ ] **Step 1: Typecheck the workspace**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 2: Run the full frontend test suite**

Run: `pnpm --filter aegis-desktop test`
Expected: PASS — every existing test plus the new ones from Tasks 5, 9, 10, 11.

- [ ] **Step 3: Run rustfmt/clippy on the touched Rust crates (no expected changes)**

Run: `cargo fmt --all -- --check`
Expected: PASS — no Rust files were modified, but verify nothing got formatted by a hook.

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 4: Manual smoke (per spec)**

1. Sign in as `admin`/`root`.
2. Pick a version.
3. Click the `+` icon in the operation-column header.
4. Verify the drawer opens with empty `Name`, default category, no description rows.
5. Click "Add description", type `en`, fill description + structure.
6. Submit. Drawer closes; new row appears in the table.
7. Repeat on a version with zero domains — verify the drawer opens with no rows and the "Add description" button is the only way to add a row.

- [ ] **Step 5: Final summary commit (no changes)**

If no changes were made in this task, skip. Otherwise `git status` should be clean and the implementation is complete.

---

## Self-Review Notes (author)

- Spec coverage: types (Task 1), api wrapper (Task 2), hook (Task 3), i18n (Task 4), drawer mode (Task 6), header button (Task 7), page wiring (Task 8), tests (Tasks 5, 9, 10, 11), verification (Task 12). All spec sections covered.
- Placeholder scan: no TBDs, no "fill in details", all code blocks contain the actual code.
- Type consistency: `CreateSdtmDomainInput` defined in Task 1; consumed in Tasks 2, 3, 6, 8, 10. `api.createSdtmDomain` defined in Task 2; consumed in Task 3. `useCreateSdtmDomain` defined in Task 3; consumed in Tasks 5, 8. `mode`, `versionId`, `onCreate` props introduced in Task 6; consumed in Tasks 8, 10. `onCreate` prop added in Task 7; consumed in Tasks 8, 9.
- All `onCreate` / `onUpdate` flows close the drawer on success (Task 8 step 4).
- `canMutate` gating is uniform: page computes it (`role === "admin" || role === "root"`), passes to both `DomainTable` and `DomainEditDrawer`.
- No Rust changes — the existing `create_sdtm_domain` command already does what we need.