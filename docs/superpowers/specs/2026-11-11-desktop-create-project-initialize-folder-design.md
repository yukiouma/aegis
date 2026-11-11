# Create Project → Initialize Project Folder — Design

> **Scope.** Desktop-only. Add an "Initialize project folder" checkbox to the
> project drawer's create mode; when it is on, creating a project also
> materializes the project's folder on disk from a chosen structure
> template. No change to the server, the `project` crate, or the
> `initialize_project_folder` command.

## Goal

Today the two halves of a new project's setup live in different places and
different windows: the project row is created from the projects list
(`/projects`), and the folder is materialized later, per branch, from the
project workspace window. A user who creates a project in the list has to
switch windows and pick a branch before a single folder appears on disk.

This change brings the second half into the create flow:

1. The project drawer, in **create mode only**, carries a checkbox
   **"Initialize project folder"** (zh-CN: 初始化项目文件夹), **checked by
   default**.
2. When the user confirms the create form and the checkbox is on, the
   **template selection** step is shown.
3. After a template is selected and confirmed, the project is **created** and
   the folder is **initialized**.
4. If the user cancels the template step, **nothing is created** — only the
   dialog closes.

## The flow, step by step

```
ProjectDrawer (create mode)
  └─ [x] Initialize project folder
     └─ user clicks "Create"
        └─ template-selection dialog opens
           ├─ user cancels        → dialog closes. No project, no folder.
           │                       The drawer stays open with the form
           │                       intact (the code was not taken).
           └─ user confirms a template
              ├─ create_project fails (e.g. duplicate code)
              │    → the dialog closes and the init never runs.
              │      The drawer stays open; its existing Alert shows
              │      create.error. The code field is right there to fix.
              └─ create_project succeeds
                   → the drawer closes (the project exists; a create-form
                     resubmit would now fail on the taken code)
                   → initialize_project_folder runs
                      ├─ fails → the project exists, the folder does not.
                      │          The page raises a Snackbar:
                      │          "The project was created, but its folder
                      │          could not be initialized: {message}"
                      └─ succeeds → the report is shown in the dialog
                                   (created / skipped counts + reasons)
```

**The project is created before the folder is initialized.** The command never
consults the server for the project — it takes the code as a string and writes
files — so this order is free, and it means a rejected duplicate code never
leaves files on disk. It also means a folder-init failure leaves a real
project row behind, which is the failure mode the requester chose
(deliberately: the project survives, the folder does not, and the user can
initialize the folder later from the workspace).

## Decisions

