# `aegis-desktop` Development

Applies to the Tauri shell under `apps/desktop/aegis-desktop/` — the
React frontend in `src/` and the Rust backend in `src-tauri/`. The
backend is a thin transport layer that proxies to the `aegis-server`;
it owns no domain logic. The frontend is the actual application surface
for the user.

Three principles, then specific conventions:

1. **Two-language frontend/backend, one feature.** Each feature
   (`features/<name>/` on the TS side; `src-tauri/src/http/<name>.rs`
   plus `src-tauri/src/commands/<name>.rs` on the Rust side) is the unit
   of work. New domain areas are added by adding one feature on each
   side, not by spreading logic across shared utilities.
2. **Backend is transport-only.** `src-tauri/src/http/` knows nothing
   about Tauri. `src-tauri/src/commands/` is a 1:1 `#[tauri::command]`
   shim over the `http/` layer. Frontend wire types live in
   `src/shared/api/types.ts` as hand-maintained mirrors of the Rust DTOs
   in `src-tauri/src/http/dto.rs` and the per-module DTOs.
3. **State lives in TanStack Query, UI stays declarative.** Pages call
   hook factories (`useFoo`) from `features/<name>/data/`; mutations
   invalidate the relevant query key through the factory in
   `src/shared/query/keys.ts`. Components are stateless beyond their
   own form state.

## 1. Workspace wiring

- The desktop app sits at `apps/desktop/aegis-desktop/`. It is a
  Cargo workspace member (via `src-tauri/Cargo.toml`) and a pnpm
  workspace package (via the root `pnpm-workspace.yaml`); `package.json`
  declares the workspace with `"@aegis/ui": "workspace:*"`.
- Shared Rust deps come from the root `Cargo.toml` via
  `{ workspace = true }` (see `serde`, `reqwest`, `tokio`,
  `thiserror`, `async-trait`, `chrono`). Anything not in the root —
  `wiremock` (dev), `tauri`, `tauri-plugin-*`, the `terminology` git
  crate, `base64`, `windows-utils` — is declared locally with a comment
  explaining *why*.
- Rust crate name is `aegis_desktop_lib` with `crate-type =
  ["staticlib", "cdylib", "rlib"]` so the Windows linker doesn't
  collide with the `main` bin (see the comment in
  `src-tauri/Cargo.toml`).
- The TS frontend uses Vite + TanStack Router. Routing is file-based
  under `src/routes/`. The generated tree is `src/routes/routeTree.gen.ts`
  — do not hand-edit; let `@tanstack/router-plugin` regenerate it.

## 2. Tauri configuration

- `src-tauri/tauri.conf.json`:
  - `beforeDevCommand = "pnpm dev"`, `devUrl = "http://localhost:1420"`,
    `frontendDist = "../dist"` (the Vite build target). The
    `pnpm tauri dev` invocation chains them.
  - The main window is created with `visible: false`. `main.tsx` calls
    `win.show()` + `win.maximize()` ~150 ms after mount so the
    first-paint window-state matches the persisted config without
    flicker.
  - `productName = "aegis-desktop"`, `identifier =
    "com.yukichen.aegis-desktop"`.
- Tauri plugins live in `src-tauri/src/lib.rs` and are initialised in
  the `tauri::Builder` chain: `tauri-plugin-dialog`, `tauri-plugin-store`
  (with the `Builder::new().build()` form), `tauri-plugin-opener`.
- The persistent HTTP client is built once in `.setup` and registered
  via `app.manage(client)` so `#[tauri::command]`s can pull it with
  `State<'_, HttpClient>`. The store name is `"auth.bin"`.

## 3. Rust backend layout

