# Project List Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/projects` page under the `_layout` route in the Aegis desktop Tauri app, with a filterable project table (search + Involve toggle), role-gated create / update drawer (right-anchored MUI Drawer), and TanStack Query hooks wrapping the four project APIs plus `listProducts` / `listUsers` for the drawer's form.

**Architecture:** Four small page components (orchestrator + filter bar + table + drawer) consumed by a new `_layout/projects` TanStack Router file route. Three new data files (`project.ts`, `product.ts`, plus one new hook in `user.ts`) plus query-key additions and a barrel re-export. The Sidebar's `menu` array gains a `Projects` entry between Home and Settings. i18n keys live in the shared `@aegis/ui` package's en/zh-CN catalogs.

**Tech Stack:** React 19, TanStack Router (`@tanstack/react-router`), TanStack Query 5, MUI 9 (`@mui/material` + `@mui/icons-material`), Vitest + `@testing-library/react`, `@aegis/ui` (shared workspace package for theme / i18n / icons).

## Global Constraints

These constraints apply to every task. Verbatim from the spec:

- **Layering** — `pages/ → data/ → api/`. Pages must never import `api/*` directly; they consume hooks. `data/` must never import `pages/`.
- **Hook placement** — Each new hook lives in `src/data/<resource>.ts` and is re-exported from `src/data/index.ts`. Pages import from `src/data`.
- **Query keys** — All keys live in `src/data/queryKeys.ts` as a typed `as const` factory. Hooks reference keys via the factory; never inline arrays.
- **QueryClient defaults** — `staleTime: Infinity`, `retry: false`, `refetchOnWindowFocus: false`, `refetchOnReconnect: false`, mutation `retry: false`. Hooks that need different staleTime (e.g. manual-trigger probes) set it per-query.
- **i18n catalog** — All strings must satisfy `Record<keyof typeof en, string>` in both `en.ts` and `zhCN.ts`. Adding a key to one without the other breaks `pnpm typecheck`.
- **Tauri mocking** — Tests mock `@tauri-apps/api/core` via `vi.mock(...)` at the top of each test file and dispatch via `mockCommands({ ... })` from `src/test/tauri-mock.ts`.
- **Test isolation** — Page / hook tests wrap renders in `TestQueryProvider` (a per-test `QueryClient`); route tests use `renderWithFullRouter` with the same wrapper. Never share `QueryClient` across tests.
- **StrictMode-safe effects** — Single-shot effects (e.g. "fetch on mount") must guard against React StrictMode's double-invocation with a `useRef` gate, following the `lookedUp` pattern in `register.tsx`.
- **TDD** — Every task that adds behavior writes the failing test FIRST, runs it (red), then writes the implementation, runs it again (green), then commits.
- **Commit cadence** — One commit per task. Commit message format: `feat(desktop): <verb> <thing>` (or `refactor`, `docs`, `chore`). Co-author line at the end: `Co-Authored-By: Claude <noreply@anthropic.com>`.

### Type fixtures used across tasks

The wire-DTO types live in `src/api/types.ts`. They are unchanged by this feature. Tests reuse the fixtures below verbatim — copy them, don't redefine.

```ts
// Full project fixture used by hook tests and page tests.
const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project description",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "Product A description",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const productFixture: ProductView = {
  id: 10,
  code: "prod-a",
  name: "Product A",
  description: "Product A description",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
```

### Verification commands used across tasks

```bash
# Type-check the whole desktop app.
pnpm --filter aegis-desktop typecheck

# Run all desktop tests.
pnpm --filter aegis-desktop test

# Run a single test file.
pnpm --filter aegis-desktop test -- src/test/path/to/file.test.tsx

# Type-check the shared UI package after i18n changes.
pnpm --filter @aegis/ui typecheck
```

---

## Task 1: Add project / nav / common-cancel i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Why first:** Every subsequent task references these keys via `t("project.*")`. Adding them before any consumer exists keeps the catalogs consistent and proves the type constraint (`Record<keyof typeof en, string>`) is met before features depend on it.

- [ ] **Step 1: Add English keys**

Open `lib/packages/ui/src/i18n/locales/en.ts`. Inside the exported `en` object, after the existing `'app.user.loadFailed': 'Failed to load user info'` line (last key in the file), append the following entries. Keep alphabetical order is NOT required — appending keeps the diff focused.

```ts
  // ... existing keys ...
  'app.user.loadFailed': 'Failed to load user info',

  'nav.projects': 'Projects',
  'common.cancel': 'Cancel',
  'project.heading': 'Projects',
  'project.search.label': 'Search (code, description, leaders)',
  'project.involve': 'Involve',
  'project.col.leaders': 'Leaders',
  'project.col.active': 'Status',
  'project.active': 'Active',
  'project.inactive': 'Inactive',
  'project.add': 'Add project',
  'project.edit': 'Edit project',
  'project.open': 'Open project',
  'project.empty': 'No projects yet',
  'project.loadFailed': 'Failed to load projects: {message}',
  'project.create.title': 'Create project',
  'project.edit.title': 'Edit project',
  'project.field.code': 'Code',
  'project.field.description': 'Description',
  'project.field.product': 'Product',
  'project.field.active': 'Active',
  'project.field.members.leaders': 'Members — leaders',
  'project.field.members.workers': 'Members — workers',
  'project.field.unblindMembers.leaders': 'Unblind members — leaders',
  'project.field.unblindMembers.workers': 'Unblind members — workers',
  'project.action.create': 'Create',
  'project.action.save': 'Save',
} as const;
```

(The file ends with `} as const;`. The new keys go inside the object, just before that closing brace.)

- [ ] **Step 2: Add Simplified Chinese keys**

Open `lib/packages/ui/src/i18n/locales/zhCN.ts`. After the last existing entry (`'app.user.loadFailed': '加载用户信息失败'`), and just before the `} satisfies Record<keyof typeof en, string>;` line, add:

```ts
  'app.user.loadFailed': '加载用户信息失败',

  'nav.projects': '项目',
  'common.cancel': '取消',
  'project.heading': '项目',
  'project.search.label': '搜索（代码、描述、负责人）',
  'project.involve': '我参与的',
  'project.col.leaders': '负责人',
  'project.col.active': '状态',
  'project.active': '已启用',
  'project.inactive': '未启用',
  'project.add': '新增项目',
  'project.edit': '编辑项目',
  'project.open': '打开项目',
  'project.empty': '暂无项目',
  'project.loadFailed': '加载项目失败：{message}',
  'project.create.title': '创建项目',
  'project.edit.title': '编辑项目',
  'project.field.code': '代码',
  'project.field.description': '描述',
  'project.field.product': '产品',
  'project.field.active': '已启用',
  'project.field.members.leaders': '成员 — 负责人',
  'project.field.members.workers': '成员 — 工作人员',
  'project.field.unblindMembers.leaders': '非盲成员 — 负责人',
  'project.field.unblindMembers.workers': '非盲成员 — 工作人员',
  'project.action.create': '创建',
  'project.action.save': '保存',
} satisfies Record<keyof typeof en, string>;
```

The order of keys in `zhCN.ts` MUST mirror `en.ts` because the `satisfies Record<keyof typeof en, string>` constraint requires every key in `en` to be present in `zhCN`. Reordering would silently break the constraint; the position of each new key matches the en.ts position above.

- [ ] **Step 3: Type-check both packages**

