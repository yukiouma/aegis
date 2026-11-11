# Create Project → Initialize Project Folder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "Initialize project folder" checkbox to the project drawer's create mode so that confirming a create also materializes the project's folder on disk from a chosen structure template.

**Architecture:** The `InitializeProjectFolderDialog` and its `useInitializeProjectFolder` hook move out of `features/project-workspace` into a new shared `features/scaffold` feature, so `project-list` and `project-workspace` can both depend on it without depending on each other. The dialog gains one optional `beforeInit` hook — awaited before the init command — which the drawer uses to create the project row first, so a rejected duplicate code never leaves files on disk. The drawer splits its submit into "create only" vs. "open the template step, then create + initialize".

**Tech Stack:** React 18 + TypeScript, TanStack Query, MUI (via `@aegis/ui/mui`), Vitest + Testing Library. No Rust change, no server change.

**Spec:** [`docs/superpowers/specs/2026-11-11-desktop-create-project-initialize-folder-design.md`](../specs/2026-11-11-desktop-create-project-initialize-folder-design.md) — the plan argues from the spec, so the spec travels with it; executors read both.

## Global Constraints

These apply to every task below. They are copied verbatim from the spec.

- **Desktop-only.** No change to the server, the `project` crate, the `scaffolding` crate, or the `initialize_project_folder` command.
- **The project is created BEFORE the folder is initialized.** A duplicate code must not leave files on disk.
- **A `beforeInit` rejection aborts the whole operation.** The init command is not run, the dialog closes, and the caller's own mutation state holds the error.
- **The project row survives a folder-init failure; the folder does not exist.** The drawer closes and the page raises a Snackbar.
- **No branch is created.** The init runs with `branch: null`; `{{ branch }}` nodes are skipped in the report.
- **No code-shape validation.** A project code with no `-` yields no `trial`; the report lists the skipped nodes with their reasons. The create form does not block it.
- **`study` is the trimmed project code** — the same string sent to `create_project`.
- **The checkbox is create-mode only**, a MUI `Checkbox` (not a `Switch`), **checked by default**, and reset to checked whenever the drawer enters create mode.
- **The init command's invoke keys** are `structureId`, `study`, `branch` — unchanged by this work.
- **Wire DTOs are duplicated by hand**, not generated. No wire shape changes here.
- **Frontend state lives in TanStack Query**; query keys come from the factory in `src/shared/query/keys.ts`, never inline arrays.
- **The default `QueryClient` is non-retrying** — `staleTime: Infinity`, `retry: false`.
- **Cross-feature imports go through the feature's `index.ts` barrel**, never a deep path. `features/scaffold` is imported as `from "../../scaffold"`.
- **Both i18n catalogs must carry the same key set** — `lib/packages/ui/src/i18n/registry.test.ts` asserts it. Any key added to `en.ts` is added to `zhCN.ts` in the same commit.
- **All user-visible strings come through `useI18n()` → `t("path.to.key")`.** New keys go in the `@aegis/ui` translation resources.

---

## File Structure

**New feature — `apps/desktop/aegis-desktop/src/features/scaffold/`:**

| File | Responsibility |
| --- | --- |
| `index.ts` | The feature's public barrel. Exports the dialog and the hook. |
| `components/InitializeProjectFolderDialog.tsx` | The template picker + init dialog. Gains `beforeInit` / `onInitError` and a pending guard. |
| `data/project-folder.ts` | `useInitializeProjectFolder` — the mutation that runs the Tauri command. |

**Modified:**

