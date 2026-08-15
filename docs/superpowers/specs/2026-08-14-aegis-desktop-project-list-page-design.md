# Add a project list page to Aegis desktop

Date: 2026-08-14
Status: Approved (brainstorming)

## Goal

Add a `Project` page to the Aegis desktop app at `/_layout/projects`
with a filterable, searchable list of projects, a create / update
drawer, and role-gated write actions. Back the page with TanStack
Query hooks that wrap the four project APIs (`listProjects`,
`getProjectByCode`, `createProject`, `updateProject`) plus the two
APIs needed to populate the drawer's form (`listProducts`,
`listUsers`).

Today the desktop app's data layer (per the
[2026-08-14 tanstack query refactor spec](2026-08-14-aegis-desktop-tanstack-query-refactor-design.md))
already wraps the auth, bootstrap, and user-current hooks. The four
project APIs are exposed by `src/api/index.ts` but unwrapped. The
`listProducts` and `listUsers` APIs are also unwrapped.

## Approach

Add a `_layout/projects` file route, four small page components
(filter bar, table, drawer, orchestrator), and three new hook files
(`project.ts`, `product.ts`, plus one new hook in `user.ts`). Match
the layering established by the tanstack-query-refactor spec: pages
consume `data/`, `data/` consumes `api/`, pages never reach `api/`
directly. Add a `Projects` entry to the `AppLayout` Sidebar menu so
users can navigate to the page.

### Why a subcomponent split

A single-file page would mix the table, drawer form, filter row, and
orchestration state in one place (~400 lines). Splitting per concern
keeps each file under ~250 lines and lets each piece have a focused
test file (the existing convention in `src/test/pages/`).

### Why local filter state, not URL params

The user-visible URL for `/projects` stays clean. Search and Involve
toggle are session-local — there is no need to bookmark or share a
filtered view. URL search params would also re-trigger `useQuery`
cache lifecycle in ways that are not needed here.

## File layout

```
apps/desktop/aegis-desktop/src/
├── data/
│   ├── project.ts                          NEW — useListProjects,
│   │                                         useProject, useCreateProject,
│   │                                         useUpdateProject
│   ├── product.ts                          NEW — useListProducts
│   ├── user.ts                             MODIFIED — add useListUsers
│   ├── queryKeys.ts                        MODIFIED — add project.*, product.*,
│   │                                         user.list()
│   └── index.ts                            MODIFIED — re-export new hooks
│
├── pages/
│   ├── project-list.tsx                    NEW — orchestrator
│   ├── ProjectFilterBar.tsx                NEW — search + Involve toggle
│   ├── ProjectTable.tsx                    NEW — table + icon buttons
│   └── ProjectDrawer.tsx                   NEW — create/update form
│
├── routes/_layout/
│   ├── projects.tsx                        NEW — route file
│   └── route.tsx                           UNCHANGED
│
├── pages/layout.tsx                        MODIFIED — add Projects menu entry
│
├── test/
│   ├── data/project.test.tsx               NEW
│   ├── data/product.test.tsx               NEW
│   ├── data/user.test.tsx                  MODIFIED — add useListUsers cases
│   ├── pages/project-list.test.tsx         NEW
│   ├── pages/project-table.test.tsx        NEW
│   ├── pages/project-drawer.test.tsx       NEW
│   └── routes/projects.test.tsx            NEW
│
lib/packages/ui/src/i18n/locales/
├── en.ts                                   MODIFIED — add project.* + nav.projects
└── zhCN.ts                                 MODIFIED — mirror the same keys
```

## Routing

### `src/routes/_layout/projects.tsx`

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { ProjectListPage } from "../../pages/project-list";