```
src-tauri/src/
  lib.rs              ← tauri::Builder, plugin chain, invoke_handler
  main.rs             ← binary entry; calls lib::run()
  commands.rs         ← `pub mod auth; pub mod …;` barrel
  commands/
    auth.rs           ← one #[tauri::command] per public function
    healthz.rs
    identity.rs
    user.rs
    user_credential.rs
    project.rs
    terminology.rs    ← `pub mod version; pub mod code_list; pub mod code_item; pub mod import;`
    domain_model.rs   ← `pub mod version; pub mod domain; pub mod variable;`
  http.rs             ← `pub mod auth; pub mod …;` barrel
  http/
    client.rs         ← HttpClient + TokenStore trait
    config.rs         ← BASE_URL + NO_AUTH_PATHS (compile-time)
    dto.rs            ← ApiError, ErrorBody, Role, …
    auth.rs           ← pure-async functions over &HttpClient
    healthz.rs
    project.rs
    …
  system.rs           ← `pub mod identity; pub mod jwt_claims;`
  system/
    identity.rs       ← Identity struct + `current()` (Windows-gated)
```

Rules:

- `commands/<x>.rs` is *always* a one-line wrapper:
  `pub async fn <name>(client: State<'_, HttpClient>, …) -> Result<…, ApiError> { http::x::<name>(&client, …).await }`.
  No business logic. The Tauri surface is a shim, nothing else.
- `http/<x>.rs` declares request/response DTOs next to the function
  that uses them. Every DTO derives `Serialize, Deserialize` and uses
  `#[serde(rename_all = "camelCase")]` to match the JS interfaces in
  `src/shared/api/types.ts`.
- `http::dto::ApiError` is the **single** error type every command
  returns. It is `#[serde(tag = "kind", rename_all = "camelCase")]`
  with struct-shaped variants (`network`, `http`, `refreshFailed`,
  `notImplemented`, `store`, `parse`) — serde tagged enums forbid
  newtype variants, so payload-bearing variants carry a `{ message }`
  field. The frontend narrows on `kind` in `src/shared/api/error.ts`.
- Wire-shape enums (e.g. `SdtmVariableType`) use the most descriptive
  `#[serde(rename = "…")]` or `rename_all = "…"` so the JS interface
  can be a plain string literal union. Stable machine-readable error
  codes from the server ride in `ErrorBody { code, message }` and are
  preserved verbatim on the frontend.
- `http::client::HttpClient` is the only thing that knows about
  `reqwest`. Tests build a fresh client with `wiremock::MockServer` and
  inject a `MemoryStore` (a `#[cfg(test)]`-only `TokenStore` impl
  backed by `HashMap`). Concurrent 401 tests share a refresh lock via
  `with_refresh_lock`.

## 4. Frontend feature module layout

```
src/features/<name>/
  index.ts            ← public barrel; only this is imported cross-feature
  data/
    <verb>.ts         ← one file per logical query/mutation family
    index.ts          ← `export * from "./<verb>"`
  components/
    <X>.tsx           ← presentational; no api imports if avoidable
    index.ts          ← re-export every component
  pages/
    <X>Page.tsx       ← route-level component; composes components + data hooks
    index.ts          ← re-export every page
```

Conventions:

- Cross-feature imports go through the barrel in `index.ts` — never
  through nested paths. `import { useLogin } from "../../auth"` is
  fine; `import { useLogin } from "../../auth/data/login"` is not.
- Pages are mounted by route files (`src/routes/…/<route>.tsx`), which
  each look like:

  ```ts
  export const Route = createFileRoute("/_authed/_layout/<x>")({
    component: <X>Page,
  });
  ```

  The route file is the only place that names a page from the routing
  side; components are imported directly by path when pages consume
  them.
- `features/app/` is the root-shell feature. Its components
  (`AppLayout`, `DocumentLangSync`, `SettingsSyncBridge`) are mounted
  by `main.tsx` and the `/_layout` route, so the barrel is empty
  (`export {};`).
- Feature names that have a Rust twin must use the same kebab-case
  name on both sides (`features/auth/` ↔ `commands/auth.rs` +
  `http/auth.rs`).

## 5. Frontend ↔ backend bridge

- `src/shared/api/index.ts` exposes a single `api` object whose every
  method is `call<T>("<command>")` — a thin `invoke()` wrapper that
  spreads the input. The `call` helper loosens Tauri's `InvokeArgs`
  bound to `Record<string, unknown>` so typed input interfaces flow
  through without per-call casts.