| Question | Decision | Why |
| --- | --- | --- |
| Where does the shared dialog live? | A new `features/scaffold/` feature holding `InitializeProjectFolderDialog` and `useInitializeProjectFolder`. Both `project-list` and `project-workspace` import it through its barrel. | `project-workspace` already imports `project-list`, so importing it back would make the two features mutually dependent. `features/scaffold` matches the existing `src-tauri/src/commands/scaffold.rs`. |
| Where does the template-selection step live? | Inside `InitializeProjectFolderDialog`, reusing it as-is. | It already is a template picker whose confirm button runs the init command. The create flow adds a *create* step ahead of that command and nothing else. |
| How does the create flow reach the dialog's confirm button? | The dialog takes an optional `beforeInit?: () => Promise<void>` hook, awaited before the init command runs. The drawer passes `beforeInit = () => create.mutateAsync(input)`. | One optional prop, no second dialog, no duplicated report markup. The hook's rejection aborts the operation before anything is written. |
| Is the project created before or after the folder? | Before. | A duplicate code must not leave files on disk. |
| What is `study` (the scaffold context's project code)? | The code the user typed, trimmed — the same value sent to `create_project`. | The scaffold derives `product` / `trial` from it by splitting on `-`, so `AK001-001` yields `ak001` / `001`. |
| What if the code has no `-`? | **No new validation.** The server imposes no code format, and the report already lists every skipped node with its reason — so the signal exists and is shown immediately after the run. A pre-flight rule on the create form would be a product decision beyond this change. | Adding one would silently narrow what codes may be created, for a condition the report already explains. |
| Is a branch created? | No. The init runs with `branch: null`. | Branches are a workspace-level concept and the server has no branch at project-creation time. `{{ branch }}` nodes are therefore skipped in the report — the per-branch init in the workspace remains the way to get branch-scoped content. |
| Does the folder-init failure leave the project? | Yes. The project row stands; the folder does not exist. | The requester's explicit choice. The project is a valid row the user can build on; the folder can be initialized later from the workspace, which is the only place a branch exists. |
| Where does the failure surface? | A Snackbar on the projects page. | The drawer closes (the project exists, so a create-form re-submit would fail on the now-taken code). A Snackbar is the house pattern for exactly this — `CrfDetailPage` and `ImportTerminologyPage` both use one. |
| What happens if the template list is empty or fails to load? | The dialog says so — an error Alert for a failed list, a "no templates" message for an empty one — and Confirm stays disabled. | In the create flow the template step is the only path to creating *with* a folder; a dead Confirm button with no explanation looks like a hang. The escape is visible: cancelling leaves the drawer open with its checkbox, and unticking it restores the plain create path. |
| Which i18n namespace? | The dialog's existing `project.branches.initialize.*` keys move to `scaffold.initialize.*`. The new checkbox label is `project.field.initializeFolder`. | The dialog is no longer branch-specific. The checkbox label lives with the other `project.field.*` keys because it is a form field on the drawer. |
| Is the drawer or the page the mutation owner? | The drawer keeps its own `useCreateProject` mutation, as it does today. | The drawer already owns the create mutation; moving it to the page would be a refactor of unrelated code. |

## The dialog's contract

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
   * the user can retry — which is what the branch flow wants, since
   * the init command skips whatever already exists.
   */
  onInitError?: (error: ApiError) => void;
  onClose: () => void;
  onInitialized: (report: InitReport) => void;
}
```

**`beforeInit` rejection.** The dialog awaits `beforeInit()` before running
the init command. A rejection means the operation is abandoned: the init
command is **not** run, the dialog closes via `onClose`, and the rejection is
swallowed by the dialog (the caller's own mutation state holds the error).
The drawer renders `create.error` in the Alert it already has, next to the
code field the user has to change.

**Init-command failure.** Two destinations, chosen by the caller:

- *Branch flow* (no `onInitError`): the dialog stays open and renders the
  error inline — today's behaviour, and the user can retry, because the
  command skips what already exists.
- *Create flow* (`onInitError` supplied): the dialog calls `onInitError` and
  renders no inline Alert. The drawer closes and the page raises a
  Snackbar.

**Confirm is disabled** while either `beforeInit` or the init command is in
flight, so a double-click cannot fire `create_project` twice. This needs a
local pending flag in the dialog: `initialize.isPending` alone does not
cover the create step.

## What changes where

### New: `features/scaffold/`

```
src/features/scaffold/
├── index.ts                    ← barrel: exports the dialog and the hook
├── components/
│   └── InitializeProjectFolderDialog.tsx   ← moved from project-workspace
└── data/
    └── project-folder.ts       ← useInitializeProjectFolder, moved from
                                  project-workspace/data/project-branches.ts