| File | Change |
| --- | --- |
| `src/features/project-workspace/index.ts` | Stop exporting the dialog and the hook directly; re-export them from `../scaffold` so existing consumers keep working. |
| `src/features/project-workspace/components/ProjectBranchesSection.tsx` | Import the dialog from `../../scaffold`. |
| `src/features/project-workspace/data/project-branches.ts` | Delete `useInitializeProjectFolder` (moved to `features/scaffold/data/project-folder.ts`). |
| `src/features/project-list/components/ProjectDrawer.tsx` | The checkbox, the split submit, the `beforeInit` / `onInitError` wiring, the pending snapshot. |
| `src/features/project-list/pages/ProjectListPage.tsx` | Own the Snackbar; pass `onFolderInitFailed` down. |
| `lib/packages/ui/src/i18n/locales/en.ts` | Rename 13 `project.branches.initialize.*` keys to `scaffold.initialize.*`; add 5 new keys. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts:146-159` | The same, with the Chinese strings. |

**Test files:**

| File | Change |
| --- | --- |
| `src/test/features/scaffold/initialize-project-folder-dialog.test.tsx` | Moved from `src/test/features/project-workspace/`, plus new cases for `beforeInit` / `onInitError` / pending. |
| `src/test/features/project-list/project-drawer.test.tsx` | New cases for the checkbox and the two-phase submit. Two existing cases are rewritten (see Task 4). |
| `src/test/features/project-list/project-list-page.test.tsx` | New case: a folder-init failure raises the Snackbar. |

---

## Review Focus

Conditions a reasonable person would expect to work, that the spec implies but that no task's tests otherwise pin down. Each line names the condition and the behavior expected; the owning task carries the test.

1. **A project code with no `-`** (e.g. `AK001`) — the project is still created and the folder is still initialized; only the `{{ trial }}` nodes are skipped, and the report says so. Not an error, not a block. *(Task 4)*
2. **The template list is empty** — the dialog says "no templates available" rather than showing a dead Confirm button, and the user can still cancel and untick the box to create a project without a folder. *(Task 3)*
3. **The template list fails to load** — the dialog shows the error instead of an empty select. *(Task 3)*
4. **Double-clicking Confirm** — `create_project` fires once, not twice. *(Task 2)*
5. **The `study` sent to the scaffold is the same string as the `code` sent to `create_project`** — both are the trimmed code the user typed, even if the form is edited afterwards. *(Task 4)*

---

### Task 1: Extract the `features/scaffold` feature

Move the dialog and its hook into a new shared feature, and rename the dialog's i18n keys to match. No behaviour change — this task is a pure move, and its test is that the existing suite still passes.

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/scaffold/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/scaffold/data/project-folder.ts`
- Create: `apps/desktop/aegis-desktop/src/features/scaffold/components/InitializeProjectFolderDialog.tsx` (moved from `src/features/project-workspace/components/`)
- Delete: `apps/desktop/aegis-desktop/src/features/project-workspace/components/InitializeProjectFolderDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-branches.ts:95-109` (remove the hook)
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts:1-20`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ProjectBranchesSection.tsx:27`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts:148-165`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts:146-163`
- Move: `apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx` → `apps/desktop/aegis-desktop/src/test/features/scaffold/initialize-project-folder-dialog.test.tsx`

**Interfaces:**
- Consumes: nothing from later tasks.
- Produces:
  - `features/scaffold/index.ts` exports `InitializeProjectFolderDialog` and `useInitializeProjectFolder`.
  - `useInitializeProjectFolder(): UseMutationResult<InitReport, ApiError, { structureId: number; study: string; branch?: string | null }>` — the signature moves unchanged.
  - `InitializeProjectFolderDialog` keeps its current props: `{ open: boolean; projectCode: string; branch?: ProjectBranchView; onClose: () => void; onInitialized: (report: InitReport) => void }`. Task 2 widens it.
  - The i18n namespace `project.branches.initialize.*` no longer exists; it is now `scaffold.initialize.*`.

- [ ] **Step 1: Create the scaffold feature's data module**

Create `apps/desktop/aegis-desktop/src/features/scaffold/data/project-folder.ts` with the hook moved verbatim from `features/project-workspace/data/project-branches.ts` (lines 95-109), keeping its doc comment and its `useMutation` body exactly as-is. Its imports become:

```ts
import { useMutation } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type InitReport,
} from "../../../shared/api";
```

- [ ] **Step 2: Move the dialog into the scaffold feature**

`git mv` the dialog file to `apps/desktop/aegis-desktop/src/features/scaffold/components/InitializeProjectFolderDialog.tsx`. Adjust its imports: the hook now comes from `"../data/project-folder"` instead of `"../data/project-branches"`; `useListTemplateStructures` still comes from `"../../templates"`; `errorMessage` and the shared types still come from `"../../../shared/api"` and `"../../../shared/api/error"`. Change nothing else — same props, same markup, same behaviour.

- [ ] **Step 3: Rename the dialog's i18n keys in both catalogs**

In `en.ts` and `zhCN.ts`, rename these 13 keys by replacing the `project.branches.initialize.` prefix with `scaffold.initialize.`:

```
initialize.title  initialize.structure  initialize.required  initialize.failed
initialize.report.created  initialize.report.skipped
initialize.report.reason.alreadyExists  initialize.report.reason.unresolvedField
initialize.report.reason.malformedTemplate  initialize.report.reason.invalidName
initialize.report.reason.noReference  initialize.report.reason.referenceFailed
initialize.report.reason.migrationFailed
```

