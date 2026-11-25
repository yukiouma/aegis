# Aegis Desktop — Sidebar User Footer & Home Cleanup

## Context

Today the `Sidebar` (in `lib/packages/ui`) renders the app title and a list of menu items but has no concept of who is logged in. The authenticated user is invisible — the only place they can confirm "yes, I'm signed in" is by glancing at the address bar. The `HomePage` (`apps/desktop/aegis-desktop/src/pages/home.tsx`) also still carries throw-away login/logout form scaffolding that long ago became dead weight once the dedicated `/login` page took over.

This change adds a small "user footer" pinned to the bottom of the sidebar with the signed-in user's name, an optional role chip, and a logout control. It also strips the dead login/logout form off the home page.

## Decisions

- **No server changes.** The user explicitly asked to keep this a desktop-only change.
- **JWT decoded without signature verification.** The access token already lives in the desktop's local token store, so the payload is trusted to come from us. Verification happens whenever the token is presented to the server on any subsequent API call, so any tampering still fails closed.
- **Generic `Sidebar` footer slot.** Keep `Sidebar` reusable across apps by accepting a `footer?: ReactNode` prop rather than baking user-display knowledge into the shared package.
- **Collapsed sidebar shows only the logout icon.** The name + chip only render when the drawer is open, matching the existing "text disappears, icons persist" pattern already used for menu items.
- **Confirm before logout.** A MUI `Dialog` guards the destructive action. On confirm: `await api.logout()` then `navigate({ to: "/login" })`. The existing `_layout` `beforeLoad` guard already redirects away from `/login` if a session exists, so no extra race protection is needed.

## Plan

### 1. JWT-decoding in tauri

**`Cargo.toml` (workspace root):**
- Add `base64 = "0.22"` to `[workspace.dependencies]`.

**`apps/desktop/aegis-desktop/src-tauri/Cargo.toml`:**
- Add `base64 = { workspace = true }` to `[dependencies]`.

**New `apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs`:**
- Single pub fn `decode_sub(token: &str) -> Result<String, ApiError>` that:
  1. Splits the JWT on `.` — must produce exactly 3 segments (header.payload.signature). Returns `ApiError::Store { message: "malformed jwt: expected 3 segments" }` otherwise.
  2. Base64-decodes the payload (segment 1, URL-safe no-pad). Returns `ApiError::Store { message: "..." }` on decode failure.
  3. Parses the result as `serde_json::Value` and extracts `sub` as `String`. Returns `ApiError::Store { message: "missing sub claim" }` if absent or not a string.
- The crate-level error variant chosen matches the existing `ApiError` enum in `http/dto.rs` (a `Store { message }` variant), so callers don't need new error machinery.

**`apps/desktop/aegis-desktop/src-tauri/src/system.rs`:**
- `pub mod jwt_claims;`

**`apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`:**
- New `#[tauri::command] pub async fn current_user(client: State<'_, HttpClient>) -> Result<UserViewResponse, ApiError>` that, inline:
  1. `let token = client.tokens().access_token().await?.ok_or_else(|| ApiError::Store { message: "no access token".into() })?;`
  2. `let code = crate::system::jwt_claims::decode_sub(&token)?;`
  3. `user::get_by_code(&client, &code).await` — calling the existing helper directly. No new `http/user.rs` function.

**`apps/desktop/aegis-desktop/src/api/index.ts`:**
- Add `getCurrentUser: (): Promise<UserView> => call<UserView>("current_user")` inside the existing `api` object.

The existing `getUserByCode` stays untouched for fetching arbitrary users.

### 2. Sidebar — bottom user area

**`lib/packages/ui/src/components/Sidebar/types.ts`:**
- Add `footer?: ReactNode` to `SidebarProps`.
- Import `ReactNode` from `'react'`.

**`lib/packages/ui/src/components/Sidebar/Sidebar.tsx`:**
- After the menu `</List>` and before the closing `</Drawer>`, render:
  ```tsx
  {footer && (
    <Box sx={{ mt: "auto" }}>
      <Divider />
      <Box sx={{ p: 1.5 }}>{footer}</Box>
    </Box>
  )}
  ```
