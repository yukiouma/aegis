# Frontend Review — `apps/desktop/aegis-desktop/src`

A tour of the codebase and concrete suggestions, organized by impact rather than
file order. The good news first: this is a clean, well-disciplined codebase with
very few red flags. The points below are mostly polish + scaling advice.

## Snapshot of what you've built

- **Routing & layout**: file-based routes via TanStack Router, two top-level
  layouts (`/_layout`, `/project/$projectCode`) each guarding auth via
  `beforeLoad`.
- **Data**: thin Tauri `invoke` wrapper (`api/index.ts`) → typed DTOs
  (`api/types.ts`) → React Query hooks (`data/*.ts`), all centralized through a
  barrel (`data/index.ts`) and a query-key factory (`data/queryKeys.ts`).
- **Cross-window state**: a small "settings sync" pattern
  (`SettingsSyncBridge.tsx` + `data/settings.ts`) using
  `@tauri-apps/plugin-store` + `aegis:settings-changed` events.
- **UI**: shared `@aegis/ui` package, MUI components, i18n via `useI18n`,
  role-gated menus, append-only `BootstrapLog` for splash flows.
- **Tests**: vitest + jsdom + Testing Library, with a `tauri-mock.ts` that
  dispatches by command name, and `renderInRouter` / `renderWithFullRouter`
  helpers. Coverage is solid for data hooks and pages.

## High-impact improvements

### 1. Centralize the duplicated auth guard

**Files**: [`src/routes/_layout/route.tsx`](apps/desktop/aegis-desktop/src/routes/_layout/route.tsx),
[`src/routes/project/$projectCode/route.tsx`](apps/desktop/aegis-desktop/src/routes/project/$projectCode/route.tsx)

The same `isLoggedIn` probe + `redirect({ to: "/login" })` block is copy-pasted
in both layouts (and the duplication is explicitly called out in a comment).
Two clean options:

- Extract a `requireAuth()` helper in `src/routes/_auth.ts` and call it from
  each layout's `beforeLoad`. Smallest change, removes the maintenance hazard —
  when the auth contract changes, you change one file.
- Better long-term: introduce a shared **pathless** layout
  (`src/routes/_authed/route.tsx`) and nest both `_layout` and
  `project/$projectCode` under it. TanStack Router supports nested pathless
  layouts natively. The guard then lives once and the URL shape doesn't need to
  change.

Either way, drop the "duplicated rather than factored out so each layout owns
its own guard" comment — that's a tax you're paying forever for no upside.

### 2. `toApiError` typing is too permissive

**File**: [`src/api/error.ts`](apps/desktop/aegis-desktop/src/api/error.ts)

```ts
if (
  typeof e === "object" &&
  e !== null &&
  "kind" in e &&
  typeof (e as { kind: unknown }).kind === "string"
) {
  return e as ApiError;
}
```

This is a runtime cast on a structurally-narrowed value: it accepts any object
with a string `kind` and trusts the rest of the shape. If the Rust side adds a
field, the TS side silently accepts a partial shape. Tighten with a real
discriminator:

```ts
function isApiError(e: unknown): e is ApiError {
  if (typeof e !== "object" || e === null) return false;
  const k = (e as { kind?: unknown }).kind;
  switch (k) {
    case "network":
    case "refreshFailed":
    case "notImplemented":
    case "store":
    case "http":
      return true;
    default:
      return false;
  }
}
```

This makes the type-narrowing actually earn its keep, and `errorMessage`'s
switch is then exhaustive without needing a fallthrough.

### 3. Wire DTO naming asymmetry

**File**: [`src/api/types.ts`](apps/desktop/aegis-desktop/src/api/types.ts)

`types.ts` opens with this comment:

> TypeScript interfaces use camelCase identifiers. Note that this is purely a
> TS-style rename — the actual JSON keys received from the aegis-server are
> snake_case … Future consumers that destructure these shapes need to know they
> must use the snake_case keys at runtime.

That's a foot-gun. If any consumer does `const { host_machine } = identity`,
they'll silently get `undefined`. Two clean fixes:

- Add a transform layer at the Tauri boundary (e.g. a generic `camelizeKeys`
  applied in the `call` helper). The DTO comment then becomes accurate.