**`project.branches.initializeFolder` does NOT move** — it is the branch-row button's `aria-label`, not a dialog key, and it stays in `project.branches.*`.

Add a new `scaffold.*` group to both catalogs. In `en.ts`, place it after the `project.*` block (after line 173, `'project.action.save'`); in `zhCN.ts`, after its `project.action.save` line.

The rename above already produces `scaffold.initialize.failed`. Add these **five** further keys to each file:

| Key | en | zh-CN |
| --- | --- | --- |
| `scaffold.initialize.listFailed` | `Could not load structure templates: {message}` | `加载结构模板失败：{message}` |
| `scaffold.initialize.noTemplates` | No structure templates are available. | 没有可用的结构模板。 |
| `project.field.initializeFolder` | Initialize project folder | 初始化项目文件夹 |
| `project.field.initializeFolder.hint` | Pick a structure template to create the project folder on this machine | 选择一个结构模板，在本机创建项目文件夹 |
| `project.folder.failed` | The project was created, but its folder could not be initialized: {message} | 项目已创建，但项目文件夹初始化失败：{message} |

The `project.field.*` and `project.folder.*` keys go in the `project.*` group, not the `scaffold.*` group. The renamed `scaffold.initialize.*` keys carry their existing values unchanged — only the key names move.

- [ ] **Step 4: Update the dialog's key references**

In the moved dialog, replace every `project.branches.initialize.` string literal with `scaffold.initialize.` — including the `SKIP_REASON_KEYS` map's seven `TranslationKey` values.

- [ ] **Step 5: Create the feature barrel**

Create `apps/desktop/aegis-desktop/src/features/scaffold/index.ts`:

```ts
// Public API of the scaffold feature.
export { InitializeProjectFolderDialog } from "./components/InitializeProjectFolderDialog";
export { useInitializeProjectFolder } from "./data/project-folder";
```

- [ ] **Step 6: Rewire project-workspace**

In `src/features/project-workspace/index.ts`, remove the `useInitializeProjectFolder` entry from the `./data/project-branches` export block and remove the `export { InitializeProjectFolderDialog } from "./components/InitializeProjectFolderDialog";` line. Add a re-export so existing consumers of the project-workspace barrel keep working:

```ts
export {
  InitializeProjectFolderDialog,
  useInitializeProjectFolder,
} from "../scaffold";
```

In `src/features/project-workspace/components/ProjectBranchesSection.tsx`, change the dialog import from `"./InitializeProjectFolderDialog"` to `"../../scaffold"`.

In `src/features/project-workspace/data/project-branches.ts`, delete the `useInitializeProjectFolder` function and its doc comment (lines 95-109), and drop the now-unused `type InitReport` entry from the `../../../shared/api` import list. The `ApiError` import stays — `useCreateProjectBranch` and `useUpdateProjectBranch` still use it.

- [ ] **Step 7: Move the dialog's test file**

`git mv apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx apps/desktop/aegis-desktop/src/test/features/scaffold/initialize-project-folder-dialog.test.tsx` and create `apps/desktop/aegis-desktop/src/test/features/scaffold/` if it does not exist. Update the dialog import in it to `"../../../features/scaffold/components/InitializeProjectFolderDialog"`. Change nothing else — the existing assertions must still hold.

- [ ] **Step 8: Verify the move is behaviour-preserving**

Run:
```bash
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop test -- src/test/features/scaffold/initialize-project-folder-dialog.test.tsx src/test/features/project-workspace/project-branches-section.test.tsx
pnpm --filter aegis-desktop typecheck
```
Expected: the i18n registry test passes (both catalogs have the same key set), both feature test files pass, and typecheck is clean.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/scaffold apps/desktop/aegis-desktop/src/features/project-workspace apps/desktop/aegis-desktop/src/test/features/scaffold apps/desktop/aegis-desktop/src/test/features/project-workspace lib/packages/ui/src/i18n/locales
git commit -m "refactor(desktop): extract the scaffold feature from project-workspace

The dialog and its hook become a shared feature so project-list and
project-workspace can both use them without depending on each other.
The dialog's i18n keys move to the scaffold.* namespace to match."
```

---

### Task 2: Add `beforeInit` and `onInitError` to the dialog

Give the dialog the two hooks the create flow needs, and the pending guard that keeps a double-click from firing `create_project` twice.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/scaffold/components/InitializeProjectFolderDialog.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/scaffold/initialize-project-folder-dialog.test.tsx`