```

The dialog moves **unchanged** except for `beforeInit`, `onInitError`, the
pending flag, and the i18n key rename. `useInitializeProjectFolder` moves out
of `project-branches.ts` — it is not branch data, and the new feature is its
natural home.

`features/project-workspace/index.ts` re-exports the dialog and the hook from
the new feature for its existing consumers, so `ProjectBranchesSection` and
the workspace page keep working with a one-line import change.

### `features/project-list/components/ProjectDrawer.tsx`

- New state `initializeFolder: boolean`, **default `true`**, reset to `true`
  when the drawer enters create mode (the same `useEffect` that resets the
  other fields).
- The control is a MUI **`Checkbox`** (role `checkbox`), rendered **only** in
  create mode, with a helper line explaining that a template is picked next.
  It is deliberately not a `Switch`: create mode already asserts that no
  switch is present.
- `onSubmit` splits by mode:
  - **create + checkbox on** → snapshot the form into a `pendingInput` ref,
    open the template dialog, and do **not** call `create_project` yet.
  - **create + checkbox off** → today's path: `create_project`, then close.
  - **edit** → today's path.
- The dialog is rendered as a **sibling** of the `<Drawer>` (the component's
  root becomes a fragment), so closing the drawer does not unmount it. It is
  mounted only while the template step is open, so the template list is
  fetched when the user asks for it.
- `beforeInit` is `() => create.mutateAsync(pendingInput)`. On success it
  calls the drawer's `onClose()` — the project exists, so the create form has
  nothing left to offer — and the project list is invalidated by the existing
  mutation's `onSuccess`.
- `onInitError` closes the template dialog and calls the page's
  `onFolderInitFailed(message)` with the composed message.
- The `pendingInput` snapshot is taken at submit time, so the code sent to
  `create_project` and the `study` handed to the scaffold are the same
  string, even if the form is edited afterwards.

### `features/project-list/pages/ProjectListPage.tsx`

Owns the Snackbar, because the drawer closes and the message has to outlive
it. The page passes an `onFolderInitFailed` callback down; the drawer calls
it with the composed message and closes itself. The drawer composes the
message (`project.folder.failed`) because it is the component that knows the
failure means "the project was created, the folder was not".

### i18n

`en.ts` and `zhCN.ts` move the dialog's fourteen `project.branches.initialize.*`
keys to `scaffold.initialize.*` and add:

| Key | en | zh-CN |
| --- | --- | --- |
| `project.field.initializeFolder` | Initialize project folder | 初始化项目文件夹 |
| `project.field.initializeFolder.hint` | Pick a structure template to create the project folder on this machine | 选择一个结构模板，在本机创建项目文件夹 |
| `scaffold.initialize.failed` | Initialize failed: {message} | 初始化失败：{message} |
| `scaffold.initialize.listFailed` | Could not load structure templates: {message} | 加载结构模板失败：{message} |
| `scaffold.initialize.noTemplates` | No structure templates are available. | 没有可用的结构模板。 |
| `project.folder.failed` | The project was created, but its folder could not be initialized: {message} | 项目已创建，但项目文件夹初始化失败：{message} |

The dialog's report and skip-reason keys move namespace unchanged.

## Testing

**Frontend** (`src/test/features/…`):

- `scaffold/initialize-project-folder-dialog.test.tsx` — the existing test
  file, moved. Adds: `beforeInit` is awaited before the init command; a
  rejecting `beforeInit` aborts the init (the command is never invoked) and
  closes the dialog; `onInitError` is called instead of the inline Alert
  when supplied; Confirm is disabled while `beforeInit` is in flight; an
  empty or failed template list shows its message; the branch flow without
  either prop is unchanged.
- `project-list/project-drawer.test.tsx` — the checkbox renders in create
  mode and not in edit mode; it defaults to checked; with it on, the submit
  opens the template dialog and does **not** call `create_project`;
  confirming the template calls `create_project` and then
  `initialize_project_folder`; cancelling calls neither and leaves the
  drawer open; a failing `create_project` keeps the drawer open with the
  error.
  **Two existing cases must be rewritten**, because the checkbox now
  defaults to on and the create no longer fires on submit: "calls
  api.createProject with the assembled shape on Submit" and the
  `create_project` failure-Alert case. Both untick the box first (so they
  keep testing the plain path), and the create-on-submit assertions move to
  the new flow's tests.
- `project-list/project-list-page.test.tsx` — a folder-init failure closes
  the drawer and raises the Snackbar.

**Rust:** no change, so no new Rust test.

## Out of scope

- Creating a default branch for a new project.
- A project-level (branch-less) initialize affordance in the workspace — the
  workspace's initialize stays per-branch.
- Validating the project code's shape before creating the folder — see the
  decision above.
- Changing the `initialize_project_folder` command, the `scaffolding` crate,
  the server, or the `project` crate.
- Making the folder layout project-specific by default — that stays the
  template author's decision, as recorded in the initialize-project-folder
  design.