export const Route = createFileRoute("/_layout/projects")({
  component: ProjectListPage,
});
```

`routeTree.gen.ts` regenerates automatically via
`@tanstack/router-plugin` (already in `package.json`). No manual
edits to the generated file.

### Sidebar entry

In `src/pages/layout.tsx`, add a `Projects` menu item between Home
and Settings:

```tsx
const menu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];
```

`ProjectsMenuIcon` is a local `() => <Workspaces />` component
imported from `@aegis/ui/icons`.

## Query keys

Additions to `src/data/queryKeys.ts`:

```ts
project: {
  all: () => ["project", "list"] as const,
  byCode: (code: string) => ["project", "byCode", code] as const,
},
product: {
  all: () => ["product", "list"] as const,
},
```

Plus, inside `user`:

```ts
user: {
  // ... existing keys
  list: () => ["user", "list"] as const,
}
```

Rationale:

- `project.byCode(code)` keyed by code, so opening the drawer for
  project A does not invalidate the list cache.
- `project.all()` is the invalidation target for create / update.
- `product.all()` and `user.list()` are not invalidated from this
  page; they change rarely and the drawer is the only consumer.

## Hook shapes

All hooks consume `api.*` from the transport layer. The transport
already throws structured `ApiError`; hooks propagate them unchanged
into `query.error` / `mutation.error`. Hooks do not shape errors.

### `src/data/project.ts`

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type ApiError, type CreateProjectInput, type ProjectView, type UpdateProjectBody } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * All projects. Fetches on mount. Inherits the global
 * `staleTime: Infinity` because the list is re-read whenever the
 * page mounts, not via refetch.
 */
export function useListProjects() {
  return useQuery<ProjectView[], ApiError>({
    queryKey: queryKeys.project.all(),
    queryFn: () => api.listProjects(),
  });
}

/**
 * Single project by code. Manual-trigger (matches `useDomainUserInfo`
 * in `user.ts`) — `enabled: false` by default; the drawer drives the
 * fetch with `refetch()` so opening the edit drawer fires a fresh
 * read without auto-firing on every mount. `staleTime: 0` keeps the
 * read always-fresh before edit.
 */
export function useProject(code: string | null) {
  return useQuery<ProjectView, ApiError>({
    queryKey: code === null ? ["project", "byCode", "__disabled__"] : queryKeys.project.byCode(code),
    queryFn: () => {
      if (code === null) throw new Error("useProject disabled");
      return api.getProjectByCode(code);
    },
    enabled: false,
    staleTime: 0,
  });
}

/**
 * Create project. On success: invalidates the project list cache
 * so the next render shows the new row. Does NOT clear the cache
 * (unlike logout) — the current user is unaffected.
 */
export function useCreateProject() {
  const qc = useQueryClient();
  return useMutation<ProjectView, ApiError, CreateProjectInput>({
    mutationFn: (input) => api.createProject(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.project.all() }),
  });
}

/**
 * Update project. On success: invalidates the project list AND the
 * single-by-code entry for the updated row, so both the table and a
 * follow-up edit-open show the new values.
 */
export function useUpdateProject() {
  const qc = useQueryClient();
  return useMutation<ProjectView, ApiError, { code: string; body: UpdateProjectBody }>({
    mutationFn: ({ code, body }) => api.updateProject(code, body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.project.all() });
      qc.invalidateQueries({ queryKey: queryKeys.project.byCode(vars.code) });
    },
  });
}
```

### `src/data/product.ts`

```ts
import { useQuery } from "@tanstack/react-query";
import { api, type ApiError, type ProductView } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * All products. Consumed by the drawer's product dropdown. Inherits
 * the global staleTime — products rarely change.
 */
export function useListProducts() {
  return useQuery<ProductView[], ApiError>({
    queryKey: queryKeys.product.all(),
    queryFn: () => api.listProducts(),
  });
}
```

### `src/data/user.ts` (additions)

Append to the existing file:

```ts
/**
 * All users. Consumed by the drawer's member pickers. Default
 * `enabled: true` because the drawer is the only consumer and only
 * opens for root/admin, where the call always succeeds in practice.
 */
export function useListUsers(options?: { enabled?: boolean }) {
  return useQuery<UserView[], ApiError>({
    queryKey: queryKeys.user.list(),
    queryFn: () => api.listUsers(),
    enabled: options?.enabled ?? true,
  });
}
```

### `src/data/index.ts`

Append the new hook re-exports:

```ts
export { useListProjects, useProject, useCreateProject, useUpdateProject } from "./project";
export { useListProducts } from "./product";
export { /* ... existing */, useListUsers } from "./user";
```

## Components

### `src/pages/project-list.tsx` — orchestrator

Local state:

```ts
const [query, setQuery] = useState("");
const [involve, setInvolve] = useState(false);
const [drawer, setDrawer] = useState<{
  mode: "closed" | "create" | "edit";
  code: string | null;
}>({ mode: "closed", code: null });
```

Data:

```ts
const projects = useListProjects();
const currentUser = useCurrentUser();
const currentCode = currentUser.data?.code ?? null;
```

Permission flag:

```ts
const role = currentUser.data?.role;
const canEdit = role === "root" || role === "admin";
```

Filter pipeline (single `useMemo` over `projects.data ?? []`):

1. **Search** — if `query` is non-empty (after `trim()`), keep rows
   where `code`, `description`, or any leader code/name (from
   `members.leaders` or `unblindMembers.leaders`) contains `query`
   case-insensitively.
2. **Involve** — if `involve && currentCode`, keep rows where
   `currentCode` appears in `members.leaders`, `members.workers`,
   `unblindMembers.leaders`, or `unblindMembers.workers` (compare
   by `.code`).
3. The pipeline is commutative; both filters AND together.

Render:

```tsx
<Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
  <Typography variant="h4">{t("project.heading")}</Typography>
  <ProjectFilterBar
    query={query}
    onQueryChange={setQuery}
    involve={involve}
    onInvolveChange={setInvolve}
  />
  <ProjectTable
    rows={filteredRows}
    loading={projects.isLoading}
    error={projects.error}
    canEdit={canEdit}
    onOpenCreate={() => setDrawer({ mode: "create", code: null })}
    onOpenEdit={(code) => setDrawer({ mode: "edit", code })}
  />
  <ProjectDrawer
    mode={drawer.mode}
    code={drawer.code}
    onClose={() => setDrawer({ mode: "closed", code: null })}
  />
</Box>
```

### `src/pages/ProjectFilterBar.tsx`

Controlled component. Receives `query`, `onQueryChange`, `involve`,
`onInvolveChange`. Layout: a flex row with the search field on the
left and the Involve checkbox pushed to the right via
`sx={{ ml: "auto" }}`.

```tsx
<Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
  <TextField
    size="small"
    label={t("project.search.label")}
    value={query}
    onChange={(e) => onQueryChange(e.target.value)}
    sx={{ minWidth: 320 }}
  />
  <FormControlLabel
    sx={{ ml: "auto" }}
    control={
      <Checkbox
        checked={involve}
        onChange={(e) => onInvolveChange(e.target.checked)}
      />
    }
    label={t("project.involve")}
  />
</Box>
```

The search field stays enabled even when `currentCode` is null;
toggling Involve with no current user just produces an empty list —
acceptable because the page already shows the empty state.

### `src/pages/ProjectTable.tsx`

Props:

```ts
interface ProjectTableProps {
  rows: ProjectView[];
  loading: boolean;
  error: ApiError | null;
  canEdit: boolean;
  onOpenCreate: () => void;
  onOpenEdit: (code: string) => void;
}
```

Skeleton:

```tsx
<TableContainer component={Paper}>
  <Table size="small">
    <TableHead>
      <TableRow>
        <TableCell>{t("project.field.code")}</TableCell>
        <TableCell>{t("project.field.description")}</TableCell>
        <TableCell>{t("project.col.leaders")}</TableCell>
        <TableCell>{t("project.col.active")}</TableCell>
        <TableCell align="right">
          {canEdit ? (
            <IconButton
              aria-label={t("project.add")}
              onClick={onOpenCreate}
            >
              <Add />
            </IconButton>
          ) : null}
        </TableCell>
      </TableRow>
    </TableHead>
    <TableBody>
      {rows.map((row) => (
        <TableRow key={row.id} hover>
          <TableCell>{row.code}</TableCell>
          <TableCell sx={{ maxWidth: 280 }}>
            <Typography noWrap>{row.description}</Typography>
          </TableCell>
          <TableCell>
            <Stack direction="row" spacing={0.5} flexWrap="wrap" useFlexGap>
              {row.members.leaders.map((u) => (
                <Chip
                  key={`m-${u.code}`}
                  variant="outlined"
                  size="small"
                  label={u.name}
                />
              ))}
              {row.unblindMembers.leaders.map((u) => (
                <Chip
                  key={`u-${u.code}`}
                  variant="filled"
                  size="small"
                  label={u.name}
                />
              ))}
              {row.members.leaders.length === 0 &&
                row.unblindMembers.leaders.length === 0 && <span>—</span>}
            </Stack>
          </TableCell>
          <TableCell>
            <Tooltip
              title={t(row.active ? "project.active" : "project.inactive")}
            >
              <span>
                {row.active ? (
                  <CheckCircle color="success" />
                ) : (
                  <Cancel color="disabled" />
                )}
              </span>
            </Tooltip>
          </TableCell>
          <TableCell align="right">
            <Stack direction="row" spacing={0.5} justifyContent="flex-end">
              {canEdit && (
                <IconButton
                  aria-label={t("project.edit")}
                  onClick={() => onOpenEdit(row.code)}
                >
                  <Edit />
                </IconButton>
              )}
              <IconButton
                aria-label={t("project.open")}
                disabled
              >
                <OpenInNew />
              </IconButton>
            </Stack>
          </TableCell>
        </TableRow>
      ))}
    </TableBody>
  </Table>
</TableContainer>
```

Behavior:

- Loading state: when `loading && rows.length === 0`, render a
  centered `<CircularProgress />` in place of the table body. When
  `loading && rows.length > 0`, render the existing rows (stale-while-
  revalidate semantics via TanStack Query).
- Error state: an `<Alert severity="error">` above the table; the
  table itself is not rendered.
- Empty state (no rows, not loading, no error): a centered
  `<Typography>{t("project.empty")}</Typography>` inside the table
  area.

### `src/pages/ProjectDrawer.tsx`

Props:

```ts
interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
}
```

Right-anchored MUI Drawer:

```tsx
<Drawer
  anchor="right"
  open={mode !== "closed"}
  onClose={onClose}
  PaperProps={{ sx: { width: 480 } }}
>
  <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
    <Typography variant="h6">
      {t(mode === "create" ? "project.create.title" : "project.edit.title")}
    </Typography>
    {/* form goes here */}
  </Box>
</Drawer>
```

Form state (controlled) — each field is its own `useState`:

```ts
const [code, setCode] = useState("");
const [description, setDescription] = useState("");
const [productId, setProductId] = useState<number | null>(null);
const [memberLeaders, setMemberLeaders] = useState<UserSummary[]>([]);
const [memberWorkers, setMemberWorkers] = useState<UserSummary[]>([]);
const [unblindLeaders, setUnblindLeaders] = useState<UserSummary[]>([]);
const [unblindWorkers, setUnblindWorkers] = useState<UserSummary[]>([]);
const [active, setActive] = useState(true);
```

Auxiliary hooks called inside the drawer (mounted only when the
drawer is open, since MUI's underlying Modal unmounts children when
`open={false}` by default):