- Or, for the cases where Rust already emits camelCase (`Identity`), split the
  type so the discrepancy is impossible to miss:

  ```ts
  // Real camelCase wire shape from Rust serde(rename_all = "camelCase").
  export interface Identity { hostMachine: string; ... }
  // Real snake_case wire shape from the server's JSON contract.
  export interface ProjectView { members: ProjectMembersView; ... }
  ```

  Pair each with a runtime test that decodes a fixture and asserts the keys
  actually present.

Whichever you pick, the comment-as-API-contract approach won't scale; a typo
in a property name won't break the build.

### 4. The four-`Autocomplete` repetition in `ProjectDrawer`

**File**: [`src/pages/ProjectDrawer.tsx`](apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx)

`memberLeaders`, `memberWorkers`, `unblindLeaders`, `unblindWorkers` are four
near-identical state slots, four near-identical `Autocomplete<UserSummary, true>`
blocks, and four near-identical `members: { leaders, workers }` builders on
submit. Two quick wins:

- Drive the four pickers off a small data structure:

  ```ts
  const slots = [
    { key: "leaders", scope: "members",     label: t("...leaders"), state: [memberLeaders, setMemberLeaders] },
    { key: "workers", scope: "members",     label: t("...workers"), state: [memberWorkers, setMemberWorkers] },
    // ...
  ] as const;
  ```

  and `.map()` the JSX. Cuts ~50 lines and makes the "members vs
  unblindMembers" symmetry explicit.
- The submit-side `{ leaders: a.map(u => u.code), workers: b.map(u => u.code) }`
  is a candidate for a `UserSummary[] → string[]` helper used in both places.

The state can also be a single `Record<SlotKey, UserSummary[]>` once the slot
config is data-driven; that gives you one `setSlot(key, value)` instead of four
setters.

### 5. The "StrictMode guard" pattern

**Files**: [`src/pages/Bootstrap.tsx`](apps/desktop/aegis-desktop/src/pages/Bootstrap.tsx),
[`src/pages/Register.tsx`](apps/desktop/aegis-desktop/src/pages/Register.tsx),
[`src/pages/ProjectDrawer.tsx`](apps/desktop/aegis-desktop/src/pages/ProjectDrawer.tsx)

Three components hand-roll the same `useRef(false)` + early-return pattern to
keep an `async` orchestrator single-fire:

```ts
const started = useRef(false);
useEffect(() => {
  if (started.current) return;
  started.current = true;
  void (async () => { ... })();
}, [...]);
```

That's not actually fixing the root cause — React18 StrictMode in dev
intentionally double-invokes effects, but the cleanup is what's used to "tear
down" the double-fire. The cleaner pattern is to either:

- Return a cleanup function that cancels the in-flight work (you already do
  `cancelled` in `data/settings.ts` — use the same idea here), or
- Recognize the ref-guard as a code smell and extract the orchestrator into a
  tiny custom hook that owns the lifecycle once:

  ```ts
  function useOnce(fn: () => Promise<void>) {
    const ref = useRef(false);
    useEffect(() => {
      if (ref.current) return;
      ref.current = true;
      void fn();
    }, [fn]);
  }
  ```

  Then the three call sites become `useOnce(async () => { ... })` and the
  pattern is documented in one place.

### 6. `useProject(code: string | null)` is awkward

**File**: [`src/data/project.ts`](apps/desktop/aegis-desktop/src/data/project.ts)

The hook handles "disabled" by composing a synthetic query key, throwing
inside the `queryFn`, and hard-coding `enabled: false`:

```ts
queryKey: code === null
  ? ["project", "byCode", "__disabled__"]
  : queryKeys.project.byCode(code),
queryFn: () => {
  if (code === null) throw new Error("useProject disabled");
  return api.getProjectByCode(code);
},
enabled: false,
```

But `enabled: false` already prevents `queryFn` from being called — the key
guard and throw are dead defenses. Simplify to:

```ts
export function useProject(code: string | null) {
  return useQuery<ProjectView, ApiError>({
    queryKey: queryKeys.project.byCode(code ?? "__disabled__"),
    queryFn: () => api.getProjectByCode(code!),
    enabled: code !== null,
    staleTime: 0,
  });
}
```

