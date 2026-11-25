# Aegis Desktop Splashscreen (Login + Register) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pre-app splashscreen that walks the user through a server health check, a login-method choice, and login — routing them to a registration page when they have no account yet — printing a log line for every step.

**Architecture:** Two new TanStack routes (`/splash`, `/register`) as siblings of the existing `_layout` pathless route, so they render without the Sidebar. A `beforeLoad` guard on `_layout` redirects unauthenticated users to `/splash`. Both pages share an app-local `SplashLog` component whose entries store i18n *keys* (not translated text) so they re-translate on locale change, plus an `api/error.ts` helper that narrows Tauri's rejection value to the `ApiError` union. Two Rust changes land first: `login_domain` sources its user code from the OS identity instead of a parameter, and `Identity` gains camelCase serialization so its TypeScript type stops lying.

**Tech Stack:** React 19, TypeScript 5.8, TanStack Router (file-based, plugin-generated route tree), MUI 9.2 (re-exported through `@aegis/ui/mui`), `@aegis/ui/i18n` for translation, Vitest + Testing Library + `@testing-library/user-event`, Tauri v2 (Rust backend), pnpm workspaces.

**Spec:** `docs/superpowers/specs/2026-08-13-aegis-desktop-splashscreen-login-register-design.md`

## Global Constraints

- **TDD.** Write the failing test, run it, watch it fail for the right reason, then implement. Every task follows this cycle.
- **Failures stop the flow.** When any step fails, log the error and render nothing further. No retry buttons, no auto-recovery. The single exception is a `not_found` login failure, which renders a Register button.
- **Log entries store translation keys, never translated strings.** `push(level, key, params?)`. Translation happens at render time in `SplashLog`.
- **i18n key parity is enforced at compile time.** `zhCN.ts` ends with `satisfies Record<keyof typeof en, string>`. Every key added to `en.ts` MUST also be added to `zhCN.ts` or `pnpm typecheck` fails in `lib/packages/ui`.
- **MUI is imported from `@aegis/ui/mui`**, never from `@mui/material` directly. The desktop app does not depend on `@mui/material` itself.
- **Interpolation syntax is `{name}`** — see `resolveMessage`/`interpolate` in `lib/packages/ui/src/i18n/registry.ts`.
- **`routeTree.gen.ts` is generated and committed.** After adding or removing a file in `src/routes/`, regenerate it with `pnpm exec vite build` from `apps/desktop/aegis-desktop` and commit the result. Never hand-edit it.
- **The server wire contract does not change.** `/api/auth/login-domain` still receives `code`, `domain_name`, `hostname`, `sid`.
- **Working directories.** Frontend commands run from `apps/desktop/aegis-desktop`. Shared-UI commands run from `lib/packages/ui`. Rust commands run from `apps/desktop/aegis-desktop/src-tauri`.

## File Structure

**Rust (`apps/desktop/aegis-desktop/src-tauri/src/`)**

| File | Responsibility | Change |
|---|---|---|
| `system/identity.rs` | OS identity lookup + wire shape | Modify: add `rename_all = "camelCase"`, update serialization test |
| `http/auth.rs` | Auth HTTP calls | Modify: `login_domain` drops `code` param, propagates the real identity error |
| `commands/auth.rs` | Tauri command surface for auth | Modify: `login_domain` drops `code` param |

**Frontend (`apps/desktop/aegis-desktop/src/`)**

| File | Responsibility | Change |
|---|---|---|
| `api/index.ts` | Tauri command wrappers | Modify: `loginDomain()` takes no argument |
| `api/error.ts` | Narrow `unknown` rejections to `ApiError`; extract code/message | Create |
| `components/SplashLog/types.ts` | `LogLevel`, `LogEntry`, `PushLog` | Create |
| `components/SplashLog/useSplashLog.ts` | Append-only log state | Create |
| `components/SplashLog/SplashLog.tsx` | Renders entries, translating at render time | Create |
| `components/SplashLog/index.ts` | Barrel | Create |
| `pages/splash.tsx` | Health check → method choice → login | Create |
| `pages/register.tsx` | Identity lookup → form → register | Create |
| `routes/splash.tsx` | Route binding for `/splash` | Create |
| `routes/register.tsx` | Route binding for `/register` | Create |
| `routes/_layout/route.tsx` | App layout | Modify: add `beforeLoad` auth guard |
| `routes/routeTree.gen.ts` | Generated route tree | Regenerated |
| `test/tauri-mock.ts` | Mock Tauri commands by name, not call order | Create |

**Shared UI (`lib/packages/ui/src/i18n/locales/`)**

| File | Change |
|---|---|
| `en.ts` | Add `splash.*` and `register.*` keys |
| `zhCN.ts` | Add the same keys, translated |

**Task order rationale:** Rust first (Tasks 1–2), because the TS wrapper signature in Task 3 depends on the command signature. Then leaf utilities with no dependants (Tasks 4–6), then the pages that consume them (Tasks 7–8), then the guard last (Task 9) — the guard's redirect target `/splash` must exist before the guard can send anyone to it.

---

### Task 1: `Identity` serializes as camelCase