- For one-off endpoints that need `WebviewWindow` (project workspace
  windows), the wrapper is a plain `async` method on `api` —
  `openProjectWorkspace` is the canonical example, using
  `WebviewWindow.getByLabel` to focus an existing window instead of
  duplicating it.
- `src/shared/api/types.ts` is hand-maintained, not generated. Every
  TS interface mirrors the Rust DTO in
  `src-tauri/src/http/<x>.rs` 1:1. **Field names are camelCase in TS
  but the actual JSON keys are snake_case** (the server wire format);
  consumers that destructure runtime payloads must use snake_case keys
  or add a transform layer. The exception is anything from
  `system::identity::Identity`, which is camelCase on both sides
  (`hostMachine`) because the Tauri command return value keeps its
  serde rename.
- `src/shared/api/error.ts` is the single place that knows the
  `ApiError` shape. Pages and tests import `toApiError(e)` (narrows
  `unknown` to `ApiError` with a `network` fallback) and
  `errorMessage(e)` (one-line string for splash logs). Never re-write
  the `kind === "http"` dance inline.

## 6. State management

- `src/shared/query/client.tsx` exposes `queryClient` and
  `QueryProvider`. Defaults: `staleTime: Infinity`, `retry: false`,
  `refetchOnWindowFocus: false`, `refetchOnReconnect: false` — the
  argument in the file header is that Tauri calls hit a local sidecar,
  not a flaky network, so retry / focus-refetch just hide bugs.
- `src/shared/query/keys.ts` is the **only** place query keys are
  constructed. Keys are tuples typed `as const` so `useQuery` and
  `useMutation` get exact tuple inference. Hooks and invalidations
  reference keys through this factory — never inline literal arrays —
  so a typo breaks one site at a time.
- Manual-trigger probes (`useHealthz`, `useIsLoggedIn`,
  `useDomainUserInfo`) use `enabled: false` by default and let the
  page drive them via `refetch()` so they fire once on mount, not
  every remount. They pin `staleTime: 0` to opt out of the global
  `Infinity`.
- Mutations invalidate the relevant key on success. Login / logout
  invalidate `queryKeys.auth.loginStatus()`; domain-model mutations
  invalidate `["domainModel", "sdtmDomains", updated.versionId]` and
  the per-id variant. When the response does not carry enough info
  (e.g. `updateSdtmVariable` only takes `{id}`), fall back to a coarse
  `["domainModel", "sdtmVariables"]` invalidation with a code comment
  explaining the gap.
- The dev client mounts `@tanstack/react-query-devtools` and
  `@tanstack/react-router-devtools` only when `import.meta.env.DEV` is
  true.

## 7. Routing, auth, and the bootstrap splash

- `src/routes/__root.tsx` is a one-liner `Outlet`. `src/main.tsx`
  constructs the router, registers the type with TanStack Router, and
  renders the provider tree: `PersistentThemeProvider` →
  `QueryProvider` → `PersistentI18nProvider` → `SettingsSyncBridge` →
  `DocumentLangSync` → `RouterProvider`.
- `main.tsx` synchronously redirects `/` and `/index.html` to
  `/bootstrap` before constructing the router
  (`window.history.replaceState`, falling back to `router.navigate`
  when `replaceState` is a no-op under the `tauri://` protocol). The
  decision is in `features/bootstrap/redirect.ts`
  (`shouldRedirectToBootstrap`) and intentionally excludes workspace
  URLs (`/project/<code>`) so their auth check does not race against
  the bootstrap probes.
- `src/routes/_authed/route.tsx` is the pathless auth guard. Its
  `beforeLoad` calls `api.isLoggedIn()`; any failure (including a
  broken token store) is treated as logged-out and the user is
  redirected to `/login`. Every authenticated page lives under
  `src/routes/_authed/`.
- `src/routes/_authed/_layout/route.tsx` mounts `AppLayout` (the
  sidebar + content shell from `features/app/components/`). Workspace
  windows mount a *different* layout
  (`src/routes/_authed/project/$projectCode/route.tsx` →
  `features/project-workspace/pages/ProjectWorkspaceLayout`).