Same behavior, smaller surface, fewer lies in the comments.

### 7. Window-focus helper is duplicated

**Files**: [`src/data/user.ts`](apps/desktop/aegis-desktop/src/data/user.ts),
[`src/pages/ProjectWorkspaceLayout.tsx`](apps/desktop/aegis-desktop/src/pages/ProjectWorkspaceLayout.tsx)

Both files reach into `@tauri-apps/api/webviewWindow` and iterate
`getAllWebviewWindows()` to find a window by label (`project:*` / `main`).
Promote a `src/lib/windows.ts` (or extend `api/index.ts`) with:

```ts
focusWindowByLabel(label: string): Promise<void>
closeProjectWindows(): Promise<void>
```

Then `useLogout`'s body becomes `await closeProjectWindows()` and
`ProjectWorkspaceLayout`'s `focusMainWindow` becomes
`await focusWindowByLabel("main")`. The pattern is almost certainly going to
grow — easier to centralize now.

### 8. `UserFooter` leaks `Role` literals via casts

**File**: [`src/pages/UserFooter.tsx`](apps/desktop/aegis-desktop/src/pages/UserFooter.tsx)

```ts
const showRoleChip =
  user?.role === ("root" as Role) || user?.role === ("admin" as Role);
```

`"root" as Role` is a no-op cast that hides a deeper problem: `Role` is a union
of three string literals, so the comparison should be straightforward. If the
cast is there because `user.role` is typed wider than `Role`, the right fix is
to tighten the type (probably `useCurrentUser`'s `data?.role` should already be
`Role | undefined`). Same with the chained ternary in `roleLabel` — a
`switch (user.role)` or a small lookup map reads better.

### 9. `LoginPage.runLogin` mixes presentation and control flow

**File**: [`src/pages/Login.tsx`](apps/desktop/aegis-desktop/src/pages/Login.tsx)

The function builds translation logs, navigates, sets an outcome state, and
dispatches a mutation. The branching on `httpCode` is the only domain logic;
the rest is UI plumbing. Two cleanups:

- Extract the "compute outcome from error" step into `src/api/error.ts` as
  `classifyLoginError(e): Outcome` so the policy lives next to the error model
  and is unit-testable without a render.
- The current `runLogin` then becomes a 3-step pipeline (push log → run
  mutation → on success push+nav, on failure classify+push+setOutcome). Easier
  to reason about and trivially testable.

### 10. Test ergonomics: a missing `--coverage` and two helper overlaps

- No coverage script in `package.json`. Consider adding
  `"test:cov": "vitest run --coverage"` plus `@vitest/coverage-v8` in devDeps
  so PR review can catch untested branches. The data-layer tests are thorough;
  page-level tests are uneven.
- `renderWithQueryClient`
  ([`test/render-with-query-client.tsx`](apps/desktop/aegis-desktop/src/test/render-with-query-client.tsx))
  and `TestQueryProvider`
  ([`test/test-query-provider.tsx`](apps/desktop/aegis-desktop/src/test/test-query-provider.tsx))
  both exist and do the same thing. Several tests use one, several use the
  other, and there's a comment justifying the duplication. Pick one (the
  `TestQueryProvider` is simpler and has the cleaner name) and delete the other
  — or repurpose one as the "test wrapper" and the other as the "test factory"
  with non-overlapping jobs.

### 11. Minor but worth doing

- **`Settings.tsx:24`** — `setLocale(event.target.value as Locale)` casts an
  arbitrary string to `Locale`. If the `<Select>` ever gets a third locale, this
  silently accepts it. Tighten with a runtime guard or a discriminated
  `MenuItem` typed list:

  ```ts
  const LOCALES = ["en", "zh-CN"] as const;
  type Locale = typeof LOCALES[number];
  ```

- **`Register.tsx:39-46`** — when `identity.refetch()` fails, the page silently
  returns without surfacing the failure on the UI (only the log). Either
  disable the form with a banner, or push the error into state so the button
  can react. Today, if the domain lookup fails, the user sees a spinner that
  becomes an empty form.