**Interfaces:**
- Consumes: the dialog and hook from Task 1.
- Produces:
  ```ts
  export interface InitializeProjectFolderDialogProps {
    open: boolean;
    /** The project code — the context's `study`. */
    projectCode: string;
    /** The branch being initialized, when opened from a branch row. */
    branch?: ProjectBranchView;
    /**
     * Awaited after the user confirms a template and before the folder
     * is materialized. Rejecting aborts the whole operation — nothing
     * is written to disk — and closes the dialog, so the caller can
     * show the error where the user can act on it.
     */
    beforeInit?: () => Promise<void>;
    /**
     * Called when the folder could not be materialized. When omitted,
     * the dialog keeps itself open and renders the error inline, so
     * the user can retry.
     */
    onInitError?: (error: ApiError) => void;
    onClose: () => void;
    onInitialized: (report: InitReport) => void;
  }
  ```
  Task 4 consumes `beforeInit` and `onInitError`.

- [ ] **Step 1: Write the failing tests**

Add to `src/test/features/scaffold/initialize-project-folder-dialog.test.tsx`. Extend the existing `renderDialog` helper so it accepts optional `beforeInit` and `onInitError` overrides, and returns them alongside `onClose` and `onInitialized`.

Add these cases:

```tsx
it("awaits beforeInit before running the command", async () => {
  const order: string[] = [];
  const { } = renderDialog({
    beforeInit: async () => {
      order.push("beforeInit");
    },
  });
  // ...select "Minimal", click Confirm...
  await waitFor(() =>
    expect(order).toEqual(["beforeInit"]),
  );
  expect(invoke).toHaveBeenCalledWith("initialize_project_folder", {
    structureId: 2,
    study: "AK001-001",
    branch: null,
  });
});

it("aborts without running the command when beforeInit rejects", async () => {
  const { onClose } = renderDialog({
    beforeInit: async () => {
      throw { kind: "http", status: 409, code: "duplicate_code", message: "dup" };
    },
  });
  // ...select "Minimal", click Confirm...
  await waitFor(() => expect(onClose).toHaveBeenCalled());
  expect(invoke).not.toHaveBeenCalledWith(
    "initialize_project_folder",
    expect.anything(),
  );
});

it("calls onInitError instead of showing the inline error when supplied", async () => {
  // mock initialize_project_folder to reject
  const { onInitError } = renderDialog({ onInitError: vi.fn() });
  // ...select "Minimal", click Confirm...
  await waitFor(() => expect(onInitError).toHaveBeenCalled());
  expect(screen.queryByText(/boom/)).not.toBeInTheDocument();
});

it("keeps the dialog open and shows the inline error when onInitError is omitted", async () => {
  // mock initialize_project_folder to reject
  renderDialog();
  // ...select "Minimal", click Confirm...
  expect(await screen.findByText(/boom/)).toBeInTheDocument();
});

it("disables Confirm while beforeInit is in flight", async () => {
  let release: () => void = () => {};
  renderDialog({
    beforeInit: () =>
      new Promise<void>((resolve) => {
        release = resolve;
      }),
  });
  // ...select "Minimal", click Confirm...
  await waitFor(() =>
    expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled(),
  );
  release();
  await waitFor(() =>
    expect(screen.getByRole("button", { name: /confirm/i })).toBeEnabled(),
  );
});

it("runs beforeInit once when Confirm is double-clicked", async () => {
  let calls = 0;
  const { } = renderDialog({
    beforeInit: async () => {
      calls += 1;
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
    },
  });
  // ...select "Minimal", click Confirm twice...
  await waitFor(() => expect(calls).toBe(1));
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/scaffold/initialize-project-folder-dialog.test.tsx
```
Expected: FAIL — the `beforeInit` and `onInitError` props do not exist, so TypeScript rejects the helper's arguments.

- [ ] **Step 3: Implement the two props**

In the dialog, add `beforeInit` and `onInitError` to `InitializeProjectFolderDialogProps` and destructure them with their defaults (`undefined`).

Replace `handleConfirm`'s body with this sequence — `beforeInit` first, the init command second, and the two failure paths kept distinct:

```tsx
async function handleConfirm() {
  if (!selected) return;
  setPreparing(true);
  try {
    if (beforeInit) {
      try {
        await beforeInit();
      } catch {
        // The caller's own mutation state holds the error. The init
        // command never runs, so nothing is written to disk.
        onClose();
        return;
      }
    }
    const result = await initialize.mutateAsync({
      structureId: selected.id,
      study: projectCode,
      branch: branch?.branch ?? null,
    });
    setReport(result);
    onInitialized(result);
  } catch (e) {
    if (onInitError) {
      onInitError(toApiError(e));
    }
    // Otherwise the inline Alert renders initialize.error, as before.
  } finally {
    setPreparing(false);
  }
}
```