- `features/bootstrap/pages/BootstrapPage.tsx` is the splash. It runs
  the health probe then the login-status probe, advances a `Stepper`
  on each result, and navigates to `/` or `/login` accordingly. The
  orchestrator is gated by a `useRef(false)` so React StrictMode's
  double-invoked effect runs the probes exactly once.

## 8. i18n, theme, and persistent settings

- All persistence lives in `features/settings/data/persist.ts`. The
  store is the on-disk `settings.bin` from `tauri-plugin-store`,
  loaded lazily as a singleton (`getStore()`); every Tauri window
  reads the same file.
- `useHydrateSettingsFromStore` is the single-fire mount-time read;
  it runs once per window and pushes the values into the
  `useThemeMode` / `useI18n` setters. The `useEffect` deps array is
  intentionally empty and the `react-hooks/exhaustive-deps` lint is
  disabled with a comment.
- `useListenForSettingsChanges` subscribes to the
  `aegis:settings-changed` event from `@tauri-apps/api/event`. The
  dynamic import of `@tauri-apps/api/event` inside the effect keeps
  the listener off the synchronous startup path.
- `persistSettings({ theme?, locale? })` writes only the keys present
  in the patch (so passing `{ theme }` does not clobber `locale`) and
  saves. The wrappers in `features/app/components/SettingsSyncBridge.tsx`
  (`PersistentThemeProvider`, `PersistentI18nProvider`) call
  `persistSettings` *and* `emit("aegis:settings-changed", payload)`
  after every provider `onModeChange` / `onLocaleChange` callback
  fires, so other windows react via `useListenForSettingsChanges`.
- `DocumentLangSync` writes `document.documentElement.lang = locale`
  on every locale change so the `lang` attribute matches the active
  translation (used by screen readers and browser features).

## 9. RBAC and role gating

- `useCurrentUser` is the single source of role info. Pages compute a
  derived boolean (`canMutate = role === "admin" || role === "root"`,
  `canManage = role === "root" || role === "admin"`) and pass it down
  to every drawer / dialog. Components never call `useCurrentUser`
  themselves.
- Role-gated UI elements take `canMutate: boolean` as a prop. Drawers
  and dialogs disable inputs *and* the submit button when the flag is
  false, not just one or the other.
- The sidebar (`features/app/components/AppLayout.tsx`) inserts the
  `managementEntry` (the Users submenu) into the menu only when the
  user is `admin` or `root`. When `current_user` has not yet resolved
  (`role` is undefined), the menu falls into the non-manager branch —
  the test `app-layout.test.tsx` pins this behaviour.

## 10. Testing

Vitest (`vitest.config.ts`) with jsdom, setup in
`src/test/helpers/setup.ts` (`@testing-library/jest-dom/vitest`,
`scrollTo` shim, no-op `IntersectionObserver` / `ResizeObserver` /
`PointerEvent` for `@dnd-kit`). Per-file `vi.mock("@tauri-apps/api/core",
() => ({ invoke: vi.fn() }))` is required — `vi.mock` is hoisted and
cannot be applied from a helper module.

Helper modules under `src/test/helpers/`:

- `tauri-mock.ts` — `mockInvoke` cast and `mockCommands(handlers)`
  dispatch by command name (so an unrelated command joining a
  page's startup sequence does not break an earlier test), plus
  `httpError(status, code, message)` to build a tagged `ApiError`.
- `render-with-query-client.tsx` — `renderWithQueryClient(ui)` wraps
  the tree in a fresh `QueryClientProvider`. Returns the standard
  render result plus the client so tests can spy on
  `invalidateQueries`.
- `test-query-provider.tsx` — `TestQueryProvider` alternative that
  uses a fresh client per *render*, used when the test wants
  isolation beyond what the global `QueryProvider` offers.
- `file-route-utils.tsx` — `renderInRouter(ui)` mounts the page under
  a one-route fake router at `/`; `renderWithFullRouter({ initialEntries
  })` mounts the real `routeTree.gen.ts` (with `__root.tsx`'s
  layout) for Sidebar / navigation tests. Both `await router.load()`
  inside `act` before `render` so the matched component has painted.
- `setup.ts` — global shims listed above.

Tests live in `src/test/features/<name>/<thing>.test.tsx`, mirroring
the feature they cover. Common patterns:

- Assert an `invoke` call with `expect(invoke).toHaveBeenCalledWith("login",
  { code, password })`.
- Assert invalidation with `vi.spyOn(client, "invalidateQueries")` and
  `expect(spy).toHaveBeenCalledWith(expect.objectContaining({ queryKey:
  queryKeys.x.y() }))`.
- Simulate errors by `throw`-ing a tagged `ApiError` from the handler:
  `mockCommands({ login: () => { throw { kind: "http", status: 401, code:
  "invalid_credentials", message: "bad" } } })`.
- For Rust, prefer unit tests next to the code (`#[cfg(test)] mod tests`)
  with `wiremock::MockServer` + `MemoryStore`. A test that needs
  target-OS-specific behaviour gates on `#[cfg(target_os = "windows")]`
  / `#[cfg(not(target_os = "windows"))]` and adds a *behavioural* test
  on the target it works on, plus a *compile-time* shape test on the
  other (e.g. `assert_login_domain_takes_only_the_client`).

## 11. UI conventions

- All MUI primitives and icons come from `@aegis/ui/mui` and
  `@aegis/ui/icons`. The shared `Sidebar`, theme, and i18n providers
  also live in `@aegis/ui`. Never import from `@mui/material` directly.
- All user-visible strings come through `useI18n()` → `t("path.to.key")`.
  Add new keys to the `@aegis/ui` translation resources, not inline.
- Drawers and dialogs own only their form state. Pages own the
  mutation hook and the open/close boolean. The component receives
  `mutationError: ApiError | null` and `mutationPending: boolean` and
  renders the corresponding `Alert`. The page wires `mutation.error`
  / `mutation.isPending` straight through.
- Drawer pattern: `mode: "create" | "edit"` with `row?: T`; an
  `useEffect` keyed on `[open, mode, row]` resets local state when
  the drawer is opened or the target row changes. A `disabled={!canMutate}`
  plus a `disabled={mutationPending}` on the submit button covers both
  halves of RBAC + in-flight locking.
- Filter inputs use `useDebouncedValue` from `src/shared/hooks/`
  (`delayMs: 300`, `maxWaitMs: 1000`) and feed the filter predicate
  through `useMemo` keyed on the debounced fragment.

## 12. Page composition

- Each page owns its search-param shape. Reads use
  `useSearch({ from: "<full route id>", strict: false })` so a page
  loaded via a partial route still sees the same params. Writes use
  `navigate({ to, params, search, replace })` and only set the keys
  they care about (`search: (prev) => ({ ...prev, versionId })`).
- Two-step "URL is stale, data has changed" reconciliation: a
  `useMemo` derives the canonical value (falling back to the first
  list entry) and a `useEffect` navigates to it with `replace: true`
  when the URL is invalid. See `SdtmDomainList.tsx` / `SdtmDomainDetail.tsx`
  for the canonical `versionId` + `lang` reconciliation.
- Back navigation: pages deep in a route tree carry their parent's
  selected filters into the back-`navigate` so the parent list view
  re-opens in the right state. Example: the detail page passes
  `versionId` + `lang` back to `/domain-model/sdtm` on Back.

## 13. Verification gate, before any PR

```bash
# Frontend
pnpm typecheck
pnpm test            # vitest run
pnpm build           # tsc && vite build

# Backend
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```

`pnpm dev` runs the Vite dev server; `pnpm tauri dev` chains it with
the Tauri shell. `pnpm tauri build` runs `pnpm build` then bundles the
binary via `tauri.conf.json`'s `bundle.targets = "all"`.

## 14. Commits and review

- One commit per logical change (feature scaffold + data hook +
  components + page + route file + tests, etc.). A lockfile drift gets
  its own `chore:` commit.
- Commit messages list the spec coverage and the verification commands
  at the bottom so reviewers can run the same gate locally.
- Any change that touches wire shape (`http/<x>.rs` DTOs,
  `shared/api/types.ts`, `commands/<x>.rs`) is paired with a TS
  update in the same commit, since the two halves break together.