```ts
const products = useListProducts();
const users = useListUsers();
const fetched = useProject(code); // enabled: false; manual-trigger.
const create = useCreateProject();
const update = useUpdateProject();
```

Seed the form when the drawer opens in edit mode. The `lookedUp`
ref mirrors `register.tsx` and guards against React StrictMode's
double-invocation of effects in development — each fresh drawer
mount resets the ref, so closing and reopening always refetches:

```ts
const lookedUp = useRef(false);
useEffect(() => {
  if (mode !== "edit" || code === null) return;
  if (lookedUp.current) return;
  lookedUp.current = true;
  void (async () => {
    const r = await fetched.refetch();
    if (r.isError || !r.data) return;
    setCode(r.data.code);
    setDescription(r.data.description);
    setProductId(r.data.product.id);
    setMemberLeaders(r.data.members.leaders);
    setMemberWorkers(r.data.members.workers);
    setUnblindLeaders(r.data.unblindMembers.leaders);
    setUnblindWorkers(r.data.unblindMembers.workers);
    setActive(r.data.active);
  })();
}, [mode, code, fetched]);
```

Form fields:

```tsx
<TextField
  label={t("project.field.code")}
  value={code}
  onChange={(e) => setCode(e.target.value)}
  disabled={mode === "edit"}
  size="small"
  required
/>

<TextField
  label={t("project.field.description")}
  value={description}
  onChange={(e) => setDescription(e.target.value)}
  multiline
  minRows={2}
  size="small"
  required
/>

<Autocomplete
  options={products.data ?? []}
  getOptionLabel={(p) => `${p.code} — ${p.name}`}
  value={products.data?.find((p) => p.id === productId) ?? null}
  onChange={(_e, value) => setProductId(value?.id ?? null)}
  renderInput={(params) => (
    <TextField
      {...params}
      label={t("project.field.product")}
      size="small"
      required
    />
  )}
/>

<Autocomplete<UserView, true>
  multiple
  options={users.data ?? []}
  getOptionLabel={(u) => `${u.code} — ${u.name}`}
  value={memberLeaders}
  onChange={(_e, value) => setMemberLeaders(value)}
  renderInput={(params) => (
    <TextField {...params} label={t("project.field.members.leaders")} size="small" />
  )}
/>

<Autocomplete<UserView, true>
  multiple
  options={users.data ?? []}
  getOptionLabel={(u) => `${u.code} — ${u.name}`}
  value={memberWorkers}
  onChange={(_e, value) => setMemberWorkers(value)}
  renderInput={(params) => (
    <TextField {...params} label={t("project.field.members.workers")} size="small" />
  )}
/>

<Autocomplete<UserView, true>
  multiple
  options={users.data ?? []}
  getOptionLabel={(u) => `${u.code} — ${u.name}`}
  value={unblindLeaders}
  onChange={(_e, value) => setUnblindLeaders(value)}
  renderInput={(params) => (
    <TextField {...params} label={t("project.field.unblindMembers.leaders")} size="small" />
  )}
/>

<Autocomplete<UserView, true>
  multiple
  options={users.data ?? []}
  getOptionLabel={(u) => `${u.code} — ${u.name}`}
  value={unblindWorkers}
  onChange={(_e, value) => setUnblindWorkers(value)}
  renderInput={(params) => (
    <TextField {...params} label={t("project.field.unblindMembers.workers")} size="small" />
  )}
/>

{mode === "edit" && (
  <FormControlLabel
    control={
      <Switch
        checked={active}
        onChange={(e) => setActive(e.target.checked)}
      />
    }
    label={t("project.field.active")}
  />
)}
```

Submit:

```tsx
const submitDisabled =
  !code.trim() ||
  !description.trim() ||
  productId === null ||
  create.isPending ||
  update.isPending;

async function onSubmit() {
  const members = { leaders: memberLeaders, workers: memberWorkers };
  const unblindMembers = { leaders: unblindLeaders, workers: unblindWorkers };
  try {
    if (mode === "create") {
      await create.mutateAsync({
        code: code.trim(),
        description: description.trim(),
        productId,
        members,
        unblindMembers,
      });
    } else if (mode === "edit" && code) {
      await update.mutateAsync({
        code,
        body: {
          description: description.trim(),
          productId,
          active,
          members,
          unblindMembers,
        },
      });
    }
    onClose();
  } catch {
    /* error surfaced below */
  }
}
```

Error surfacing and footer:

```tsx
{(create.error || update.error) && (
  <Alert severity="error">
    {errorMessage(
      create.error ?? update.error,
    )}
  </Alert>
)}

<Stack direction="row" spacing={1} justifyContent="flex-end">
  <Button onClick={onClose}>{t("common.cancel")}</Button>
  <Button
    variant="contained"
    disabled={submitDisabled}
    onClick={() => void onSubmit()}
  >
    {t(mode === "create" ? "project.action.create" : "project.action.save")}
  </Button>
</Stack>
```

Notes:

- `Autocomplete<UserView, true>` — the second generic param is the
  `multiple` flag; required by MUI's typing to make `value` an
  array.
- `getOptionLabel` uses code + name so the picker shows
  `alice — Alice` rather than just `alice`, matching how the table
  chips display the name.
- The `active` switch is omitted in create mode because the server
  defaults new projects to active; including the switch would
  expose a UI surface that the server does not honor on create.

## Role gating summary

- `canEdit = role === 'root' || role === 'admin'` is computed in the
  orchestrator from `useCurrentUser`.