Add `const [preparing, setPreparing] = useState(false);` alongside the existing `structureId` / `report` state. Import `toApiError` from `"../../../shared/api/error"` alongside `errorMessage`.

**The inline error Alert must be suppressed when `onInitError` is supplied.** The existing markup renders the Alert whenever `initialize.isError` is true. If a caller supplies `onInitError`, the error is reported through that callback instead, so the Alert's condition becomes:

```tsx
{initialize.isError && !onInitError && (
  <Alert severity="error" sx={{ mt: 2 }}>
    {t("scaffold.initialize.failed", {
      message: errorMessage(initialize.error),
    })}
  </Alert>
)}
```

Without this, the create flow would report the failure twice — once inline in the dialog and once through the Snackbar.

- [ ] **Step 4: Wire the pending guard**

The Confirm button's `disabled` prop becomes `disabled={!selected || preparing || initialize.isPending}`. The Cancel button's `disabled` prop becomes `disabled={preparing || initialize.isPending}`.

- [ ] **Step 5: Run the tests to verify they pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/scaffold/initialize-project-folder-dialog.test.tsx
```
Expected: PASS — all the pre-existing cases plus the five new ones.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/scaffold apps/desktop/aegis-desktop/src/test/features/scaffold
git commit -m "feat(scaffold): add beforeInit and onInitError to the dialog

beforeInit runs before the init command and aborts it on rejection;
onInitError lets a caller take over the failure instead of showing
the inline Alert."
```

---

### Task 3: Surface the template-list failure and the empty state

The template select currently renders an empty dropdown when `list_structures` fails or returns nothing, and Confirm stays disabled with no explanation. In the create flow this is the only path to creating a project *with* a folder, so the reason has to be visible.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/scaffold/components/InitializeProjectFolderDialog.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/scaffold/initialize-project-folder-dialog.test.tsx`

**Interfaces:**
- Consumes: the dialog from Task 1; the props from Task 2 (unchanged by this task).
- Produces: the dialog renders `scaffold.initialize.listFailed` (with the error message) when the structure list fails, and `scaffold.initialize.noTemplates` when the list is empty. Confirm stays disabled in both cases.

- [ ] **Step 1: Write the failing tests**

Add to the dialog's test file:

```tsx
it("shows an error when the template list fails to load", async () => {
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd === "list_structures") {
      return Promise.reject({ kind: "network", message: "boom" });
    }
    return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
  });
  renderDialog();
  expect(await screen.findByText(/could not load structure templates/i))
    .toBeInTheDocument();
  expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled();
});

it("shows an empty-state message when there are no templates", async () => {
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd === "list_structures") return Promise.resolve([]);
    return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
  });
  renderDialog();
  expect(await screen.findByText(/no structure templates/i))
    .toBeInTheDocument();
  expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled();
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/scaffold/initialize-project-folder-dialog.test.tsx
```
Expected: FAIL — no list-error or empty-state markup exists yet.

- [ ] **Step 3: Implement the two states**

The dialog already has `const structures = useListTemplateStructures();`. Add, above the `TextField` that holds the select:

```tsx
{structures.isError && (
  <Alert severity="error">
    {t("scaffold.initialize.listFailed", {
      message: errorMessage(structures.error),
    })}
  </Alert>
)}
{structures.data?.length === 0 && (
  <Alert severity="info">{t("scaffold.initialize.noTemplates")}</Alert>
)}
```

`errorMessage` is already imported. `Alert` is already imported from `"@aegis/ui/mui"`.

Note the second condition reads `structures.data?.length === 0` — `data` is `undefined` while loading, and `undefined?.length` is `undefined`, which is not `=== 0`, so the empty-state does not flash during loading.

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/scaffold/initialize-project-folder-dialog.test.tsx
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/scaffold apps/desktop/aegis-desktop/src/test/features/scaffold
git commit -m "feat(scaffold): explain an empty or failed template list

A dead Confirm button with no reason looks like a hang; in the create
flow this dialog is the only path to creating a project with a folder."
```

---

### Task 4: Add the checkbox and the two-phase submit to the drawer

The drawer's create mode gets the "Initialize project folder" checkbox, and its submit splits: with the box ticked, the create is deferred to the dialog's `beforeInit`; with it unticked, the create happens immediately as today.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx:38-42` (props), `:56-96` (state and reset effect), `:125-165` (submit), `:170-318` (render)
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx`