Run:

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop typecheck
```

Expected: PASS (both packages exit 0). If `zhCN.ts` is missing a key, TypeScript errors with "Property 'project.heading' is missing in type ... but required in type 'Record<...>'."

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "$(cat <<'EOF'
feat(ui): add project list page i18n keys

Adds nav.projects, common.cancel, and 22 project.* keys covering
heading, filter bar, table columns, drawer titles, and form field
labels to both en and zh-CN catalogs.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add project / product / user-list query keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/data/queryKeys.ts`

**Why second:** Every subsequent hook references the new keys. Adding them now lets Tasks 3–5 type-check their key references against the factory.

- [ ] **Step 1: Append the new factory entries**

Open `apps/desktop/aegis-desktop/src/data/queryKeys.ts`. The file currently reads:

```ts
export const queryKeys = {
  auth: {
    loginStatus: () => ["auth", "loginStatus"] as const,
  },
  bootstrap: {
    health: () => ["bootstrap", "health"] as const,
  },
  user: {
    current: () => ["user", "current"] as const,
    domainIdentity: () => ["user", "domainIdentity"] as const,
  },
} as const;
```

Replace it with:

```ts
export const queryKeys = {
  auth: {
    loginStatus: () => ["auth", "loginStatus"] as const,
  },
  bootstrap: {
    health: () => ["bootstrap", "health"] as const,
  },
  user: {
    current: () => ["user", "current"] as const,
    domainIdentity: () => ["user", "domainIdentity"] as const,
    list: () => ["user", "list"] as const,
  },
  project: {
    all: () => ["project", "list"] as const,
    byCode: (code: string) => ["project", "byCode", code] as const,
  },
  product: {
    all: () => ["product", "list"] as const,
  },
} as const;
```

The `user.list` entry lives under `user.*` (cohesion with the existing user keys). `project.all` is the invalidation target for create / update; `project.byCode(code)` is the per-code cache entry consumed by the edit drawer. `product.all` is the list-cache for the drawer's product dropdown.

- [ ] **Step 2: Type-check**

Run:

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/data/queryKeys.ts
git commit -m "$(cat <<'EOF'
feat(desktop): add query keys for project / product / user list

