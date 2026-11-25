# Expose `windows_utils::get_user_info` to the desktop frontend

Date: 2026-08-13
Status: Approved (brainstorming)

## Goal

Add a JS-callable Tauri command + TypeScript wrapper that returns the
OS-level domain user tuple (currently produced by
`windows_utils::get_user_info`) to the Aegis desktop frontend.

Today the tuple is consumed internally by `commands::auth::login_domain`
via `system::identity::current()`, but is never exposed to the frontend.

## Approach

Reuse the existing `system::identity::current()` wrapper (which already
maps `windows_utils::get_user_info` into an `Identity` struct), make
`Identity` serializable, register a new Tauri command named
`get_domain_user_info`, and add a thin TS wrapper that mirrors it.

No duplication of the Windows → `Identity` mapping. No new wire shape.

## Wire shape

`Identity` becomes the single wire shape (Rust + TS).

| Field           | Type   | Source                                          |
|-----------------|--------|-------------------------------------------------|
| `domain`        | string | `windows_utils::DomainUserInfo::domain`         |
| `host_machine`  | string | `windows_utils::DomainUserInfo::host_machine`   |
| `sid`           | string | `windows_utils::DomainUserInfo::sid`            |
| `userid`        | string | `windows_utils::DomainUserInfo::userid`         |

On Windows the command returns `Ok(Identity { .. })`. On non-Windows it
returns `Err("OS identity lookup requires Windows".into())`, mirroring
the existing `system::identity::current()` contract.

The TS identifier is `Identity`, matching the Rust struct. Fields use
camelCase identifiers per the file convention in `api/types.ts`; JSON
keys remain snake_case.

## API naming

| Layer        | Name                       |
|--------------|----------------------------|
| Tauri command| `get_domain_user_info`     |
| TS method    | `api.getDomainUserInfo()`  |
| Rust command | `commands::identity::get_domain_user_info` |

Chosen by user in brainstorming over `get_user_info` and `get_identity`
to match the `DomainUserInfo` struct name.

## File changes

### Rust

1. `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs`
   - Add `#[derive(serde::Serialize)]` to `pub struct Identity`. Existing
     derives (`Debug, Clone, PartialEq, Eq`) stay.
   - No changes to `current()` — its signature and behavior already match
     what the new command needs.

2. `apps/desktop/aegis-desktop/src-tauri/src/commands/identity.rs` (new)
   ```rust
   use crate::system::identity::{self, Identity};

   #[tauri::command]
   pub fn get_domain_user_info() -> Result<Identity, String> {
       identity::current()
   }
   ```

3. `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`
   - Add `pub mod identity;` (alphabetical: between `healthz` and
     `product`).

4. `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`
   - In `tauri::generate_handler!`, add a `// identity` group with
     `commands::identity::get_domain_user_info`. Placed between
     `// auth` and `// user-credential` to reflect that identity
     conceptually feeds the auth flow.

### TypeScript

5. `apps/desktop/aegis-desktop/src/api/types.ts`
   - Add (near the existing `// Auth` block, before `RegisterUserInput`,
     since identity relates to login):
     ```ts
     // Mirrors `system::identity::Identity`.
     export interface Identity {
       domain: string;
       hostMachine: string;
       sid: string;
       userid: string;
     }
     ```

6. `apps/desktop/aegis-desktop/src/api/index.ts`
   - Add `Identity` to the `import type { ... } from "./types"` block.
   - Add a `// identity` section to the `api` object (between `// auth`
     and `// user-credential`):
     ```ts
     // identity
     getDomainUserInfo: (): Promise<Identity> =>
       call<Identity>("get_domain_user_info"),
     ```
   - Add `Identity` to the re-export `export type { ... }` block at the
     bottom.

## Testing

The Rust side already has tests for `system::identity::current()`
(`identity_fields_are_public_strings` and the `non_windows_returns_err`
guard). No new Rust tests are required for the new command — it is a
one-line passthrough.

The TS side has no unit-test convention for `src/api/`. No new TS tests.

Manual verification:
- `cargo check -p aegis-desktop` passes on Windows.
- `cargo check -p aegis-desktop` passes on non-Windows (the
  `non_windows_returns_err` path stays).
- Existing `src/__root__.test.tsx` / equivalent renders without TS errors
  after `pnpm tsc --noEmit` (or the project's equivalent).

## Out of scope

- Renaming `Identity` (would touch every existing call site of
  `identity::current()`).
- Exposing any new fields beyond what `windows_utils::DomainUserInfo`
  already produces.
- Changing the existing `login_domain` flow or the
  `RegisterUserInput` shape.
- Adding a unit test for the new TS wrapper (no existing convention).