Tauri v2 auto-converts camelCase command *arguments* from JS to snake_case Rust parameters, but does **not** rename *return values*. `Identity` derives `Serialize` with no `rename_all`, so `get_domain_user_info` returns `{"host_machine": ...}` while `src/api/types.ts` declares `hostMachine`. Reading `identity.hostMachine` yields `undefined` today. Task 8's register page cannot work until this is fixed.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs`
- Modify: `apps/desktop/aegis-desktop/src/api/types.ts` (comment only)
- Test: `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces: `Identity` JSON wire shape `{ domain, hostMachine, sid, userid }`. Task 8 reads `identity.hostMachine`.

- [ ] **Step 1: Update the failing test**

In `src-tauri/src/system/identity.rs`, replace the existing `identity_serializes_with_snake_case_keys` test with:

```rust
    #[test]
    fn identity_serializes_with_camel_case_keys() {
        let id = Identity {
            domain: "corp.example".into(),
            host_machine: "ws-001".into(),
            sid: "S-1-5-21-1234".into(),
            userid: "alice".into(),
        };
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(
            json,
            r#"{"domain":"corp.example","hostMachine":"ws-001","sid":"S-1-5-21-1234","userid":"alice"}"#
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop/src-tauri
cargo test -p aegis-desktop identity_serializes_with_camel_case_keys
```

Expected: FAIL — assertion mismatch, left contains `"host_machine"`.

- [ ] **Step 3: Add the serde rename**

In `src-tauri/src/system/identity.rs`, add the attribute to the struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub domain: String,
    pub host_machine: String,
    pub sid: String,
    pub userid: String,
}
```

Leave the doc comment above the struct in place and append to it:

```rust
/// Wire form is camelCase (`hostMachine`) so it matches the TypeScript
/// `Identity` interface in `src/api/types.ts`. Tauri does not rename
/// command *return* values the way it renames arguments, so the rename
/// has to happen here.
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop/src-tauri
cargo test -p aegis-desktop identity_serializes_with_camel_case_keys
```

Expected: PASS.

- [ ] **Step 5: Correct the misleading comment in `api/types.ts`**

In `apps/desktop/aegis-desktop/src/api/types.ts`, replace the comment block directly above `export interface Identity` (currently lines 27–29) with:

```ts
// Mirrors `system::identity::Identity` in src-tauri. That struct carries
// `#[serde(rename_all = "camelCase")]`, so unlike the other response
// shapes in this file its JSON keys really are camelCase — `hostMachine`,
// not `host_machine`.
```

Leave the file-top comment and every other interface untouched.

- [ ] **Step 6: Run the full Rust suite and the frontend typecheck**

```bash
cd apps/desktop/aegis-desktop/src-tauri && cargo test -p aegis-desktop
cd apps/desktop/aegis-desktop && pnpm typecheck
```

Expected: both PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs \
        apps/desktop/aegis-desktop/src/api/types.ts
git commit -m "fix(desktop): serialize Identity as camelCase to match its TS type"
```

---

### Task 2: `login_domain` drops its `code` parameter

The user code passed to `login_domain` was always going to be the OS identity's `userid`, so requiring the frontend to supply it duplicated a value the Rust side already reads. The outbound HTTP body is unchanged — only the source of `code` moves.

This task also removes a `map_err` that swallows genuine Windows lookup failures (returned by `identity::current()` as `ApiError::Store`) into a misleading `NotImplemented { detail: "loginDomain requires Windows" }`.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs:61-82`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/auth.rs:20-26`
- Test: `apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: `identity::current() -> Result<Identity, ApiError>` (unchanged).
- Produces: `http::auth::login_domain(c: &HttpClient) -> Result<(), ApiError>` and Tauri command `login_domain` taking no arguments beyond injected state. Task 3 depends on the zero-argument command.

- [ ] **Step 1: Write the failing test**

Append this test inside the existing `mod tests` block at the bottom of `src-tauri/src/http/auth.rs` (after `logout_clears_tokens`). It is gated to non-Windows because on Windows `identity::current()` performs a real OS lookup:

```rust
    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn login_domain_propagates_the_identity_error() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let c = HttpClient::new(server.uri(), store.clone());

        // No `code` argument: the user code now comes from the OS identity.
        let err = login_domain(&c).await.unwrap_err();

        // The error is whatever `identity::current()` returned, not a
        // rewritten one. On non-Windows that is `NotImplemented`.
        match err {
            ApiError::NotImplemented { detail } => {
                assert!(detail.contains("Windows"), "got {detail}");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
        assert!(store.access_token().await.unwrap().is_none());
    }
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop/src-tauri
cargo test -p aegis-desktop login_domain_propagates_the_identity_error
```

Expected: FAIL to compile — `login_domain` takes 2 arguments but 1 was supplied.

> On Windows this test is compiled out, so it will report "0 tests run". That is expected; proceed to Step 3 and rely on Step 5's full-suite run plus the manual check in Task 7.

- [ ] **Step 3: Change `http::auth::login_domain`**

In `src-tauri/src/http/auth.rs`, replace the function signature and first statement (currently lines 61–71):

```rust
/// Log in using the OS-level domain identity. The user code is taken from
/// `identity::current().userid` — the caller supplies nothing.
pub async fn login_domain(c: &HttpClient) -> Result<(), ApiError> {
    let id = identity::current()?;
    let body = LoginDomainRequest {
        code: id.userid,
        domain_name: id.domain,
        hostname: id.host_machine,
        sid: id.sid,
    };
```

Everything from `let bytes = c` onward in that function is unchanged.

Note the `?` replacing the previous `.map_err(|_| ApiError::NotImplemented { detail: "loginDomain requires Windows" })`. `identity::current()` already returns `ApiError`, so the conversion was pure information loss.

- [ ] **Step 4: Change the Tauri command**

In `src-tauri/src/commands/auth.rs`, replace the whole `login_domain` command:

```rust
#[tauri::command]
pub async fn login_domain(client: State<'_, HttpClient>) -> Result<(), ApiError> {
    auth::login_domain(&client).await
}
```

No change to `src-tauri/src/lib.rs` — the handler registration is by path, not by arity.

- [ ] **Step 5: Run the full Rust suite**

```bash
cd apps/desktop/aegis-desktop/src-tauri
cargo test -p aegis-desktop
```

Expected: PASS, no warnings about an unused `code` binding.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/auth.rs
git commit -m "refactor(desktop): source login_domain user code from OS identity"
```

---

### Task 3: `api.loginDomain()` takes no argument

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/api/index.ts:34-35`
- Test: `apps/desktop/aegis-desktop/src/test/api.test.ts:24-28`

**Interfaces:**
- Consumes: Tauri command `login_domain` with no arguments (Task 2).
- Produces: `api.loginDomain(): Promise<void>`. Task 7's splash page calls it.

- [ ] **Step 1: Update the failing test**

In `src/test/api.test.ts`, replace the existing `loginDomain` test (lines 24–28) with:

```ts
  it("loginDomain -> invoke('login_domain') with no args", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.loginDomain();
    expect(mockInvoke).toHaveBeenCalledWith("login_domain");
  });
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/api.test.ts -t "loginDomain"
```

Expected: FAIL — `invoke` was called with `("login_domain", { code: undefined })`.

- [ ] **Step 3: Change the wrapper**

In `src/api/index.ts`, replace lines 34–35:

```ts
  loginDomain: (): Promise<void> => call<void>("login_domain"),
```

No change to the `call` helper — it already invokes without an args object when `args === undefined`.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/api.test.ts
```

Expected: PASS (all tests in the file).

- [ ] **Step 5: Typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm typecheck
```

Expected: PASS. `src/pages/home.tsx` does not call `loginDomain`, so nothing else breaks.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/api/index.ts \
        apps/desktop/aegis-desktop/src/test/api.test.ts
git commit -m "refactor(desktop): drop the code argument from api.loginDomain"
```

---

### Task 4: `api/error.ts` — narrow Tauri rejections to `ApiError`

Tauri rejects with the serialized `ApiError` object (`{ kind, ... }`). Both new pages branch on the HTTP `code`, so the narrowing lives in one place instead of being duplicated. A non-`ApiError` rejection (a thrown JS `Error`, a string) degrades to `{ kind: "network", message: String(e) }` so `httpCode` returns `null` and callers take the generic-failure branch.

**Files:**
- Create: `apps/desktop/aegis-desktop/src/api/error.ts`
- Test: `apps/desktop/aegis-desktop/src/test/api/error.test.ts`

**Interfaces:**
- Consumes: `ApiError` from `src/api/types.ts`.
- Produces:
  - `toApiError(e: unknown): ApiError`
  - `httpCode(e: unknown): string | null`
  - `errorMessage(e: unknown): string`

  Tasks 7 and 8 import `httpCode` and `errorMessage`.

- [ ] **Step 1: Write the failing test**

Create `src/test/api/error.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import { errorMessage, httpCode, toApiError } from "../../api/error";

describe("toApiError", () => {
  it("passes through an object carrying a string `kind`", () => {
    const e = { kind: "refreshFailed" };
    expect(toApiError(e)).toEqual({ kind: "refreshFailed" });
  });

  it("wraps a thrown Error as a network error", () => {
    expect(toApiError(new Error("boom"))).toEqual({
      kind: "network",
      message: "Error: boom",
    });
  });

  it("wraps a plain string as a network error", () => {
    expect(toApiError("nope")).toEqual({ kind: "network", message: "nope" });
  });

  it("wraps null as a network error", () => {
    expect(toApiError(null)).toEqual({ kind: "network", message: "null" });
  });

  it("wraps an object with a non-string kind as a network error", () => {
    const e = toApiError({ kind: 42 });
    expect(e.kind).toBe("network");
  });
});

describe("httpCode", () => {
  it("returns the code for an http error", () => {
    expect(
      httpCode({ kind: "http", status: 404, code: "not_found", message: "no" }),
    ).toBe("not_found");
  });

  it("returns user_inactive for an inactive-account error", () => {
    expect(
      httpCode({ kind: "http", status: 403, code: "user_inactive", message: "x" }),
    ).toBe("user_inactive");
  });

  it("returns null for a network error", () => {
    expect(httpCode({ kind: "network", message: "dns" })).toBeNull();
  });

  it("returns null for a non-ApiError rejection", () => {
    expect(httpCode(new Error("boom"))).toBeNull();
  });
});

describe("errorMessage", () => {
  it("formats a network error", () => {
    expect(errorMessage({ kind: "network", message: "dns" })).toBe("dns");
  });

  it("formats an http error as code and message", () => {
    expect(
      errorMessage({ kind: "http", status: 401, code: "invalid_credentials", message: "bad" }),
    ).toBe("invalid_credentials: bad");
  });

  it("formats a refreshFailed error", () => {
    expect(errorMessage({ kind: "refreshFailed" })).toBe("refresh failed");
  });

  it("formats a notImplemented error using its detail", () => {
    expect(
      errorMessage({ kind: "notImplemented", detail: "requires Windows" }),
    ).toBe("requires Windows");
  });

  it("formats a store error", () => {
    expect(errorMessage({ kind: "store", message: "locked" })).toBe("locked");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/api/error.test.ts
```

Expected: FAIL — cannot resolve `../../api/error`.

- [ ] **Step 3: Write the implementation**

Create `src/api/error.ts`:

```ts
// Tauri rejects a command with the serialized `ApiError` object, typed as
// `unknown` on the JS side. These helpers narrow that value once so pages
// do not each re-implement the `kind === "http"` dance.

import type { ApiError } from "./types";

/**
 * Narrow an unknown rejection value to `ApiError`. Anything that is not a
 * tagged `ApiError` (a thrown JS `Error`, a string, null) degrades to a
 * `network` error carrying its stringified form, so callers always get a
 * usable message and `httpCode` returns null for it.
 */
export function toApiError(e: unknown): ApiError {
  if (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    typeof (e as { kind: unknown }).kind === "string"
  ) {
    return e as ApiError;
  }
  return { kind: "network", message: String(e) };
}

/**
 * The server's stable, machine-readable error token (`not_found`,
 * `user_inactive`, `invalid_credentials`, ...) for HTTP errors, or null
 * for every other failure kind.
 */
export function httpCode(e: unknown): string | null {
  const err = toApiError(e);
  return err.kind === "http" ? err.code : null;
}

/** A human-readable one-line rendering of any failure, for the splash log. */
export function errorMessage(e: unknown): string {
  const err = toApiError(e);
  switch (err.kind) {
    case "network":
      return err.message;
    case "http":
      return `${err.code}: ${err.message}`;
    case "refreshFailed":
      return "refresh failed";
    case "notImplemented":
      return err.detail;
    case "store":
      return err.message;
  }
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/api/error.test.ts
```

Expected: PASS, 14 tests.

- [ ] **Step 5: Typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/api/error.ts \
        apps/desktop/aegis-desktop/src/test/api/error.test.ts
git commit -m "feat(desktop): add ApiError narrowing helpers"
```

---

### Task 5: i18n keys for the splash and register pages

Both locale files must gain the same keys. `zhCN.ts` closes with `satisfies Record<keyof typeof en, string>`, so a missing key is a typecheck failure in `lib/packages/ui`, not a runtime fallback.

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`
- Test: `lib/packages/ui/src/i18n/registry.test.ts`

**Interfaces:**
- Produces: the `TranslationKey` union gains every `splash.*` and `register.*` key below. Tasks 6–8 pass these keys to `t()` and to `push()`.

- [ ] **Step 1: Write the failing test**

Append to the existing top-level `describe` in `lib/packages/ui/src/i18n/registry.test.ts` (match the file's existing import style — it already imports `translate` and/or `getCatalog`; add whichever is missing):

```ts
describe('splash and register catalogs', () => {
  const splashAndRegisterKeys = [
    'splash.title',
    'splash.step.health',
    'splash.step.method',
    'splash.step.credentials',
    'splash.method.account',
    'splash.method.domain',
    'splash.method.continue',
    'splash.field.code',
    'splash.field.password',
    'splash.action.login',
    'splash.action.loginDomain',
    'splash.action.register',
    'splash.hint.notFound',
    'splash.hint.inactive',
    'splash.log.healthCheck.start',
    'splash.log.healthCheck.ok',
    'splash.log.healthCheck.failed',
    'splash.log.method.selected',
    'splash.log.login.start',
    'splash.log.login.ok',
    'splash.log.login.failed',
    'splash.log.login.notFound',
    'splash.log.login.inactive',
    'register.title',
    'register.field.userCode',
    'register.field.domainName',
    'register.field.hostname',
    'register.field.sid',
    'register.field.userName',
    'register.field.password',
    'register.action.register',
    'register.hint.contactAdmin',
    'register.log.identity.start',
    'register.log.identity.ok',
    'register.log.identity.failed',
    'register.log.register.start',
    'register.log.register.ok',
    'register.log.register.failed',
  ] as const;

  it.each(splashAndRegisterKeys)('has a non-empty en and zh-CN message for %s', (key) => {
    expect(translate('en', key)).not.toBe(key);
    expect(translate('en', key).length).toBeGreaterThan(0);
    expect(translate('zh-CN', key)).not.toBe(key);
    expect(translate('zh-CN', key).length).toBeGreaterThan(0);
  });

  it('interpolates the message variable in a failure log line', () => {
    expect(translate('en', 'splash.log.login.failed', { message: 'boom' })).toContain(
      'boom',
    );
    expect(translate('zh-CN', 'splash.log.login.failed', { message: 'boom' })).toContain(
      'boom',
    );
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd lib/packages/ui
pnpm vitest run src/i18n/registry.test.ts
```

Expected: FAIL — `translate('en', 'splash.title')` returns the key itself, because `resolveMessage` falls back to the key when it is absent from every catalog.

- [ ] **Step 3: Add the English keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, insert before the closing `} as const;`:

```ts
  'splash.title': 'Welcome to Aegis',
  'splash.step.health': 'Server health check',
  'splash.step.method': 'Choose a login method',
  'splash.step.credentials': 'Sign in',
  'splash.method.account': 'Account and password',
  'splash.method.domain': 'Domain information',
  'splash.method.continue': 'Continue',
  'splash.field.code': 'Account',
  'splash.field.password': 'Password',
  'splash.action.login': 'Login',
  'splash.action.loginDomain': 'Login with domain',
  'splash.action.register': 'Register',
  'splash.hint.notFound': 'No account matches these credentials. You can register a new one.',
  'splash.hint.inactive': 'Your account is not active yet. Please contact your administrator.',
  'splash.log.healthCheck.start': 'Checking server health...',
  'splash.log.healthCheck.ok': 'Server is healthy: {status}',
  'splash.log.healthCheck.failed': 'Server health check failed: {message}',
  'splash.log.method.selected': 'Login method selected: {method}',
  'splash.log.login.start': 'Signing in...',
  'splash.log.login.ok': 'Login succeeded. Entering the app...',
  'splash.log.login.failed': 'Login failed: {message}',
  'splash.log.login.notFound': 'No account matches these credentials.',
  'splash.log.login.inactive': 'This account is not active yet.',

  'register.title': 'Register a new account',
  'register.field.userCode': 'User code',
  'register.field.domainName': 'Domain',
  'register.field.hostname': 'Hostname',
  'register.field.sid': 'SID',
  'register.field.userName': 'User name',
  'register.field.password': 'Password',
  'register.action.register': 'Register',
  'register.hint.contactAdmin':
    'Registration submitted. Contact your administrator to activate your account.',
  'register.log.identity.start': 'Reading domain user information...',
  'register.log.identity.ok': 'Domain user information loaded for {userid}.',
  'register.log.identity.failed': 'Could not read domain user information: {message}',
  'register.log.register.start': 'Submitting registration...',
  'register.log.register.ok': 'Registration succeeded for {userCode}.',
  'register.log.register.failed': 'Registration failed: {message}',
```

- [ ] **Step 4: Add the Simplified Chinese keys**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, insert before the closing `} satisfies Record<keyof typeof en, string>;`:

```ts
  'splash.title': '欢迎使用 Aegis',
  'splash.step.health': '服务器健康检查',
  'splash.step.method': '选择登录方式',
  'splash.step.credentials': '登录',
  'splash.method.account': '账号与密码',
  'splash.method.domain': '域信息',
  'splash.method.continue': '继续',
  'splash.field.code': '账号',
  'splash.field.password': '密码',
  'splash.action.login': '登录',
  'splash.action.loginDomain': '使用域信息登录',
  'splash.action.register': '注册',
  'splash.hint.notFound': '没有与此凭据匹配的账号，您可以注册一个新账号。',
  'splash.hint.inactive': '您的账号尚未启用，请联系管理员。',
  'splash.log.healthCheck.start': '正在检查服务器健康状态……',
  'splash.log.healthCheck.ok': '服务器状态正常：{status}',
  'splash.log.healthCheck.failed': '服务器健康检查失败：{message}',
  'splash.log.method.selected': '已选择登录方式：{method}',
  'splash.log.login.start': '正在登录……',
  'splash.log.login.ok': '登录成功，正在进入应用……',
  'splash.log.login.failed': '登录失败：{message}',
  'splash.log.login.notFound': '没有与此凭据匹配的账号。',
  'splash.log.login.inactive': '该账号尚未启用。',

  'register.title': '注册新账号',
  'register.field.userCode': '用户代码',
  'register.field.domainName': '域',
  'register.field.hostname': '主机名',
  'register.field.sid': 'SID',
  'register.field.userName': '用户名',
  'register.field.password': '密码',
  'register.action.register': '注册',
  'register.hint.contactAdmin': '注册已提交，请联系管理员启用您的账号。',
  'register.log.identity.start': '正在读取域用户信息……',
  'register.log.identity.ok': '已加载 {userid} 的域用户信息。',
  'register.log.identity.failed': '无法读取域用户信息：{message}',
  'register.log.register.start': '正在提交注册信息……',
  'register.log.register.ok': '{userCode} 注册成功。',
  'register.log.register.failed': '注册失败：{message}',
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cd lib/packages/ui
pnpm vitest run src/i18n/registry.test.ts
```

Expected: PASS.

- [ ] **Step 6: Run the full shared-UI suite and typecheck**

```bash
cd lib/packages/ui && pnpm test && pnpm typecheck
```

Expected: both PASS. If `pnpm typecheck` reports a missing property on the `satisfies` clause in `zhCN.ts`, a key is present in `en.ts` but absent from `zhCN.ts` — add it.

- [ ] **Step 7: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts \
        lib/packages/ui/src/i18n/registry.test.ts
git commit -m "feat(ui): add splash and register i18n keys for en and zh-CN"
```

---

### Task 6: `SplashLog` component and `useSplashLog` hook

App-local, not in `@aegis/ui`: only the splash and register pages use it. Entries store the translation **key and params**, not translated text, so switching language re-renders existing log lines in the new language.

**Files:**
- Create: `apps/desktop/aegis-desktop/src/components/SplashLog/types.ts`
- Create: `apps/desktop/aegis-desktop/src/components/SplashLog/useSplashLog.ts`
- Create: `apps/desktop/aegis-desktop/src/components/SplashLog/SplashLog.tsx`
- Create: `apps/desktop/aegis-desktop/src/components/SplashLog/index.ts`
- Test: `apps/desktop/aegis-desktop/src/test/components/splash-log.test.tsx`

**Interfaces:**
- Consumes: `TranslationKey`, `useI18n` from `@aegis/ui/i18n`; MUI from `@aegis/ui/mui`; the keys added in Task 5.
- Produces:
  - `type LogLevel = "info" | "success" | "error"`
  - `interface LogEntry { id: number; level: LogLevel; key: TranslationKey; params?: Record<string, string> }`
  - `type PushLog = (level: LogLevel, key: TranslationKey, params?: Record<string, string>) => void`
  - `useSplashLog(): { entries: LogEntry[]; push: PushLog }` — `push` is referentially stable (`useCallback` with `[]`), so it is safe in a `useEffect` dependency array.
  - `<SplashLog entries={...} />` — renders `data-testid="splash-log"` on the container and `data-testid="splash-log-<level>"` on each row.

  Tasks 7 and 8 import `{ SplashLog, useSplashLog }` from `../components/SplashLog`.

- [ ] **Step 1: Write the failing test**

Create `src/test/components/splash-log.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { SplashLog, useSplashLog } from "../../components/SplashLog";

afterEach(() => {
  cleanup();
});

/** Drives the hook from a button so the test exercises the real API. */
function Harness() {
  const { entries, push } = useSplashLog();
  const { setLocale } = useI18n();
  return (
    <>
      <button onClick={() => push("info", "splash.log.healthCheck.start")}>
        add-info
      </button>
      <button onClick={() => push("success", "splash.log.login.ok")}>
        add-success
      </button>
      <button
        onClick={() => push("error", "splash.log.login.failed", { message: "boom" })}
      >
        add-error
      </button>
      <button onClick={() => setLocale("zh-CN")}>to-zh</button>
      <SplashLog entries={entries} />
    </>
  );
}

function renderHarness() {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <Harness />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("SplashLog", () => {
  it("renders an empty container when there are no entries", () => {
    renderHarness();
    expect(screen.getByTestId("splash-log")).toBeEmptyDOMElement();
  });

  it("appends entries in order and keeps them", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));

    const rows = screen.getByTestId("splash-log").children;
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Checking server health...");
    expect(rows[1]).toHaveTextContent("Login succeeded. Entering the app...");
  });

  it("tags each entry with its level", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));
    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("splash-log-info")).toBeInTheDocument();
    expect(screen.getByTestId("splash-log-success")).toBeInTheDocument();
    expect(screen.getByTestId("splash-log-error")).toBeInTheDocument();
  });

  it("interpolates params into the message", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("splash-log-error")).toHaveTextContent(
      "Login failed: boom",
    );
  });

  it("re-translates existing entries when the locale changes", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    expect(screen.getByTestId("splash-log")).toHaveTextContent(
      "Checking server health...",
    );

    await act(async () => {
      await userEvent.click(screen.getByText("to-zh"));
    });

    expect(screen.getByTestId("splash-log")).toHaveTextContent(
      "正在检查服务器健康状态……",
    );
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/components/splash-log.test.tsx
```

Expected: FAIL — cannot resolve `../../components/SplashLog`.

- [ ] **Step 3: Write `types.ts`**

Create `src/components/SplashLog/types.ts`:

```ts
import type { TranslationKey } from "@aegis/ui/i18n";

export type LogLevel = "info" | "success" | "error";

/**
 * A single line in the splash log. It stores the translation *key* and its
 * params rather than translated text, so entries logged before a language
 * switch re-render in the new language.
 */
export interface LogEntry {
  id: number;
  level: LogLevel;
  key: TranslationKey;
  params?: Record<string, string>;
}

export type PushLog = (
  level: LogLevel,
  key: TranslationKey,
  params?: Record<string, string>,
) => void;
```

- [ ] **Step 4: Write `useSplashLog.ts`**

Create `src/components/SplashLog/useSplashLog.ts`:

```ts
import { useCallback, useRef, useState } from "react";

import type { LogEntry, PushLog } from "./types";

export interface SplashLogState {
  entries: LogEntry[];
  push: PushLog;
}

/**
 * Append-only log state for the splash and register pages.
 *
 * `push` is referentially stable, so callers can safely list it in a
 * `useEffect` dependency array without re-running the effect. Ids come
 * from a counter ref rather than the array index or a timestamp, so React
 * keys stay stable and unique.
 */
export function useSplashLog(): SplashLogState {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const nextId = useRef(0);

  const push = useCallback<PushLog>((level, key, params) => {
    const id = nextId.current;
    nextId.current += 1;
    setEntries((previous) => [...previous, { id, level, key, params }]);
  }, []);

  return { entries, push };
}
```

- [ ] **Step 5: Write `SplashLog.tsx`**

Create `src/components/SplashLog/SplashLog.tsx`:

```tsx
import { Paper, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { LogEntry, LogLevel } from "./types";

const LEVEL_COLOR: Record<LogLevel, string> = {
  info: "text.secondary",
  success: "success.main",
  error: "error.main",
};

export interface SplashLogProps {
  entries: LogEntry[];
}

/** Scrollable, append-only transcript of what the page has done so far. */
export function SplashLog({ entries }: SplashLogProps) {
  const { t } = useI18n();

  return (
    <Paper
      variant="outlined"
      data-testid="splash-log"
      sx={{ mt: 2, p: 1.5, maxHeight: 200, overflowY: "auto" }}
    >
      {entries.map((entry) => (
        <Typography
          key={entry.id}
          data-testid={`splash-log-${entry.level}`}
          variant="body2"
          sx={{ fontFamily: "monospace", color: LEVEL_COLOR[entry.level] }}
        >
          {t(entry.key, entry.params)}
        </Typography>
      ))}
    </Paper>
  );
}
```

- [ ] **Step 6: Write `index.ts`**

Create `src/components/SplashLog/index.ts`:

```ts
export { SplashLog } from "./SplashLog";
export type { SplashLogProps } from "./SplashLog";
export { useSplashLog } from "./useSplashLog";
export type { SplashLogState } from "./useSplashLog";
export type { LogEntry, LogLevel, PushLog } from "./types";
```

- [ ] **Step 7: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/components/splash-log.test.tsx
```

Expected: PASS, 5 tests.

- [ ] **Step 8: Typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm typecheck
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src/components/SplashLog \
        apps/desktop/aegis-desktop/src/test/components/splash-log.test.tsx
git commit -m "feat(desktop): add SplashLog component and useSplashLog hook"
```

---

### Task 7: Splash page at `/splash`

A vertical MUI `Stepper` with three steps. Step 0 runs `healthz` automatically on mount. Step 1 offers the method choice. Step 2 collects credentials and logs in. Every failure except `not_found` is a dead stop.

`StepContent` renders its children inside a `Collapse`. Even with `unmountOnExit`, the exit transition runs on a timer, so the previous step's content lingers in the DOM for the transition duration after `activeStep` changes — long enough to race a test assertion that the old content is gone. Each `StepContent` therefore also guards its body on `activeStep`, making removal synchronous with the state change. `unmountOnExit` stays as a belt-and-braces measure so nothing inactive is left behind for a screen reader.

> **MUI v9 API note:** `StepContent` has no `TransitionProps` prop — v9 moved to the slots pattern. Use `slotProps={{ transition: { unmountOnExit: true } }}`. (`TransitionProps` typechecks as an error: *Property 'TransitionProps' does not exist on type 'StepContentProps'*.)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/tauri-mock.ts`
- Create: `apps/desktop/aegis-desktop/src/pages/splash.tsx`
- Create: `apps/desktop/aegis-desktop/src/routes/splash.tsx`
- Modify: `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts` (regenerated, never hand-edited)
- Test: `apps/desktop/aegis-desktop/src/test/routes/splash.test.tsx`

**Interfaces:**
- Consumes: `api.healthz()`, `api.login(code, password)`, `api.loginDomain()` (Task 3); `httpCode`, `errorMessage` (Task 4); the `splash.*` keys (Task 5); `SplashLog`, `useSplashLog` (Task 6).
- Produces:
  - `SplashPage` exported from `src/pages/splash.tsx`.
  - Route `/splash`. Task 9's guard redirects here.
  - `mockCommands(handlers)` and `mockInvoke` exported from `src/test/tauri-mock.ts`. Tasks 8 and 9 import them.

- [ ] **Step 1: Write the shared Tauri mock helper**

Create `src/test/tauri-mock.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { Mock } from "vitest";

export type CommandHandlers = Record<
  string,
  (args?: Record<string, unknown>) => unknown
>;

/**
 * The mocked `invoke`. The importing test file must itself call
 * `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` — `vi.mock`
 * is hoisted per-file and cannot be applied from a helper module.
 */
export const mockInvoke = invoke as unknown as Mock;

/**
 * Dispatch mocked Tauri commands by name rather than by call order, so a
 * test does not break when an unrelated command joins a page's startup
 * sequence. A handler that throws becomes a rejected promise, which is how
 * a genuinely failing command surfaces to the caller.
 */
export function mockCommands(handlers: CommandHandlers): void {
  mockInvoke.mockImplementation(
    (cmd: string, args?: Record<string, unknown>) => {
      const handler = handlers[cmd];
      if (!handler) {
        return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
      }
      try {
        return Promise.resolve(handler(args));
      } catch (e) {
        return Promise.reject(e);
      }
    },
  );
}

/** Build the `ApiError` shape Tauri rejects an HTTP failure with. */
export function httpError(status: number, code: string, message = "err") {
  return { kind: "http", status, code, message };
}
```

- [ ] **Step 2: Write the failing test**

Create `src/test/routes/splash.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { httpError, mockCommands, mockInvoke } from "../tauri-mock";

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

afterEach(() => {
  cleanup();
});

function renderSplash() {
  return renderWithFullRouter({
    initialEntries: ["/splash"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

/** Health check passes, then advance to the credentials step. */
async function advanceToCredentials(method: "account" | "domain") {
  await screen.findByText(/Server is healthy/i);
  if (method === "domain") {
    await userEvent.click(screen.getByRole("radio", { name: /Domain information/i }));
  }
  await userEvent.click(screen.getByRole("button", { name: /Continue/i }));
}

describe("SplashPage — health check", () => {
  it("logs success and advances to the method step", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();

    expect(await screen.findByText(/Server is healthy: ok/i)).toBeInTheDocument();
    expect(
      await screen.findByRole("radio", { name: /Account and password/i }),
    ).toBeInTheDocument();
  });

  it("stops on the health step when healthz fails", async () => {
    mockCommands({
      healthz: () => {
        throw { kind: "network", message: "connection refused" };
      },
    });

    await renderSplash();

    expect(
      await screen.findByText(/Server health check failed: connection refused/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("radio", { name: /Account and password/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Continue/i })).not.toBeInTheDocument();
  });
});

describe("SplashPage — account login", () => {
  it("calls login and navigates home on success", async () => {
    mockCommands({ healthz: () => "ok", login: () => undefined, is_logged_in: () => true });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "alice");
    await userEvent.type(screen.getByLabelText(/Password/i), "secret");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login", {
      code: "alice",
      password: "secret",
    });
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("offers a Register button when the account is not found", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(404, "not_found", "no such user");
      },
      get_domain_user_info: () => {
        throw { kind: "notImplemented", detail: "requires Windows" };
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "ghost");
    await userEvent.type(screen.getByLabelText(/Password/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/No account matches these credentials\./i),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /Register/i }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/register"));
  });

  it("shows the contact-administrator hint for an inactive account", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(403, "user_inactive", "inactive");
      },
      is_logged_in: () => true,
    });

    await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "bob");
    await userEvent.type(screen.getByLabelText(/Password/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/not active yet\. Please contact your administrator/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
  });

  it("logs the message and stops for any other failure", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(401, "invalid_credentials", "bad password");
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "alice");
    await userEvent.type(screen.getByLabelText(/Password/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: invalid_credentials: bad password/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/splash");
  });
});

describe("SplashPage — domain login", () => {
  it("calls login_domain with no arguments and navigates home", async () => {
    mockCommands({
      healthz: () => "ok",
      login_domain: () => undefined,
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("domain");

    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Login with domain/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login_domain");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("logs a notImplemented failure and stops", async () => {
    mockCommands({
      healthz: () => "ok",
      login_domain: () => {
        throw { kind: "notImplemented", detail: "loginDomain requires Windows" };
      },
      is_logged_in: () => true,
    });

    await renderSplash();
    await advanceToCredentials("domain");

    await userEvent.click(screen.getByRole("button", { name: /Login with domain/i }));

    expect(
      await screen.findByText(/Login failed: loginDomain requires Windows/i),
    ).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/splash.test.tsx
```

Expected: FAIL — the router has no `/splash` route, so nothing renders and `findByText(/Server is healthy/)` times out.

- [ ] **Step 4: Write the page**

Create `src/pages/splash.tsx`:

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
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
  Step,
  StepContent,
  StepLabel,
  Stepper,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../api";
import { errorMessage, httpCode } from "../api/error";
import { SplashLog, useSplashLog } from "../components/SplashLog";

type LoginMethod = "account" | "domain";

/** Which terminal state the login attempt landed in, if any. */
type Outcome = "none" | "notFound" | "inactive" | "failed";

export function SplashPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useSplashLog();

  const [activeStep, setActiveStep] = useState(0);
  const [healthFailed, setHealthFailed] = useState(false);
  const [method, setMethod] = useState<LoginMethod>("account");
  const [accountCode, setAccountCode] = useState("");
  const [password, setPassword] = useState("");
  const [inFlight, setInFlight] = useState(false);
  const [outcome, setOutcome] = useState<Outcome>("none");

  // React StrictMode invokes effects twice in development. The ref keeps
  // the health check to a single request.
  const healthStarted = useRef(false);

  useEffect(() => {
    if (healthStarted.current) return;
    healthStarted.current = true;

    void (async () => {
      push("info", "splash.log.healthCheck.start");
      try {
        const status = await api.healthz();
        push("success", "splash.log.healthCheck.ok", { status });
        setActiveStep(1);
      } catch (e) {
        push("error", "splash.log.healthCheck.failed", {
          message: errorMessage(e),
        });
        setHealthFailed(true);
      }
    })();
  }, [push]);

  const runLogin = useCallback(
    async (attempt: () => Promise<void>) => {
      setInFlight(true);
      setOutcome("none");
      push("info", "splash.log.login.start");
      try {
        await attempt();
        push("success", "splash.log.login.ok");
        await navigate({ to: "/" });
      } catch (e) {
        const failureCode = httpCode(e);
        if (failureCode === "not_found") {
          push("error", "splash.log.login.notFound");
          setOutcome("notFound");
        } else if (failureCode === "user_inactive") {
          push("error", "splash.log.login.inactive");
          setOutcome("inactive");
        } else {
          push("error", "splash.log.login.failed", { message: errorMessage(e) });
          setOutcome("failed");
        }
      } finally {
        setInFlight(false);
      }
    },
    [navigate, push],
  );

  function onContinue() {
    push("info", "splash.log.method.selected", {
      method: t(method === "account" ? "splash.method.account" : "splash.method.domain"),
    });
    setActiveStep(2);
  }

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("splash.title")}
        </Typography>

        <Stepper activeStep={activeStep} orientation="vertical">
          <Step>
            <StepLabel error={healthFailed}>{t("splash.step.health")}</StepLabel>
          </Step>

          <Step>
            <StepLabel>{t("splash.step.method")}</StepLabel>
            <StepContent slotProps={{ transition: { unmountOnExit: true } }}>
              {activeStep === 1 && (
                <>
                  <RadioGroup
                    value={method}
                    onChange={(event) => setMethod(event.target.value as LoginMethod)}
                  >
                    <FormControlLabel
                      value="account"
                      control={<Radio />}
                      label={t("splash.method.account")}
                    />
                    <FormControlLabel
                      value="domain"
                      control={<Radio />}
                      label={t("splash.method.domain")}
                    />
                  </RadioGroup>
                  <Button variant="contained" onClick={onContinue} sx={{ mt: 1 }}>
                    {t("splash.method.continue")}
                  </Button>
                </>
              )}
            </StepContent>
          </Step>

          <Step>
            <StepLabel error={outcome !== "none"}>
              {t("splash.step.credentials")}
            </StepLabel>
            <StepContent slotProps={{ transition: { unmountOnExit: true } }}>
              {activeStep === 2 && (
                <>
                  {method === "account" ? (
                    <Stack spacing={2} sx={{ maxWidth: 320 }}>
                      <TextField
                        label={t("splash.field.code")}
                        value={accountCode}
                        onChange={(event) => setAccountCode(event.target.value)}
                        size="small"
                      />
                      <TextField
                        label={t("splash.field.password")}
                        type="password"
                        value={password}
                        onChange={(event) => setPassword(event.target.value)}
                        size="small"
                      />
                      <Button
                        variant="contained"
                        disabled={inFlight || !accountCode || !password}
                        onClick={() =>
                          void runLogin(() => api.login(accountCode, password))
                        }
                      >
                        {t("splash.action.login")}
                      </Button>
                    </Stack>
                  ) : (
                    <Button
                      variant="contained"
                      disabled={inFlight}
                      onClick={() => void runLogin(() => api.loginDomain())}
                    >
                      {t("splash.action.loginDomain")}
                    </Button>
                  )}

                  {outcome === "notFound" && (
                    <Box sx={{ mt: 2 }}>
                      <Alert severity="warning" sx={{ mb: 1 }}>
                        {t("splash.hint.notFound")}
                      </Alert>
                      <Button
                        variant="outlined"
                        onClick={() => void navigate({ to: "/register" })}
                      >
                        {t("splash.action.register")}
                      </Button>
                    </Box>
                  )}

                  {outcome === "inactive" && (
                    <Alert severity="warning" sx={{ mt: 2 }}>
                      {t("splash.hint.inactive")}
                    </Alert>
                  )}
                </>
              )}
            </StepContent>
          </Step>
        </Stepper>

        <SplashLog entries={entries} />
      </Paper>
    </Box>
  );
}
```

- [ ] **Step 5: Write the route binding**

Create `src/routes/splash.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";

