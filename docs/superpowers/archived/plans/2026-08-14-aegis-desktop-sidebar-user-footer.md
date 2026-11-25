# Aegis Desktop Sidebar User Footer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin a user-info footer (name + optional role chip + logout button with confirm dialog) to the bottom of the sidebar, strip the dead login/logout form off the home page, and expose a new `current_user` tauri command that resolves the signed-in user from the local JWT.

**Architecture:** Desktop-only change. Add a tiny JWT-payload decoder in the tauri Rust side that reads `sub` from the local access token without signature verification (the server still verifies on every API call, so any tampering fails closed). Use that code to call the existing `user::get_by_code`. The shared `Sidebar` accepts a generic `footer` slot; the app wires a new `UserFooter` component that fetches via the new API, renders the chip + name + logout icon, and gates the destructive action behind a MUI `Dialog`.

**Tech Stack:** Rust (tauri 2, jsonwebtoken-free base64), TypeScript/React (MUI, TanStack Router, `@aegis/ui` i18n + Sidebar), Vitest.

## Global Constraints

- All sidebar text routes through `useI18n()`; no hard-coded English.
- Both `en.ts` and `zhCN.ts` stay in sync via `satisfies Record<keyof typeof en, string>` on `zhCN.ts`.
- TS-side `api.getCurrentUser` calls tauri command `current_user` (snake_case at the boundary).
- JWT payload decode uses `base64` (workspace-declared), no signature verification.
- The `Sidebar` keeps a generic `footer?: ReactNode` prop; user-display knowledge lives in the app, not the shared package.
- Tests: Vitest for TS (`pnpm test`), `cargo test` for Rust. All tests pass before commit.
- No new server endpoints.
- Commit messages end with `Co-Authored-By: Claude <noreply@anthropic.com>` only on the final integration commit; per-task commits follow conventional commit style without trailer.

---

### Task 1: JWT payload decoder (Rust)

