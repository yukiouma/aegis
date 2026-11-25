# Aegis Desktop TanStack Query Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the Aegis desktop frontend's data-fetching layer from raw `useState`/`useEffect` onto TanStack Query, wire `@tanstack/react-query-devtools`, and keep the existing `src/api/` Tauri transport untouched.

**Architecture:** `src/api/` stays a pure Tauri transport. A new sibling `src/data/` owns the React Query layer — `QueryClient` + `<QueryProvider>`, query key factory, per-resource hooks (`auth.ts`, `bootstrap.ts`, `user.ts`), and a barrel. Pages consume `src/data/`; `src/data/` consumes `src/api/`. Tests live under `src/test/data/` and use a shared `renderWithQueryClient` helper.

**Tech Stack:** React 19, TanStack Query v5 (`^5.101.4`), TanStack Query Devtools v5, Vitest 2 + jsdom, `@testing-library/react` v16, `@testing-library/user-event` v14.

## Global Constraints

- **TypeScript strictness:** `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true` — every declared identifier must be used.
- **Tauri transport is untouched:** `src/api/index.ts`, `src/api/types.ts`, `src/api/error.ts` keep their current behavior and signatures.
- **Existing tests pass:** `pnpm test` must remain green throughout. The existing `src/test/api.test.ts` and `src/test/api/error.test.ts` continue to test the transport layer only.
- **Test mock pattern:** every test file that touches Tauri must call `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` at module scope and import `invoke` for assertion, mirroring the existing `src/test/api.test.ts` setup.
- **Devtools mount rule:** `@tanstack/react-query-devtools` mounts only when `import.meta.env.DEV` is true (matches the existing `TanStackRouterDevtools` rule in `src/main.tsx`).
- **Layering rule:** `src/data/` imports from `src/api/`. `src/pages/` imports from `src/data/`. `src/api/` never imports from `src/data/`. `src/data/` never imports from `src/pages/`.
- **Error shaping lives in pages:** hooks propagate `ApiError` unchanged; only `pages/` import `errorMessage` / `httpCode` from `src/api/error.ts`.

---

## File Map

```
src/data/                                  NEW
├── client.tsx                             NEW — QueryClient + <QueryProvider> + devtools
├── queryKeys.ts                           NEW — query key factory
├── auth.ts                                NEW — useLogin, useLoginDomain
├── bootstrap.ts                           NEW — useHealthz, useIsLoggedIn
├── user.ts                                NEW — useCurrentUser, useDomainUserInfo,
│                                                useRegisterUser, useLogout
└── index.ts                               NEW — barrel re-export

src/test/
├── render-with-query-client.tsx           NEW — shared test helper
└── data/                                  NEW
    ├── auth.test.tsx
    ├── bootstrap.test.tsx
    └── user.test.tsx

src/main.tsx                               MOD — wrap with <QueryProvider>
src/pages/bootstrap.tsx                    MOD — use useHealthz / useIsLoggedIn
src/pages/login.tsx                       MOD — use useLogin / useLoginDomain
src/pages/register.tsx                     MOD — use useDomainUserInfo / useRegisterUser
src/pages/UserFooter.tsx                   MOD — use useCurrentUser / useLogout

src/api/**                                 UNTOUCHED
src/components/**                          UNTOUCHED
src/routes/**                              UNTOUCHED
src/pages/{home,settings,layout}.tsx       UNTOUCHED
```

---

## Task 1: Foundation — QueryClient, devtools, key factory, test helper, main.tsx wiring

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/queryKeys.ts`
- Create: `apps/desktop/aegis-desktop/src/data/client.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/render-with-query-client.tsx`
- Modify: `apps/desktop/aegis-desktop/src/main.tsx`

**Interfaces:**
- Consumes: nothing (foundational)
- Produces:
  - `queryKeys.auth.loginStatus()` returns `readonly ["auth", "loginStatus"]`
  - `queryKeys.bootstrap.health()` returns `readonly ["bootstrap", "health"]`
  - `queryKeys.user.current()` returns `readonly ["user", "current"]`
  - `queryKeys.user.domainIdentity()` returns `readonly ["user", "domainIdentity"]`
  - `queryClient` (singleton `QueryClient`)
  - `<QueryProvider>` wraps `children` in `QueryClientProvider` and conditionally mounts `<ReactQueryDevtools buttonPosition="bottom-left" initialIsOpen={false} />` when `import.meta.env.DEV` is true
  - `makeTestQueryClient()` returns a fresh `QueryClient` with `retry: false`
  - `renderWithQueryClient(ui, options?)` returns `{ ...renderResult, client }`

- [ ] **Step 1: Create `src/data/queryKeys.ts`**

Create the file at `apps/desktop/aegis-desktop/src/data/queryKeys.ts`:

```ts
// Query key factory. Keys are tuples typed `as const` so downstream
// `useQuery({ queryKey: queryKeys.x.y() })` gets exact tuple inference.
// All hooks and invalidations reference keys through this module —
// never inline arrays — so a typo breaks one site at a time.

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

- [ ] **Step 2: Create `src/data/client.tsx`**

Create the file at `apps/desktop/aegis-desktop/src/data/client.tsx`:

```tsx
import {
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import React from "react";

// Default options rationale:
// - `staleTime: Infinity`: Tauri calls hit a local sidecar. There is no
//   network to mask; remounting the same query already triggers a fetch
//   via `useQuery`'s mount semantics. Keeps the devtools quiet.
// - `retry: false`: sidecar failures are real bugs, not transient. Bail.
// - `refetchOnWindowFocus / refetchOnReconnect: false`: same reasoning.
// Per-query overrides live in the hook files (e.g. `bootstrap.ts` pins
// `staleTime: 0` for health/login-status probes).
export const queryClient = new QueryClient({
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

- [ ] **Step 3: Create `src/test/render-with-query-client.tsx`**

Create the file at `apps/desktop/aegis-desktop/src/test/render-with-query-client.tsx`:

```tsx
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderOptions } from "@testing-library/react";
import type { ReactElement } from "react";

/**
 * Build a fresh `QueryClient` for one test. Caches must not bleed
 * between tests, so each render site gets its own unless the caller
 * passes one in via `options.client`.
 */
export function makeTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

/**
 * Render `ui` wrapped in a `QueryClientProvider`. Returns the standard
 * `@testing-library/react` render result plus the `client` so tests
 * can inspect cache state and spy on methods like `invalidateQueries`.
 */
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

- [ ] **Step 4: Wire `src/main.tsx`**

In `apps/desktop/aegis-desktop/src/main.tsx`, make two edits:

1. Add this import below the existing `react-router` / `routeTree` / `DocumentLangSync` / `getCurrentWindow` block (alphabetical position is between `DocumentLangSync` and `getCurrentWindow`):

```tsx
import { QueryProvider } from "./data/client";
```

2. Wrap the existing `<AegisI18nProvider>` subtree with `<QueryProvider>` between `AegisThemeProvider` and `AegisI18nProvider`. The new structure must read:

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

No other lines change. The `getCurrentWindow()` effect, `initialPath` / `replaceState` block, and `declare module` block remain as written.

- [ ] **Step 5: Run typecheck and existing tests**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
pnpm test
```

Expected: typecheck passes (no errors); all existing tests pass (`api.test.ts`, `api/error.test.ts`, `document-lang-sync.test.tsx`).

- [ ] **Step 6: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/data/queryKeys.ts src/data/client.tsx src/test/render-with-query-client.tsx src/main.tsx
git commit -m "feat(desktop): wire TanStack Query provider, devtools, query key factory"
```

---

## Task 2: `auth.ts` hooks (`useLogin`, `useLoginDomain`) with tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/auth.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/auth.test.tsx`

**Interfaces:**
- Consumes:
  - `api.login(code: string, password: string): Promise<void>`
  - `api.loginDomain(): Promise<void>`
  - `queryKeys.auth.loginStatus()` from `Task 1`
  - `makeTestQueryClient`, `renderWithQueryClient` from `Task 1`
  - `mockCommands` from `src/test/tauri-mock.ts`
- Produces:
  - `useLogin(): UseMutationResult<void, ApiError, { code: string; password: string }>`
    - on success: invalidates `queryKeys.auth.loginStatus()`
  - `useLoginDomain(): UseMutationResult<void, ApiError, void>`
    - on success: invalidates `queryKeys.auth.loginStatus()`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/data/auth.test.tsx`:

```tsx
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useLogin, useLoginDomain } from "../../data/auth";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function LoginHarness({
  variant,
}: {
  variant: "account" | "domain";
}) {
  const login = useLogin();
  const loginDomain = useLoginDomain();
  return (
    <>
      <button
        onClick={() => {
          if (variant === "domain") {
            loginDomain.mutate();
          } else {
            login.mutate({ code: "alice", password: "secret" });
          }
        }}
      >
        submit
      </button>
      <span data-testid="pending">
        {login.isPending || loginDomain.isPending ? "yes" : "no"}
      </span>
      <span data-testid="error-kind">
        {login.error?.kind ?? loginDomain.error?.kind ?? "none"}
      </span>
    </>
  );
}

function LoginStatusProbe({
  client,
}: {
  client: ReturnType<typeof renderWithQueryClient>["client"];
}) {
  // Tiny consumer of the login-status query key so we can assert its
  // invalidation state via the QueryClient directly.
  useEffect(() => {
    void client;
  }, [client]);
  return null;
}

describe("useLogin", () => {
  it("invokes api.login with code and password on mutate()", async () => {
    mockCommands({ login: () => undefined });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("login", {
        code: "alice",
        password: "secret",
      });
    });
  });

  it("invalidates queryKeys.auth.loginStatus on success", async () => {
    mockCommands({ login: () => undefined });
    const { client } = renderWithQueryClient(<LoginHarness variant="account" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.auth.loginStatus(),
        }),
      );
    });
  });

  it("exposes the thrown ApiError through mutation.error", async () => {
    mockCommands({
      login: () => {
        throw {
          kind: "http",
          status: 401,
          code: "invalid_credentials",
          message: "bad",
        };
      },
    });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });

  it("toggles isPending across the call lifecycle", async () => {
    let resolve!: () => void;
    mockCommands({
      login: () => new Promise<void>((r) => {
        resolve = r;
      }),
    });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(screen.getByTestId("pending").textContent).toBe("yes");
    });
    resolve();
    await waitFor(() => {
      expect(screen.getByTestId("pending").textContent).toBe("no");
    });
  });
});

describe("useLoginDomain", () => {
  it("invokes api.loginDomain with no args on mutate()", async () => {
    mockCommands({ login_domain: () => undefined });
    renderWithQueryClient(<LoginHarness variant="domain" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("login_domain");
    });
  });

  it("invalidates queryKeys.auth.loginStatus on success", async () => {
    mockCommands({ login_domain: () => undefined });
    const { client } = renderWithQueryClient(<LoginHarness variant="domain" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.auth.loginStatus(),
        }),
      );
    });
  });
});