import { SplashPage } from "../pages/splash";

export const Route = createFileRoute("/splash")({
  component: SplashPage,
});
```

- [ ] **Step 6: Regenerate the route tree**

The `/register` route does not exist yet, so create a placeholder now to keep the generated tree stable across Tasks 7 and 8 — the splash page's Register button navigates to `/register`, and `navigate({ to: "/register" })` will not typecheck until the route is in the tree.

Create `src/pages/register.tsx` with a stub that Task 8 replaces wholesale:

```tsx
export function RegisterPage() {
  return null;
}
```

Create `src/routes/register.tsx` (final form — Task 8 does not change it):

```tsx
import { createFileRoute } from "@tanstack/react-router";

import { RegisterPage } from "../pages/register";

export const Route = createFileRoute("/register")({
  component: RegisterPage,
});
```

Then regenerate:

```bash
cd apps/desktop/aegis-desktop
pnpm exec vite build
```

Verify both routes landed in the generated tree:

```bash
grep -c "'/splash'" src/routes/routeTree.gen.ts
grep -c "'/register'" src/routes/routeTree.gen.ts
```

Expected: a non-zero count for each. `dist/` is gitignored, so the build output is not committed.

- [ ] **Step 7: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/splash.test.tsx
```

Expected: PASS, 7 tests.