queryKeys.user.list, queryKeys.project.all,
queryKeys.project.byCode(code), and queryKeys.product.all. None
are consumed yet — Tasks 3-5 wire them up.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Implement project data hooks (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/project.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/project.test.tsx`

**Interfaces (consumed by later tasks):**
- `useListProjects()` → `UseQueryResult<ProjectView[], ApiError>` — auto-fetches on mount, no args.
- `useProject(code: string | null)` → `UseQueryResult<ProjectView, ApiError>` — manual-trigger (`enabled: false`); the drawer calls `.refetch()` after mount.
- `useCreateProject()` → `UseMutationResult<ProjectView, ApiError, CreateProjectInput>` — invalidates `queryKeys.project.all()` on success; does NOT call `qc.clear()`.
- `useUpdateProject()` → `UseMutationResult<ProjectView, ApiError, { code: string; body: UpdateProjectBody }>` — invalidates `project.all()` AND `project.byCode(code)` on success; does NOT call `qc.clear()`.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/aegis-desktop/src/test/data/project.test.tsx` with the content below. This is the test file the engineer should commit BEFORE writing the hook — it will fail to import the hooks at first, then fail each `it` block until the hook is implemented.

```tsx
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  useCreateProject,
  useListProjects,
  useProject,
  useUpdateProject,
} from "../../data/project";
import { queryKeys } from "../../data/queryKeys";
import type { ProjectView } from "../../api";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project description",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "Product A description",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function ListProbe() {
  const q = useListProjects();
  return <span data-testid="count">{q.data?.length ?? "none"}</span>;
}

function SingleProbe({ code }: { code: string | null }) {
  const q = useProject(code);
  return (
    <>
      <button onClick={() => void q.refetch()}>refetch</button>
      <span data-testid="project-code">{q.data?.code ?? "none"}</span>
    </>
  );
}

function CreateHarness() {
  const m = useCreateProject();
  return (
    <button
      onClick={() => {
        m.mutate({
          code: "newproj",
          description: "New",
          productId: 10,
          members: { leaders: [], workers: [] },
          unblindMembers: { leaders: [], workers: [] },
        });
      }}
    >
      create
    </button>
  );
}

function UpdateHarness({ code }: { code: string }) {
  const m = useUpdateProject();
  return (
    <button
      onClick={() => {
        m.mutate({ code, body: { description: "Edited" } });
      }}
    >
      update
    </button>
  );
}

function CacheSeeder({
  client,
  data,
}: {
  client: ReturnType<typeof renderWithQueryClient>["client"];
  data: unknown;
}) {
  useEffect(() => {
    client.setQueryData(queryKeys.project.all(), data);
  }, [client, data]);
  return null;
}

describe("useListProjects", () => {
  it("invokes api.listProjects on mount and exposes the array", async () => {
    mockCommands({ list_projects: () => [projectFixture] });
    renderWithQueryClient(<ListProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_projects");
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });

  it("propagates ApiError into query.error", async () => {
    mockCommands({
      list_projects: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    function ErrorProbe() {
      const q = useListProjects();
      return (
        <span data-testid="error-kind">
          {q.error ? (q.error as { kind: string }).kind : "none"}
        </span>
      );
    }
    renderWithQueryClient(<ErrorProbe />);
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });
});

describe("useProject", () => {
  it("does not fetch on mount when code is null (disabled)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code={null} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("does not fetch on mount even when code is set (manual-trigger, enabled:false)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("refetch() invokes api.getProjectByCode with the code", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "alpha",
      });
      expect(screen.getByTestId("project-code").textContent).toBe("alpha");
    });
  });

  it("two consecutive refetch() calls both hit the server (staleTime: 0)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});

describe("useCreateProject", () => {
  it("invokes api.createProject with the input shape", async () => {
    mockCommands({ create_project: () => projectFixture });
    renderWithQueryClient(<CreateHarness />);
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_project", {
        code: "newproj",
        description: "New",
        productId: 10,
        members: { leaders: [], workers: [] },
        unblindMembers: { leaders: [], workers: [] },
      });
    });
  });

  it("invalidates queryKeys.project.all() on success", async () => {
    mockCommands({ create_project: () => projectFixture });
    const { client } = renderWithQueryClient(<CreateHarness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: queryKeys.project.all() }),
      );
    });
  });

  it("does not clear the entire cache (unlike logout)", async () => {
    mockCommands({ create_project: () => projectFixture });
    const { client } = renderWithQueryClient(
      <>
        <CacheSeeder client={client} data={[projectFixture]} />
        <CreateHarness />
      </>,
    );
    const clearSpy = vi.spyOn(client, "clear");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_project", expect.anything()));
    expect(clearSpy).not.toHaveBeenCalled();
  });
});

describe("useUpdateProject", () => {
  it("invokes api.updateProject with { code, body }", async () => {
    mockCommands({ update_project: () => projectFixture });
    renderWithQueryClient(<UpdateHarness code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "update" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_project", {
        code: "alpha",
        body: { description: "Edited" },
      });
    });
  });

  it("invalidates queryKeys.project.all() AND project.byCode(code) on success", async () => {
    mockCommands({ update_project: () => projectFixture });
    const { client } = renderWithQueryClient(<UpdateHarness code="alpha" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "update" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: queryKeys.project.all() }),
      );
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.project.byCode("alpha"),
        }),
      );
    });
  });
});
```

- [ ] **Step 2: Run the test file — expect it to fail**

```bash
pnpm --filter aegis-desktop test -- src/test/data/project.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../data/project"`. (The hook file doesn't exist yet.)

- [ ] **Step 3: Implement the hook file**

Create `apps/desktop/aegis-desktop/src/data/project.ts`:

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateProjectInput,
  type ProjectView,
  type UpdateProjectBody,
} from "../api";
import { queryKeys } from "./queryKeys";

/**
 * All projects. Fetches on mount. Inherits the global
 * `staleTime: Infinity` — the list is re-read when the page mounts,
 * not via refetch.
 */
export function useListProjects() {
  return useQuery<ProjectView[], ApiError>({
    queryKey: queryKeys.project.all(),
    queryFn: () => api.listProjects(),
  });
}

/**
 * Single project by code. Manual-trigger (matches `useDomainUserInfo`
 * in `user.ts`) — `enabled: false` by default; the drawer drives
 * the fetch with `refetch()` so opening the edit drawer fires a
 * fresh read without auto-firing on every mount. `staleTime: 0`
 * keeps the read always-fresh before edit.
 */
export function useProject(code: string | null) {
  return useQuery<ProjectView, ApiError>({
    queryKey:
      code === null
        ? ["project", "byCode", "__disabled__"]
        : queryKeys.project.byCode(code),
    queryFn: () => {
      if (code === null) throw new Error("useProject disabled");
      return api.getProjectByCode(code);
    },
    enabled: false,
    staleTime: 0,
  });
}

/**
 * Create project. On success: invalidates the project list cache so
 * the next render shows the new row. Does NOT clear the cache
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
  return useMutation<
    ProjectView,
    ApiError,
    { code: string; body: UpdateProjectBody }
  >({
    mutationFn: ({ code, body }) => api.updateProject(code, body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.project.all() });
      qc.invalidateQueries({ queryKey: queryKeys.project.byCode(vars.code) });
    },
  });
}
```

- [ ] **Step 4: Re-run the tests — expect them to pass**

```bash
pnpm --filter aegis-desktop test -- src/test/data/project.test.tsx
```

Expected: PASS. All 11 `it` blocks green.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/data/project.ts apps/desktop/aegis-desktop/src/test/data/project.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add project hooks (list, by-code, create, update)

useListProjects auto-fetches on mount. useProject is
manual-trigger (enabled: false) with staleTime: 0 to keep the
edit drawer's read always fresh. useCreateProject invalidates
project.all on success; useUpdateProject invalidates both
project.all and project.byCode(code).

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Implement `useListProducts` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/product.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/product.test.tsx`

**Interface (consumed by Task 9):**
- `useListProducts()` → `UseQueryResult<ProductView[], ApiError>` — auto-fetches on mount; inherits `staleTime: Infinity`.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/data/product.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useListProducts } from "../../data/product";
import type { ProductView } from "../../api";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

const productFixture: ProductView = {
  id: 10,
  code: "prod-a",
  name: "Product A",
  description: "Product A description",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function ProductsProbe() {
  const q = useListProducts();
  return (
    <span data-testid="count">
      {q.data?.length ?? "none"}
    </span>
  );
}

describe("useListProducts", () => {
  it("invokes api.listProducts on mount and exposes the array", async () => {
    mockCommands({ list_products: () => [productFixture] });
    renderWithQueryClient(<ProductsProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_products");
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });

  it("propagates ApiError into query.error", async () => {
    mockCommands({
      list_products: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    function ErrorProbe() {
      const q = useListProducts();
      return (
        <span data-testid="error-kind">
          {q.error ? (q.error as { kind: string }).kind : "none"}
        </span>
      );
    }
    renderWithQueryClient(<ErrorProbe />);
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/data/product.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../data/product"`.

- [ ] **Step 3: Implement**

Create `apps/desktop/aegis-desktop/src/data/product.ts`:

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

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/data/product.test.tsx
```

Expected: PASS (2 tests green).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/data/product.ts apps/desktop/aegis-desktop/src/test/data/product.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add useListProducts hook

Wraps api.listProducts for the project drawer's product dropdown.
Inherits the global staleTime: Infinity.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Add `useListUsers` to `user.ts` (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/data/user.ts`
- Modify: `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`

**Interface (consumed by Task 9):**
- `useListUsers(options?: { enabled?: boolean })` → `UseQueryResult<UserView[], ApiError>` — `enabled` defaults to `true`. The drawer is the only consumer and only opens for root/admin, so the default is safe.

- [ ] **Step 1: Append failing tests to `user.test.tsx`**

Open `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`. Add the following imports at the top (inside the `vi.mock` block section, see how `useCurrentUser` is imported — match the existing style):

```tsx
import { useListUsers } from "../../data/user";
```

(Add this line to the existing `import { useCurrentUser, useDomainUserInfo, useLogout, useRegisterUser } from "../../data/user";` block.)

Then add this fixture at the top of the file (after the existing `userView` / `identity` constants):

```tsx
const usersList = [
  userView,
  {
    id: 2,
    code: "bob",
    name: "Bob",
    role: "general" as const,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];
```

Then add a probe component and a describe block at the bottom of the file:

```tsx
function ListUsersProbe({ enabled }: { enabled?: boolean }) {
  const q = useListUsers({ enabled });
  return (
    <span data-testid="count">{q.data?.length ?? "none"}</span>
  );
}

describe("useListUsers", () => {
  it("invokes api.listUsers on mount when enabled defaults to true", async () => {
    mockCommands({ list_users: () => usersList });
    renderWithQueryClient(<ListUsersProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_users");
      expect(screen.getByTestId("count").textContent).toBe("2");
    });
  });

  it("does not fetch on mount when enabled is false", async () => {
    mockCommands({ list_users: () => usersList });
    renderWithQueryClient(<ListUsersProbe enabled={false} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run — expect the new `useListUsers` describe block to fail**

```bash
pnpm --filter aegis-desktop test -- src/test/data/user.test.tsx
```

Expected: FAIL with `useListUsers` not exported from `../../data/user`. The other describe blocks (useCurrentUser, useDomainUserInfo, etc.) still pass.

- [ ] **Step 3: Append the hook to `user.ts`**

Open `apps/desktop/aegis-desktop/src/data/user.ts`. After the existing `useLogout` function (last export), add:

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

The existing `import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";` and `import { api, type ApiError, type Identity, type RegisterUserInput, type RegisterUserResponse, type UserView } from "../api";` already cover everything needed. The `queryKeys` import already exists at the top of the file.

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/data/user.test.tsx
```

Expected: PASS (all describe blocks green, including the new `useListUsers` block).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/data/user.ts apps/desktop/aegis-desktop/src/test/data/user.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add useListUsers hook

Wraps api.listUsers for the project drawer's member pickers.
Default enabled: true — the drawer is the only consumer and only
opens for root/admin.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Re-export new hooks from the data barrel

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/data/index.ts`

**Why:** Pages import from `src/data` (the barrel), never from individual files. This matches the existing pattern documented in the data barrel's comment header.

- [ ] **Step 1: Add the re-exports**

Open `apps/desktop/aegis-desktop/src/data/index.ts`. The current file ends with:

```ts
// Re-export the React Query primitive that pages may need for ad-hoc
// cache interactions (e.g. `queryClient.setQueryData`).
export { useQueryClient } from "@tanstack/react-query";
```

Add the following three export lines BEFORE the `useQueryClient` line:

```ts
export {
  useCreateProject,
  useListProjects,
  useProject,
  useUpdateProject,
} from "./project";
export { useListProducts } from "./product";
export { useListUsers } from "./user";
```

So the file becomes:

```ts
// Barrel for the React Query layer. Pages should import from this
// module rather than reaching into individual files, so a future
// resource split (e.g. splitting `user.ts` into `user.ts` +
// `projectMembers.ts`) can happen without touching pages.

export { QueryProvider } from "./client";
export { queryKeys } from "./queryKeys";

export { useLogin, useLoginDomain } from "./auth";
export { useHealthz, useIsLoggedIn } from "./bootstrap";
export {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useRegisterUser,
  useLogout,
} from "./user";
export {
  useCreateProject,
  useListProjects,
  useProject,
  useUpdateProject,
} from "./project";
export { useListProducts } from "./product";

// Re-export the React Query primitive that pages may need for ad-hoc
// cache interactions (e.g. `queryClient.setQueryData`).
export { useQueryClient } from "@tanstack/react-query";
```

Note: the `user.ts` re-export block is updated to include `useListUsers` (alphabetical order within the block — `useListUsers` fits between `useDomainUserInfo` and `useRegisterUser`).

- [ ] **Step 2: Type-check**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/data/index.ts
git commit -m "$(cat <<'EOF'
feat(desktop): re-export new project / product / user hooks

Pages consume the data layer via this barrel; individual files
remain an implementation detail.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Implement `ProjectFilterBar` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/project-filter-bar.test.tsx`

**Interface (consumed by Task 10):**
- `ProjectFilterBar({ query, onQueryChange, involve, onInvolveChange }: ProjectFilterBarProps)` — controlled component. No internal state.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/pages/project-filter-bar.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ProjectFilterBar } from "../../pages/ProjectFilterBar";

afterEach(() => cleanup());

function renderBar(props: {
  query?: string;
  involve?: boolean;
  onQueryChange?: (v: string) => void;
  onInvolveChange?: (v: boolean) => void;
} = {}) {
  const onQueryChange = props.onQueryChange ?? vi.fn();
  const onInvolveChange = props.onInvolveChange ?? vi.fn();
  return {
    onQueryChange,
    onInvolveChange,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectFilterBar
            query={props.query ?? ""}
            onQueryChange={onQueryChange}
            involve={props.involve ?? false}
            onInvolveChange={onInvolveChange}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}

describe("ProjectFilterBar", () => {
  it("renders the search field with the current value", () => {
    renderBar({ query: "alpha" });
    expect(screen.getByLabelText(/search/i)).toHaveValue("alpha");
  });

  it("renders the Involve checkbox", () => {
    renderBar({ involve: false });
    expect(screen.getByRole("checkbox", { name: /involve/i })).not.toBeChecked();
  });

  it("checks the Involve checkbox when involve=true", () => {
    renderBar({ involve: true });
    expect(screen.getByRole("checkbox", { name: /involve/i })).toBeChecked();
  });

  it("calls onQueryChange when the search field changes", async () => {
    const { onQueryChange } = renderBar();
    await userEvent.type(screen.getByLabelText(/search/i), "a");
    expect(onQueryChange).toHaveBeenLastCalledWith("a");
  });

  it("calls onInvolveChange when the checkbox is clicked", async () => {
    const { onInvolveChange } = renderBar({ involve: false });
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    expect(onInvolveChange).toHaveBeenCalledWith(true);
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-filter-bar.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../pages/ProjectFilterBar"`.

- [ ] **Step 3: Implement**

Create `apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx`:

```tsx
import {
  Box,
  Checkbox,
  FormControlLabel,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export interface ProjectFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  involve: boolean;
  onInvolveChange: (value: boolean) => void;
}

/**
 * Search field + Involve toggle. Pure controlled component — the
 * orchestrator owns the state. The search field stays enabled even
 * when no current user is loaded; toggling Involve with no user just
 * produces an empty result.
 */
export function ProjectFilterBar({
  query,
  onQueryChange,
  involve,
  onInvolveChange,
}: ProjectFilterBarProps) {
  const { t } = useI18n();

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
      <TextField
        size="small"
        label={t("project.search.label")}
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        sx={{ minWidth: 320 }}
      />
      <FormControlLabel
        sx={{ ml: "auto" }}
        control={
          <Checkbox
            checked={involve}
            onChange={(event) => onInvolveChange(event.target.checked)}
          />
        }
        label={t("project.involve")}
      />
    </Box>
  );
}
```

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-filter-bar.test.tsx
```