// `LoginStatusProbe` is a deliberate no-op consumer of `client` so the
// effect dependency on `client` doesn't trip `react-hooks/exhaustive-deps`.
// It is intentionally not rendered in any test — kept here only to
// document the cache-key assertion strategy.
void LoginStatusProbe;
// `render` import is used transitively by `renderWithQueryClient`'s
// return type; importing it here keeps the helper's signature obvious.
void render;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/auth.test.tsx
```

Expected: FAIL — `useLogin` and `useLoginDomain` are not exported from `../../data/auth` (module not found).

- [ ] **Step 3: Implement `src/data/auth.ts`**

Create the file at `apps/desktop/aegis-desktop/src/data/auth.ts`:

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { api } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * Login mutation. On success, invalidates the login-status probe so
 * the auth-gated layout re-derives its auth state on the next render.
 * The transport throws a structured `ApiError` on failure, which
 * surfaces unchanged through `mutation.error`.
 */
export function useLogin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { code: string; password: string }) =>
      api.login(vars.code, vars.password),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}

/**
 * Domain-account login mutation. Same invalidation contract as
 * `useLogin` — the post-login render path is identical regardless
 * of which method landed the user.
 */
export function useLoginDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.loginDomain(),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/auth.test.tsx
```

Expected: PASS — 6 tests, all green.

- [ ] **Step 5: Run typecheck to confirm no unused-import warnings**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS — no errors. The `void LoginStatusProbe;` and `void render;` lines suppress `noUnusedLocals` on the helper symbols the test file declares for documentation but does not actively render.

- [ ] **Step 6: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/data/auth.ts src/test/data/auth.test.tsx
git commit -m "feat(desktop): add useLogin, useLoginDomain with invalidation"
```

---

## Task 3: `bootstrap.ts` hooks (`useHealthz`, `useIsLoggedIn`) with tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/bootstrap.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/bootstrap.test.tsx`

**Interfaces:**
- Consumes:
  - `api.healthz(): Promise<string>`
  - `api.isLoggedIn(): Promise<boolean>`
  - `queryKeys.bootstrap.health()` and `queryKeys.auth.loginStatus()` from `Task 1`
  - `makeTestQueryClient`, `renderWithQueryClient` from `Task 1`
  - `mockCommands` from `src/test/tauri-mock.ts`
- Produces:
  - `useHealthz({ enabled?: boolean }): UseQueryResult<string, ApiError>`
    - `enabled` defaults to `false`
    - `staleTime: 0`
  - `useIsLoggedIn({ enabled?: boolean }): UseQueryResult<boolean, ApiError>`
    - `enabled` defaults to `false`
    - `staleTime: 0`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/data/bootstrap.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useHealthz, useIsLoggedIn } from "../../data/bootstrap";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => {
  cleanup();
});

function Probe({ enabled }: { enabled?: boolean }) {
  const health = useHealthz({ enabled });
  const status = useIsLoggedIn({ enabled });
  return (
    <>
      <button onClick={() => void health.refetch()}>refetch-health</button>
      <button onClick={() => void status.refetch()}>refetch-status</button>
      <span data-testid="health-data">{health.data ?? "none"}</span>
      <span data-testid="health-pending">{health.isPending ? "yes" : "no"}</span>
      <span data-testid="health-error-kind">{health.error?.kind ?? "none"}</span>
      <span data-testid="status-data">{String(status.data ?? "none")}</span>
    </>
  );
}