- [ ] **Step 8: Run the full frontend suite and typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm test && pnpm typecheck
```

Expected: both PASS. Existing tests are unaffected — the `_layout` guard does not exist yet.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/splash.tsx \
        apps/desktop/aegis-desktop/src/pages/register.tsx \
        apps/desktop/aegis-desktop/src/routes/splash.tsx \
        apps/desktop/aegis-desktop/src/routes/register.tsx \
        apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts \
        apps/desktop/aegis-desktop/src/test/tauri-mock.ts \
        apps/desktop/aegis-desktop/src/test/routes/splash.test.tsx
git commit -m "feat(desktop): add splashscreen login page at /splash"
```

---

### Task 8: Register page at `/register`

Reads the OS identity on mount into four disabled fields, collects a user name and password, and submits. Success is a dead end: the form is replaced by a contact-your-administrator alert, with no navigation offered — the account cannot be used until an admin activates it, so returning to login would only reproduce the same failure.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/register.tsx` (replaces the Task 7 stub)
- Test: `apps/desktop/aegis-desktop/src/test/routes/register.test.tsx`

**Interfaces:**
- Consumes: `api.getDomainUserInfo()` returning `Identity { domain, hostMachine, sid, userid }` (camelCase per Task 1); `api.registerUser(input)`; `errorMessage` (Task 4); the `register.*` keys (Task 5); `SplashLog`, `useSplashLog` (Task 6); `mockCommands`, `mockInvoke` (Task 7).
- Produces: `RegisterPage` exported from `src/pages/register.tsx`. The route binding already exists from Task 7 and does not change.

- [ ] **Step 1: Write the failing test**

Create `src/test/routes/register.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { httpError, mockCommands, mockInvoke } from "../tauri-mock";