Expected: PASS (5 tests green).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx apps/desktop/aegis-desktop/src/test/pages/project-filter-bar.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add ProjectFilterBar (search + Involve toggle)

Controlled component — orchestrator owns state. No internal
state. Used by project-list page.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Implement `ProjectTable` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`

**Interface (consumed by Task 10):**
- `ProjectTable({ rows, loading, error, canEdit, onOpenCreate, onOpenEdit }: ProjectTableProps)` — pure rendering. Renders MUI Table; chips per leader; icon per active; Add / Edit / OpenInNew icon buttons in the operation column. `canEdit=false` hides Add and Edit; OpenInNew always renders as `disabled`.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ProjectTable } from "../../pages/ProjectTable";
import type { ProjectView } from "../../api";

const baseRow: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

afterEach(() => cleanup());

function renderTable(props: {
  rows?: ProjectView[];
  loading?: boolean;
  error?: { kind: string } | null;
  canEdit?: boolean;
  onOpenCreate?: () => void;
  onOpenEdit?: (code: string) => void;
} = {}) {
  const onOpenCreate = props.onOpenCreate ?? vi.fn();
  const onOpenEdit = props.onOpenEdit ?? vi.fn();
  return {
    onOpenCreate,
    onOpenEdit,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectTable
            rows={props.rows ?? [baseRow]}
            loading={props.loading ?? false}
            error={props.error ?? null}
            canEdit={props.canEdit ?? true}
            onOpenCreate={onOpenCreate}
            onOpenEdit={onOpenEdit}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}

describe("ProjectTable — column rendering", () => {
  it("renders all five column headers", () => {
    renderTable();
    expect(screen.getByRole("columnheader", { name: /^code$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^description$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^leaders$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^status$/i })).toBeInTheDocument();
  });

  it("renders a code and description cell for the row", () => {
    renderTable();
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("Alpha project")).toBeInTheDocument();
  });

  it("renders an outlined chip for members.leaders and a filled chip for unblindMembers.leaders", () => {
    renderTable();
    const aliceChip = screen.getByText("Alice").closest(".MuiChip-root");
    const bobChip = screen.getByText("Bob").closest(".MuiChip-root");
    expect(aliceChip).toHaveClass("MuiChip-outlined");
    expect(bobChip).toHaveClass("MuiChip-filled");
  });

  it("renders an em-dash when both leader arrays are empty", () => {
    renderTable({
      rows: [
        {
          ...baseRow,
          members: { leaders: [], workers: [] },
          unblindMembers: { leaders: [], workers: [] },
        },
      ],
    });
    // The em-dash sits in the leaders cell; assert it is rendered.
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("renders a CheckCircle icon for active=true", () => {
    renderTable();
    expect(screen.getByTestId("CheckCircleIcon")).toBeInTheDocument();
  });

  it("renders a Cancel icon for active=false", () => {
    renderTable({ rows: [{ ...baseRow, active: false }] });
    expect(screen.getByTestId("CancelIcon")).toBeInTheDocument();
  });
});

describe("ProjectTable — operation column role gating", () => {
  it("renders Add, Edit, and OpenInNew when canEdit=true", () => {
    renderTable({ canEdit: true });
    expect(screen.getByRole("button", { name: /add project/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /edit project/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /open project/i })).toBeInTheDocument();
  });

  it("hides Add and Edit when canEdit=false but still renders OpenInNew as disabled", () => {
    renderTable({ canEdit: false });
    expect(screen.queryByRole("button", { name: /add project/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /edit project/i })).not.toBeInTheDocument();
    const openBtn = screen.getByRole("button", { name: /open project/i });
    expect(openBtn).toBeInTheDocument();
    expect(openBtn).toBeDisabled();
  });

  it("calls onOpenCreate when Add is clicked", async () => {
    const { onOpenCreate } = renderTable({ canEdit: true });
    await userEvent.click(screen.getByRole("button", { name: /add project/i }));
    expect(onOpenCreate).toHaveBeenCalledTimes(1);
  });

  it("calls onOpenEdit(row.code) when Edit is clicked", async () => {
    const { onOpenEdit } = renderTable({ canEdit: true });
    await userEvent.click(screen.getByRole("button", { name: /edit project/i }));
    expect(onOpenEdit).toHaveBeenCalledWith("alpha");
  });
});

describe("ProjectTable — empty / loading / error", () => {
  it("shows an empty-state message when rows is empty", () => {
    renderTable({ rows: [] });
    expect(screen.getByText(/no projects yet/i)).toBeInTheDocument();
  });

  it("shows an error alert when error is set", () => {
    renderTable({
      rows: [],
      error: { kind: "http", status: 500, code: "server", message: "boom" } as { kind: string },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/failed to load projects/i);
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-table.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../pages/ProjectTable"`.