- **`ProjectList.tsx` filter** — the `q.length > 0` check is fine, but
  `leaderMatches` calls `toLowerCase()` on every leader's code/name on every
  keystroke. For a typical project list this is irrelevant, but if the dataset
  grows, a precomputed `lowerCode`/`lowerName` per row in the same memo would
  be O(n) once instead of O(n·m) per filter pass.

- **`Layout.tsx`** — the `useState(sidebarOpen)` defaults to `true`. If you'd
  like the sidebar to remember its state per user, lift it to the
  `SettingsSyncBridge` pattern you already have (the persistence pipeline is
  reusable for any user pref).

- **`routes/_layout/route.tsx`** — the `api.isLoggedIn()` call on every
  navigation can be cached at the QueryClient level (the existing
  `useIsLoggedIn` hook exists for this). Routing through the cache means a
  navigation to `/projects` from `/settings` doesn't re-probe Tauri. Bonus: it
  gives you a single place to invalidate on logout.

- **Type-only import consistency** — `api/index.ts` does
  `export type { ApiError } from "./types"` but other files use
  `import type { ... }`. Minor, but lint-clean would be `export type`
  everywhere (or nowhere) — the codebase is mixed today.

## Things to keep doing (these are good)

- **Centralized `queryKeys` factory**. The "every hook goes through it" rule is
  enforced by comments and not by a lint rule; consider adding an ESLint rule
  (or a `no-restricted-syntax`) banning inline `["project", ...]` arrays. You've
  already taken the pain once with `useProject`'s synthetic key; a lint rule
  prevents the next one.
- **The `errorMessage`/`httpCode`/`toApiError` trio** in `api/error.ts` is
  exactly the right shape — single source of truth, narrow-once, no `unknown`
  leaking into pages.
- **`SettingsSyncBridge`** with the on-disk store + event bus is a clean pattern
  for cross-window prefs. Reusing it for other UI prefs (sidebar open/closed,
  table density, etc.) is essentially free.
- **The comment density** in this codebase is unusual but a net positive — the
  rationale for every non-obvious choice (StrictMode guards, manual
  `refetch()` triggers, logout-before-cache-clear ordering, etc.) is right next
  to the code. Keep it up.

## Recommended order of attack