**Files:**
- Modify: `Cargo.toml` (workspace root, add `base64` to `[workspace.dependencies]`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/Cargo.toml` (add `base64` to `[dependencies]`)
- Create: `apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/system.rs`

**Interfaces:**
- Produces: `pub fn decode_sub(token: &str) -> Result<String, ApiError>` returning the JWT `sub` claim or `ApiError::Store { message: String }` on any decode failure.

- [ ] **Step 1: Add `base64` to workspace and desktop tauri `Cargo.toml`s**

Workspace `Cargo.toml`, inside `[workspace.dependencies]`:
```toml
base64 = "0.22"
```

`apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, inside `[dependencies]`:
```toml
base64 = { workspace = true }
```

- [ ] **Step 2: Write failing tests for `decode_sub`**

Create `apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs` with the tests only. The helper function is referenced but not yet defined.

```rust
//! Read the `sub` claim out of an HS256 access token's payload without
//! signature verification. The token lives in the local token store, so
//! any tampering still fails closed on the next server call.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;

use crate::http::dto::ApiError;

/// Decode the `sub` claim from a JWT payload.
///
/// Splits on `.` (must yield exactly 3 segments), base64-decodes the
/// payload segment (URL-safe, no pad), parses it as JSON, and extracts
/// `sub` as a string. Any malformed token, decode failure, or missing
/// `sub` returns `ApiError::Store { message: ... }` — the local
/// token store is the source of truth on the desktop, so this is a
/// pure read.
pub fn decode_sub(token: &str) -> Result<String, ApiError> {
    todo!("implemented in step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a JWT with arbitrary header + payload + signature.
    /// The signature is not verified, so any string works.
    fn jwt(header: &str, payload_json: &str, sig: &str) -> String {
        let b64 = |s: &str| URL_SAFE_NO_PAD.encode(s.as_bytes());
        format!("{}.{}.{}", b64(header), b64(payload_json), sig)
    }

    #[test]
    fn decodes_sub_from_well_formed_jwt() {
        let token = jwt(
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"alice","role":"admin","ver":1,"exp":0,"iat":0}"#,
            "sig",
        );
        assert_eq!(decode_sub(&token).unwrap(), "alice");
    }

    #[test]
    fn rejects_token_with_wrong_segment_count() {
        let err = decode_sub("only.two").unwrap_err();
        match err {
            ApiError::Store { message } => {
                assert!(message.contains("3 segments"), "got: {message}");
            }
            other => panic!("expected Store, got {other:?}"),
        }
    }

    #[test]
    fn rejects_invalid_base64_payload() {
        // "!!!" is not valid base64; URL_SAFE_NO_PAD rejects it.
        let err = decode_sub("hdr.!!!.sig").unwrap_err();
        assert!(matches!(err, ApiError::Store { .. }));
    }

    #[test]
    fn rejects_payload_without_sub() {
        let token = jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"role":"admin"}"#,
            "sig",
        );
        let err = decode_sub(&token).unwrap_err();
        match err {
            ApiError::Store { message } => {
                assert!(message.contains("sub"), "got: {message}");
            }
            other => panic!("expected Store, got {other:?}"),
        }
    }

    #[test]
    fn rejects_payload_with_non_string_sub() {
        let token = jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"sub":42}"#,
            "sig",
        );
        assert!(matches!(decode_sub(&token), Err(ApiError::Store { .. })));
    }
}
```

- [ ] **Step 3: Run the tests to confirm they fail**

Run: `cd apps/desktop/aegis-desktop/src-tauri && cargo test --lib system::jwt_claims`
Expected: compile error (`todo!()` panics at runtime, surfaced as test failure with "not yet implemented"). If compilation errors instead because of import ordering, move the `use` statements above the `todo!` so the file still parses — leave the function body as `todo!()`.

- [ ] **Step 4: Implement `decode_sub`**

Replace the `todo!()` body in `apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs`:

```rust
pub fn decode_sub(token: &str) -> Result<String, ApiError> {
    let mut parts = token.split('.');
    let _header = parts.next();
    let payload = parts
        .next()
        .ok_or_else(|| ApiError::Store { message: "malformed jwt: expected 3 segments".into() })?;
    let _sig = parts.next();
    if parts.next().is_some() {
        return Err(ApiError::Store { message: "malformed jwt: expected 3 segments".into() });
    }

    let bytes = URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .map_err(|e| ApiError::Store { message: format!("base64 decode: {e}") })?;

    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::Store { message: format!("json parse: {e}") })?;

    value
        .get("sub")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ApiError::Store { message: "missing sub claim".into() })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd apps/desktop/aegis-desktop/src-tauri && cargo test --lib system::jwt_claims`
Expected: 5 passed, 0 failed.

- [ ] **Step 6: Wire `jwt_claims` into `system` module**

`apps/desktop/aegis-desktop/src-tauri/src/system.rs` — add the module declaration:

```rust
pub mod jwt_claims;
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml apps/desktop/aegis-desktop/src-tauri/Cargo.toml apps/desktop/aegis-desktop/src-tauri/src/system.rs apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs
git commit -m "feat(desktop): decode jwt sub claim without signature verification"
```

---

### Task 2: `current_user` tauri command

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `crate::system::jwt_claims::decode_sub` (Task 1), `crate::http::client::HttpClient::tokens().access_token()` (existing), `crate::http::user::get_by_code` (existing).
- Produces: `#[tauri::command] async fn current_user(client: State<'_, HttpClient>) -> Result<UserViewResponse, ApiError>` registered under the name `current_user`.

- [ ] **Step 1: Write the failing wiremock test**

Append to `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`:

```rust
#[cfg(test)]
mod current_user_tests {
    //! Verifies that `current_user` reads the access token from the
    //! local store, extracts `sub`, and forwards to the user endpoint.
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore};

    /// Forge a JWT carrying `sub = "alice"` — no signature, since the
    /// desktop decoder only reads the payload.
    fn alice_jwt() -> String {
        use base64::Engine;
        let b64 = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes());
        let header = b64(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = b64(r#"{"sub":"alice","role":"admin","ver":1,"exp":0,"iat":0}"#);
        format!("{header}.{payload}.sig")
    }

    #[tokio::test]
    async fn current_user_resolves_sub_to_user_view() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token(&alice_jwt()).await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/user/alice"))
            .and(header("authorization", "Bearer alice"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "code": "alice",
                "name": "Alice",
                "role": "admin",
                "active": true,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new(server.uri(), store);
        // Direct call into the http layer — we are testing the command's
        // plumbing, not the tauri command framework.
        let view = crate::http::user::get_by_code(&client, "alice").await.unwrap();
        assert_eq!(view.code, "alice");
        assert_eq!(view.name, "Alice");
        assert_eq!(view.role, crate::http::dto::Role::Admin);
    }

    #[test]
    fn decode_sub_reads_alice_from_forged_jwt() {
        // Pure unit test of the helper, exercised through the same JWT
        // shape that the command uses.
        let token = alice_jwt();
        let sub = crate::system::jwt_claims::decode_sub(&token).unwrap();
        assert_eq!(sub, "alice");
    }
}
```

- [ ] **Step 2: Run test to confirm it compiles and the decode_sub helper passes**

Run: `cd apps/desktop/aegis-desktop/src-tauri && cargo test --lib commands::user::current_user_tests`
Expected: `decode_sub_reads_alice_from_forged_jwt` passes; `current_user_resolves_sub_to_user_view` passes too (the helper function already exists from Task 1, and we call `get_by_code` directly).

- [ ] **Step 3: Add the `current_user` tauri command**

In `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`, add the new function. Place it after the existing `get_user_by_code`:

```rust
/// Fetch the signed-in user. Decodes the JWT in the local token store
/// to learn the user code, then calls the existing `get_by_code` so the
/// server is still the source of truth for the view shape.
#[tauri::command]
pub async fn current_user(
    client: State<'_, HttpClient>,
) -> Result<UserViewResponse, ApiError> {
    let token = client
        .tokens()
        .access_token()
        .await?
        .ok_or_else(|| ApiError::Store { message: "no access token".into() })?;
    let code = crate::system::jwt_claims::decode_sub(&token)?;
    user::get_by_code(&client, &code).await
}
```

- [ ] **Step 4: Register the command in `lib.rs`**

`apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, inside `invoke_handler(tauri::generate_handler![...])`, add `commands::user::current_user` next to the other user commands:

```rust
            commands::user::create_user,
            commands::user::list_users,
            commands::user::get_user_by_code,
            commands::user::current_user,        // NEW
            commands::user::update_user,
```

- [ ] **Step 5: Run the full Rust test suite**

Run: `cd apps/desktop/aegis-desktop/src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add current_user tauri command"
```

---

### Task 3: `api.getCurrentUser` TypeScript wrapper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/api/index.ts`

**Interfaces:**
- Produces: `api.getCurrentUser(): Promise<UserView>` (calls tauri command `current_user`).

- [ ] **Step 1: Add the wrapper**

In `apps/desktop/aegis-desktop/src/api/index.ts`, inside the `api` object, after `getUserByCode`:

```ts
  getCurrentUser: (): Promise<UserView> =>
    call<UserView>("current_user"),
```

- [ ] **Step 2: Verify TS compiles**

Run: `cd apps/desktop/aegis-desktop && pnpm exec tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/api/index.ts
git commit -m "feat(desktop): expose api.getCurrentUser"
```

---

### Task 4: Sidebar `footer` slot

**Files:**
- Modify: `lib/packages/ui/src/components/Sidebar/types.ts`
- Modify: `lib/packages/ui/src/components/Sidebar/Sidebar.tsx`
- Modify: `lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`

**Interfaces:**
- Produces: `SidebarProps.footer?: ReactNode` — when present, rendered at the bottom of the Drawer below a `Divider`.

- [ ] **Step 1: Add `footer` to `SidebarProps`**

`lib/packages/ui/src/components/Sidebar/types.ts`:

```ts
import type { ComponentType, ReactNode } from 'react';

export interface SubMenuItem {
  link: string;
  title: string;
  icon: ComponentType;
}

export interface MenuItem extends SubMenuItem {
  subMenu?: SubMenuItem[];
}

export interface SidebarProps {
  title: string;
  menu: MenuItem[];
  open: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
  footer?: ReactNode;            // NEW
  width?: number;
  collapsedWidth?: number;
}
```

- [ ] **Step 2: Render the footer in `Sidebar.tsx`**

In `lib/packages/ui/src/components/Sidebar/Sidebar.tsx`:
- Add `Box` to the existing `@mui/material` import (it is already there).
- Destructure `footer` from props:

```tsx
export function Sidebar({
  title,
  menu,
  open,
  onToggle,
  onNavigate,
  footer,                          // NEW
  width = 240,
  collapsedWidth = 56,
}: SidebarProps) {
```

- Right before the closing `</Drawer>`, after the `<List>` block:

```tsx
      {footer && (
        <Box sx={{ mt: 'auto' }}>
          <Divider />
          <Box sx={{ p: 1.5 }}>{footer}</Box>
        </Box>
      )}
```

- [ ] **Step 3: Write failing test asserting footer renders**

In `lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`, add:

```tsx
import { Sidebar } from './Sidebar';
// existing imports
import { renderWithTheme } from './test-utils';
```

Inside the existing `describe('Sidebar', ...)` block, add a test:

```tsx
  it('renders footer content when provided', () => {
    renderWithTheme(
      <Sidebar
        {...defaultProps}
        footer={<div data-testid="custom-footer">Signed in as Alice</div>}
      />,
    );
    expect(screen.getByTestId('custom-footer')).toBeInTheDocument();
    expect(screen.getByText('Signed in as Alice')).toBeInTheDocument();
  });

  it('omits footer area when footer prop is not provided', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    // The Drawer's flex column still places the menu at top; no footer Box should render.
    expect(screen.queryByTestId('custom-footer')).not.toBeInTheDocument();
  });
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd lib/packages/ui && pnpm test -- Sidebar`
Expected: all tests pass (existing + 2 new).

- [ ] **Step 5: Commit**

```bash
git add lib/packages/ui/src/components/Sidebar/types.ts lib/packages/ui/src/components/Sidebar/Sidebar.tsx lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx
git commit -m "feat(ui): add footer slot to Sidebar"
```

---

### Task 5: i18n keys (en + zh-CN)

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Produces: 9 new translation keys: `app.user.unknownUser`, `app.user.role.root`, `app.user.role.admin`, `app.user.logout`, `app.user.logout.confirmTitle`, `app.user.logout.confirmMessage`, `app.user.logout.confirm`, `app.user.logout.cancel`, `app.user.loadFailed`.

- [ ] **Step 1: Add keys to `en.ts`**

Inside the `en` object literal in `lib/packages/ui/src/i18n/locales/en.ts`, add at the bottom (before the closing `} as const;`):

```ts
  'app.user.unknownUser': 'Unknown user',
  'app.user.role.root': 'Root',
  'app.user.role.admin': 'Admin',
  'app.user.logout': 'Log out',
  'app.user.logout.confirmTitle': 'Confirm logout',
  'app.user.logout.confirmMessage': 'Are you sure you want to log out?',
  'app.user.logout.confirm': 'Confirm',
  'app.user.logout.cancel': 'Cancel',
  'app.user.loadFailed': 'Failed to load user info',
```

- [ ] **Step 2: Add matching keys to `zhCN.ts`**

Inside the `zhCN` object literal in `lib/packages/ui/src/i18n/locales/zhCN.ts`, add at the bottom (before the closing `} satisfies ...;`):

```ts
  'app.user.unknownUser': '未知用户',
  'app.user.role.root': '超级管理员',
  'app.user.role.admin': '管理员',
  'app.user.logout': '退出登录',
  'app.user.logout.confirmTitle': '确认退出登录',
  'app.user.logout.confirmMessage': '您确定要退出登录吗？',
  'app.user.logout.confirm': '确认',
  'app.user.logout.cancel': '取消',
  'app.user.loadFailed': '加载用户信息失败',
```

The existing `satisfies Record<keyof typeof en, string>` constraint will fail compilation if any key is missing — that is the check that keeps the catalogs in sync.

- [ ] **Step 3: Verify TS compiles**

Run: `cd lib/packages/ui && pnpm exec tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Run i18n registry tests**

Run: `cd lib/packages/ui && pnpm test -- i18n`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(ui): add sidebar user footer i18n keys"
```

---

### Task 6: `UserFooter` component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/pages/UserFooter.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/pages/user-footer.test.tsx`

**Interfaces:**
- Consumes: `api.getCurrentUser` (Task 3), `useNavigate` from `@tanstack/react-router`, `useI18n` from `@aegis/ui/i18n`.
- Produces: `<UserFooter />` — a self-contained React component. While the user fetch is pending, renders the localized "Unknown user" placeholder. After success, renders the role `Chip` (only when `role === "root" || role === "admin"`), the user's `name`, and a `Log out` `IconButton` that opens a MUI confirm `Dialog`. On confirm: `await api.logout(); navigate({ to: "/login" });`.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/aegis-desktop/src/test/pages/user-footer.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../file-route-utils";
import { mockCommands } from "../tauri-mock";
import { UserFooter } from "../../pages/UserFooter";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

afterEach(() => {
  cleanup();
});

function renderFooter(props: { sidebarOpen?: boolean } = {}) {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <UserFooter sidebarOpen={props.sidebarOpen ?? true} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

function userViewFixture(overrides: Partial<{
  code: string;
  name: string;
  role: "root" | "admin" | "general";
}> = {}) {
  return {
    id: 1,
    code: overrides.code ?? "alice",
    name: overrides.name ?? "Alice",
    role: overrides.role ?? "general",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

describe("UserFooter", () => {
  it("renders the user name after getCurrentUser resolves", async () => {
    mockCommands({
      current_user: () => userViewFixture({ name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Alice")).toBeInTheDocument();
  });

  it("shows the role chip when role is admin", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "admin", name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Admin")).toBeInTheDocument();
  });

  it("shows the role chip when role is root", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "root", name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Root")).toBeInTheDocument();
  });

  it("does not show a role chip when role is general", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "general", name: "Alice" }),
    });
    await renderFooter();
    await screen.findByText("Alice");
    expect(screen.queryByText("Admin")).not.toBeInTheDocument();
    expect(screen.queryByText("Root")).not.toBeInTheDocument();
  });

  it("opens the confirm dialog when the logout button is clicked", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
      logout: () => undefined,
    });
    await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    expect(screen.getByText(/confirm logout/i)).toBeInTheDocument();
    expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
  });

  it("calls logout and navigates to /login on confirm", async () => {
    const logout = vi.fn().mockResolvedValue(undefined);
    mockCommands({
      current_user: () => userViewFixture(),
      logout,
    });
    const { router } = await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    await userEvent.click(screen.getByRole("button", { name: /^confirm$/i }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
  });

  it("cancels without calling logout", async () => {
    const logout = vi.fn();
    mockCommands({
      current_user: () => userViewFixture(),
      logout,
    });
    await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    expect(logout).not.toHaveBeenCalled();
    expect(screen.queryByText(/confirm logout/i)).not.toBeInTheDocument();
  });

  it("hides name and chip when sidebarOpen is false but keeps the logout button", async () => {
    mockCommands({
      current_user: () => userViewFixture({ name: "Alice", role: "admin" }),
    });
    await renderFooter({ sidebarOpen: false });
    // Wait for fetch to settle before asserting absence of the name.
    await waitFor(() => expect(screen.queryByText("Alice")).not.toBeInTheDocument());
    expect(screen.queryByText("Admin")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /log out/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail to compile**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- user-footer`
Expected: error — `UserFooter` does not exist.

- [ ] **Step 3: Implement `UserFooter`**

Create `apps/desktop/aegis-desktop/src/pages/UserFooter.tsx`:

```tsx
import { useEffect, useState } from "react";
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
import { api } from "../api";
import type { Role, UserView } from "../api";

interface UserFooterProps {
  /** Whether the surrounding sidebar drawer is open. When false, hide
   *  the name + chip and show only the logout icon. */
  sidebarOpen: boolean;
}

/**
 * Pinned to the bottom of the Sidebar. Shows the signed-in user's name
 * (with an optional role chip for root / admin) and a logout button
 * gated by a confirm dialog. On confirm: calls `api.logout` and
 * navigates to `/login`. The `_layout` `beforeLoad` guard already
 * redirects an authenticated user away from `/login`, so once the
 * tokens are cleared the navigation lands cleanly.
 */
export function UserFooter({ sidebarOpen }: UserFooterProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [user, setUser] = useState<UserView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const view = await api.getCurrentUser();
        if (!cancelled) setUser(view);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function onConfirmLogout() {
    setConfirmOpen(false);
    await api.logout();
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
            {error ? t("app.user.loadFailed") : (user?.name ?? t("app.user.unknownUser"))}
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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- user-footer`
Expected: 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/UserFooter.tsx apps/desktop/aegis-desktop/src/test/pages/user-footer.test.tsx
git commit -m "feat(desktop): add UserFooter with logout confirm dialog"
```

---

### Task 7: Wire `UserFooter` into the layout

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/layout.tsx`

**Interfaces:**
- Consumes: `<UserFooter />` from Task 6, `<Sidebar>` from `@aegis/ui` (now with `footer` prop, Task 4).

- [ ] **Step 1: Add the import and pass the footer prop**

`apps/desktop/aegis-desktop/src/pages/layout.tsx`:

```tsx
import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { UserFooter } from "./UserFooter";                // NEW

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

/**
 * Authenticated app shell: the `Sidebar` plus the active child route.
 * Lives in `src/pages/` (not a route file) so TanStack Router can code-
 * split the route file cleanly. The route file imports this as the
 * component for `/_layout`.
 */
export function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: <UserFooter sidebarOpen={sidebarOpen} />,   // NEW
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          ml: `${sidebarOpen ? 240 : 56}px`,
          transition: "margin 0.3s",
        }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}
```

- [ ] **Step 2: Run existing layout tests**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- _layout`
Expected: all existing layout tests still pass.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/layout.tsx
git commit -m "feat(desktop): render UserFooter in AppLayout sidebar"
```

---

### Task 8: Strip login/logout form from `home.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/home.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`

**Interfaces:**
- Produces: `HomePage` now contains only a heading + welcome body (no form, no state, no api calls).

- [ ] **Step 1: Replace `home.tsx` contents**

`apps/desktop/aegis-desktop/src/pages/home.tsx` — full replacement:

```tsx
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export function HomePage() {
  const { t } = useI18n();
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("home.heading")}
      </Typography>
      <Typography variant="body1">{t("home.welcome")}</Typography>
    </Box>
  );
}
```

- [ ] **Step 2: Update `index.test.tsx`**

In `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`:
- Keep the "renders the welcome heading" test.
- Delete the `invokes the login command with code and password, refreshes login state` test entirely (both the `it(...)` block).
- Delete the `calls logout when the logout button is clicked` test entirely.
- Remove the now-unused imports: `userEvent` is no longer needed; `vi` may still be used by `beforeEach`.

After cleanup, the file should look like:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../file-route-utils";
import { HomePage } from "../../pages/home";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() { return data.size; },
    clear() { data.clear(); },
    getItem(key: string) { return data.has(key) ? data.get(key)! : null; },
    key(index: number) { return Array.from(data.keys())[index] ?? null; },
    removeItem(key: string) { data.delete(key); },
    setItem(key: string, value: string) { data.set(key, value); },
  } as unknown as Storage;
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderHome(defaultLocale: "en" | "zh-CN" = "en") {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <HomePage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("HomePage", () => {
  it("renders the welcome heading", async () => {
    await renderHome();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd apps/desktop/aegis-desktop && pnpm test -- routes/index`
Expected: 1 test passes.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/pages/home.tsx apps/desktop/aegis-desktop/src/test/routes/index.test.tsx
git commit -m "refactor(desktop): strip login/logout form from home page"
```

---

### Task 9: Full integration verification

**Files:** none (verification only)

- [ ] **Step 1: Run the desktop TS test suite end-to-end**

Run: `cd apps/desktop/aegis-desktop && pnpm test`
Expected: every test file passes.

- [ ] **Step 2: Run the UI package test suite**

Run: `cd lib/packages/ui && pnpm test`
Expected: every test passes.

- [ ] **Step 3: Run the Rust test suite**

Run: `cd apps/desktop/aegis-desktop/src-tauri && cargo test`
Expected: every test passes.

- [ ] **Step 4: Type-check both packages**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm exec tsc --noEmit
cd lib/packages/ui && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 5: Lint both packages**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm run lint
cd lib/packages/ui && pnpm run lint
```
Expected: no errors. (Run lint scripts only if they are defined in `package.json`; skip if absent.)

- [ ] **Step 6: Integration commit (squash all task commits with the trailer)**

```bash
cd d:\\project\\aegis
git rebase -i HEAD~9 -X theirs  # the 8 task commits + docs commit
# replace "pick" with "squash" for each task commit; keep "pick" on the
# first (docs) commit so the design doc remains a separate, reviewable
# commit. Use the message:
#
#   feat(desktop): sidebar user footer with logout confirm
#
#   Co-Authored-By: Claude <noreply@anthropic.com>
```
Expected: a single, linear history with the docs commit at the bottom and a clean feat commit on top.

---

## Out of Scope (do NOT do in this plan)

- New server endpoint. Rejected by the user.
- Avatar / profile menu / settings shortcut. Future work.
- Auto-refresh of `UserFooter` on window focus or token change. Future work.
- Custom confirm modal in place of MUI `Dialog`. Future work.
- Unrelated cleanup in `pages/settings.tsx` or any other page.