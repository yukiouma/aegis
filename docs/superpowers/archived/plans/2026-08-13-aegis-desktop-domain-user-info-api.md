# Expose `windows_utils::get_user_info` to the desktop frontend — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a JS-callable Tauri command `get_domain_user_info` and a TS `api.getDomainUserInfo()` wrapper that return the OS-level domain user tuple (`domain`, `host_machine`, `sid`, `userid`) to the Aegis desktop frontend.

**Architecture:** Reuse the existing `system::identity::current()` wrapper (which maps `windows_utils::get_user_info` into an `Identity` struct). Make `Identity` `serde::Serialize`, register a one-line `#[tauri::command]` that delegates to `identity::current()`, add an `Identity` interface to `api/types.ts`, and add a thin `api.getDomainUserInfo()` wrapper. No new wire shape, no new mapping logic.

**Tech Stack:** Rust 2024 (workspace), Tauri v2, `serde` (workspace), `serde_json` (workspace). Frontend: TypeScript, existing `api/index.ts` `call<T>` helper, React 19 / Vite 7.

**Spec:** `docs/superpowers/specs/2026-08-13-aegis-desktop-domain-user-info-api-design.md`

## Global Constraints

- Workspace is on `resolver = "3"`. `serde`, `serde_json` are inherited via `{ workspace = true }`. No new Cargo deps.
- Source module style: `src/<module>.rs` + `src/<module>/` directory. **No `mod.rs`.** `commands.rs` and `system.rs` are the module roots that re-export their submodules.
- `windows-utils` is Windows-only by design. `system::identity::current()` is already `#[cfg(target_os = "windows")]`-gated; the new command must compile on every target (do not add `cfg` gates to the command itself).
- TS wire-DTO convention (per `src/api/types.ts` top comment): interfaces use camelCase identifiers, but JSON keys at runtime are snake_case. No transform layer — consumers must use snake_case keys when destructuring JSON.
- Tauri command name style: snake_case `verb_noun`. TS method style: camelCase `verbNoun`. Existing `call<T>(cmd, args?)` helper handles `invoke<T>` typing.

## File Structure

| File                                                                     | Owner task | Notes                                                      |
|--------------------------------------------------------------------------|------------|------------------------------------------------------------|
| `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs`            | T1         | Add `Serialize` derive; add serialize test                  |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/identity.rs`          | T2         | New file: one `#[tauri::command]` shim                      |
| `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`                   | T2         | Add `pub mod identity;`                                    |
| `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`                        | T2         | Register command in `generate_handler!`                    |
| `apps/desktop/aegis-desktop/src/api/types.ts`                            | T3         | Add `Identity` interface                                   |
| `apps/desktop/aegis-desktop/src/api/index.ts`                            | T4         | Add import, wrapper method, re-export                      |

---

### Task 1: Make `Identity` serializable

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs:8-14` (derive line)

**Interfaces:**
- Produces: `pub struct Identity { domain: String, host_machine: String, sid: String, userid: String }` that implements `serde::Serialize`. Field order unchanged. `current()` signature unchanged.

- [ ] **Step 1: Add a failing serialize test**

In `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs`, inside the existing `#[cfg(test)] mod tests` block (after the existing `non_windows_returns_err` test), add:

```rust
#[test]
fn identity_serializes_with_snake_case_keys() {
    let id = Identity {
        domain: "corp.example".into(),
        host_machine: "ws-001".into(),
        sid: "S-1-5-21-1234".into(),
        userid: "alice".into(),
    };
    let json = serde_json::to_string(&id).expect("serialize");
    assert_eq!(
        json,
        r#"{"domain":"corp.example","host_machine":"ws-001","sid":"S-1-5-21-1234","userid":"alice"}"#
    );
}
```

- [ ] **Step 2: Run the test and confirm it fails**

Run from repo root:

```bash
cargo test -p aegis-desktop --lib system::identity::tests::identity_serializes_with_snake_case_keys
```

Expected: FAIL — `Identity` does not implement `Serialize` (compile error: `the trait Serialize is not implemented for Identity`).

- [ ] **Step 3: Add `serde::Serialize` to the `Identity` derive list**

In `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs:8`, change the derive line from:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
```

to:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Identity {
```

`serde` is already in scope via `{ workspace = true }` in `src-tauri/Cargo.toml`; no import needed.

- [ ] **Step 4: Run the test and confirm it passes**

Run:

```bash
cargo test -p aegis-desktop --lib system::identity::tests
```

