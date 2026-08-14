# Refactor Aegis desktop frontend onto TanStack Query

Date: 2026-08-14
Status: Approved (brainstorming)

## Goal

Move the Aegis desktop frontend's data-fetching layer from raw
`useState` + `useEffect` onto TanStack Query (`@tanstack/react-query` +
`@tanstack/react-query-devtools`), so pages get loading/error state,
request deduplication, cache, and the devtools panel for free.

Today the desktop app talks to its Rust backend through a thin Tauri
`invoke()` wrapper exposed as the `api` object in
[`src/api/index.ts`](apps/desktop/aegis-desktop/src/api/index.ts).
Each page calls `api.*` directly and manages its own lifecycle state
with `useState`, refs against React StrictMode double-mount, and
hand-written try/catch. The deps
(`@tanstack/react-query@^5.101.4`,
`@tanstack/react-query-devtools@^5.101.4`) are already pinned in
`package.json` but neither is wired up.

## Approach

Keep `src/api/` as the pure Tauri transport layer (unchanged). Add a
new sibling directory `src/data/` that owns the React Query layer:
`QueryClient` + provider, devtools, query key factory, and per-resource
hooks. Pages consume `src/data/` and never call `api.*` directly.

Layering rule, enforced by code review:

```
pages/  ──imports──▶  data/  ──imports──▶  api/
   ▲                     │                    │
   └──imports (errors)───┘                    │
                                              ▼
                              (@tauri-apps/api/core invoke)
```

`data/` never imports from `pages/`. `api/` never imports from `data/`.
Both layers reuse `error.ts` helpers (`toApiError`, `httpCode`,
`errorMessage`) for shaping failures.

### Why a sibling directory rather than nesting under `api/`

The user explicitly requested the query layer not live under `src/api/`.
`src/api/` is "how to call the backend" (transport). `src/data/` is
"how to expose that backend as cached, reactive data" (orchestration).
Mixing them obscures both. `src/data/` is also broad enough to hold
future concerns (e.g. transform layers, optimistic-update helpers)
without renaming.

## File layout

```
src/
├── api/                          # unchanged — pure Tauri transport
│   ├── index.ts
│   ├── types.ts
│   └── error.ts
│
├── data/                         # NEW — React Query layer
│   ├── client.tsx                #   QueryClient + <QueryProvider> + devtools
│   ├── queryKeys.ts              #   query key factory
│   ├── auth.ts                   #   useLogin, useLoginDomain
│   ├── bootstrap.ts              #   useHealthz, useIsLoggedIn
│   ├── user.ts                   #   useCurrentUser, useDomainUserInfo,
│   │                             #   useRegisterUser, useLogout
│   └── index.ts                  #   barrel re-export
│
├── components/                   # unchanged
├── pages/                        # refactored to consume hooks
├── routes/                       # unchanged
└── test/
    ├── api/                      # existing — keep as-is
    ├── data/                     # NEW — hook tests
    │   ├── auth.test.tsx
    │   ├── bootstrap.test.tsx
    │   └── user.test.tsx
    └── render-with-query-client.tsx   # NEW shared helper
```

## QueryClient defaults

```ts
new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      retry: false,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
    },
    mutations: { retry: false },
  },
});
```

Rationale:

- `staleTime: Infinity` — Tauri calls hit the local sidecar, not a
  remote network. "Stale" means "user navigated away and came back,"
  which already triggers mount-time fetching via `useQuery`. Keeping
  the cached value hot avoids surprise refetches when multiple pages
  mount the same query.
- `retry: false` — sidecar failures are real bugs, not transient
  network glitches. Bail fast and surface the error.
- `refetchOnWindowFocus: false` / `refetchOnReconnect: false` — same
  reasoning; the local sidecar is always present when the app is open.

The bootstrap/login probes (`useHealthz`, `useIsLoggedIn`) opt out of
the global `staleTime: Infinity` by setting `staleTime: 0` per-query
(see `bootstrap.ts` below). They are the only exceptions; all other
read hooks inherit the global default.

## Query keys