const IDENTITY = {
  domain: "corp.example",
  hostMachine: "ws-001",
  sid: "S-1-5-21-1234",
  userid: "alice",
};

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

afterEach(() => {
  cleanup();
});

function renderRegister() {
  return renderWithFullRouter({
    initialEntries: ["/register"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

async function fillAndSubmit() {
  await userEvent.type(await screen.findByLabelText(/User name/i), "Alice");
  await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
  await userEvent.click(screen.getByRole("button", { name: /Register/i }));
}

describe("RegisterPage — identity", () => {
  it("fills the four identity fields and disables them", async () => {
    mockCommands({ get_domain_user_info: () => IDENTITY });

    await renderRegister();

    const userCode = await screen.findByLabelText(/User code/i);
    expect(userCode).toHaveValue("alice");
    expect(userCode).toBeDisabled();

    expect(screen.getByLabelText(/Domain/i)).toHaveValue("corp.example");
    expect(screen.getByLabelText(/Domain/i)).toBeDisabled();
    expect(screen.getByLabelText(/Hostname/i)).toHaveValue("ws-001");
    expect(screen.getByLabelText(/Hostname/i)).toBeDisabled();
    expect(screen.getByLabelText(/SID/i)).toHaveValue("S-1-5-21-1234");
    expect(screen.getByLabelText(/SID/i)).toBeDisabled();

    expect(screen.getByLabelText(/User name/i)).toBeEnabled();
    expect(screen.getByLabelText(/^Password$/i)).toBeEnabled();
  });

  it("logs the failure and renders no form when the identity lookup fails", async () => {
    mockCommands({
      get_domain_user_info: () => {
        throw { kind: "notImplemented", detail: "requires Windows" };
      },
    });

    await renderRegister();

    expect(
      await screen.findByText(/Could not read domain user information: requires Windows/i),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText(/User name/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
  });
});

describe("RegisterPage — submission", () => {
  it("disables Register until both editable fields are filled", async () => {
    mockCommands({ get_domain_user_info: () => IDENTITY });

    await renderRegister();
    await screen.findByLabelText(/User name/i);

    expect(screen.getByRole("button", { name: /Register/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/User name/i), "Alice");
    expect(screen.getByRole("button", { name: /Register/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    expect(screen.getByRole("button", { name: /Register/i })).toBeEnabled();
  });

  it("sends the full input, built from the identity", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => ({}),
    });

    await renderRegister();
    await fillAndSubmit();

    expect(mockInvoke).toHaveBeenCalledWith("register_user", {
      userCode: "alice",
      userName: "Alice",
      domainName: "corp.example",
      hostname: "ws-001",
      sid: "S-1-5-21-1234",
      password: "secret",
    });
  });

  it("replaces the form with the contact-administrator hint on success", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => ({}),
    });

    await renderRegister();
    await fillAndSubmit();

    expect(
      await screen.findByText(/Contact your administrator to activate your account/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/User name/i)).not.toBeInTheDocument();
  });

  it("logs the failure message and keeps the form on failure", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => {
        throw httpError(409, "already_exists", "user exists");
      },
    });

    await renderRegister();
    await fillAndSubmit();

    expect(
      await screen.findByText(/Registration failed: already_exists: user exists/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Contact your administrator to activate your account/i),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Register/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/register.test.tsx
```

Expected: FAIL — `RegisterPage` is still the Task 7 stub returning `null`, so no fields render.

- [ ] **Step 3: Write the page**

Replace the entire contents of `src/pages/register.tsx`:

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

import { api, type Identity } from "../api";
import { errorMessage } from "../api/error";
import { SplashLog, useSplashLog } from "../components/SplashLog";

export function RegisterPage() {
  const { t } = useI18n();
  const { entries, push } = useSplashLog();

  const [identity, setIdentity] = useState<Identity | null>(null);
  const [userName, setUserName] = useState("");
  const [password, setPassword] = useState("");
  const [inFlight, setInFlight] = useState(false);
  const [registered, setRegistered] = useState(false);

  // React StrictMode invokes effects twice in development. The ref keeps
  // the identity lookup to a single request.
  const lookupStarted = useRef(false);

  useEffect(() => {
    if (lookupStarted.current) return;
    lookupStarted.current = true;

    void (async () => {
      push("info", "register.log.identity.start");
      try {
        const info = await api.getDomainUserInfo();
        push("success", "register.log.identity.ok", { userid: info.userid });
        setIdentity(info);
      } catch (e) {
        push("error", "register.log.identity.failed", {
          message: errorMessage(e),
        });
      }
    })();
  }, [push]);

  async function onRegister() {
    if (!identity) return;
    setInFlight(true);
    push("info", "register.log.register.start");
    try {
      await api.registerUser({
        userCode: identity.userid,
        userName,
        domainName: identity.domain,
        hostname: identity.hostMachine,
        sid: identity.sid,
        password,
      });
      push("success", "register.log.register.ok", { userCode: identity.userid });
      setRegistered(true);
    } catch (e) {
      push("error", "register.log.register.failed", { message: errorMessage(e) });
    } finally {
      setInFlight(false);
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

        {identity && !registered && (
          <Stack spacing={2} sx={{ maxWidth: 360 }}>
            <TextField
              label={t("register.field.userCode")}
              value={identity.userid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.domainName")}
              value={identity.domain}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.hostname")}
              value={identity.hostMachine}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.sid")}
              value={identity.sid}
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
              disabled={inFlight || !userName || !password}
              onClick={() => void onRegister()}
            >
              {t("register.action.register")}
            </Button>
          </Stack>
        )}

        <SplashLog entries={entries} />
      </Paper>
    </Box>
  );
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/register.test.tsx
```

Expected: PASS, 6 tests.

> If `getByLabelText(/Domain/i)` matches more than one field, MUI has rendered a label that also contains "domain". Tighten the query to `getByLabelText(/^Domain$/i)`.

- [ ] **Step 5: Run the full frontend suite and typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm test && pnpm typecheck
```

Expected: both PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/register.tsx \
        apps/desktop/aegis-desktop/src/test/routes/register.test.tsx
git commit -m "feat(desktop): add registration page at /register"
```

---

### Task 9: Auth guard on `_layout`

One guard covers `/` and `/settings` and every future page under the layout. This lands last because its redirect target `/splash` must already exist.

`api.isLoggedIn()` throwing (a token-store error) is treated as "not logged in" rather than propagated, so a broken store sends the user to the splash instead of crashing the router.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/routes/_layout/route.tsx:1-13`
- Test: `apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx`

**Interfaces:**
- Consumes: `api.isLoggedIn()`; the `/splash` route (Task 7); `mockCommands`, `mockInvoke` (Task 7).
- Produces: no new exports. Behavioural change only.

- [ ] **Step 1: Write the failing test**

Rewrite `src/test/routes/_layout.test.tsx`. The existing three tests keep their assertions but gain a logged-in stub; two new tests cover the guard:

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

afterEach(() => {
  cleanup();
});

function renderRoot(initialEntries: string[] = ["/"]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("AppLayout (authenticated)", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the Sidebar and the Home page content at /", async () => {
    const { router } = await renderRoot(["/"]);

    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/");
  });

  it("navigates to /settings when the Settings menu item is clicked", async () => {
    const { router } = await renderRoot(["/"]);

    await userEvent.click(screen.getByText("Settings"));

    expect(router.state.location.pathname).toBe("/settings");
    expect(
      screen.getByRole("heading", { level: 4, name: /settings/i }),
    ).toBeInTheDocument();
  });

  it("navigates back to / when the Home menu item is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);

    await userEvent.click(screen.getByText("Home"));

    expect(router.state.location.pathname).toBe("/");
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});

describe("AppLayout (unauthenticated)", () => {
  it("redirects / to /splash when not logged in", async () => {
    mockCommands({ is_logged_in: () => false, healthz: () => "ok" });

    const { router } = await renderRoot(["/"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/splash"));
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });

  it("redirects /settings to /splash when not logged in", async () => {
    mockCommands({ is_logged_in: () => false, healthz: () => "ok" });

    const { router } = await renderRoot(["/settings"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/splash"));
  });

  it("redirects to /splash when the login check itself fails", async () => {
    mockCommands({
      is_logged_in: () => {
        throw { kind: "store", message: "auth.bin is locked" };
      },
      healthz: () => "ok",
    });

    const { router } = await renderRoot(["/"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/splash"));
  });

  it("does not guard /splash itself", async () => {
    mockCommands({ is_logged_in: () => false, healthz: () => "ok" });

    const { router } = await renderRoot(["/splash"]);

    expect(router.state.location.pathname).toBe("/splash");
    expect(await screen.findByText(/Server is healthy: ok/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/_layout.test.tsx
```

Expected: the three "authenticated" tests PASS (the guard does not exist, so the stub is simply unused), and the four "unauthenticated" tests FAIL — the pathname stays `/` or `/settings` instead of becoming `/splash`.

- [ ] **Step 3: Add the guard**

In `src/routes/_layout/route.tsx`, replace the import block and the `Route` definition (lines 1–13). Everything from `export default function AppLayout()` down is unchanged.

```tsx
import React from "react";
import {
  createFileRoute,
  Outlet,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../../api";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

export const Route = createFileRoute("/_layout")({
  // Every page under this layout requires a session. A failing
  // `is_logged_in` (a broken token store) counts as logged out, so the
  // user lands on the splash rather than seeing the router throw.
  beforeLoad: async () => {
    let loggedIn = false;
    try {
      loggedIn = await api.isLoggedIn();
    } catch {
      loggedIn = false;
    }
    if (!loggedIn) {
      throw redirect({ to: "/splash" });
    }
  },
  component: AppLayout,
});
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd apps/desktop/aegis-desktop
pnpm vitest run src/test/routes/_layout.test.tsx
```

Expected: PASS, 7 tests.

- [ ] **Step 5: Run every suite and typecheck**

```bash
cd apps/desktop/aegis-desktop && pnpm test && pnpm typecheck
cd lib/packages/ui && pnpm test && pnpm typecheck
cd apps/desktop/aegis-desktop/src-tauri && cargo test -p aegis-desktop
```

Expected: all PASS. In particular `src/test/routes/splash.test.tsx` must still pass — its successful-login tests navigate to `/`, which now runs the guard, and they already stub `is_logged_in: () => true`.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_layout/route.tsx \
        apps/desktop/aegis-desktop/src/test/routes/_layout.test.tsx
git commit -m "feat(desktop): redirect unauthenticated users to the splashscreen"
```

---

## Manual Verification

After Task 9, run the app against a live `aegis-server` on Windows:

```bash
cd apps/desktop/aegis-desktop
pnpm tauri dev
```

Walk each path:

1. **Server down** — stop the server, launch the app. The splash shows a red `Server health check failed: ...` line and the method step never opens.
2. **Account login, good credentials** — start the server, log in. The log ends with `Login succeeded. Entering the app...` and the Sidebar appears.
3. **Account login, unknown user** — a red `No account matches these credentials.` line, a warning alert, and a Register button that opens `/register`.
4. **Account login, inactive user** — a red inactive line and the contact-administrator alert, with no Register button.
5. **Domain login** — one button, no fields. On a domain-joined Windows host it should succeed without typing anything.
6. **Register** — the four identity fields are filled and greyed out; the user code matches your Windows login. Submit and confirm the contact-administrator alert replaces the form.
7. **Guard** — with a valid session, relaunch: the app opens straight into the splash, passes the health check, and login proceeds. Log out from the home page, then navigate to `/settings`; you should be bounced to `/splash`.
8. **Language switch** — from Settings, switch to Simplified Chinese, log out, and confirm the splash renders Chinese step labels and log lines.