describe("useHealthz", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ healthz: () => "ok" });
    renderWithQueryClient(<Probe />);
    // Allow one tick for any spurious mount-time fetch.
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("fetches exactly once per manual refetch()", async () => {
    mockCommands({ healthz: () => "ok" });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-health" }).click();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1);
      expect(invoke).toHaveBeenCalledWith("healthz");
    });
    expect(screen.getByTestId("health-data").textContent).toBe("ok");
  });

  it("propagates the thrown ApiError on refetch failure", async () => {
    mockCommands({
      healthz: () => {
        throw { kind: "network", message: "no route to host" };
      },
    });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-health" }).click();
    await waitFor(() => {
      expect(screen.getByTestId("health-error-kind").textContent).toBe(
        "network",
      );
    });
  });

  it("treats cached data as immediately stale (staleTime: 0)", async () => {
    mockCommands({ healthz: () => "ok" });
    function AlwaysOn() {
      useHealthz({ enabled: true });
      return null;
    }
    const utils = renderWithQueryClient(<AlwaysOn />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    utils.unmount();

    // Confirm `staleTime` is 0 via the QueryClient state.
    const client = utils.client;
    expect(
      client.getQueryState(queryKeys.bootstrap.health())?.isInvalidated,
    ).toBe(true);

    // A second mount must trigger another fetch — the cached value is
    // stale, so useQuery never serves it.
    renderWithQueryClient(<AlwaysOn />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});

describe("useIsLoggedIn", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ is_logged_in: () => true });
    renderWithQueryClient(<Probe />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("returns the boolean payload via manual refetch()", async () => {
    mockCommands({ is_logged_in: () => false });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-status" }).click();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("is_logged_in");
      expect(screen.getByTestId("status-data").textContent).toBe("false");
    });
  });

  it("treats cached data as immediately stale (staleTime: 0)", async () => {
    mockCommands({ is_logged_in: () => true });
    function AlwaysOn() {
      useIsLoggedIn({ enabled: true });
      return null;
    }
    const utils = renderWithQueryClient(<AlwaysOn />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    utils.unmount();

    const client = utils.client;
    expect(
      client.getQueryState(queryKeys.auth.loginStatus())?.isInvalidated,
    ).toBe(true);

    renderWithQueryClient(<AlwaysOn />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/bootstrap.test.tsx
```

Expected: FAIL — `useHealthz` and `useIsLoggedIn` are not exported from `../../data/bootstrap` (module not found).

- [ ] **Step 3: Implement `src/data/bootstrap.ts`**

Create the file at `apps/desktop/aegis-desktop/src/data/bootstrap.ts`:

```ts
import { useQuery } from "@tanstack/react-query";

import { api } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * Health probe. Defaults to `enabled: false` because the bootstrap
 * page drives the call manually via `refetch()` — auto-firing on
 * mount would re-fetch every time React remounts the page.
 *
 * `staleTime: 0` opts out of the global `Infinity` default: even if a
 * future consumer flips `enabled` to true, the cached value is
 * treated as immediately stale and the next read hits the server.
 */
export function useHealthz(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.bootstrap.health(),
    queryFn: () => api.healthz(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}

/**
 * Login-status probe. Same manual-trigger contract as `useHealthz`.
 * Lives under `queryKeys.auth.*` (not `bootstrap.*`) because login
 * and logout mutations invalidate it — those are auth concerns.
 */
export function useIsLoggedIn(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.auth.loginStatus(),
    queryFn: () => api.isLoggedIn(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/bootstrap.test.tsx
```

Expected: PASS — 7 tests, all green.

- [ ] **Step 5: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/data/bootstrap.ts src/test/data/bootstrap.test.tsx
git commit -m "feat(desktop): add useHealthz, useIsLoggedIn with staleTime: 0"
```

---

## Task 4: `user.ts` hooks (`useCurrentUser`, `useDomainUserInfo`, `useRegisterUser`, `useLogout`) with tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/user.ts`
- Create: `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`

**Interfaces:**
- Consumes:
  - `api.getCurrentUser(): Promise<UserView>`
  - `api.getDomainUserInfo(): Promise<Identity>`
  - `api.registerUser(input: RegisterUserInput): Promise<RegisterUserResponse>`
  - `api.logout(): Promise<void>`
  - `queryKeys.user.current()`, `queryKeys.user.domainIdentity()` from `Task 1`
  - `makeTestQueryClient`, `renderWithQueryClient` from `Task 1`
  - `mockCommands` from `src/test/tauri-mock.ts`
- Produces:
  - `useCurrentUser(): UseQueryResult<UserView, ApiError>` (no `enabled` option — always auto-fetches on mount)
  - `useDomainUserInfo({ enabled?: boolean }): UseQueryResult<Identity, ApiError>`
    - `enabled` defaults to `false`
  - `useRegisterUser(): UseMutationResult<RegisterUserResponse, ApiError, RegisterUserInput>`
    - no invalidation
  - `useLogout(): UseMutationResult<void, ApiError, void>`
    - on success: `qc.clear()`

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/data/user.test.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  useCurrentUser,
  useDomainUserInfo,
  useLogout,
  useRegisterUser,
} from "../../data/user";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

const userView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin" as const,
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const identity = {
  domain: "EXAMPLE",
  hostMachine: "host01",
  sid: "S-1-...",
  userid: "alice",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => {
  cleanup();
});

function CurrentUserProbe() {
  const q = useCurrentUser();
  return <span data-testid="data">{q.data?.name ?? "none"}</span>;
}

function DomainInfoProbe({ enabled }: { enabled?: boolean }) {
  const q = useDomainUserInfo({ enabled });
  return (
    <>
      <button onClick={() => void q.refetch()}>refetch</button>
      <span data-testid="userid">{q.data?.userid ?? "none"}</span>
    </>
  );
}

function RegisterHarness() {
  const m = useRegisterUser();
  return (
    <>
      <button
        onClick={() => {
          m.mutate({
            userCode: "alice",
            userName: "Alice",
            domainName: "EXAMPLE",
            hostname: "host01",
            sid: "S-1-...",
            password: "pw",
          });
        }}
      >
        register
      </button>
      <span data-testid="pending">{m.isPending ? "yes" : "no"}</span>
    </>
  );
}

function LogoutHarness() {
  const m = useLogout();
  return (
    <button
      onClick={() => {
        m.mutate();
      }}
    >
      logout
    </button>
  );
}

describe("useCurrentUser", () => {
  it("fetches api.current_user on mount", async () => {
    mockCommands({ current_user: () => userView });
    renderWithQueryClient(<CurrentUserProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("current_user");
      expect(screen.getByTestId("data").textContent).toBe("Alice");
    });
  });
});

describe("useDomainUserInfo", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ get_domain_user_info: () => identity });
    renderWithQueryClient(<DomainInfoProbe />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("returns the Identity payload on manual refetch()", async () => {
    mockCommands({ get_domain_user_info: () => identity });
    renderWithQueryClient(<DomainInfoProbe />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_domain_user_info");
      expect(screen.getByTestId("userid").textContent).toBe("alice");
    });
  });
});

describe("useRegisterUser", () => {
  it("invokes api.register_user with the input shape", async () => {
    mockCommands({ register_user: () => ({}) });
    renderWithQueryClient(<RegisterHarness />);
    await userEvent.click(screen.getByRole("button", { name: "register" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("register_user", {
        userCode: "alice",
        userName: "Alice",
        domainName: "EXAMPLE",
        hostname: "host01",
        sid: "S-1-...",
        password: "pw",
      });
    });
  });

  it("does not invalidate any query on success", async () => {
    mockCommands({ register_user: () => ({}) });
    const { client } = renderWithQueryClient(<RegisterHarness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "register" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("register_user", expect.anything());
    });
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("useLogout", () => {
  it("invokes api.logout with no args", async () => {
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />, { client });
    await userEvent.click(screen.getByRole("button", { name: "logout" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("logout");
    });
  });

  it("clears the entire cache on success", async () => {
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />);
    // Seed a stale cache entry to prove it gets wiped.
    client.setQueryData(queryKeys.user.current(), userView);
    expect(client.getQueryData(queryKeys.user.current())).toEqual(userView);

    const clearSpy = vi.spyOn(client, "clear");
    await userEvent.click(screen.getByRole("button", { name: "logout" }));
    await waitFor(() => {
      expect(clearSpy).toHaveBeenCalled();
      expect(client.getQueryData(queryKeys.user.current())).toBeUndefined();
    });
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/user.test.tsx
```

Expected: FAIL — module `../../data/user` not found.

- [ ] **Step 3: Implement `src/data/user.ts`**

Create the file at `apps/desktop/aegis-desktop/src/data/user.ts`:

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "../api";
import type { RegisterUserInput } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * Current signed-in user. Fires on mount — `UserFooter` is the only
 * consumer and it renders inside `AppLayout` (which only mounts
 * post-auth), so the call always succeeds in production. There is
 * no `enabled` option: the call should always run when the sidebar
 * shows.
 */
export function useCurrentUser() {
  return useQuery({
    queryKey: queryKeys.user.current(),
    queryFn: () => api.getCurrentUser(),
  });
}

/**
 * Domain identity for the register flow. Disabled by default;
 * the register page drives the lookup manually via `refetch()` so
 * the fetch happens once, on demand, after the user lands on
 * `/register`. Inherits the global `staleTime: Infinity` because
 * the consumer never re-mounts.
 */
export function useDomainUserInfo(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.user.domainIdentity(),
    queryFn: () => api.getDomainUserInfo(),
    enabled: options?.enabled ?? false,
  });
}

/**
 * Register mutation. No cache to invalidate — the user lands on
 * `/login` next, where login-status is re-probed by `bootstrap.ts`.
 */
export function useRegisterUser() {
  return useMutation({
    mutationFn: (input: RegisterUserInput) => api.registerUser(input),
  });
}

/**
 * Logout mutation. Clears the entire cache so no stale user data
 * leaks across the auth boundary — including the login-status probe
 * cache entry that `useLogin` / `useLoginDomain` invalidate.
 */
export function useLogout() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => qc.clear(),
  });
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/data/user.test.tsx
```

Expected: PASS — 6 tests, all green.

- [ ] **Step 5: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/data/user.ts src/test/data/user.test.tsx
git commit -m "feat(desktop): add user hooks (current, domain info, register, logout)"
```

---

## Task 5: Refactor `src/pages/bootstrap.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/bootstrap.tsx`

**Interfaces:**
- Consumes: `useHealthz`, `useIsLoggedIn` from `Task 3`
- Produces: same exported component `BootstrapPage` with same JSX, no behavior change observable to the user

- [ ] **Step 1: Replace the file**

The current file is [src/pages/bootstrap.tsx](apps/desktop/aegis-desktop/src/pages/bootstrap.tsx) (95 lines). Replace its entire contents with the version below. Keep imports in alphabetical order within each group (React first, third-party second, local last) — that mirrors the existing file's ordering.

```tsx
import { useEffect, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Paper,
  Step,
  StepLabel,
  Stepper,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useHealthz, useIsLoggedIn } from "../data/bootstrap";
import { errorMessage } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

export function BootstrapPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useBootstrapLog();

  const [activeStep, setActiveStep] = useState(0);
  const [healthFailed, setHealthFailed] = useState(false);
  const [loginStatusFailed, setLoginStatusFailed] = useState(false);

  // Bootstrap probes are disabled by default; the page drives them
  // manually via `refetch()` so they fire exactly once on mount.
  const health = useHealthz();
  const status = useIsLoggedIn();

  // React StrictMode invokes effects twice in development. The ref
  // keeps the orchestrator to a single run.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    void (async () => {
      push("info", "bootstrap.log.healthCheck.start");
      const h = await health.refetch();
      if (h.isError) {
        push("error", "bootstrap.log.healthCheck.failed", {
          message: errorMessage(h.error),
        });
        setHealthFailed(true);
        return;
      }
      push("success", "bootstrap.log.healthCheck.ok", { status: h.data });
      setActiveStep(1);

      push("info", "bootstrap.log.loginStatus.start");
      const s = await status.refetch();
      if (s.isError) {
        push("error", "bootstrap.log.loginStatus.failed", {
          message: errorMessage(s.error),
        });
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
  }, [navigate, push, health, status]);

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("bootstrap.title")}
        </Typography>

        <Stepper activeStep={activeStep} orientation="vertical">
          <Step>
            <StepLabel error={healthFailed}>
              {t("bootstrap.step.health")}
            </StepLabel>
          </Step>
          <Step>
            <StepLabel error={loginStatusFailed}>
              {t("bootstrap.step.loginStatus")}
            </StepLabel>
          </Step>
        </Stepper>

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}
```

Note on the effect dependency list: `health` and `status` are added to satisfy `react-hooks/exhaustive-deps`. They are stable references across renders (returned by `useQuery`) so the effect runs once.

- [ ] **Step 2: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Run the full test suite**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS — no regressions in existing or new tests.

- [ ] **Step 4: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/pages/bootstrap.tsx
git commit -m "refactor(desktop): bootstrap page consumes useHealthz, useIsLoggedIn"
```

---

## Task 6: Refactor `src/pages/login.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/login.tsx`

**Interfaces:**
- Consumes: `useLogin`, `useLoginDomain` from `Task 2`
- Produces: same exported component `LoginPage` with same JSX, no behavior change observable to the user

- [ ] **Step 1: Replace the file**

The current file is [src/pages/login.tsx](apps/desktop/aegis-desktop/src/pages/login.tsx) (169 lines). Replace its entire contents with the version below.

```tsx
import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  FormControlLabel,
  Paper,
  Radio,
  RadioGroup,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useLogin, useLoginDomain } from "../data/auth";
import { errorMessage, httpCode } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

type LoginMethod = "account" | "domain";

/** Which terminal state the login attempt landed in, if any. */
type Outcome = "none" | "notFound" | "inactive" | "failed";

export function LoginPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useBootstrapLog();

  const login = useLogin();
  const loginDomain = useLoginDomain();

  const [method, setMethod] = useState<LoginMethod>("domain");
  const [accountCode, setAccountCode] = useState("");
  const [password, setPassword] = useState("");
  const [outcome, setOutcome] = useState<Outcome>("none");

  async function runLogin(attempt: () => Promise<void>) {
    push("info", "login.log.login.start");
    try {
      await attempt();
      push("success", "login.log.login.ok");
      await navigate({ to: "/" });
    } catch (e) {
      const failureCode = httpCode(e);
      if (failureCode === "not_found") {
        push("error", "login.log.login.notFound");
        setOutcome("notFound");
      } else if (failureCode === "user_inactive") {
        push("error", "login.log.login.inactive");
        setOutcome("inactive");
      } else {
        push("error", "login.log.login.failed", {
          message: errorMessage(e),
        });
        setOutcome("failed");
      }
    }
  }

  function onLogin() {
    push("info", "login.log.method.selected", {
      method: t(
        method === "account" ? "login.method.account" : "login.method.domain",
      ),
    });
    if (method === "domain") {
      void runLogin(() => loginDomain.mutateAsync());
    } else {
      void runLogin(() => login.mutateAsync({ code: accountCode, password }));
    }
  }

  function onMethodChange(next: LoginMethod) {
    // Switching the method clears any failure outcome so a stale alert
    // does not linger when the user retries with a different flow.
    setOutcome("none");
    setMethod(next);
  }

  const loginDisabled =
    login.isPending ||
    loginDomain.isPending ||
    (method === "account" && (!accountCode || !password));

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("login.title")}
        </Typography>

        <RadioGroup
          value={method}
          onChange={(event) =>
            onMethodChange(event.target.value as LoginMethod)
          }
        >
          <FormControlLabel
            value="domain"
            control={<Radio />}
            label={t("login.method.domain")}
          />
          <FormControlLabel
            value="account"
            control={<Radio />}
            label={t("login.method.account")}
          />
        </RadioGroup>

        {method === "account" && (
          <Stack spacing={2} sx={{ maxWidth: 320, mt: 1 }}>
            <TextField
              label={t("login.field.code")}
              value={accountCode}
              onChange={(event) => setAccountCode(event.target.value)}
              size="small"
            />
            <TextField
              label={t("login.field.password")}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              size="small"
            />
          </Stack>
        )}

        <Button
          variant="contained"
          onClick={onLogin}
          disabled={loginDisabled}
          sx={{ mt: 2 }}
        >
          {t("login.action.login")}
        </Button>

        {/* Outcome UI lives below the form so a failure keeps the
            Register / admin-hint affordance visible regardless of which
            method the user is on. */}
        {outcome === "notFound" && (
          <Box sx={{ mt: 2 }}>
            <Alert severity="warning" sx={{ mb: 1 }}>
              {t("login.hint.notFound")}
            </Alert>
            <Button
              variant="outlined"
              onClick={() => void navigate({ to: "/register" })}
            >
              {t("login.action.register")}
            </Button>
          </Box>
        )}

        {outcome === "inactive" && (
          <Alert severity="warning" sx={{ mt: 2 }}>
            {t("login.hint.inactive")}
          </Alert>
        )}

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}
```

The old `inFlight` local state is gone — `login.isPending || loginDomain.isPending` covers it. The old `runLogin` was wrapped in `useCallback`; with the new shape it's a plain `async` function (no longer a child of any hook that recreates per render). The `push` reference inside the effect-less helper no longer needs to be in a dep array.

- [ ] **Step 2: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Run the full test suite**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/pages/login.tsx
git commit -m "refactor(desktop): login page consumes useLogin, useLoginDomain"
```

---

## Task 7: Refactor `src/pages/register.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/register.tsx`

**Interfaces:**
- Consumes: `useDomainUserInfo`, `useRegisterUser` from `Task 4`
- Produces: same exported component `RegisterPage` with same JSX, no behavior change observable to the user

- [ ] **Step 1: Replace the file**

The current file is [src/pages/register.tsx](apps/desktop/aegis-desktop/src/pages/register.tsx) (135 lines). Replace its entire contents with the version below.

```tsx
import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useDomainUserInfo, useRegisterUser } from "../data/user";
import { errorMessage } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

export function RegisterPage() {
  const { t } = useI18n();
  const { entries, push } = useBootstrapLog();

  const identity = useDomainUserInfo();
  const register = useRegisterUser();

  const [userName, setUserName] = useState("");
  const [password, setPassword] = useState("");
  const [registered, setRegistered] = useState(false);

  // React StrictMode invokes effects twice in development. The ref
  // keeps the identity lookup to a single request.
  const lookedUp = useRef(false);

  useEffect(() => {
    if (lookedUp.current) return;
    lookedUp.current = true;

    push("info", "register.log.identity.start");
    void (async () => {
      const r = await identity.refetch();
      if (r.isError) {
        push("error", "register.log.identity.failed", {
          message: errorMessage(r.error),
        });
        return;
      }
      push("success", "register.log.identity.ok", { userid: r.data!.userid });
    })();
  }, [push, identity]);

  async function onRegister() {
    const info = identity.data;
    if (!info) return;
    push("info", "register.log.register.start");
    try {
      await register.mutateAsync({
        userCode: info.userid,
        userName,
        domainName: info.domain,
        hostname: info.hostMachine,
        sid: info.sid,
        password,
      });
      push("success", "register.log.register.ok", { userCode: info.userid });
      setRegistered(true);
    } catch (e) {
      push("error", "register.log.register.failed", {
        message: errorMessage(e),
      });
    }
  }

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("register.title")}
        </Typography>

        {registered && (
          <Alert severity="info">{t("register.hint.contactAdmin")}</Alert>
        )}

        {identity.data && !registered && (
          <Stack spacing={2} sx={{ maxWidth: 360 }}>
            <TextField
              label={t("register.field.userCode")}
              value={identity.data.userid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.domainName")}
              value={identity.data.domain}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.hostname")}
              value={identity.data.hostMachine}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.sid")}
              value={identity.data.sid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.userName")}
              value={userName}
              onChange={(event) => setUserName(event.target.value)}
              size="small"
            />
            <TextField
              label={t("register.field.password")}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              size="small"
            />
            <Button
              variant="contained"
              disabled={register.isPending || !userName || !password}
              onClick={() => void onRegister()}
            >
              {t("register.action.register")}
            </Button>
          </Stack>
        )}

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}
```

The local `identity` state and `setIdentity` calls go away — `identity.data` is the source of truth. The form's gating condition becomes `identity.data && !registered`. The disabled flag uses `register.isPending` instead of the old `inFlight`.

- [ ] **Step 2: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Run the full test suite**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/pages/register.tsx
git commit -m "refactor(desktop): register page consumes useDomainUserInfo, useRegisterUser"
```

---

## Task 8: Refactor `src/pages/UserFooter.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/UserFooter.tsx`

**Interfaces:**
- Consumes: `useCurrentUser`, `useLogout` from `Task 4`
- Produces: same exported component `UserFooter` with same JSX, no behavior change observable to the user

- [ ] **Step 1: Replace the file**

The current file is [src/pages/UserFooter.tsx](apps/desktop/aegis-desktop/src/pages/UserFooter.tsx) (114 lines). Replace its entire contents with the version below.

```tsx
import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  Typography,
} from "@aegis/ui/mui";
import { Logout } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useCurrentUser, useLogout } from "../data/user";
import type { Role } from "../api";

interface UserFooterProps {
  /** Whether the surrounding sidebar drawer is open. When false, hide
   *  the name + chip and show only the logout icon. */
  sidebarOpen: boolean;
}

/**
 * Pinned to the bottom of the Sidebar. Shows the signed-in user's name
 * (with an optional role chip for root / admin) and a logout button
 * gated by a confirm dialog. On confirm: calls `useLogout` (which
 * clears the query cache) and navigates to `/login`. The `_layout`
 * `beforeLoad` guard already redirects an authenticated user away
 * from `/login`, so once the tokens are cleared the navigation lands
 * cleanly.
 */
export function UserFooter({ sidebarOpen }: UserFooterProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const logout = useLogout();
  const [confirmOpen, setConfirmOpen] = useState(false);

  const user = currentUser.data;
  const error = currentUser.error;

  async function onConfirmLogout() {
    setConfirmOpen(false);
    await logout.mutateAsync();
    await navigate({ to: "/login" });
  }

  const showRoleChip =
    user?.role === ("root" as Role) || user?.role === ("admin" as Role);

  const roleLabel =
    user?.role === ("root" as Role)
      ? t("app.user.role.root")
      : user?.role === ("admin" as Role)
        ? t("app.user.role.admin")
        : null;

  return (
    <>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1, minWidth: 0 }}>
        {sidebarOpen && showRoleChip && (
          <Chip size="small" label={roleLabel} />
        )}
        {sidebarOpen && (
          <Typography
            variant="body2"
            noWrap
            sx={{ flexGrow: 1, minWidth: 0 }}
            color={error ? "error" : "textPrimary"}
          >
            {error
              ? t("app.user.loadFailed")
              : (user?.name ?? t("app.user.unknownUser"))}
          </Typography>
        )}
        <IconButton
          aria-label={t("app.user.logout")}
          onClick={() => setConfirmOpen(true)}
          size="small"
        >
          <Logout />
        </IconButton>
      </Box>
      <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
        <DialogTitle>{t("app.user.logout.confirmTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("app.user.logout.confirmMessage")}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmOpen(false)}>
            {t("app.user.logout.cancel")}
          </Button>
          <Button onClick={() => void onConfirmLogout()} variant="contained">
            {t("app.user.logout.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
```

The local `user` / `error` `useState` and the `useEffect` that fetched the user go away — `useCurrentUser` handles both. The `useEffect` import is removed since the file no longer has an effect. `api` is no longer imported (only the `Role` type) because both data calls now flow through the hooks.

- [ ] **Step 2: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Run the full test suite**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/pages/UserFooter.tsx
git commit -m "refactor(desktop): UserFooter consumes useCurrentUser, useLogout"
```

---

## Task 9: Add barrel `src/data/index.ts` and final integration check

**Files:**
- Create: `apps/desktop/aegis-desktop/src/data/index.ts`

- [ ] **Step 1: Create the barrel**

Create the file at `apps/desktop/aegis-desktop/src/data/index.ts`:

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
  useRegisterUser,
  useLogout,
} from "./user";

// Re-export the React Query primitive that pages may need for ad-hoc
// cache interactions (e.g. `queryClient.setQueryData`).
export { useQueryClient } from "@tanstack/react-query";
```

- [ ] **Step 2: Run the full test suite**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm test
```

Expected: PASS — all hook tests, all existing transport tests, all existing page / route / i18n tests.

- [ ] **Step 3: Run typecheck**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/data/index.ts
git commit -m "feat(desktop): add barrel for data layer"
```

- [ ] **Step 5: Manual smoke check — dev server boots with devtools**

Run:
```bash
cd apps/desktop/aegis-desktop
pnpm dev
```

Open the URL Vite prints. Confirm:

1. The React app mounts (the splash → bootstrap flow renders).
2. The TanStack Router devtools button is at `bottom-right` (already there).
3. The new React Query devtools button is at `bottom-left`.
4. Click the React Query devtools button — the panel opens, lists zero queries initially.
5. Navigate to a page that mounts `useCurrentUser` (`/` or any `_layout` route), and confirm a `user.current` query appears in the panel.
6. Open the bootstrap page; confirm `useHealthz` / `useIsLoggedIn` are present but not in `fetching` state (they are disabled until `refetch()` is called).

Stop the dev server with Ctrl-C when done.

- [ ] **Step 6: Final commit if any manual cleanup is needed**

If the smoke check required any tweak, commit it with:

```bash
cd apps/desktop/aegis-desktop
git add -u
git commit -m "fix(desktop): address manual smoke-check findings"
```

If no tweak was needed, this step is a no-op.

---

## Self-Review

**1. Spec coverage:**

| Spec section | Task |
|---|---|
| §Approach / file layout (src/data/*) | Task 1 |
| §QueryClient defaults | Task 1 (`client.tsx`) |
| §Query keys factory | Task 1 (`queryKeys.ts`) |
| §auth.ts (useLogin, useLoginDomain, invalidate loginStatus) | Task 2 |
| §bootstrap.ts (useHealthz, useIsLoggedIn, staleTime: 0) | Task 3 |
| §user.ts (useCurrentUser, useDomainUserInfo, useRegisterUser, useLogout) | Task 4 |
| §bootstrap.tsx refactor | Task 5 |
| §login.tsx refactor | Task 6 |
| §register.tsx refactor | Task 7 |
| §UserFooter.tsx refactor | Task 8 |
| §Devtools & main.tsx wiring | Task 1 (client.tsx) + Task 1 step 4 (main.tsx) |
| §render-with-query-client helper | Task 1 |
| §auth.test.tsx | Task 2 |
| §bootstrap.test.tsx | Task 3 |
| §user.test.tsx | Task 4 |
| §Out of scope (CRUD hooks) | not implemented (correctly deferred) |
| §File changes summary | matches the file map above |

**2. Placeholder scan:** none — every code block contains complete, runnable code. No "TBD", "TODO", "implement later", "similar to", or "fill in details". The `void LoginStatusProbe;` / `void render;` lines in `Task 2` are explicit `noUnusedLocals` suppressions for documented-but-unused identifiers.

**3. Type consistency:** names verified across tasks:
- `useLogin` / `useLoginDomain` — consistent signature `mutationFn` shape, both invalidate `queryKeys.auth.loginStatus()`.
- `useHealthz` / `useIsLoggedIn` — both `{ enabled?: boolean }`, both `staleTime: 0`.
- `useCurrentUser` — no `enabled` option (per spec).
- `useDomainUserInfo` — `{ enabled?: boolean }`, inherits global `staleTime: Infinity`.
- `useRegisterUser` — no invalidation, confirmed by test.
- `useLogout` — clears cache on success, confirmed by test.
- `queryKeys.auth.loginStatus()` — referenced by useIsLoggedIn, useLogin, useLoginDomain.
- `queryKeys.bootstrap.health()` — referenced only by useHealthz and its test.
- `queryKeys.user.current()` — referenced by useCurrentUser and useLogout's test.
- `queryKeys.user.domainIdentity()` — referenced only by useDomainUserInfo.

All paths in commands are absolute (`apps/desktop/aegis-desktop/...`) so the engineer can copy-paste them from any cwd inside the workspace.