`src/data/queryKeys.ts` exports a single factory. Keys are tuples
typed `as const` so downstream `useQuery({ queryKey: queryKeys.x.y() })`
gets exact tuple inference.

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

### Key placement rules

- `auth.loginStatus` lives under `auth.*` (not `bootstrap.*`) because
  it is invalidated by login/logout mutations — both auth concerns.
- `bootstrap.health` lives under `bootstrap.*` because only the
  bootstrap page reads it.
- `user.current` and `user.domainIdentity` live under `user.*` for
  obvious cohesion.

## Hook shapes

All hooks consume `api.*` from the transport layer. The transport
already throws structured `ApiError` objects; hooks propagate them
unchanged into `query.error` / `mutation.error`. Hooks do not shape
errors — that stays in `pages/` via `errorMessage` / `httpCode`.

### `src/data/auth.ts`

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api";
import { queryKeys } from "./queryKeys";

/** Login mutation. On success, refetch the login-status probe so the
 *  next render of the auth-gated layout knows we're in. */
export function useLogin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { code: string; password: string }) =>
      api.login(vars.code, vars.password),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}

export function useLoginDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.loginDomain(),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}
```

### `src/data/bootstrap.ts`

Both reads default to `enabled: false`. The bootstrap page drives them
manually via `refetch()` so they fire exactly once per mount instead
of re-fetching on every remount. Both set `staleTime: 0` so the cached
result is treated as immediately stale — even if a future consumer
flips `enabled` to true, the next read still hits the server.

```ts
import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import { queryKeys } from "./queryKeys";