- `ProjectTable`:
  - `canEdit === false`: Add icon button in the header is omitted.
  - `canEdit === false`: Edit icon button in each row is omitted.
  - OpenInNew always renders (per spec, "leave it to do nothing for
    now"); rendered as `disabled`.
- `ProjectDrawer` is always mounted but only opens from UI actions
  (`onOpenCreate` / `onOpenEdit`), both gated by `canEdit`. A
  programmatic open by a general user would still render the form —
  the server is the source of truth for authorization; this matches
  the existing layering (authz lives on the server, the UI hides
  affordances).

## i18n keys

Add to both `lib/packages/ui/src/i18n/locales/en.ts` and
`zhCN.ts`. Both files must remain `Record<keyof typeof en, string>`
compatible, so every key is added to both at once.

| Key | en | zh-CN |
|---|---|---|
| `nav.projects` | Projects | 项目 |
| `project.heading` | Projects | 项目 |
| `project.search.label` | Search (code, description, leaders) | 搜索（代码、描述、负责人） |
| `project.involve` | Involve | 我参与的 |
| `project.col.leaders` | Leaders | 负责人 |
| `project.col.active` | Status | 状态 |
| `project.active` | Active | 已启用 |
| `project.inactive` | Inactive | 未启用 |
| `project.add` | Add project | 新增项目 |
| `project.edit` | Edit project | 编辑项目 |
| `project.open` | Open project | 打开项目 |
| `project.empty` | No projects yet | 暂无项目 |
| `project.loadFailed` | Failed to load projects: {message} | 加载项目失败：{message} |
| `project.create.title` | Create project | 创建项目 |
| `project.edit.title` | Edit project | 编辑项目 |
| `project.field.code` | Code | 代码 |
| `project.field.description` | Description | 描述 |
| `project.field.product` | Product | 产品 |
| `project.field.active` | Active | 已启用 |
| `project.field.members.leaders` | Members — leaders | 成员 — 负责人 |
| `project.field.members.workers` | Members — workers | 成员 — 工作人员 |
| `project.field.unblindMembers.leaders` | Unblind members — leaders | 非盲成员 — 负责人 |
| `project.field.unblindMembers.workers` | Unblind members — workers | 非盲成员 — 工作人员 |
| `project.action.create` | Create | 创建 |
| `project.action.save` | Save | 保存 |
| `common.cancel` | Cancel | 取消 |

## Tests

### `src/test/data/project.test.tsx`

Mirror the pattern in `src/test/data/user.test.tsx`. Coverage:

- `useListProjects`: fetches `list_projects` on mount; result
  propagates into `query.data`; error propagates into `query.error`.
- `useProject(null)`: no fetch on mount (disabled); queryKey is the
  disabled placeholder.
- `useProject("alpha")`: `refetch()` calls `get_project_by_code`
  with `{ code: "alpha" }`; with `staleTime: 0`, two consecutive
  `refetch()` calls produce two `invoke("get_project_by_code")`
  calls (cache never satisfies).
- `useCreateProject`:
  - calls `create_project` with the input.
  - on success: invalidates `queryKeys.project.all()` exactly once;
    does NOT call `qc.clear()`.
- `useUpdateProject`:
  - calls `update_project` with `{ code, body }`.
  - on success: invalidates both `project.all()` and
    `project.byCode(code)`.

### `src/test/data/product.test.tsx`

- `useListProducts`: fetches `list_products` on mount.

### `src/test/data/user.test.tsx`

Append cases:

- `useListUsers`: fetches `list_users` on mount when `enabled`
  defaults to `true`.
- `useListUsers({ enabled: false })`: no fetch on mount.

### `src/test/pages/project-table.test.tsx`

- Renders all column headers.
- For a row with `members.leaders = [alice]` and
  `unblindMembers.leaders = [bob]`: renders one `Chip variant=
  "outlined"` labelled `Alice` and one `Chip variant="filled"`
  labelled `Bob`.
- `active=true` row renders a `CheckCircle` icon (testable via
  `getByLabelText` or `container.querySelector('[data-testid=
  "CheckCircleIcon"]')` — fallback to `screen.getByRole("img", {
  hidden: true })` for SVG icons).
- `active=false` row renders a `Cancel` icon.
- `canEdit=false`: Add icon button NOT in document; Edit icon
  button NOT in document; OpenInNew icon button IS in document
  with `disabled` attribute set.
- `canEdit=true`: all three icon buttons render.
- Clicking Edit calls `onOpenEdit(row.code)`.
- Empty members / unblindMembers leader arrays render the em-dash.

### `src/test/pages/project-drawer.test.tsx`

- Render with `mode="create"`: shows "Create project" title; `code`
  field enabled; `active` switch NOT in document.
- Render with `mode="edit", code="alpha"` and a mocked project
  fetch: `code` field disabled; `active` switch in document; form
  fields populated from the fetched project.
- Submit in create mode calls `useCreateProject.mutateAsync` with
  the assembled shape (productId from Autocomplete selection, members
  and unblindMembers from the four pickers).
- Submit in edit mode calls `useUpdateProject.mutateAsync` with
  `{ code: "alpha", body: { description, productId, active, members,
  unblindMembers } }` (no `code` inside `body`).
- Submit button `disabled` while `isPending` is `true`.
- Mutation error renders an `Alert severity="error"` with
  `errorMessage(error)`.
- Drawer `open` prop is `false` when `mode === "closed"`.

### `src/test/pages/project-list.test.tsx`

- Renders heading + filter bar + table.
- Search filter narrows rows by code/description/leader
  case-insensitively.
- Involve toggle without a current user produces an empty list (no
  crash).
- With current user `alice` (mocked via `useCurrentUser` /
  `current_user` handler), Involve shows only projects where
  `alice` is in members / unblindMembers.
- Search + Involve combine with AND.
- `canEdit=true`: Add icon button present; clicking opens drawer
  with `mode="create"`.
- `canEdit=false`: Add icon button absent.
- Edit click opens drawer with `mode="edit", code=<row.code>`.
- Drawer close resets `mode` to `"closed"`; the Alert and form
  unmount.

### `src/test/routes/projects.test.tsx`

- `mockCommands({ is_logged_in: () => true })`, mount full router at
  `/projects`:
  - `<Sidebar>` (data-testid `sidebar`) visible.
  - Heading "Projects" visible.
  - `<HomePage>` heading NOT visible.
- Click "Projects" in the sidebar from `/settings` navigates to
  `/projects` (router state assertion).
- Visiting `/projects` while not logged in redirects to `/login`
  (covered by the parent `_layout/route.tsx`'s `beforeLoad`, but
  asserted here as a regression guard).

## Error handling summary

| Failure | Surface |
|---|---|
| `listProjects` throws | `Alert severity="error"` above table; table not rendered. |
| `getProjectByCode` throws (edit open) | Drawer shows `Alert` in form area; form stays interactive. |
| `createProject` throws | Drawer shows `Alert` with `errorMessage(error)`; drawer stays open. |
| `updateProject` throws | Same as create. |
| `useCurrentUser` fails | Filter and table still work; Involve toggle produces an empty list (no crash). |
| `listProducts` fails inside drawer | Product Autocomplete shows no options; submit disabled while `productId === null`. |
| `listUsers` fails inside drawer | Member Autocompletes show no options; submit still enabled if members arrays are empty. |

Error narrowing goes through `toApiError` / `errorMessage` /
`httpCode` from `src/api/error.ts` — already the convention; no new
helpers.

## Out of scope (deferred to a later feature)

- The OpenInNew action: rendered as `disabled` per spec; future
  "go to project detail page" is a separate feature.
- Server-side search / pagination (the project list is small enough
  for client-side filter).
- Worker column in the table — only leaders render chips, per spec.
- Optimistic updates on create / update.
- Sorting, pagination, column resizing.
- Form-level validation rules beyond required-field enforcement
  (no max length, no regex on code, etc.). The server is the source
  of truth; the drawer surfaces server errors via the mutation
  error path.

## File changes summary

**New files**

- `apps/desktop/aegis-desktop/src/data/project.ts`
- `apps/desktop/aegis-desktop/src/data/product.ts`
- `apps/desktop/aegis-desktop/src/pages/project-list.tsx`
- `apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx`
- `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`
- `apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx`
- `apps/desktop/aegis-desktop/src/routes/_layout/projects.tsx`
- `apps/desktop/aegis-desktop/src/test/data/project.test.tsx`
- `apps/desktop/aegis-desktop/src/test/data/product.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-list.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-drawer.test.tsx`
- `apps/desktop/aegis-desktop/src/test/routes/projects.test.tsx`

**Modified files**

- `apps/desktop/aegis-desktop/src/data/user.ts` — add `useListUsers`
- `apps/desktop/aegis-desktop/src/data/queryKeys.ts` — add
  `project.*`, `product.all()`, `user.list()`
- `apps/desktop/aegis-desktop/src/data/index.ts` — re-export new
  hooks
- `apps/desktop/aegis-desktop/src/test/data/user.test.tsx` — add
  `useListUsers` cases
- `apps/desktop/aegis-desktop/src/pages/layout.tsx` — add Projects
  menu entry between Home and Settings
- `lib/packages/ui/src/i18n/locales/en.ts` — add project.* +
  nav.projects + common.cancel
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — mirror the same
  keys
- `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` —
  regenerated by `@tanstack/router-plugin`

**Untouched**

- `apps/desktop/aegis-desktop/src/api/**` — pure Tauri transport
- `apps/desktop/aegis-desktop/src/components/**`
- `apps/desktop/aegis-desktop/src/routes/__root.tsx`,
  `_layout/route.tsx`, `_layout/index.tsx`, `_layout/settings.tsx`
- `apps/desktop/aegis-desktop/src/pages/{home,settings,UserFooter,
  bootstrap}.tsx`
- `apps/desktop/aegis-desktop/src/main.tsx`
- All other tests, vitest config, package.json