**Interfaces:**
- Consumes: `InitializeProjectFolderDialog` and its `beforeInit` / `onInitError` props from Tasks 1-2.
- Produces:
  ```ts
  export interface ProjectDrawerProps {
    mode: "closed" | "create" | "edit";
    code: string | null;
    onClose: () => void;
    /** Called when the project was created but its folder could not be
     *  materialized. The drawer closes itself; the page surfaces the
     *  message, because the drawer is gone by then. */
    onFolderInitFailed?: (message: string) => void;
  }
  ```
  `ProjectListPage` passes `onFolderInitFailed`.

- [ ] **Step 1: Write the failing drawer tests**

Add to `src/test/features/project-list/project-drawer.test.tsx`:

```tsx
it("shows the initialize-folder checkbox in create mode, checked by default", async () => {
  mockCommands({ list_users: () => [userFixture] });
  await renderDrawer("create");
  const box = screen.getByRole("checkbox", { name: /initialize project folder/i });
  expect(box).toBeInTheDocument();
  expect(box).toBeChecked();
});

it("does not show the initialize-folder checkbox in edit mode", async () => {
  mockCommands({ list_users: () => [userFixture], get_project_by_code: () => projectFixture });
  await renderDrawer("edit", "alpha");
  expect(
    screen.queryByRole("checkbox", { name: /initialize project folder/i }),
  ).not.toBeInTheDocument();
});

it("opens the template dialog instead of creating when the box is checked", async () => {
  mockCommands({ list_users: () => [userFixture] });
  await renderDrawer("create");
  await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
  await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  expect(
    await screen.findByRole("heading", { name: /initialize project folder/i }),
  ).toBeInTheDocument();
  expect(invoke).not.toHaveBeenCalledWith("create_project", expect.anything());
});

it("creates the project and initializes the folder when the template is confirmed", async () => {
  mockCommands({
    list_users: () => [userFixture],
    list_structures: () => [{ id: 2, name: "Minimal", rootpath: "/t", level: "template", projectCode: null, templateId: null }],
    create_project: () => projectFixture,
    initialize_project_folder: () => ({ root: "D:\\studies", created: [], skipped: [] }),
  });
  await renderDrawer("create");
  await userEvent.type(screen.getByLabelText(/\bcode\b/i), "AK001-001");
  await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  await userEvent.click(await screen.findByRole("combobox"));
  await userEvent.click(await screen.findByRole("option", { name: "Minimal" }));
  await userEvent.click(screen.getByRole("button", { name: /confirm/i }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("create_project", expect.objectContaining({ code: "AK001-001" })),
  );
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("initialize_project_folder", {
      structureId: 2,
      study: "AK001-001",
      branch: null,
    }),
  );
});

it("sends the same code to create_project and to the scaffold", async () => {
  mockCommands({
    list_users: () => [userFixture],
    list_structures: () => [{ id: 2, name: "Minimal", rootpath: "/t", level: "template", projectCode: null, templateId: null }],
    create_project: () => projectFixture,
    initialize_project_folder: () => ({ root: "D:\\studies", created: [], skipped: [] }),
  });
  await renderDrawer("create");
  // "AK001" has no "-", so it yields no `trial` — the create must
  // still go through, and both commands must receive the same string.
  await userEvent.type(screen.getByLabelText(/\bcode\b/i), "AK001");
  await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  await userEvent.click(await screen.findByRole("combobox"));
  await userEvent.click(await screen.findByRole("option", { name: "Minimal" }));
  await userEvent.click(screen.getByRole("button", { name: /confirm/i }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "create_project",
      expect.objectContaining({ code: "AK001" }),
    ),
  );
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("initialize_project_folder", {
      structureId: 2,
      study: "AK001",
      branch: null,
    }),
  );
});

it("creates nothing when the template dialog is cancelled", async () => {
  mockCommands({ list_users: () => [userFixture], list_structures: () => [...] });
  await renderDrawer("create");
  await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
  await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  await userEvent.click(await screen.findByRole("button", { name: /cancel/i }));
  expect(invoke).not.toHaveBeenCalledWith("create_project", expect.anything());
  expect(invoke).not.toHaveBeenCalledWith("initialize_project_folder", expect.anything());
});

it("keeps the drawer open with the error when create_project fails", async () => {
  mockCommands({
    list_users: () => [userFixture],
    list_structures: () => [...],
    create_project: () => {
      throw { kind: "http", status: 409, code: "duplicate_code", message: "dup" };
    },
  });
  await renderDrawer("create");
  // ...type code + description, click Create, select template, Confirm...
  expect(await screen.findByRole("alert")).toHaveTextContent(/duplicate_code/i);
  expect(invoke).not.toHaveBeenCalledWith("initialize_project_folder", expect.anything());
});

it("lets the user create without a folder when no templates exist", async () => {
  mockCommands({
    list_users: () => [userFixture],
    list_structures: () => [],
    create_project: () => projectFixture,
  });
  await renderDrawer("create");
  await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
  await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  // The dialog explains that there is nothing to pick.
  expect(await screen.findByText(/no structure templates/i)).toBeInTheDocument();
  // Cancelling returns to the drawer; unticking the box then creates
  // the project with no folder at all.
  await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
  await userEvent.click(
    screen.getByRole("checkbox", { name: /initialize project folder/i }),
  );
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "create_project",
      expect.objectContaining({ code: "newproj" }),
    ),
  );
  expect(invoke).not.toHaveBeenCalledWith(
    "initialize_project_folder",
    expect.anything(),
  );
});
```