1. Centralize the auth guard (#1) — 15 minutes, removes the duplicated-block
   foot-gun.
2. Tighten `toApiError` and decide on the wire-naming story (#2, #3) — together
   ~1 hour, prevents silent data-shape bugs.
3. Extract the `useOnce` helper (#5) — 30 minutes, then delete the three
   `useRef(false)` blocks.
4. Refactor the four-pickers into a data-driven slot config (#4) — 1 hour,
   pure cleanup.
5. Promote the window helpers (#7) — 30 minutes, future-proofs.
6. Test ergonomics (#10) — 45 minutes.

The other items are smaller fixes you can pick up opportunistically.

## Codebase organization

The current organization is mostly sound — this section audits the file layout
itself, separate from the code-quality points above.

### What's already working

- **`api/` is a clean seam.** `index.ts` = transport, `error.ts` = error
  narrowing, `types.ts` = wire DTOs. Pages never reach past `data/`; data hooks
  never reach past `api/`. That's the right kind of layering.
- **Data hooks are co-located by resource** (`user.ts`, `project.ts`,
  `product.ts`) with a **single barrel** (`data/index.ts`). Good "one way in"
  pattern.
- **`pages/` vs `routes/` split** is justified and documented (see comment in
  [`Layout.tsx`](../../apps/desktop/aegis-desktop/src/pages/Layout.tsx)).
  Components are testable in isolation; route files are thin glue.
- **`@aegis/ui` is the design-system boundary.** Pages don't import MUI
  primitives directly — they import `@aegis/ui/mui` and `@aegis/ui/icons`. The
  shell can be swapped without touching business code.
- **Test infrastructure is centralized in `test/`** (mock dispatch, render
  helpers, query provider). Tests for pages/components live next to where
  production code lives.

### Things worth tightening

#### 1. The "everything in `src/data/`" bag

`data/` currently mixes three different concerns and it's worth naming them:

| Current name | Concern | Better location |
|---|---|---|
| `auth.ts`, `bootstrap.ts`, `user.ts`, `project.ts`, `product.ts` | Server cache hooks | `src/data/` (keep) |
| `queryKeys.ts`, `client.tsx` | React Query infra | `src/data/query/` |
| `settings.ts` | Persistent cross-window prefs (store + events) | `src/prefs/` or `src/data/prefs/` |
| `index.ts` | Public barrel | stays |

`s/settings.ts/prefs/` is the most useful split because `settings.ts` isn't
really a "server cache" hook — it's a Tauri-store + event-bus adapter with two
React-facing hooks bolted on. Mixing it with `useListProjects` confuses the
layering.

#### 2. `components/` has only one folder

[`components/BootstrapLog/`](../../apps/desktop/aegis-desktop/src/components/BootstrapLog/)
is the only thing in `components/`. Two reasonable interpretations:

- **It's a placeholder**: more shared components will land here (e.g. a
  `<RoleChip>`, a `<EmptyState>`) — in which case the folder name is fine,
  just keep it as the "shared, non-page" bucket and don't let `pages/` grow
  subcomponents.
- **It's mis-named**: the BootstrapLog is really a **feature** used by
  Bootstrap/Login/Register pages, not a generic shared component. Consider
  moving it under `src/features/auth/` (or `src/features/splash/`) and rename
  `components/` to whatever the next shared widget actually is.

Either is defensible, but right now `components/` looks like a "scrap drawer"
because it has one tenant.

#### 3. Root-level `.tsx` files are a mixed bag

[`main.tsx`](../../apps/desktop/aegis-desktop/src/main.tsx),
[`bootstrap-redirect.ts`](../../apps/desktop/aegis-desktop/src/bootstrap-redirect.ts),
[`DocumentLangSync.tsx`](../../apps/desktop/aegis-desktop/src/DocumentLangSync.tsx),
[`SettingsSyncBridge.tsx`](../../apps/desktop/aegis-desktop/src/SettingsSyncBridge.tsx)
all live in `src/` directly. None of them are pages, none are data hooks, none
are components. They're **app shell glue**.

Two clean options:

- **Collect into a single `src/app/` folder**: `app/main.tsx`,
  `app/Providers.tsx` (the chain in `App()`), `app/DocumentLangSync.tsx`,
  `app/SettingsSyncBridge.tsx`, `app/bootstrap-redirect.ts`. Once the four
  files exist in `app/`, the next shell concern has an obvious home.
- **Keep them flat but co-locate the providers**: move the four providers from
  `main.tsx` into a single `Providers.tsx` so `main.tsx` is just routing +
  render. The current `App()` function with its nested provider wrappers is
  doing too much.

Pick the first — a dedicated `app/` folder signals "this is the entry point
and its plumbing, don't grow here casually."

#### 4. `vite-env.d.ts` belongs at the project root

`src/vite-env.d.ts` should be at the repo root (or in `src/types/`). Vite
specifically looks for `vite-env.d.ts` next to `vite.config.ts`. It's working
where it is, but it's unusual.

#### 5. Barrel files are a tradeoff you're already paying for

[`data/index.ts`](../../apps/desktop/aegis-desktop/src/data/index.ts) is a
re-export hub, and [`api/index.ts`](../../apps/desktop/aegis-desktop/src/api/index.ts)
re-exports 18 types from `types.ts`. Two specific hazards worth knowing:

- **Re-exported types lose their origin in error stacks** — when something
  blows up in a page, the stack points at the barrel, not the file with the
  actual type. Not a bug, just an annoyance.
- **The barrel becomes a "god file"** the moment you start re-exporting more
  than 30 things. Yours is at the edge. If `data/` grows another resource
  (e.g. `permission.ts`, `audit.ts`, etc., on top of the existing
  `product.ts`), consider **per-resource sub-barrels** (`data/auth/index.ts`)
  so each resource is self-contained.

You can also drop the `export type { ... } from "./types"` block in
`api/index.ts` if pages import directly from `api/types.ts` — the re-export is
convenience, not necessity.

#### 6. Tests live in `src/test/` — keep the centralized root, enforce the mirror

This is a deliberate choice (see `src/test/data/` mirroring `src/data/`). Pros:
one place to find all tests and test utilities. Cons: when you move a source
file you have to remember to move its test too, and the parallelism is implicit
not enforced.

Since the project is adopting a feature-sliced layout, **the mirror gets
deeper, not flatter**. The end state is `src/test/features/<x>/...` mirroring
`src/features/<x>/...`, plus top-level utilities under `src/test/`. Two
practices that keep the mirror honest as the codebase grows:

- **An eslint rule** requiring `src/test/<mirror-path>.test.{ts,tsx}` to exist
  for every non-trivial source file under `src/`. Cheap to write, catches
  orphans immediately.
- **A naming convention** for test files: snake-case (e.g.
  `project-list.test.tsx`) rather than camelCase (`ProjectList.test.tsx`).
  Keeps directory listings tidy and aligns with what most of the existing
  tests in this project already do.

Co-location is rejected on purpose for this project — the centralized test
root is the team's chosen convention, and the reorganization respects it
rather than fighting it.

### Reorganization: Option B (feature-sliced, adopted)

End-state layout (rendered via `tree -a -I 'node_modules'`):

```
src/
├── main.tsx                               # Vite entry; referenced by index.html
├── app/
│   ├── Providers.tsx
│   ├── SettingsSyncBridge.tsx
│   ├── DocumentLangSync.tsx
│   └── bootstrap-redirect.ts
├── features/
│   ├── auth/
│   │   ├── pages/
│   │   │   ├── Login.tsx
│   │   │   ├── Register.tsx
│   │   │   ├── Bootstrap.tsx
│   │   │   └── Home.tsx                  # post-login landing
│   │   ├── components/
│   │   │   └── BootstrapLog/
│   │   │       ├── BootstrapLog.tsx
│   │   │       ├── useBootstrapLog.ts
│   │   │       ├── types.ts
│   │   │       └── index.ts
│   │   └── data/
│   │       ├── auth.ts
│   │       ├── bootstrap.ts
│   │       ├── user.ts                   # identity lookup + register + logout
│   │       └── index.ts                  # barrel
│   ├── projects/
│   │   ├── pages/
│   │   │   ├── ProjectList.tsx
│   │   │   ├── ProjectTable.tsx
│   │   │   ├── ProjectDrawer.tsx
│   │   │   └── ProjectFilterBar.tsx
│   │   └── data/
│   │       ├── project.ts
│   │       ├── product.ts
│   │       └── index.ts
│   ├── users/
│   │   ├── pages/
│   │   │   ├── UserList.tsx
│   │   │   ├── UserTable.tsx
│   │   │   ├── UserFilterBar.tsx
│   │   │   └── UserFooter.tsx
│   │   └── data/
│   │       ├── user.ts                   # list + update (split from auth)
│   │       └── index.ts
│   ├── settings/
│   │   └── pages/
│   │       └── Settings.tsx
│   └── workspace/
│       ├── pages/
│       │   ├── ProjectWorkspaceLayout.tsx
│       │   ├── ProjectDashboard.tsx
│       │   └── ProjectConfiguration.tsx
│       └── data/                          # reserved for workspace-only hooks
├── routes/                                # TanStack Router file-based routes
│   ├── __root.tsx
│   ├── routeTree.gen.ts                   # generated by @tanstack/router-plugin
│   ├── bootstrap.tsx                      # → features/auth/pages/Bootstrap
│   ├── login.tsx                          # → features/auth/pages/Login
│   ├── register.tsx                       # → features/auth/pages/Register
│   ├── _layout/
│   │   ├── route.tsx                      # authenticated shell + AppLayout
│   │   ├── index.tsx                      # → features/auth/pages/Home
│   │   ├── projects.tsx                   # → features/projects/pages/ProjectList
│   │   ├── settings.tsx                   # → features/settings/pages/Settings
│   │   └── users.tsx                      # → features/users/pages/UserList
│   └── project/
│       └── $projectCode/
│           ├── route.tsx                  # workspace shell
│           ├── index.tsx                  # redirect → /dashboard
│           ├── dashboard.tsx              # → features/workspace/pages/ProjectDashboard
│           └── configuration.tsx          # → features/workspace/pages/ProjectConfiguration
└── shared/
    ├── api/
    │   ├── index.ts
    │   ├── error.ts
    │   └── types.ts
    ├── prefs/
    │   ├── settings.ts
    │   └── windows.ts                    # new home for getAllWebviewWindows helpers
```

`vite-env.d.ts` lives at the project root (next to `vite.config.ts`), not under
`src/`.

Test directory (kept under `src/test/` per the centralized-tests constraint):

```
src/test/
├── setup.ts                                # vitest setup, jsdom polyfills
├── tauri-mock.ts                           # mockCommands / mockInvoke
├── render-with-query-client.tsx            # render helper (with client)
├── file-route-utils.tsx                    # renderInRouter / renderWithFullRouter
├── test-query-provider.tsx                 # TestQueryProvider
├── api/                                    # mirrors src/shared/api/
│   ├── api.test.ts
│   ├── error.test.ts
│   └── open-project-workspace.test.ts
├── app/                                    # mirrors src/app/
│   ├── bootstrap-redirect.test.ts
│   └── document-lang-sync.test.tsx
├── shared/
│   └── prefs/
│       ├── settings.test.tsx
│       └── windows.test.ts
├── features/                               # mirrors src/features/
│   ├── auth/
│   │   ├── data/
│   │   │   ├── auth.test.tsx
│   │   │   ├── bootstrap.test.tsx
│   │   │   └── user.test.tsx
│   │   ├── pages/
│   │   │   ├── Login.test.tsx
│   │   │   ├── Register.test.tsx
│   │   │   └── Bootstrap.test.tsx
│   │   └── components/
│   │       └── bootstrap-log.test.tsx
│   ├── projects/
│   │   ├── data/
│   │   │   ├── project.test.tsx
│   │   │   └── product.test.tsx
│   │   └── pages/
│   │       ├── project-list.test.tsx
│   │       ├── project-table.test.tsx
│   │       ├── project-drawer.test.tsx
│   │       └── project-filter-bar.test.tsx
│   ├── users/
│   │   ├── data/
│   │   │   └── user.test.tsx
│   │   └── pages/
│   │       ├── user-list.test.tsx
│   │       ├── user-table.test.tsx
│   │       ├── user-filter-bar.test.tsx
│   │       └── user-footer.test.tsx
│   ├── settings/
│   │   └── pages/
│   │       └── Settings.test.tsx
│   └── workspace/
│       ├── pages/
│       │   ├── project-workspace-layout.test.tsx
│       │   ├── ProjectDashboard.test.tsx
│       │   └── ProjectConfiguration.test.tsx
│       └── routes/
│           └── project-workspace.test.tsx
└── routes/                                 # mirrors src/routes/
    ├── _layout.test.tsx
    ├── bootstrap.test.tsx
    ├── index.test.tsx
    ├── login.test.tsx
    ├── register.test.tsx
    ├── projects.test.tsx
    └── settings.test.tsx
```

Three constraints shape this layout:

- **`src/main.tsx` stays at the root** of `src/` because it's the Vite entry
  point referenced from `index.html`. Moving it into `src/app/` would break
  the dev server / production build references.
- **`src/routes/` stays as the single TanStack Router source** because the
  router plugin scans one directory (`routesDirectory: 'src/routes'` in
  `vite.config.ts`). Route files remain thin — each one imports its page
  component from the corresponding `features/<x>/pages/` and re-exports it as
  the route's `component`. Features own the UI and data; `routes/` owns the
  URL shape.
- **`src/test/` stays as the single test root** for both feature tests and
  shared utilities. The folder mirrors `src/` 1:1 (`src/test/features/auth/`
  tests `src/features/auth/`, `src/test/shared/prefs/` tests
  `src/shared/prefs/`, etc.), and the top-level utilities (`tauri-mock.ts`,
  `setup.ts`, etc.) stay flat under `src/test/`. Co-location is rejected in
  favor of the centralized, discoverable test root.

### Why this layout

- **Vertical slices mean a feature can be deleted without touching the rest of
  the tree.** When `features/users/` gets retired, nothing else in the tree
  references it.
- **`routes/` owns the URL shape, features own the UI and data.** Route files
  become three-line shims (`import { ProjectListPage } from
  "@/features/projects/pages/ProjectList"; export const Route =
  createFileRoute("/_layout/projects")({ component: ProjectListPage });`).
  This is the only seam where URL structure meets feature code.
- **`src/main.tsx` stays at the root of `src/`** as the Vite entry point; the
  provider chain moves into `app/Providers.tsx` and is rendered from
  `main.tsx`. The wiring is one component, the orchestration is one file —
  each easy to find.
- **Tests live under `src/test/`, mirroring `src/` 1:1.** Feature tests land in
  `src/test/features/<x>/`, glue tests in `src/test/app/`, shared-utility tests
  in `src/test/shared/`. The mirror is a convention enforced by code review
  (or, ideally, an eslint rule that requires
  `src/test/<mirror-path>.test.{ts,tsx}` for every non-trivial source file).
  Tradeoff: when you move a source file, you have to move its test too. The
  payoff: one place to find every test, and the test directory stays out of
  the feature folders.
- **`shared/prefs/` becomes the obvious home** for the cross-window settings
  pattern AND the window-management helpers — both are app-glue concerns, not
  server cache.
- **Path aliases (`@/features/...`, `@/shared/...`)** replace the
  `../../data/...` chain. One-time `tsconfig` + `vite.config.ts` setup,
  removes a class of bugs from refactors.

### Trade-offs to acknowledge

- **Per-feature sub-barrels grow.** Each `features/<x>/data/index.ts` is
  small but multiplies by the number of features. Worth it because the
  alternative (one giant `data/index.ts`) is the problem you have today.
- **Cross-feature imports need an explicit boundary.** If `features/projects`
  needs `features/users/data`, that's a code-review moment — usually a sign
  the dependency should move to `shared/`.
- **`shared/prefs/windows.ts` is new** and should land alongside the
  reorganization rather than as a separate PR, so the move commits read
  cleanly in history.

### Migration order

1. **Add `tsconfig` path alias `@/` → `src/`** and the matching Vite resolve
   alias. No source moves yet — just make the new addresses work.
2. **Create `app/` and `shared/prefs/`**; move `bootstrap-redirect.ts`,
   `DocumentLangSync.tsx`, `SettingsSyncBridge.tsx` into `app/`. Move
   `data/settings.ts` to `shared/prefs/settings.ts`. Update imports.
3. **Extract `Providers.tsx`** in `app/` from `main.tsx`'s `App()` component,
   so `main.tsx` shrinks to: import router, import `Providers`, render.
   `main.tsx` itself stays at `src/` root.
4. **Split `data/user.ts`** into `features/auth/data/user.ts` (identity +
   register + logout) and `features/users/data/user.ts` (list + update). The
   `useCurrentUser` hook goes into `features/auth/data/` since the sidebar
   footer lives in `features/users/pages/`.
5. **Create feature folders** one feature at a time, in this order:
   `auth → users → projects → workspace → settings`. For each feature:
   - Move the page components into `features/<x>/pages/`.
   - Move the data hooks into `features/<x>/data/`.
   - **Update the route file in `src/routes/`** to import the page from its
     new home. The route file itself does NOT move — it stays in
     `src/routes/` per the TanStack Router constraint. After the rewrite, a
     route file looks like:

     ```ts
     import { createFileRoute } from "@tanstack/react-router";
     import { ProjectListPage } from "@/features/projects/pages/ProjectList";

     export const Route = createFileRoute("/_layout/projects")({
       component: ProjectListPage,
     });
     ```

   - **Move the feature's tests into `src/test/features/<x>/`** — keep the
     existing mirror convention. For example, `src/test/pages/ProjectList.test.tsx`
     becomes `src/test/features/projects/pages/project-list.test.tsx`, and
     `src/test/data/project.test.tsx` becomes
     `src/test/features/projects/data/project.test.tsx`.
6. **Move `BootstrapLog/`** into `features/auth/components/`. Move its test
   from `src/test/components/` to `src/test/features/auth/components/`.
7. **Move `vite-env.d.ts`** to the project root (next to `vite.config.ts`).
8. **Run `pnpm tsc --noEmit` + `pnpm test`** after each step. Expect a few
   import-path corrections and at least one stale test mock.

The total is ~half a day for a careful pass with the build green at every
commit. Each step is independently revertable.