- The Drawer root is a flex column (`display: flex` from the `Box` chain in MUI's Drawer), so `mt: auto` pins the footer to the bottom.

**`lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`:**
- Add a `renders footer content when provided` test that asserts a known string appears in the document.
- Add a `does not render footer area when footer prop is omitted` test (or just rely on the existing tests, since the absence of a Divider-only check is fine).

### 3. Layout — wire the user area

**New `apps/desktop/aegis-desktop/src/pages/UserFooter.tsx`:**
- Local component that calls `api.getCurrentUser()` on mount (via `useEffect`) and stores the result.
- Renders an error message in red if the call fails (graceful degradation — sidebar still shows the logout button).
- Renders a MUI `Box` with `display: flex`, `alignItems: center`, `gap: 1`:
  - Left: `Chip` (size small, label = localized role name) — only when `role === "root" || role === "admin"`.
  - Middle: user `name` (Typography `body2`, noWrap, flexGrow: 1). Falls back to localized `app.user.unknownUser` until the fetch lands.
  - Right: MUI `IconButton` with `<Logout />` from `@aegis/ui/icons` (which re-exports `@mui/icons-material`). `aria-label` is localized `app.user.logout`.
- State machine for the confirm dialog: a boolean `confirmOpen` plus a `confirmLogout` async handler. The `Dialog` body has Cancel (closes) and Confirm (calls handler).

**`apps/desktop/aegis-desktop/src/pages/layout.tsx`:**
- `import { UserFooter } from "./UserFooter";`
- Pass `footer={<UserFooter />}` to `<Sidebar />`.

### 4. i18n keys

Add to both `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts` (plus update the `satisfies Record<keyof typeof en, string>` constraint on `zhCN.ts`):

| key | en | zh-CN |
|---|---|---|
| `app.user.unknownUser` | Unknown user | 未知用户 |
| `app.user.role.root` | Root | 超级管理员 |
| `app.user.role.admin` | Admin | 管理员 |
| `app.user.logout` | Log out | 退出登录 |
| `app.user.logout.confirmTitle` | Confirm logout | 确认退出登录 |
| `app.user.logout.confirmMessage` | Are you sure you want to log out? | 您确定要退出登录吗？ |
| `app.user.logout.confirm` | Confirm | 确认 |
| `app.user.logout.cancel` | Cancel | 取消 |
| `app.user.loadFailed` | Failed to load user info | 加载用户信息失败 |

### 5. home.tsx cleanup

**`apps/desktop/desktop/src/pages/home.tsx`:**
- Drop the `useState`/login/logout/error plumbing.
- Drop the now-unused imports (`useState`, `Box`, `Button`, `Stack`, `TextField`, `Typography`, `api`).
- Keep `useI18n` and a small heading + welcome body so the route still has content.

**`apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`:**
- Delete the two test cases that drove the now-removed login/logout buttons.
- Keep the "renders the welcome heading" test.

### 6. Tests

- **Rust (`apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs` unit tests):**
  - Happy path: a known JWT decodes to the expected `sub`.
  - Wrong segment count → `ApiError::Store`.
  - Invalid base64 → `ApiError::Store`.
  - Missing `sub` → `ApiError::Store`.
- **Rust (`apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`):**
  - Add a `current_user` test that mocks the HTTP server with wiremock: token containing `sub = "u1"` + a mocked `/api/user/u1` returning a known `UserViewResponse`. Asserts the decoded `UserViewResponse`.
- **TS (`lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`):**
  - New tests for the `footer` prop (see section 2).
- **TS (`apps/desktop/aegis-desktop/src/test/pages/layout.test.tsx` — new file):**
  - User name renders after the fetch resolves.
  - Role chip renders for `role: "root"` and `role: "admin"`, absent for `role: "general"`.
  - Logout button click opens the confirm dialog.
  - Confirm button calls `api.logout` and `navigate({ to: "/login" })`.
  - Cancel closes the dialog without calling `api.logout`.
  - Collapsed sidebar (`open: false`) hides the name + chip but keeps the logout icon visible.

## Files Touched

- `Cargo.toml` (workspace)
- `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`
- `apps/desktop/aegis-desktop/src-tauri/src/system.rs`
- `apps/desktop/aegis-desktop/src-tauri/src/system/jwt_claims.rs` *(new)*
- `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`
- `apps/desktop/aegis-desktop/src/api/index.ts`
- `apps/desktop/aegis-desktop/src/pages/home.tsx`
- `apps/desktop/aegis-desktop/src/pages/layout.tsx`
- `apps/desktop/aegis-desktop/src/pages/UserFooter.tsx` *(new)*
- `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`
- `apps/desktop/aegis-desktop/src/test/pages/layout.test.tsx` *(new)*
- `lib/packages/ui/src/components/Sidebar/Sidebar.tsx`
- `lib/packages/ui/src/components/Sidebar/Sidebar.test.tsx`
- `lib/packages/ui/src/components/Sidebar/types.ts`
- `lib/packages/ui/src/i18n/locales/en.ts`
- `lib/packages/ui/src/i18n/locales/zhCN.ts`

## Out of Scope

- Adding `getCurrentUser` to the server (explicitly rejected by the user).
- Refreshing the user info on window focus / token change (a single fetch on mount is enough for this iteration).
- Replacing `MUI Dialog` with a custom confirm modal.
- Adding an avatar / profile menu / settings shortcut next to the name.