- [ ] **Step 2: Rewrite the two existing cases that the default-on checkbox breaks**

In the same test file:

- **"calls api.createProject with the assembled shape on Submit"** — untick the checkbox before submitting, so it keeps testing the plain path. Insert, after filling the code field:

  ```tsx
  await userEvent.click(
    screen.getByRole("checkbox", { name: /initialize project folder/i }),
  );
  ```

- **"shows an Alert with the error message when create_project fails"** — same: untick the checkbox first. The "does not show the active switch in create mode" case is unaffected: the new control is a `Checkbox` (role `checkbox`), not a `Switch`.

- [ ] **Step 3: Run the drawer tests to verify they fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/project-list/project-drawer.test.tsx
```
Expected: FAIL — no checkbox exists, and the submit does not open a template dialog.

- [ ] **Step 4: Add the checkbox and the state to the drawer**

In `ProjectDrawer.tsx`:

Add `Checkbox` to the `@aegis/ui/mui` import list.

Add the state and the new prop:

```tsx
export interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
  onFolderInitFailed?: (message: string) => void;
}
```

```tsx
const [initializeFolder, setInitializeFolder] = useState(true);
const [pendingInput, setPendingInput] = useState<CreateProjectInput | null>(null);
```

`pendingInput` is the **single** piece of state that drives the template step: it is non-null exactly while the template dialog is open, and it carries the assembled `CreateProjectInput` captured at submit time. A separate `templateDialogOpen` boolean would be a second source of truth for the same thing.

In the reset `useEffect` that clears the form when `mode === "create"`, add `setInitializeFolder(true);` and `setPendingInput(null);` alongside the other resets. The effect's dependency array stays `[mode]`.

Render the checkbox in create mode only, after the language `FormControl`:

```tsx
{mode === "create" && (
  <Box>
    <FormControlLabel
      control={
        <Checkbox
          checked={initializeFolder}
          onChange={(e) => setInitializeFolder(e.target.checked)}
        />
      }
      label={t("project.field.initializeFolder")}
    />
    <Typography variant="body2" color="text.secondary">
      {t("project.field.initializeFolder.hint")}
    </Typography>
  </Box>
)}
```

- [ ] **Step 5: Split the submit**

In `onSubmit`, the create branch becomes conditional on the checkbox. When the box is on, the drawer opens the template dialog and does **not** create yet — the dialog's `beforeInit` does the creating.

The assembled `CreateProjectInput` must be captured at submit time so the code sent to `create_project` and the `study` handed to the scaffold are the same string.

Replace the create branch of `onSubmit` with:

```tsx
if (mode === "create") {
  const input: CreateProjectInput = {
    code: formCode.trim(),
    description: description.trim(),
    members,
    unblindMembers,
    configurations,
  };
  if (initializeFolder) {
    // The template step creates the project, in its `beforeInit`.
    setPendingInput(input);
    return;
  }
  await create.mutateAsync(input);
  onClose();
  return;
}
```

- [ ] **Step 6: Render the scaffold dialog beside the drawer**

The dialog must be a **sibling** of the `<Drawer>`, not inside it — the drawer unmounts its children when `mode` is not `"closed"`, and the dialog has to outlive the drawer on the failure path. Change the component's return from a bare `<Drawer>` to a fragment:

```tsx
return (
  <>
    <Drawer ...>
      ...
    </Drawer>
    {pendingInput && (
      <InitializeProjectFolderDialog
        open
        projectCode={pendingInput.code}
        beforeInit={async () => {
          await create.mutateAsync(pendingInput);
          // The project exists now — close the drawer so a resubmit
          // cannot fail on the taken code.
          onClose();
        }}
        onInitError={(error) => {
          setPendingInput(null);
          onFolderInitFailed?.(
            t("project.folder.failed", { message: errorMessage(error) }),
          );
        }}
        onClose={() => setPendingInput(null)}
        onInitialized={() => {
          // Deliberately empty: the report the dialog just received is
          // rendered by the dialog itself and must stay on screen. The
          // drawer is already closed — `beforeInit` closed it.
        }}
        // `beforeInit` rejects when `create_project` fails. The dialog
        // closes itself in that case (its own `onClose`, above), and
        // the drawer's `create.error` Alert renders the reason next to
        // the code field the user has to fix. The drawer stays open.
      />
    )}
  </>
);
```

Note `onClose` here clears `pendingInput`, which unmounts the dialog. That is what closes the template step once the report has been read — the user dismisses the report with the dialog's own Close button, which calls this `onClose`.

The drawer is already closed by then: `beforeInit` called `onClose()` right after `create_project` resolved, so the drawer's `mode` is `"closed"` and its children are unmounted — but the dialog is a sibling, so it survives and still shows the report.

- [ ] **Step 7: Add the Snackbar to the projects page**

In `ProjectListPage.tsx`, add the Snackbar state and the callback, and pass `onFolderInitFailed` down.

```tsx
const [folderError, setFolderError] = useState<string | null>(null);
```

```tsx
<ProjectDrawer
  mode={drawer.mode}
  code={drawer.code}
  onClose={() => setDrawer({ mode: "closed", code: null })}
  onFolderInitFailed={(message) => setFolderError(message)}