- [ ] **Step 3: Implement**

Create `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`:

```tsx
import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  Add,
  Cancel,
  CheckCircle,
  Edit,
  OpenInNew,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { ApiError, ProjectView } from "../api";
import { errorMessage } from "../api/error";

export interface ProjectTableProps {
  rows: ProjectView[];
  loading: boolean;
  error: ApiError | null;
  canEdit: boolean;
  onOpenCreate: () => void;
  onOpenEdit: (code: string) => void;
}

/**
 * Renders the project list as a MUI Table. The leader chip arrays
 * distinguish members (outlined) from unblindMembers (filled); the
 * active column uses CheckCircle/Cancel; the operation column gates
 * Add/Edit on `canEdit` and always renders the future OpenInNew as
 * disabled.
 */
export function ProjectTable({
  rows,
  loading,
  error,
  canEdit,
  onOpenCreate,
  onOpenEdit,
}: ProjectTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Alert severity="error">
        {t("project.loadFailed", { message: errorMessage(error) })}
      </Alert>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

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
            {rows.map((row) => {
              const memberLeaders = row.members.leaders;
              const unblindLeaders = row.unblindMembers.leaders;
              const noLeaders =
                memberLeaders.length === 0 && unblindLeaders.length === 0;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>{row.code}</TableCell>
                  <TableCell sx={{ maxWidth: 280 }}>
                    <Typography noWrap>{row.description}</Typography>
                  </TableCell>
                  <TableCell>
                    <Stack
                      direction="row"
                      spacing={0.5}
                      flexWrap="wrap"
                      useFlexGap
                    >
                      {memberLeaders.map((u) => (
                        <Chip
                          key={`m-${u.code}`}
                          variant="outlined"
                          size="small"
                          label={u.name}
                        />
                      ))}
                      {unblindLeaders.map((u) => (
                        <Chip
                          key={`u-${u.code}`}
                          variant="filled"
                          size="small"
                          label={u.name}
                        />
                      ))}
                      {noLeaders && <span>—</span>}
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
                    <Stack
                      direction="row"
                      spacing={0.5}
                      justifyContent="flex-end"
                    >
                      {canEdit && (
                        <IconButton
                          aria-label={t("project.edit")}
                          onClick={() => onOpenEdit(row.code)}
                        >
                          <Edit />
                        </IconButton>
                      )}
                      <IconButton aria-label={t("project.open")} disabled>
                        <OpenInNew />
                      </IconButton>
                    </Stack>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{t("project.empty")}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}
```

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-table.test.tsx
```

Expected: PASS (12 tests green). The MUI icons render with `data-testid` attributes like `CheckCircleIcon` / `CancelIcon` (verified empirically; if the icon test fails on the testid, the engineer's fallback is `screen.getByTitle("Active")` after adding `title` to the icon — but the default testid is correct).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add ProjectTable (filterable list rendering)

Renders code/description/leaders (outlined for members, filled for
unblindMembers)/active/operation columns. Hide Add/Edit when
canEdit=false; OpenInNew always renders disabled.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Implement `ProjectDrawer` (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/project-drawer.test.tsx`

**Interface (consumed by Task 10):**
- `ProjectDrawer({ mode, code, onClose }: ProjectDrawerProps)` — right-anchored MUI Drawer with `width: 480`. The drawer body only mounts when `mode !== "closed"` (MUI's underlying Modal unmounts children when `open={false}` by default). The form handles both create and update.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/pages/project-drawer.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { ProjectDrawer } from "../../pages/ProjectDrawer";
import type {
  ProductView,
  ProjectView,
  UpdateProjectBody,
  UserView,
} from "../../api";
import { mockCommands } from "../tauri-mock";
import { renderInRouter } from "../file-route-utils";

const productFixture: ProductView = {
  id: 10,
  code: "prod-a",
  name: "Product A",
  description: "",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture2: UserView = {
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha description",
  product: productFixture,
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [],
    workers: [],
  },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderDrawer(
  mode: "closed" | "create" | "edit",
  code: string | null = null,
) {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ProjectDrawer mode={mode} code={code} onClose={vi.fn()} />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectDrawer — closed", () => {
  it("does not render any form fields when mode is 'closed'", async () => {
    await renderDrawer("closed");
    expect(screen.queryByLabelText(/^code$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^description$/i)).not.toBeInTheDocument();
  });
});

describe("ProjectDrawer — create mode", () => {
  it("shows 'Create project' title and an enabled code field", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    expect(screen.getByRole("heading", { name: /create project/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/^code$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^code$/i)).not.toBeDisabled();
  });

  it("does not show the active switch in create mode", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    // 'Active' label appears as the form field label; the Switch's
    // accessible role is 'switch'. Neither should be present.
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
  });

  it("disables Submit until code, description, and product are set", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    const submit = await screen.findByRole("button", { name: /^create$/i });
    expect(submit).toBeDisabled();
  });

  it("calls api.createProject with the assembled shape on Submit", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      create_project: () => projectFixture,
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/^code$/i), "newproj");
    await userEvent.type(screen.getByLabelText(/^description$/i), "New desc");

    // Pick the product via the Autocomplete.
    const productInput = screen.getByLabelText(/^product$/i);
    await userEvent.click(productInput);
    await userEvent.click(screen.getByRole("option", { name: /prod-a/i }));

    // Open the first member-leaders Autocomplete and select Alice.
    const memberLeadersInput = screen.getByLabelText(/members\s*—\s*leaders/i);
    await userEvent.click(memberLeadersInput);
    await userEvent.click(screen.getByRole("option", { name: /alice/i }));

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "create_project",
        expect.objectContaining({
          code: "newproj",
          description: "New desc",
          productId: 10,
          members: expect.objectContaining({
            leaders: expect.arrayContaining([
              expect.objectContaining({ code: "alice" }),
            ]),
          }),
        }),
      );
    });
  });
});