/** `staleTime: 0` — the health check is a probe, not a cached value. */
export function useHealthz(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.bootstrap.health(),
    queryFn: () => api.healthz(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}

/** `staleTime: 0` — login status is a security-relevant probe. */
export function useIsLoggedIn(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.auth.loginStatus(),
    queryFn: () => api.isLoggedIn(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}
```

### `src/data/user.ts`

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api";
import type { RegisterUserInput } from "../api";
import { queryKeys } from "./queryKeys";

/** Current user. Fires on mount; UserFooter needs it. Auto-fetched. */
export function useCurrentUser() {
  return useQuery({
    queryKey: queryKeys.user.current(),
    queryFn: () => api.getCurrentUser(),
  });
}

/** Domain identity for the register flow. Disabled; driven manually. */
export function useDomainUserInfo(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.user.domainIdentity(),
    queryFn: () => api.getDomainUserInfo(),
    enabled: options?.enabled ?? false,
  });
}

/** Register mutation. No cache to invalidate — the user lands on the
 *  login page next, where login-status is re-probed. */
export function useRegisterUser() {
  return useMutation({
    mutationFn: (input: RegisterUserInput) => api.registerUser(input),
  });
}

/** Logout mutation. Clears the entire cache so no stale user data
 *  leaks across the auth boundary — including the login-status probe
 *  cache entry that `useLogin` / `useLoginDomain` invalidate. */
export function useLogout() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => qc.clear(),
  });
}
```

## Page refactor strategy

Each page keeps its JSX, BootstrapLog calls, and i18n usage. The
data-fetching lifecycle (state, effect, error handling, in-flight
flags) is replaced by the corresponding hook.

### `src/pages/bootstrap.tsx`

`useEffect` becomes a one-shot orchestrator over two manual
`refetch()` calls. The `started` ref still guards against StrictMode
double-mount. Branching (`isError` / `isSuccess`) reads off the manual
fetch result, not the hook's reactive state, so the effect does not
re-run when the hook transitions.

```ts
const health = useHealthz();
const status = useIsLoggedIn();
const started = useRef(false);

useEffect(() => {
  if (started.current) return;
  started.current = true;
  void (async () => {
    push("info", "bootstrap.log.healthCheck.start");
    const h = await health.refetch();
    if (h.isError) {
      push("error", "bootstrap.log.healthCheck.failed", { message: errorMessage(h.error) });
      setHealthFailed(true);
      return;
    }
    push("success", "bootstrap.log.healthCheck.ok", { status: h.data });
    setActiveStep(1);

    push("info", "bootstrap.log.loginStatus.start");
    const s = await status.refetch();
    if (s.isError) {
      push("error", "bootstrap.log.loginStatus.failed", { message: errorMessage(s.error) });
      setLoginStatusFailed(true);
      return;
    }
    if (s.data) {
      push("success", "bootstrap.log.loginStatus.ok");
      await navigate({ to: "/" });
    } else {
      push("info", "bootstrap.log.loginStatus.notLoggedIn");
      await navigate({ to: "/login" });
    }
  })();
}, []);
```

### `src/pages/login.tsx`

`runLogin` is replaced by direct `mutateAsync()` calls. The
`loginDisabled` flag becomes `login.isPending || loginDomain.isPending`.
`outcome` stays as local state since it is purely UI-side.

```ts
const login = useLogin();
const loginDomain = useLoginDomain();

async function onLogin() {
  push("info", "login.log.method.selected", { method: ... });
  try {
    if (method === "domain") await loginDomain.mutateAsync();
    else await login.mutateAsync({ code: accountCode, password });
    push("success", "login.log.login.ok");
    await navigate({ to: "/" });
  } catch (e) {
    const failureCode = httpCode(e);
    if (failureCode === "not_found") { /* notFound branch */ }
    else if (failureCode === "user_inactive") { /* inactive branch */ }
    else { /* failed branch */ }
  }
}

const loginDisabled =
  login.isPending || loginDomain.isPending ||
  (method === "account" && (!accountCode || !password));
```

### `src/pages/register.tsx`

`identity` becomes `useDomainUserInfo()`; `register` becomes
`useRegisterUser()`. The form gates on `identity.data` and
`register.isPending`.

```ts
const identity = useDomainUserInfo();
const register = useRegisterUser();
const lookedUp = useRef(false);

useEffect(() => {
  if (lookedUp.current) return;
  lookedUp.current = true;
  push("info", "register.log.identity.start");
  void (async () => {
    const r = await identity.refetch();
    if (r.isError) push("error", ..., { message: errorMessage(r.error) });
    else push("success", ..., { userid: r.data!.userid });
  })();
}, []);

async function onRegister() {
  const info = identity.data;
  if (!info) return;
  push("info", "register.log.register.start");
  try {
    await register.mutateAsync({ ... });
    push("success", ..., { userCode: info.userid });
    setRegistered(true);
  } catch (e) {
    push("error", ..., { message: errorMessage(e) });
  }
}
```

### `src/pages/UserFooter.tsx`

The local `user` / `error` state becomes `currentUser.data` /
`currentUser.error`. The `cancelled` flag in the existing effect goes
away — `useQuery` handles cancellation internally.

```ts
const currentUser = useCurrentUser();
const logout = useLogout();
const user = currentUser.data;
const error = currentUser.error;

async function onConfirmLogout() {
  setConfirmOpen(false);
  await logout.mutateAsync();
  await navigate({ to: "/login" });
}
```

### Pages with no API changes

`home.tsx`, `settings.tsx`, `layout.tsx`, `__root.tsx`,
`_layout/route.tsx` — untouched.

## Devtools & `main.tsx` wiring

`src/data/client.tsx` exports `QueryProvider`:

```tsx
export function QueryProvider({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
      {import.meta.env.DEV && (
        <ReactQueryDevtools initialIsOpen={false} buttonPosition="bottom-left" />
      )}
    </QueryClientProvider>
  );
}
```

- Devtools only mount in dev (mirrors the `TanStackRouterDevtools`
  pattern already in `main.tsx`).
- `buttonPosition="bottom-left"` so it does not collide with
  `TanStackRouterDevtools` at `bottom-right`.
- `initialIsOpen={false}` — collapsed by default.

`src/main.tsx` wraps with `<QueryProvider>` between
`AegisThemeProvider` and `AegisI18nProvider`:

```tsx
<AegisThemeProvider>
  <QueryProvider>
    <AegisI18nProvider>
      <DocumentLangSync />
      <RouterProvider router={router} />
      {import.meta.env.DEV && (
        <TanStackRouterDevtools router={router} position="bottom-right" />
      )}
    </AegisI18nProvider>
  </QueryProvider>
</AegisThemeProvider>
```

## Tests

### Shared helper

`src/test/render-with-query-client.tsx` — each test gets a fresh
`QueryClient` so caches do not bleed between tests.

```ts
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderOptions } from "@testing-library/react";
import type { ReactElement } from "react";

export function makeTestQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

export function renderWithQueryClient(
  ui: ReactElement,
  options?: { client?: QueryClient } & RenderOptions,
) {
  const client = options?.client ?? makeTestQueryClient();
  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { ...render(ui, { wrapper: Wrapper, ...options }), client };
}
```

### Test files

Mirror the existing `src/test/api/` layout — new tests live under
`src/test/data/`:

- **`src/test/data/auth.test.tsx`** covers `useLogin`, `useLoginDomain`
  - successful mutation calls `api.login` / `api.loginDomain` with the
    right args
  - `onSuccess` invalidates `queryKeys.auth.loginStatus()`
  - mutation `isPending` toggles correctly across the call
  - thrown `ApiError` from the transport propagates into `mutation.error`

- **`src/test/data/bootstrap.test.tsx`** covers `useHealthz`,
  `useIsLoggedIn`
  - `enabled` defaults to `false` → no fetch on mount
  - `refetch()` triggers exactly one fetch
  - `staleTime` is `0` (assert via
    `queryClient.getQueryState(...).dataUpdatedAt`)
  - with `enabled: true`, mount triggers one fetch; second mount also
    fetches (cache never satisfies)
  - errors propagate into `query.error`

- **`src/test/data/user.test.tsx`** covers `useCurrentUser`,
  `useDomainUserInfo`, `useRegisterUser`, `useLogout`
  - `useCurrentUser` fetches on mount
  - `useDomainUserInfo` defaults to disabled; refetch works
  - `useRegisterUser` on success does NOT invalidate any query
  - `useLogout` on success calls `qc.clear()` (verified by seeding a
    stale `user.current` cache entry and asserting it is gone after
    `mutateAsync()`)

### Test infrastructure assumptions

- Existing `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))`
  pattern continues to work. Each test file mocks `invoke` and uses
  `mockCommands({ ... })` from `src/test/tauri-mock.ts` to dispatch by
  command name.
- `src/test/api.test.ts` is untouched — still tests the transport
  layer.
- `vitest.config.ts` already has the `@testing-library/react` peer
  setup; no config change.
- `setup.ts` already silences `window.scrollTo`. No change.

## Out of scope

- Hooks for the unused CRUD commands in `src/api/index.ts`
  (`listUsers`, `getUserByCode`, `updateUser`, `createUser`,
  `listProducts`, `getProductByCode`, `updateProduct`,
  `createProject`, `listProjects`, `getProjectByCode`,
  `updateProject`, `updateUserCredential`, `refresh`). These will be
  added when the pages that consume them land.
- Switching `data/` from manual `refetch()`-based reads to fully
  reactive `useQuery` reads (e.g. always-on `health` polling). The
  current bootstrap contract is one-shot on mount.
- Optimistic updates. No current consumer needs them.

## File changes summary

**New files**

- `src/data/client.tsx`
- `src/data/queryKeys.ts`
- `src/data/auth.ts`
- `src/data/bootstrap.ts`
- `src/data/user.ts`
- `src/data/index.ts`
- `src/test/render-with-query-client.tsx`
- `src/test/data/auth.test.tsx`
- `src/test/data/bootstrap.test.tsx`
- `src/test/data/user.test.tsx`

**Modified files**

- `src/main.tsx` — wrap with `<QueryProvider>`
- `src/pages/bootstrap.tsx` — use `useHealthz` / `useIsLoggedIn`
- `src/pages/login.tsx` — use `useLogin` / `useLoginDomain`
- `src/pages/register.tsx` — use `useDomainUserInfo` / `useRegisterUser`
- `src/pages/UserFooter.tsx` — use `useCurrentUser` / `useLogout`

**Untouched**

- `src/api/**` — pure Tauri transport
- `src/components/**`
- `src/routes/**`
- `src/pages/{home,settings,layout}.tsx`
- All other tests
- `package.json` — `@tanstack/react-query` and `-devtools` already pinned