/>
```

Render the Snackbar as a sibling of the drawer, following the house pattern from `ImportTerminologyPage.tsx`:

```tsx
<Snackbar
  open={folderError !== null}
  autoHideDuration={6000}
  onClose={() => setFolderError(null)}
>
  <Alert severity="error" onClose={() => setFolderError(null)}>
    {folderError}
  </Alert>
</Snackbar>
```

Add `Alert` and `Snackbar` to the `@aegis/ui/mui` import list.

- [ ] **Step 8: Write the failing page test**

Add to `src/test/features/project-list/project-list-page.test.tsx`:

```tsx
it("shows a Snackbar when the folder could not be initialized", async () => {
  mockCommands({
    current_user: () => adminUser,
    list_projects: () => [],
    list_users: () => [adminUser],
    list_structures: () => [
      { id: 2, name: "Minimal", rootpath: "/t", level: "template", projectCode: null, templateId: null },
    ],
    create_project: () => projectA,
    initialize_project_folder: () => {
      throw { kind: "parse", message: "rootpath is not writable" };
    },
  });
  await renderPage(adminUser, []);
  await userEvent.click(await screen.findByRole("button", { name: /add project/i }));
  await userEvent.type(await screen.findByLabelText(/\bcode\b/i), "AK001-001");
  await userEvent.type(await screen.findByLabelText(/\bdescription\b/i), "New desc");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  await userEvent.click(await screen.findByRole("combobox"));
  await userEvent.click(await screen.findByRole("option", { name: "Minimal" }));
  await userEvent.click(screen.getByRole("button", { name: /confirm/i }));
  expect(
    await screen.findByText(/project was created, but its folder could not be initialized/i),
  ).toBeInTheDocument();
});
```

The page test's `renderPage` helper currently calls `mockCommands` itself — extend it to accept extra handlers, or call `mockCommands` again after `renderPage` in this case. Either way, `create_project` and `initialize_project_folder` must be mocked for this case.

- [ ] **Step 9: Run the tests to verify they fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/project-list/project-drawer.test.tsx src/test/features/project-list/project-list-page.test.tsx
```
Expected: FAIL — the checkbox and the template step do not exist yet.

- [ ] **Step 10: Run the tests to verify they pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/project-list/project-drawer.test.tsx src/test/features/project-list/project-list-page.test.tsx
```
Expected: PASS.

- [ ] **Step 11: Run the full frontend verification gate**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```
Expected: all clean.

- [ ] **Step 12: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list apps/desktop/aegis-desktop/src/test/features/project-list
git commit -m "feat(desktop): initialize the project folder when creating a project

The drawer's create mode gains an \"Initialize project folder\" checkbox,
checked by default. With it ticked, confirming the create opens the
template step; confirming a template creates the project and then
materializes its folder. A folder failure leaves the project standing
and is reported by a Snackbar on the projects page."
```

---

## Verification

After all four tasks:

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```

The Rust half is unchanged by this work — the `cargo` commands run only to confirm nothing was disturbed.