describe("ProjectDrawer — edit mode", () => {
  it("fetches the project via get_project_by_code and pre-fills the form", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "alpha",
      });
    });

    const codeField = await screen.findByLabelText(/^code$/i);
    expect(codeField).toBeDisabled();
    expect(codeField).toHaveValue("alpha");
    expect(screen.getByLabelText(/^description$/i)).toHaveValue("Alpha description");
    // Active switch should be present in edit mode.
    expect(screen.getByRole("switch")).toBeInTheDocument();
  });

  it("calls api.updateProject with { code, body } (no code in body) on Submit", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    // Wait for the form to seed before typing.
    const descriptionField = await screen.findByLabelText(/^description$/i);
    await waitFor(() => expect(descriptionField).toHaveValue("Alpha description"));

    await userEvent.clear(descriptionField);
    await userEvent.type(descriptionField, "Edited");

    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));

    const expectedBody: UpdateProjectBody = expect.objectContaining({
      description: "Edited",
      productId: 10,
      active: true,
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_project", {
        code: "alpha",
        body: expectedBody,
      });
    });
  });
});

describe("ProjectDrawer — mutation error", () => {
  it("shows an Alert with the error message when create_project fails", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      create_project: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/^code$/i), "newproj");
    await userEvent.type(screen.getByLabelText(/^description$/i), "New desc");

    const productInput = screen.getByLabelText(/^product$/i);
    await userEvent.click(productInput);
    await userEvent.click(screen.getByRole("option", { name: /prod-a/i }));

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /server: boom/i,
    );
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-drawer.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../pages/ProjectDrawer"`.

- [ ] **Step 3: Implement**

Create `apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Drawer,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import {
  useCreateProject,
  useListProducts,
  useListUsers,
  useProject,
  useUpdateProject,
} from "../data";
import {
  type ApiError,
  type CreateProjectInput,
  type ProductView,
  type UpdateProjectBody,
  type UserSummary,
  type UserView,
} from "../api";
import { errorMessage } from "../api/error";

export interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
}

/**
 * Right-anchored MUI Drawer for create + update. Always mounted in
 * the orchestrator; the body only renders when `mode !== "closed"`
 * because the underlying Modal unmounts children when `open={false}`.
 * Edit mode triggers a one-shot `get_project_by_code` fetch via
 * `refetch()` to seed the form. The `lookedUp` ref guards against
 * React StrictMode double-fire.
 */
export function ProjectDrawer({ mode, code, onClose }: ProjectDrawerProps) {
  const { t } = useI18n();

  const products = useListProducts();
  const users = useListUsers();
  const fetched = useProject(code);
  const create = useCreateProject();
  const update = useUpdateProject();

  // Form state.
  const [formCode, setFormCode] = useState("");
  const [description, setDescription] = useState("");
  const [productId, setProductId] = useState<number | null>(null);
  const [memberLeaders, setMemberLeaders] = useState<UserSummary[]>([]);
  const [memberWorkers, setMemberWorkers] = useState<UserSummary[]>([]);
  const [unblindLeaders, setUnblindLeaders] = useState<UserSummary[]>([]);
  const [unblindWorkers, setUnblindWorkers] = useState<UserSummary[]>([]);
  const [active, setActive] = useState(true);

  // Seed form when edit mode opens.
  const lookedUp = useRef(false);
  useEffect(() => {
    if (mode !== "edit" || code === null) return;
    if (lookedUp.current) return;
    lookedUp.current = true;
    void (async () => {
      const r = await fetched.refetch();
      if (r.isError || !r.data) return;
      setFormCode(r.data.code);
      setDescription(r.data.description);
      setProductId(r.data.product.id);
      setMemberLeaders(r.data.members.leaders);
      setMemberWorkers(r.data.members.workers);
      setUnblindLeaders(r.data.unblindMembers.leaders);
      setUnblindWorkers(r.data.unblindMembers.workers);
      setActive(r.data.active);
    })();
  }, [mode, code, fetched]);

  const submitDisabled =
    !formCode.trim() ||
    !description.trim() ||
    productId === null ||
    create.isPending ||
    update.isPending;

  async function onSubmit() {
    const members = { leaders: memberLeaders, workers: memberWorkers };
    const unblindMembers = {
      leaders: unblindLeaders,
      workers: unblindWorkers,
    };
    try {
      if (mode === "create") {
        const input: CreateProjectInput = {
          code: formCode.trim(),
          description: description.trim(),
          productId,
          members,
          unblindMembers,
        };
        await create.mutateAsync(input);
      } else if (mode === "edit" && code) {
        const body: UpdateProjectBody = {
          description: description.trim(),
          productId,
          active,
          members,
          unblindMembers,
        };
        await update.mutateAsync({ code, body });
      }
      onClose();
    } catch {
      /* error surfaced below via create.error / update.error */
    }
  }

  const mutationError: ApiError | null =
    create.error ?? update.error ?? null;

  return (
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

        <TextField
          label={t("project.field.code")}
          value={formCode}
          onChange={(event) => setFormCode(event.target.value)}
          disabled={mode === "edit"}
          size="small"
          required
        />

        <TextField
          label={t("project.field.description")}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          multiline
          minRows={2}
          size="small"
          required
        />

        <Autocomplete
          options={products.data ?? []}
          getOptionLabel={(p: ProductView) => `${p.code} — ${p.name}`}
          value={
            products.data?.find((p) => p.id === productId) ?? null
          }
          onChange={(_e, value: ProductView | null) =>
            setProductId(value?.id ?? null)
          }
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.product")}
              size="small"
              required
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={memberLeaders}
          onChange={(_e, value) => setMemberLeaders(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.members.leaders")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={memberWorkers}
          onChange={(_e, value) => setMemberWorkers(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.members.workers")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={unblindLeaders}
          onChange={(_e, value) => setUnblindLeaders(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.unblindMembers.leaders")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={unblindWorkers}
          onChange={(_e, value) => setUnblindWorkers(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.unblindMembers.workers")}
              size="small"
            />
          )}
        />

        {mode === "edit" && (
          <FormControlLabel
            control={
              <Switch
                checked={active}
                onChange={(event) => setActive(event.target.checked)}
              />
            }
            label={t("project.field.active")}
          />
        )}

        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
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
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-drawer.test.tsx
```

Expected: PASS (all 7 tests green). If any Autocomplete interaction test fails, the most likely cause is that the option text doesn't match — confirm that the option's accessible name is `prod-a — Product A` (the `getOptionLabel` produces `prod-a — Product A`, but `@testing-library/user-event` matches against the rendered text). If a test fails on `screen.getByRole("option", { name: /prod-a/i })`, the engineer should use `{ name: /product a/i }` instead (case-insensitive). Apply that fix only to the failing test case.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx apps/desktop/aegis-desktop/src/test/pages/project-drawer.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add ProjectDrawer (right-ancheted create/edit form)