Expected: PASS — both the existing `identity_fields_are_public_strings` and `non_windows_returns_err` (if on non-Windows) tests still pass, plus the new `identity_serializes_with_snake_case_keys`.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs
git commit -m "feat(desktop): derive serde::Serialize for system::identity::Identity"
```

---

### Task 2: Add the `get_domain_user_info` Tauri command

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/identity.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs:2-7` (add `pub mod identity;`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:21-50` (add command to `generate_handler!`)

**Interfaces:**
- Consumes: `pub fn identity::current() -> Result<Identity, String>` from Task 1's file.
- Produces: `pub fn commands::identity::get_domain_user_info() -> Result<Identity, String>` registered as Tauri command `get_domain_user_info`.

- [ ] **Step 1: Create `commands/identity.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/identity.rs` with:

```rust
use crate::system::identity::{self, Identity};

/// Returns the OS-level domain user tuple that backs the
/// `loginDomain` request body. Delegates to
/// `system::identity::current` — the single place that maps
/// `windows_utils::get_user_info` into the wire-shape `Identity`.
#[tauri::command]
pub fn get_domain_user_info() -> Result<Identity, String> {
    identity::current()
}
```

- [ ] **Step 2: Register the new module in `commands.rs`**

In `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`, change:

```rust
//! Tauri command shims that delegate 1:1 to the `http` layer.
pub mod auth;
pub mod healthz;
pub mod product;
pub mod project;
pub mod user;
pub mod user_credential;
```

to:

```rust
//! Tauri command shims that delegate 1:1 to the `http` layer.
pub mod auth;
pub mod healthz;
pub mod identity;
pub mod product;
pub mod project;
pub mod user;
pub mod user_credential;
```

(Alphabetical placement between `healthz` and `product`.)

- [ ] **Step 3: Register the command in `lib.rs`**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, in the `tauri::generate_handler!` macro list, add a new `// identity` group between `// auth` (currently lines 22-27) and `// user-credential` (currently line 28). The macro list should now read:

```rust
    .invoke_handler(tauri::generate_handler![
        // auth
        commands::auth::login,
        commands::auth::login_domain,
        commands::auth::is_logged_in,
        commands::auth::refresh,
        commands::auth::logout,
        // identity
        commands::identity::get_domain_user_info,
        // user-credential
        commands::user_credential::register_user,
        commands::user_credential::update_user_credential,
        // user
        commands::user::create_user,
        commands::user::list_users,
        commands::user::get_user_by_code,
        commands::user::update_user,
        // product
        commands::product::create_product,
        commands::product::list_products,
        commands::product::get_product_by_code,
        commands::product::update_product,
        // project
        commands::project::create_project,
        commands::project::list_projects,
        commands::project::get_project_by_code,
        commands::project::update_project,
        // health
        commands::healthz::healthz,
        // legacy greet (kept for the existing test)
        greet,
    ])
```

(The only insertion is `// identity` + `commands::identity::get_domain_user_info,` after the `// auth` block and before the `// user-credential` block.)

- [ ] **Step 4: Verify it compiles**

Run:

```bash
cargo check -p aegis-desktop
```

Expected: SUCCESS, no errors. The new command file, module registration, and handler registration all type-check.

- [ ] **Step 5: Re-run the Identity tests**

Run:

```bash
cargo test -p aegis-desktop --lib system::identity::tests
```

Expected: PASS — same as Task 1 step 4. (No regression from adding the derive or the command.)

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/identity.rs apps/desktop/aegis-desktop/src-tauri/src/commands.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add get_domain_user_info Tauri command"
```

---

### Task 3: Add the `Identity` TS interface

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/api/types.ts` (insert new interface)

**Interfaces:**
- Produces: `export interface Identity { domain: string; hostMachine: string; sid: string; userid: string; }` available from `apps/desktop/aegis-desktop/src/api/types.ts`. JSON keys at runtime are snake_case (`host_machine`) per the file-top comment.

- [ ] **Step 1: Insert the `Identity` interface**

In `apps/desktop/aegis-desktop/src/api/types.ts`, immediately before the `// Auth` comment at line 27 (i.e. right after the closing `}` of the `ApiError` discriminated union), insert:

```ts
// Mirrors `system::identity::Identity` in src-tauri. JSON keys are
// snake_case at runtime (per the file-top wire-DTO comment), so
// destructuring must use `host_machine`, not `hostMachine`.
export interface Identity {
  domain: string;
  hostMachine: string;
  sid: string;
  userid: string;
}
```

- [ ] **Step 2: Type-check the desktop app**

From `apps/desktop/aegis-desktop/`, run the project's type-check command (matches the existing convention):

```bash
pnpm tsc --noEmit
```

(If the project uses a different invocation such as `pnpm check-types` or `pnpm typecheck`, use that instead — check `apps/desktop/aegis-desktop/package.json` scripts.)

Expected: SUCCESS, no errors. The new `Identity` interface parses cleanly.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/api/types.ts
git commit -m "feat(desktop): add Identity wire-DTO interface"
```

---

### Task 4: Add the `getDomainUserInfo` TS API wrapper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/api/index.ts` (import block, `api` object, re-export block)

**Interfaces:**
- Consumes: `export interface Identity` from Task 3.
- Produces: `api.getDomainUserInfo(): Promise<Identity>` that calls the `get_domain_user_info` Tauri command via the existing `call<T>` helper. `Identity` is also re-exported from the `api` barrel.

- [ ] **Step 1: Add `Identity` to the import block**

In `apps/desktop/aegis-desktop/src/api/index.ts`, the current import (lines 3-17) ends with `UserView,`. Add `Identity` to the import list. The block should read:

```ts
import type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  Identity,
  ProductView,
  ProjectView,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserView,
} from "./types";
```

(Insertion is `Identity,` between `CreateUserInput,` and `ProductView,` — alphabetical placement matching the existing ordering convention.)

- [ ] **Step 2: Add the wrapper method to the `api` object**

In `apps/desktop/aegis-desktop/src/api/index.ts`, in the `api` const object (currently lines 29-76), insert a new `// identity` section between the `// auth` block (currently lines 30-37) and the `// user-credential` block (currently line 39). The relevant region of the object should read:

```ts
export const api = {
  // auth
  login: (code: string, password: string): Promise<void> =>
    call<void>("login", { code, password }),
  loginDomain: (code: string): Promise<void> =>
    call<void>("login_domain", { code }),
  isLoggedIn: (): Promise<boolean> => call<boolean>("is_logged_in"),
  refresh: (): Promise<void> => call<void>("refresh"),
  logout: (): Promise<void> => call<void>("logout"),

  // identity
  getDomainUserInfo: (): Promise<Identity> =>
    call<Identity>("get_domain_user_info"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
```

(The only insertion is the blank line, the `// identity` comment, and the `getDomainUserInfo` method.)

- [ ] **Step 3: Add `Identity` to the re-export block**

In `apps/desktop/aegis-desktop/src/api/index.ts`, in the re-export `export type { ... }` block at the bottom (currently lines 79-97), add `Identity`. The block should read:

```ts
export type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  Identity,
  ProductView,
  ProjectMembers,
  ProjectMembersView,
  ProjectView,
  Role,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserSummary,
  UserView,
} from "./types";
```

(Insertion is `Identity,` between `CreateUserInput,` and `ProductView,` — alphabetical placement.)

- [ ] **Step 4: Type-check the desktop app**

Run the same type-check command used in Task 3 step 2:

```bash
cd apps/desktop/aegis-desktop && pnpm tsc --noEmit
```

Expected: SUCCESS, no errors. The wrapper, the import, and the re-export all type-check.

- [ ] **Step 5: Run any existing desktop test suite that covers `src/api`**

From `apps/desktop/aegis-desktop/`, run the project's existing test command (Vitest is the convention based on sibling plans):

```bash
pnpm test --run
```

Expected: All existing tests still pass. No new test files are added in this plan (per spec "Out of scope: Adding a unit test for the new TS wrapper (no existing convention)"). If the project has no test script, skip this step.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/api/index.ts
git commit -m "feat(desktop): expose getDomainUserInfo via api wrapper"
```

---

## Self-Review

**1. Spec coverage:**
- Reuse existing `system::identity::current()` → covered (Task 2 step 1 delegates to it; Task 1 does not modify it).
- Add `#[derive(serde::Serialize)]` to `Identity` → covered (Task 1 step 3).
- New `commands::identity::get_domain_user_info` → covered (Task 2 step 1).
- Register `pub mod identity;` in `commands.rs` → covered (Task 2 step 2).
- Register command in `tauri::generate_handler!` between `// auth` and `// user-credential` → covered (Task 2 step 3).
- Add `Identity` interface to `types.ts` (camelCase identifiers, JSON keys snake_case) → covered (Task 3 step 1).
- Import `Identity` in `index.ts` (alphabetical) → covered (Task 4 step 1).
- Add `getDomainUserInfo` wrapper under `// identity` section → covered (Task 4 step 2).
- Re-export `Identity` (alphabetical) → covered (Task 4 step 3).

**2. Placeholder scan:** No "TBD", "TODO", "implement later", "similar to Task N", or other red flags. All code blocks contain full content.

**3. Type consistency:**
- Rust `Identity` field order (`domain`, `host_machine`, `sid`, `userid`) → Task 1 step 1 test asserts this exact JSON order. Task 3 step 1 TS interface uses the same fields (camelCase identifiers). ✓
- TS method name `getDomainUserInfo` ↔ Tauri command `get_domain_user_info` ↔ Rust function `commands::identity::get_domain_user_info` → all three steps reference the same names. ✓
- `call<Identity>("get_domain_user_info")` matches the `call<T>(cmd, args?)` helper signature in `index.ts:22-27`. ✓

**4. Out-of-scope items:** No renaming of `Identity`, no new fields, no changes to `login_domain` flow, no new TS unit test. Plan does not touch any of these. ✓