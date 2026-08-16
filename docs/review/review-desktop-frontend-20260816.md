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