Reuses a single MUI Drawer for create + update. Edit mode seeds
the form via a manual-trigger get_project_by_code refetch
(lookedUp ref guards against StrictMode double-fire).

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Implement `project-list` orchestrator (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/project-list.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/project-list.test.tsx`

**Interface (consumed by Task 11):**
- Default export: `ProjectListPage` — top-level React component. No props.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/pages/project-list.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { ProjectListPage } from "../../pages/project-list";
import type { ProjectView, UserView } from "../../api";
import { mockCommands } from "../tauri-mock";
import { renderInRouter } from "../file-route-utils";

const projectA: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "alice", name: "Alice" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectB: ProjectView = {
  id: 2,
  code: "beta",
  description: "Beta project",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: { leaders: [], workers: [] },
  unblindMembers: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  active: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectC: ProjectView = {
  ...projectA,
  id: 3,
  code: "gamma",
  description: "Gamma project",
  members: { leaders: [{ code: "zoe", name: "Zoe" }], workers: [] },
  unblindMembers: { leaders: [], workers: [] },
};

const adminUser: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const generalUser: UserView = {
  ...adminUser,
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderPage(user: UserView, projects: ProjectView[]) {
  mockCommands({
    current_user: () => user,
    list_projects: () => projects,
  });
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ProjectListPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectListPage — basic rendering", () => {
  it("renders the heading and one row per project", async () => {
    await renderPage(adminUser, [projectA, projectB]);
    expect(
      await screen.findByRole("heading", { name: /projects/i, level: 4 }),
    ).toBeInTheDocument();
    expect(await screen.findByText("alpha")).toBeInTheDocument();
    expect(await screen.findByText("beta")).toBeInTheDocument();
  });
});

describe("ProjectListPage — search filter", () => {
  it("filters rows by code (case-insensitive)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "BET");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("beta")).toBeInTheDocument();
    });
  });

  it("filters rows by description", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "gamma project");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("gamma")).toBeInTheDocument();
    });
  });

  it("filters rows by leader code/name", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "zoe");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("gamma")).toBeInTheDocument();
    });
  });
});

describe("ProjectListPage — Involve filter", () => {
  it("shows only projects where the current user is in any members array when Involve is checked", async () => {
    // alice is leader of projectA, unblindLeader of projectB, but
    // not involved in projectC (Zoe leads projectC).
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.getByText("beta")).toBeInTheDocument();
      expect(screen.queryByText("gamma")).not.toBeInTheDocument();
    });
  });

  it("search AND Involve combine (commutative order)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "alpha");
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.queryByText("beta")).not.toBeInTheDocument();
      expect(screen.queryByText("gamma")).not.toBeInTheDocument();
    });
  });
});

describe("ProjectListPage — role gating", () => {
  it("shows the Add button for admin", async () => {
    await renderPage(adminUser, [projectA]);
    expect(
      await screen.findByRole("button", { name: /add project/i }),
    ).toBeInTheDocument();
  });

  it("hides the Add button for general users", async () => {
    await renderPage(generalUser, [projectA]);
    await screen.findByText("alpha");
    expect(
      screen.queryByRole("button", { name: /add project/i }),
    ).not.toBeInTheDocument();
  });

  it("opens the drawer with mode='create' when Add is clicked", async () => {
    await renderPage(adminUser, [projectA]);
    await userEvent.click(
      await screen.findByRole("button", { name: /add project/i }),
    );
    expect(
      await screen.findByRole("heading", { name: /create project/i }),
    ).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-list.test.tsx
```

Expected: FAIL with `Failed to resolve import "../../pages/project-list"`.

- [ ] **Step 3: Implement**

Create `apps/desktop/aegis-desktop/src/pages/project-list.tsx`:

```tsx
import { useMemo, useState } from "react";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListProjects } from "../data";
import type { ProjectView } from "../api";
import { ProjectDrawer } from "./ProjectDrawer";
import { ProjectFilterBar } from "./ProjectFilterBar";
import { ProjectTable } from "./ProjectTable";

interface DrawerState {
  mode: "closed" | "create" | "edit";
  code: string | null;
}

/**
 * Project list page. Owns the search / Involve filter state and the
 * drawer mode; passes filtered rows down to the table. Filters are
 * applied client-side as a single useMemo over the project list.
 */
export function ProjectListPage() {
  const { t } = useI18n();

  const projects = useListProjects();
  const currentUser = useCurrentUser();
  const currentCode = currentUser.data?.code ?? null;
  const role = currentUser.data?.role;
  const canEdit = role === "root" || role === "admin";

  const [query, setQuery] = useState("");
  const [involve, setInvolve] = useState(false);
  const [drawer, setDrawer] = useState<DrawerState>({
    mode: "closed",
    code: null,
  });

  const filteredRows = useMemo<ProjectView[]>(() => {
    const all = projects.data ?? [];
    const trimmed = query.trim();
    const q = trimmed.toLowerCase();
    return all.filter((row) => {
      // Search filter.
      if (q.length > 0) {
        const inCode = row.code.toLowerCase().includes(q);
        const inDescription = row.description.toLowerCase().includes(q);
        const inLeaders =
          leaderMatches(row.members.leaders, q) ||
          leaderMatches(row.unblindMembers.leaders, q);
        if (!inCode && !inDescription && !inLeaders) return false;
      }
      // Involve filter.
      if (involve && currentCode) {
        const inMembers =
          row.members.leaders.some((u) => u.code === currentCode) ||
          row.members.workers.some((u) => u.code === currentCode) ||
          row.unblindMembers.leaders.some((u) => u.code === currentCode) ||
          row.unblindMembers.workers.some((u) => u.code === currentCode);
        if (!inMembers) return false;
      }
      return true;
    });
  }, [projects.data, query, involve, currentCode]);

  return (
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
  );
}

function leaderMatches(
  leaders: { code: string; name: string }[],
  q: string,
): boolean {
  return leaders.some(
    (u) =>
      u.code.toLowerCase().includes(q) || u.name.toLowerCase().includes(q),
  );
}
```

Note: the `leaderMatches` helper takes the structural minimum `{ code: string; name: string }[]` so it accepts both `UserSummary[]` (the wire type on `ProjectView.members.leaders`) and `UserView[]` (the type from `useListUsers`) via covariance. Using the bare structural type avoids a TypeScript mismatch where `UserSummary` (which has only `code` + `name`) would not be assignable to a `UserView[]` parameter (which requires `id`, `role`, `active`, `createdAt`, `updatedAt`).

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/pages/project-list.test.tsx
```

Expected: PASS (9 tests green).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/project-list.tsx apps/desktop/aegis-desktop/src/test/pages/project-list.test.tsx
git commit -m "$(cat <<'EOF'
feat(desktop): add project-list page orchestrator

Owns search / Involve filter state and drawer mode. Filters via
a single useMemo over the project list (search by code /
description / leader code or name; Involve by current-user code
in any of the four member arrays). Delegates rendering to
ProjectFilterBar, ProjectTable, and ProjectDrawer.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Add the `/projects` route and the Sidebar entry

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_layout/projects.tsx`
- Modify: `apps/desktop/aegis-desktop/src/pages/layout.tsx`

**Why:** TanStack Router file routes are generated into `routeTree.gen.ts` from the `src/routes/` directory by `@tanstack/router-plugin` (already configured in `vite.config.ts`). Adding the route file + updating the Sidebar menu wires the page into the navigation surface.

- [ ] **Step 1: Create the route file**

Create `apps/desktop/aegis-desktop/src/routes/_layout/projects.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { ProjectListPage } from "../../pages/project-list";

export const Route = createFileRoute("/_layout/projects")({
  component: ProjectListPage,
});
```

- [ ] **Step 2: Update the Sidebar menu in `layout.tsx`**

Open `apps/desktop/aegis-desktop/src/pages/layout.tsx`. The current top of the file is:

```tsx
import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { UserFooter } from "./UserFooter";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsMenuIcon />;
```

Replace the icon import line and the two local icon-component lines with:

```tsx
import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import {
  Home as HomeIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { UserFooter } from "./UserFooter";

const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsMenuIcon />;
```

Then update the `menu` array in the `AppLayout` function. The current body of `AppLayout` starts:

```tsx
export function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];
```

Replace the `menu` array with:

```tsx
  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];
```

- [ ] **Step 3: Regenerate `routeTree.gen.ts`**

The `@tanstack/router-plugin` regenerates `routeTree.gen.ts` automatically when `vite dev` runs, but for the build pipeline it also runs as a Vite plugin during the typecheck build. To force regeneration without booting the dev server, run the typecheck — the plugin's `tanstack-router/generator` runs before `tsc`:

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: PASS. If the typecheck complains about `routeTree.gen.ts` not having `/_layout/projects` registered, run the dev server briefly (`pnpm --filter aegis-desktop dev`) and exit — the plugin writes the generated file on start. Or run the build: `pnpm --filter aegis-desktop build`.

If neither regenerates the file automatically, regenerate manually by running `pnpm exec tsr generate` (the CLI provided by `@tanstack/router-plugin`) from the `aegis-desktop` directory:

```bash
cd apps/desktop/aegis-desktop
pnpm exec tsr generate
```

Verify the regenerated file contains:

```ts
const LayoutProjectsRoute = LayoutProjectsRouteImport.update({
  id: '/projects',
  path: '/projects',
  getParentRoute: () => LayoutRouteRoute,
} as any)
```

and that `FileRoutesByFullPath` includes `/projects`.

- [ ] **Step 4: Type-check**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_layout/projects.tsx apps/desktop/aegis-desktop/src/pages/layout.tsx apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts
git commit -m "$(cat <<'EOF'
feat(desktop): add /projects route and Sidebar Projects entry

Route file at routes/_layout/projects.tsx wires ProjectListPage
into the auth-gated _layout. Sidebar gains a Projects entry
between Home and Settings, using the Workspaces icon.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Full router integration test for `/projects`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/routes/projects.test.tsx`

**Why:** The route file alone is not enough — verify that the Sidebar renders a Projects link, that clicking it navigates to `/projects`, that the ProjectListPage renders under the layout, and that the layout's `beforeLoad` redirect still works when not logged in.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/routes/projects.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { mockCommands, mockInvoke } from "../tauri-mock";
import { TestQueryProvider } from "../test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderRoot(initialEntries: string[] = ["/projects"]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("/projects — authenticated", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_projects: () => [],
    });
  });

  it("renders the Sidebar and the Project page heading at /projects", async () => {
    const { router } = await renderRoot(["/projects"]);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { name: /projects/i, level: 4 }),
    ).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/projects");
  });

  it("renders a Projects link in the Sidebar", async () => {
    await renderRoot(["/"]);
    expect(
      await screen.findByRole("link", { name: /projects/i }),
    ).toBeInTheDocument();
  });

  it("navigates from /settings to /projects when Projects is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);
    await userEvent.click(
      await screen.findByRole("link", { name: /projects/i }),
    );
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/projects"),
    );
  });
});

describe("/projects — unauthenticated", () => {
  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/projects"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run — expect failure**

```bash
pnpm --filter aegis-desktop test -- src/test/routes/projects.test.tsx
```

Expected: FAIL because either the route doesn't exist (404 / no match for `/projects`) or the page doesn't render. The test fails at `screen.getByTestId("sidebar")` or at `screen.findByRole("heading", { name: /projects/i, level: 4 })`.

- [ ] **Step 3: Confirm Task 11 has been completed**

If Task 11's commit hasn't been applied yet, the route file doesn't exist — Task 12 cannot pass until Task 11 is done. Re-run Task 11's verification (`pnpm --filter aegis-desktop typecheck`) before continuing.

- [ ] **Step 4: Re-run — expect pass**

```bash
pnpm --filter aegis-desktop test -- src/test/routes/projects.test.tsx
```

Expected: PASS (4 tests green).

- [ ] **Step 5: Run the full desktop test suite**

```bash
pnpm --filter aegis-desktop test
```

Expected: ALL desktop tests pass — the original suite (login, register, bootstrap, settings, etc.) plus all 12 new test files from this plan.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/routes/projects.test.tsx
git commit -m "$(cat <<'EOF'
test(desktop): add full-router integration tests for /projects

Verifies the route is registered, the Sidebar Projects link
navigates correctly, the page renders under the auth-gated
layout, and unauthenticated visitors are redirected to /login.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Final verification

After all 12 tasks complete, run the full desktop + UI package checks:

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

All four commands must exit 0. If any test fails or type error appears, return to the relevant task and fix inline before considering the feature complete.

## File changes summary

**New files**
- `lib/packages/ui/src/i18n/locales/en.ts` (modified)
- `lib/packages/ui/src/i18n/locales/zhCN.ts` (modified)
- `apps/desktop/aegis-desktop/src/data/queryKeys.ts` (modified)
- `apps/desktop/aegis-desktop/src/data/project.ts`
- `apps/desktop/aegis-desktop/src/data/product.ts`
- `apps/desktop/aegis-desktop/src/data/user.ts` (modified — added `useListUsers`)
- `apps/desktop/aegis-desktop/src/data/index.ts` (modified)
- `apps/desktop/aegis-desktop/src/pages/ProjectFilterBar.tsx`
- `apps/desktop/aegis-desktop/src/pages/ProjectTable.tsx`
- `apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx`
- `apps/desktop/aegis-desktop/src/pages/project-list.tsx`
- `apps/desktop/aegis-desktop/src/pages/layout.tsx` (modified)
- `apps/desktop/aegis-desktop/src/routes/_layout/projects.tsx`
- `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (regenerated)
- `apps/desktop/aegis-desktop/src/test/data/project.test.tsx`
- `apps/desktop/aegis-desktop/src/test/data/product.test.tsx`
- `apps/desktop/aegis-desktop/src/test/data/user.test.tsx` (modified — appended `useListUsers` cases)
- `apps/desktop/aegis-desktop/src/test/pages/project-filter-bar.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-table.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-drawer.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/project-list.test.tsx`
- `apps/desktop/aegis-desktop/src/test/routes/projects.test.tsx`

**Untouched (per the spec)**
- `apps/desktop/aegis-desktop/src/api/**` — pure Tauri transport.
- `apps/desktop/aegis-desktop/src/components/**`
- `apps/desktop/aegis-desktop/src/routes/__root.tsx`, `_layout/route.tsx`, `_layout/index.tsx`, `_layout/settings.tsx`
- `apps/desktop/aegis-desktop/src/pages/{home,settings,UserFooter,bootstrap}.tsx`
- `apps/desktop/aegis-desktop/src/main.tsx`
- All existing tests, vitest config, package.json.

## Out of scope (deferred)

Per the spec — these features are NOT part of this plan:

- The OpenInNew action (rendered as `disabled`; future "go to project detail" is a separate feature).
- Server-side search / pagination (the project list is small enough for client-side filter).
- Worker column in the table — only leaders render chips, per spec.
- Optimistic updates on create / update.
- Sorting, pagination, column resizing.
- Form-level validation rules beyond required-field enforcement.