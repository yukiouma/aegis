# aegis-desktop Tauri / Impl Command Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor 73 `#[tauri::command]` shims across five leaf modules into a thin Tauri wrapper plus a plain `pub async fn xxx_impl`, behind a new `SharedAppState` / `RequestContext` / `Caller` triple, so a future LLM-agent integration can drive the same code path without Tauri's runtime.

**Architecture:** Introduce `SharedAppState` (a single struct holding all four currently-separately-managed items: `HttpClient` + `TraceIdGenerator` + `LogSubmitter` + `LogGuard`) and `RequestContext { trace_id, caller }` plus `Caller { User, Agent, System }`. Each refactored command file contains two siblings: a `#[tauri::command]` wrapper that constructs `RequestContext` (fresh trace id, `Caller::User`) and forwards, and a `pub async fn xxx_impl(&SharedAppState, &RequestContext, …)` that owns the existing `info_span!` + `TRACE_ID.scope` + enter/success/failed log machinery verbatim. Out-of-scope modules (`auth`, `user`, `user_credential`, `identity`, `healthz`, `webview_log`, legacy `greet`) keep their current shape during the transition.

**Tech Stack:** Rust 2024 edition, Tauri 2, `tracing` + `tracing-subscriber`, `tokio::task_local!`, serde. No new dependencies — only re-organising existing types and adding one new module.

## Global Constraints

These come from the spec and project; every task's requirements implicitly include them.

- **Type name**: `SharedAppState`. NOT `AppState`, NOT `SharedApp`.
- **`SharedAppState` wraps**: `HttpClient`, `TraceIdGenerator`, `LogSubmitter`, `LogGuard` — all four currently-managed items in `.setup`.
- **Accessors are methods**, not `pub` fields: `state.http_client()`, `state.trace_id_generator()`, `state.log_submitter()`. `LogGuard` has no accessor.
- **Caller enum**: `User`, `Agent`, `System`. `serde::Serialize` with `rename_all = "snake_case"`. `Display` writes lowercase ("user", "agent", "system").
- **`_impl` is `pub`** so a future agent path can reach it from outside the `commands::` tree.
- **Command layout**: co-located — tauri wrapper and `_impl` in the **same file**. No parallel `impls/` module tree.
- **Migration scope**: 5 modules, 73 commands (see scope table below).
- **Out of scope**: `commands/auth.rs`, `commands/user.rs`, `commands/user_credential.rs`, `commands/identity.rs`, `commands/healthz.rs`, `commands/webview_log.rs`, legacy `greet`. They keep their current `State<'_, HttpClient>` + `State<'_, TraceIdGenerator>` shape.
- **Transition rule**: the four legacy `app.manage(...)` calls in `.setup` (for `client`, `generator`, `submitter`, `log_guard`) STAY during this PR — they continue to back legacy commands — alongside a new fifth `app.manage(SharedAppState::new(...))` for the refactored commands.
- **`TRACE_ID` `tokio::task_local!` declaration** at `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs:22-24` stays put. Only the refactored `_impl` functions import it; the wrappers do not.
- **No new tests**: this PR enables future tests but does not add them.
- **No macros**: per-command boilerplate stays grep-friendly.
- **Co-Authored-By** trailer on every commit (per the project git setting).

### Scope table

| Module                                  | Files                                                              | Commands |
|-----------------------------------------|--------------------------------------------------------------------|----------|
| `commands/mission.rs`                   | 1                                                                  | 9        |
| `commands/project.rs`                   | 1                                                                  | 4        |
| `commands/crf/*`                        | `version.rs`, `form.rs`, `item.rs`, `option.rs`, `unit.rs`, `annotation.rs`, `domain_annotation.rs` | 29 |
| `commands/domain_model/*`               | `version.rs`, `domain.rs`, `variable.rs`                           | 15       |
| `commands/terminology/*`                | `version.rs`, `code_list.rs`, `code_item.rs`, `import.rs`          | 16       |
| **Total**                               | **16 files**                                                       | **73**   |

---

## File Structure

### New files

| File | Responsibility |
|------|----------------|
| `apps/desktop/aegis-desktop/src-tauri/src/state.rs` | Defines `Caller`, `RequestContext`, `SharedAppState`, plus accessors and a unit test for `Caller::Display` and `Caller::Serialize`. |

### Modified files

| File | Change |
|------|--------|
| `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` | One new `app.manage(SharedAppState::new(...))` line inside `.setup`; the existing four `app.manage(...)` lines stay. No comments deleted. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs` | All 9 commands refactored (wrapper + `_impl` per command). |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs` | All 4 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/version.rs` | 2 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` | 8 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs` | 4 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/option.rs` | 3 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/unit.rs` | 3 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs` | 4 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/version.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/domain.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/variable.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/version.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` | 5 commands refactored. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs` | 1 command refactored. |

### Unchanged files

- All `http/` modules — the http-layer functions stay exactly as they are, taking `&HttpClient` only.
- Out-of-scope `commands/auth.rs`, `commands/user.rs`, `commands/user_credential.rs`, `commands/identity.rs`, `commands/healthz.rs`, `commands/webview_log.rs`.
- `lib.rs::generate_handler!` surface — same function names registered.
- Frontend (`src/features/**`, `src/api/index.ts`) — no changes needed.

---

## Task 1: Add `src-tauri/src/state.rs`

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/state.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (one new `mod state;` line)
- Test: the `#[cfg(test)] mod tests` inside `state.rs`

**Interfaces** (consumed by later tasks):
- `pub enum Caller { User, Agent, System }` — `Clone`, `Copy`, `Debug`, `Serialize`; `Display` is lowercase.
- `pub struct RequestContext { pub trace_id: String, pub caller: Caller }` — pub fields for readability.
- `pub struct SharedAppState` with:
  - `pub fn new(http_client: HttpClient, trace_id_generator: TraceIdGenerator, log_submitter: LogSubmitter, log_guard: LogGuard) -> Self`
  - `pub fn http_client(&self) -> &HttpClient`
  - `pub fn trace_id_generator(&self) -> &TraceIdGenerator`
  - `pub fn log_submitter(&self) -> &LogSubmitter`

This task proves the wiring works before any command is refactored — Task 2 depends on `SharedAppState::new` being constructible.

- [ ] **Step 1.1: Check whether `LogSubmitter` is `Clone`**

This affects how Task 2 wires `.setup`. The spec flagged this as a planning-time check.

Run:
```bash
cd d:/projects/rusty/aegis
cargo doc --no-deps -p logging-utils --document-private-items 2>/dev/null | grep -A 2 "pub struct LogSubmitter"
grep -n "derive.*Clone" apps/desktop/aegis-desktop/src-tauri/src/lib.rs || true
grep -rn "LogSubmitter" lib/crates/logging-utils/src/lib.rs
```

Expected: `LogSubmitter` either has `#[derive(Clone)]` (or implements `Clone` by hand) or it does not. Note which.

- [ ] **Step 1.2: Confirm cargo build is green before any change**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile. If it fails, fix the workspace first.

- [ ] **Step 1.3: Write the failing test for `Caller::Display`**

Create `apps/desktop/aegis-desktop/src-tauri/src/state.rs` with only the `Caller` enum and a `#[cfg(test)] mod tests` block:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Caller {
    User,
    Agent,
    System,
}

impl std::fmt::Display for Caller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // filled in next step
        match self {
            Caller::User => f.write_str("TODO"),
            Caller::Agent => f.write_str("TODO"),
            Caller::System => f.write_str("TODO"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_lowercase() {
        assert_eq!(format!("{}", Caller::User), "user");
        assert_eq!(format!("{}", Caller::Agent), "agent");
        assert_eq!(format!("{}", Caller::System), "system");
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Caller::User).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&Caller::Agent).unwrap(), "\"agent\"");
        assert_eq!(serde_json::to_string(&Caller::System).unwrap(), "\"system\"");
    }
}
```

Add to `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` near the other `mod` lines:
```rust
mod state;
```

- [ ] **Step 1.4: Run the test and confirm it fails**

Run:
```bash
cargo test -p aegis-desktop --lib state::tests
```

Expected: both tests fail — `display_is_lowercase` returns "TODO", `serializes_snake_case` may already pass because `rename_all = "snake_case"` is in place. The Display failure is the gate.

- [ ] **Step 1.5: Replace the TODO strings with lowercase**

Edit the `impl std::fmt::Display for Caller` body to write `"user"`, `"agent"`, `"system"` instead of `"TODO"`.

- [ ] **Step 1.6: Run tests to confirm they pass**

Run:
```bash
cargo test -p aegis-desktop --lib state::tests
```

Expected: both tests pass.

- [ ] **Step 1.7: Add `RequestContext` and `SharedAppState` types**

Append to `state.rs`:

```rust
use crate::http::client::HttpClient;
// `LogGuard`, `LogSubmitter`, `TraceIdGenerator` all live in the
// `logging_utils` workspace crate, re-exported via `crate::logging_utils`.
// `crate::trace_id_setup` only owns `load_or_create` and
// `DEVICE_PREFIX_FILE_NAME`; the type itself comes from `logging_utils`.
use crate::logging_utils::{LogGuard, LogSubmitter, TraceIdGenerator};

pub struct RequestContext {
    pub trace_id: String,
    pub caller: Caller,
}

pub struct SharedAppState {
    http_client: HttpClient,
    trace_id_generator: TraceIdGenerator,
    log_submitter: LogSubmitter,
    log_guard: LogGuard,
}

impl SharedAppState {
    pub fn new(
        http_client: HttpClient,
        trace_id_generator: TraceIdGenerator,
        log_submitter: LogSubmitter,
        log_guard: LogGuard,
    ) -> Self {
        Self {
            http_client,
            trace_id_generator,
            log_submitter,
            log_guard,
        }
    }

    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    pub fn trace_id_generator(&self) -> &TraceIdGenerator {
        &self.trace_id_generator
    }

    pub fn log_submitter(&self) -> &LogSubmitter {
        &self.log_submitter
    }
}
```

Note: `LogGuard` has no accessor — it's held purely for its `Drop` side effect.

- [ ] **Step 1.8: Add a constructor-and-accessor unit test**

Append to `mod tests`:

```rust
    #[test]
    fn shared_app_state_exposes_through_accessors() {
        // Build minimal stubs. We can't easily build a real HttpClient /
        // TraceIdGenerator / LogSubmitter / LogGuard without Tauri, so
        // this test only verifies type construction through the new()
        // shape compiles and the four arguments line up.
        //
        // Real coverage lives in cargo check -p aegis-desktop (the
        // lib wires everything up in Task 2) and in the http-layer
        // tests that already exist.
        fn _shape_check(
            h: HttpClient,
            t: TraceIdGenerator,
            l: LogSubmitter,
            g: LogGuard,
        ) -> crate::state::SharedAppState {
            crate::state::SharedAppState::new(h, t, l, g)
        }
    }
```

This is purely a compile-time check — adjust the constructor call so the four arguments line up if your real types differ in arity.

- [ ] **Step 1.9: Run `cargo check` for the desktop crate**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile. Fix any import errors before committing — most likely the import path for `LogGuard` / `LogSubmitter` / `TraceIdGenerator` will need a tweak depending on how the `logging_utils` crate is exposed.

- [ ] **Step 1.10: Run all `aegis-desktop` lib tests**

Run:
```bash
cargo test -p aegis-desktop --lib
```

Expected: pre-existing tests still green; new `state::tests` pass.

- [ ] **Step 1.11: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/state.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add SharedAppState, RequestContext, Caller types

Introduces the type triple the tauri/impl split depends on:
  - SharedAppState wraps the four currently-separately-managed items
    (HttpClient, TraceIdGenerator, LogSubmitter, LogGuard). Accessors
    return references; LogGuard has no accessor (Drop only).
  - RequestContext carries the per-invocation trace_id and caller.
  - Caller { User, Agent, System } is Serialize (snake_case) and
    Display (lowercase) — neither variant is used yet but the types
    are in place so later tasks can populate caller deterministically.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 2: Wire `SharedAppState` into `.setup`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (the `.setup` closure at lines 127-217)

**Interfaces (consumed):** `SharedAppState::new` from Task 1.

This task produces the runtime registration that downstream tasks (3-7) will read through `State<'_, SharedAppState>`.

- [ ] **Step 2.1: Identify insertion point**

In [lib.rs:127-217](apps/desktop/aegis-desktop/src-tauri/src/lib.rs#L127-L217), find the line that ends with `app.manage(client);` (currently line 216). The new `app.manage(SharedAppState::new(...))` line goes immediately after it, before the `.build(...)` call.

- [ ] **Step 2.2: Build the `SharedAppState` after the legacy items are constructed**

Insert immediately after `app.manage(client);`:

```rust
    // Single aggregation point. The four legacy `app.manage(...)` calls
    // above stay because commands/auth.rs, commands/user.rs,
    // commands/user_credential.rs, commands/identity.rs, and
    // commands/healthz.rs still take the bare State<'_, HttpClient> +
    // State<'_, TraceIdGenerator> shape. This fifth manage is what the
    // refactored modules (crf, domain_model, mission, project,
    // terminology) and a future agent integration will resolve.
    let shared = state::SharedAppState::new(
        client.clone(),       // HttpClient is Clone (wraps Arc<reqwest::Client>)
        generator.clone(),    // TraceIdGenerator is Clone
        submitter.clone(),    // LogSubmitter is Clone (verified in Task 1.1)
        log_guard,            // moved, no clone needed
    );
    app.manage(shared);
```

Notes on each argument:
- **`client.clone()`** — `HttpClient` is `Clone`; this shares the underlying `Arc<reqwest::Client>` with the original `app.manage(client)` registration. The two managed-state entries share the same http client.
- **`generator.clone()`** — `TraceIdGenerator` is `Clone` (per its `#[derive(Debug, Clone)]`); both managed entries mint ids with the same wire format because they carry the same device prefix.
- **`submitter.clone()`** — Only valid if Step 1.1 confirmed `LogSubmitter` is `Clone`. If it isn't, **stop here** and change this task to: drop the `app.manage(submitter);` line above (and update Task 1's `SharedAppState::new` signature to take `LogSubmitter` by value, removing the existing app.manage), then update any downstream consumers of `State<'_, LogSubmitter>` if any exist (search: `grep -rn 'State<'\''_, LogSubmitter' src-tauri`).
- **`log_guard`** — moved, no clone. The original `app.manage(log_guard);` at the existing line 206 is removed in this step.

- [ ] **Step 2.3: Re-run `cargo check`**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile. If `LogSubmitter` is not `Clone`, the type error is here — fix per the stop-rule in Step 2.2.

- [ ] **Step 2.4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): register SharedAppState in setup

Adds the fifth app.manage(...) for SharedAppState::new(...) alongside
the four legacy calls. Legacy commands (auth/user/user_credential/
identity/healthz) keep the old shape; refactored commands (crf/
domain_model/mission/project/terminology) and any future agent caller
will resolve through State<'_, SharedAppState>.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Tasks 3-7: Refactor the five in-scope modules

These tasks all share the same template. The differences between modules are:
1. The file paths.
2. The set of commands per file.
3. The `commands/<mod>::function(...)` call inside each `_impl` (this is the http-layer module that file routes to).

**The pattern, repeated:**

For every command currently of the form:
```rust
#[tauri::command]
pub async fn xxx(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    arg_a: T_a,
    arg_b: T_b,
) -> Result<Resp, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "xxx");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                http_mod::xxx(&client, arg_a, arg_b).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}
```

replace it with:
```rust
#[tauri::command]
pub async fn xxx(
    state: tauri::State<'_, crate::state::SharedAppState>,
    arg_a: T_a,
    arg_b: T_b,
) -> Result<Resp, ApiError> {
    let req_ctx = crate::state::RequestContext {
        trace_id: state.trace_id_generator().client_side(),
        caller: crate::state::Caller::User,
    };
    xxx_impl(state.inner(), &req_ctx, arg_a, arg_b).await
}

pub async fn xxx_impl(
    state: &crate::state::SharedAppState,
    req_ctx: &crate::state::RequestContext,
    arg_a: T_a,
    arg_b: T_b,
) -> Result<Resp, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "xxx"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                http_mod::xxx(&state.http_client(), arg_a, arg_b).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}
```

**Per-file edit cycle** (each task follows these steps):

- [ ] Open the file with the Read tool.
- [ ] For every command in the file, apply the transformation above.
- [ ] Update the file's `use crate::http::client::TRACE_ID;` import — both the wrapper and `_impl` can use it; keep the import at the top of the file.
- [ ] Verify `cargo check -p aegis-desktop` is clean.

The mechanical substitutions inside the `_impl` body, no matter which command:

| Old                                                            | New                                                                |
|----------------------------------------------------------------|--------------------------------------------------------------------|
| `let trace_id = generator.client_side();`                      | (deleted; trace_id comes from `req_ctx`)                           |
| `&client` inside the http call                                 | `&state.http_client()`                                              |
| `client.something(...)` inside the http call                   | `state.http_client().something(...)` (rare; only if a method exists) |
| `tracing::info_span!("command", trace_id = %trace_id, command = "xxx")` | `tracing::info_span!("command", trace_id = %req_ctx.trace_id, caller = %req_ctx.caller, command = "xxx")` |
| `TRACE_ID.scope(trace_id, async { ... })`                      | `TRACE_ID.scope(req_ctx.trace_id.clone(), async { ... })`          |

---

## Task 3: Refactor `commands/mission.rs` (9 commands, 1 file)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs`

**Interfaces produced (consumed by future agent path):**
- `pub async fn list_missions_by_project_impl(...)`
- `pub async fn add_assignee_impl(...)`
- `pub async fn remove_assignee_impl(...)`
- `pub async fn create_mission_impl(...)`
- `pub async fn list_issues_by_mission_impl(...)`
- `pub async fn create_issue_impl(...)`
- `pub async fn patch_issue_state_impl(...)`
- `pub async fn update_issue_description_impl(...)`
- `pub async fn append_comment_impl(...)`

- [ ] **Step 3.1: Read the file**

Read `apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs` end-to-end. Note each command's full signature (argument names, types) and the http-module call inside its body. The current commands are:

```rust
list_missions_by_project(client, generator, project_id) -> http::mission::list_by_project
add_assignee(client, generator, mission_id, user_code)
remove_assignee(client, generator, mission_id, user_code)
create_mission(client, generator, project_id, body: CreateMissionRequest)
list_issues_by_mission(client, generator, mission_id)
create_issue(client, generator, mission_id, body: CreateIssueRequest)
patch_issue_state(client, generator, issue_id, state: IssueState)
update_issue_description(client, generator, issue_id, description: String)
append_comment(client, generator, issue_id, body: AppendCommentRequest)
```

- [ ] **Step 3.2: Apply the refactor to each of the 9 commands**

For each `#[tauri::command]` function in the file, replace it with the wrapper + `_impl` pair using the pattern in the preamble above. Preserve every argument exactly; only swap `client`/`generator` (the two `State<'_, _>` params) for the new pattern. The wrapper keeps the same arg list minus `client`/`generator`; the `_impl` keeps the same arg list plus `state: &SharedAppState, req_ctx: &RequestContext` as the first two params.

- [ ] **Step 3.3: cargo check**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile. If it fails, the most common errors are:
- Missing `use crate::state::{Caller, RequestContext, SharedAppState};` at the top of the file — add it.
- Borrow checker complaining about `req_ctx.trace_id.clone()` inside `TRACE_ID.scope` — this should not happen; if it does, paste the error into a follow-up issue.
- Forgetting to update one command's body — `cargo check` will point at the file/line.

- [ ] **Step 3.4: cargo fmt**

Run:
```bash
cargo fmt --all
```

Expected: rustfmt may re-collapse multi-line `info_span!` invocations; that's fine.

- [ ] **Step 3.5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs
git commit -m "refactor(desktop): split mission.rs commands into tauri wrappers + _impl

Applies the tauri/impl split pattern to all 9 commands in
commands/mission.rs. Each command now has a thin #[tauri::command]
wrapper that constructs a RequestContext (fresh trace id, Caller::User)
and forwards to a sibling pub async fn xxx_impl that owns the
existing info_span! / TRACE_ID.scope / enter-success-failed machinery.
The http/mission module is unchanged.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 4: Refactor `commands/project.rs` (4 commands, 1 file)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs`

**Interfaces produced:**
- `pub async fn create_project_impl(...)`
- `pub async fn list_projects_impl(...)`
- `pub async fn get_project_by_code_impl(...)`
- `pub async fn update_project_impl(...)`

- [ ] **Step 4.1: Read the file**

Read `apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs`. The current commands:

```rust
create_project(
    client, generator,
    code: String,
    description: String,
    configurations: Option<...>,
    members: Option<...>,
    unblind_members: Option<...>,
) -> http::project::create
list_projects(client, generator) -> http::project::list
get_project_by_code(client, generator, code: String) -> http::project::get_by_code
update_project(client, generator, code: String, body: UpdateProjectRequest) -> http::project::update
```

- [ ] **Step 4.2: Apply the refactor to all 4 commands**

Same pattern as Task 3. Keep every argument exactly as it is.

- [ ] **Step 4.3: cargo check**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile.

- [ ] **Step 4.4: cargo fmt**

Run:
```bash
cargo fmt --all
```

- [ ] **Step 4.5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs
git commit -m "refactor(desktop): split project.rs commands into tauri wrappers + _impl

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 5: Refactor `commands/crf/*` (29 commands, 7 files)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/version.rs` (2 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/form.rs` (8 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/item.rs` (4 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/option.rs` (3 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/unit.rs` (3 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/annotation.rs` (4 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/crf/domain_annotation.rs` (5 commands)

**Interfaces produced (29 `pub async fn xxx_impl` siblings):**

- `version.rs`: `list_crf_versions_impl`, `import_als_impl`
- `form.rs`: `list_crf_forms_by_version_impl`, `create_crf_form_impl`, `update_crf_form_impl`, `delete_crf_form_impl`, `get_crf_form_by_id_impl`, `get_crf_form_details_impl`, `search_crf_forms_by_version_impl`, `set_crf_form_approved_impl`
- `item.rs`: `list_crf_items_by_form_impl`, `get_crf_item_by_id_impl`, `update_crf_item_impl`, `search_crf_items_by_version_impl`
- `option.rs`: `update_crf_option_impl`, `get_crf_option_by_id_impl`, `search_crf_options_by_version_impl`
- `unit.rs`: `update_crf_unit_impl`, `get_crf_unit_by_id_impl`, `search_crf_units_by_version_impl`
- `annotation.rs`: `create_crf_annotation_impl`, `update_crf_annotation_impl`, `delete_crf_annotation_impl`, `search_crf_annotations_by_version_impl`
- `domain_annotation.rs`: `create_crf_domain_annotation_impl`, `list_crf_domain_annotations_by_form_impl`, `update_crf_domain_annotation_impl`, `delete_crf_domain_annotation_impl`, `search_crf_domain_annotations_by_version_impl`

- [ ] **Step 5.1: Refactor `crf/version.rs`**

Apply the same wrapper + `_impl` pattern to `list_crf_versions` and `import_als`.

- [ ] **Step 5.2: Refactor `crf/form.rs`**

Apply to 8 commands listed above.

- [ ] **Step 5.3: Refactor `crf/item.rs`**

Apply to 4 commands listed above.

- [ ] **Step 5.4: Refactor `crf/option.rs`**

Apply to 3 commands listed above.

- [ ] **Step 5.5: Refactor `crf/unit.rs`**

Apply to 3 commands listed above.

- [ ] **Step 5.6: Refactor `crf/annotation.rs`**

Apply to 4 commands listed above.

- [ ] **Step 5.7: Refactor `crf/domain_annotation.rs`**

Apply to 5 commands listed above.

- [ ] **Step 5.8: cargo check across the entire crate**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile. If a file imports something you didn't expect (e.g. `commands::request::Something`), add it explicitly at the top.

- [ ] **Step 5.9: cargo fmt**

Run:
```bash
cargo fmt --all
```

- [ ] **Step 5.10: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/crf/
git commit -m "refactor(desktop): split crf/* commands into tauri wrappers + _impl

29 commands across commands/crf/{version,form,item,option,unit,
annotation,domain_annotation}.rs now have the tauri/impl split. http/
crf/* is unchanged.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 6: Refactor `commands/domain_model/*` (15 commands, 3 files)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/version.rs` (5 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/domain.rs` (5 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/variable.rs` (5 commands)

**Interfaces produced (15 `pub async fn xxx_impl` siblings):**

- `version.rs`: `create_sdtm_version_impl`, `list_sdtm_versions_impl`, `get_sdtm_version_by_id_impl`, `update_sdtm_version_impl`, `delete_sdtm_version_impl`
- `domain.rs`: `create_sdtm_domain_impl`, `list_sdtm_domains_by_version_impl`, `get_sdtm_domain_by_id_impl`, `update_sdtm_domain_impl`, `delete_sdtm_domain_impl`
- `variable.rs`: `create_sdtm_variable_impl`, `list_sdtm_variables_by_domain_impl`, `get_sdtm_variable_by_id_impl`, `update_sdtm_variable_impl`, `delete_sdtm_variable_impl`

- [ ] **Step 6.1: Refactor `domain_model/version.rs`**

Apply to 5 commands.

- [ ] **Step 6.2: Refactor `domain_model/domain.rs`**

Apply to 5 commands.

- [ ] **Step 6.3: Refactor `domain_model/variable.rs`**

Apply to 5 commands.

- [ ] **Step 6.4: cargo check**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile.

- [ ] **Step 6.5: cargo fmt**

Run:
```bash
cargo fmt --all
```

- [ ] **Step 6.6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/
git commit -m "refactor(desktop): split domain_model/* commands into tauri wrappers + _impl

15 commands across commands/domain_model/{version,domain,variable}.rs
now have the tauri/impl split. http/domain_model/* is unchanged.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 7: Refactor `commands/terminology/*` (16 commands, 4 files)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/version.rs` (5 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_list.rs` (5 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` (5 commands)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs` (1 command)

**Interfaces produced (16 `pub async fn xxx_impl` siblings):**

- `version.rs`: `create_terminology_version_impl`, `list_terminology_versions_impl`, `get_terminology_version_by_id_impl`, `update_terminology_version_impl`, `delete_terminology_version_impl`
- `code_list.rs`: `create_code_list_impl`, `list_code_lists_impl`, `get_code_list_by_id_impl`, `update_code_list_impl`, `delete_code_list_impl`
- `code_item.rs`: `create_code_item_impl`, `list_code_items_impl`, `update_code_item_impl`, `delete_code_item_impl`, `list_code_items_by_version_and_code_impl`
- `import.rs`: `import_terminology_impl`

- [ ] **Step 7.1: Refactor `terminology/version.rs`**

Apply to 5 commands.

- [ ] **Step 7.2: Refactor `terminology/code_list.rs`**

Apply to 5 commands.

- [ ] **Step 7.3: Refactor `terminology/code_item.rs`**

Apply to 5 commands.

- [ ] **Step 7.4: Refactor `terminology/import.rs`**

Apply to 1 command (`import_terminology`).

- [ ] **Step 7.5: cargo check**

Run:
```bash
cargo check -p aegis-desktop
```

Expected: clean compile.

- [ ] **Step 7.6: cargo fmt**

Run:
```bash
cargo fmt --all
```

- [ ] **Step 7.7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/
git commit -m "refactor(desktop): split terminology/* commands into tauri wrappers + _impl

16 commands across commands/terminology/{version,code_list,code_item,
import}.rs now have the tauri/impl split. http/terminology/* is
unchanged.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 8: Final verification

This is the gate before opening a PR. Every step is a verification, no edits.

- [ ] **Step 8.1: cargo fmt clean**

Run:
```bash
cargo fmt --all -- --check
```

Expected: no output, exit 0.

- [ ] **Step 8.2: cargo clippy**

Run:
```bash
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 8.3: cargo test on the desktop crate**

Run:
```bash
cargo test -p aegis-desktop
```

Expected: all tests green; the new `state::tests` (Caller::Display, Caller::Serialize, SharedAppState compile-shape) pass alongside the unchanged http-layer tests (`login_persists_tokens`, `login_propagates_401`, etc.).

- [ ] **Step 8.4: cargo check the whole workspace**

Run:
```bash
cargo check --workspace
```

Expected: clean compile. Catches accidental fallout into library crates.

- [ ] **Step 8.5: confirm the lib's `generate_handler!` list is unchanged**

Run:
```bash
grep -E "(login|list_users|create_project|list_crf|list_sdtm_domain|import_terminology)\b" \
  apps/desktop/aegis-desktop/src-tauri/src/lib.rs \
  | head -3
```

Expected: every refactored command is still listed in `generate_handler!` (same names as before — only the bodies changed).

- [ ] **Step 8.6: confirm no legacy command files were modified**

Run:
```bash
git status --porcelain | grep -E "(auth|user|user_credential|identity|healthz|webview_log)\.rs$" || \
git diff --name-only HEAD~8..HEAD | grep -E "(auth|user|user_credential|identity|healthz|webview_log)\.rs$" || \
echo "no legacy files modified"
```

Expected: the message "no legacy files modified" — these files should not appear in this PR's diff.

- [ ] **Step 8.7: confirm 73 `_impl` functions exist**

Run:
```bash
grep -rn "pub async fn .*_impl(" apps/desktop/aegis-desktop/src-tauri/src/commands/ \
  | wc -l
```

Expected: at least 73. (The number may be higher if a future test file declares an `_impl`, but for this PR, expect exactly 73.)

- [ ] **Step 8.8: confirm every refactored wrapper hardcodes `Caller::User`**

Run:
```bash
grep -rn "Caller::User" apps/desktop/aegis-desktop/src-tauri/src/commands/crf/ \
  apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/ \
  apps/desktop/aegis-desktop/src-tauri/src/commands/mission.rs \
  apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs \
  apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/ \
  | wc -l
```

Expected: 73 (one per refactored wrapper).

- [ ] **Step 8.9: build the desktop bundle**

Run:
```bash
pnpm --filter aegis-desktop tauri build
```

Expected: clean build. The frontend's `invoke<...>("...")` calls will bind to the same Tauri command names registered via `generate_handler!`, so no frontend changes are needed.

- [ ] **Step 8.10: manual smoke test**

Launch the built `.exe`, log in, exercise at least one command from each of the five refactored modules (`mission`, `project`, `crf`, `domain_model`, `terminology`). For each:
  - A JSON log line appears with `"caller": "user"` and a `"command": "..."` field.
  - A corresponding JSON HTTP request line follows with the same `"trace_id"`.
  - Trace ID's middle segment matches the persisted device-prefix file at `%APPDATA%/com.yukichen.aegis-desktop/aegis-desktop-device-prefix`.
  - Legacy commands (`login`, `listUsers`, etc.) still produce JSON log lines with the same trace-id format.

- [ ] **Step 8.11: open the PR**

```bash
git push -u origin refactor/desktop_tauri-command-decoupling
gh pr create --base main \
  --title "refactor(desktop): split 73 tauri commands into tauri wrapper + _impl" \
  --body "$(cat <<'EOF'
## Summary

Splits 73 #[tauri::command] shims across commands/{crf,domain_model,
mission,project,terminology}/* into a thin Tauri wrapper plus a
pub async fn xxx_impl sibling. Behind a new SharedAppState /
RequestContext / Caller triple in src-tauri/src/state.rs, so a future
LLM-agent integration can drive the same code path without Tauri's
runtime.

## Out of scope (intentionally not changed)

- commands/auth.rs, commands/user.rs, commands/user_credential.rs,
  commands/identity.rs, commands/healthz.rs, commands/webview_log.rs
  and the legacy greet. They keep their existing State<'_, HttpClient>
  + State<'_, TraceIdGenerator> shape. A follow-up PR extends the
  pattern to them.
- The four legacy app.manage(...) calls in .setup stay for those
  legacy commands; the new app.manage(SharedAppState::new(...)) is a
  fifth entry.

## What changed

- New: src-tauri/src/state.rs (Caller, RequestContext, SharedAppState)
- Modified: src-tauri/src/lib.rs (one new app.manage line)
- Modified: 14 command files across 5 modules — wrapper + _impl pairs

## Why

Same: spec docs/superpowers/specs/2026-10-09-aegis-desktop-tauri-
impl-split-design.md.

## Verification

- cargo fmt --all -- --check
- cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
- cargo test -p aegis-desktop
- cargo check --workspace
- Manual smoke (one command from each module, JSON log shape verified)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Self-Review Notes

**Spec coverage:**
- ✓ `SharedAppState` shape — Task 1
- ✓ `RequestContext`/`Caller` — Task 1
- ✓ Tauri wrapper pattern (mint id, `Caller::User`, forward to `_impl`) — Tasks 3-7 apply it
- ✓ `_impl` shape (info_span with `caller` field, `TRACE_ID.scope`, enter/success/failed, `.instrument`) — Tasks 3-7 apply it
- ✓ Wiring in `.setup` — Task 2
- ✓ Accessor methods (not public fields) — Task 1
- ✓ Co-located layout (no parallel `impls/`) — implied by Tasks 3-7 editing each command file in place
- ✓ `_impl` is `pub` — Tasks 3-7 write `pub async fn`
- ✓ Out-of-scope modules unchanged — Step 8.6 verifies
- ✓ No new tests added — no test-creation steps beyond the `state.rs` smoke tests
- ✓ LogSubmitter Clone verification — Step 1.1 checks, Step 2.2 stops if absent

**Internal consistency:**
- The `_impl` pattern in the Tasks 3-7 preamble matches what Task 1 produced (Caller lowercase, etc.)
- Every task's commit message references the same approach
- Verification commands in Task 8 mirror the spec's verification gate

**Open items for the implementer:**
- If `LogSubmitter` is not Clone (Step 1.1): follow the alternate flow in Step 2.2's stop-rule.
- If any command in a file has a non-`&HttpClient` first arg or non-`Result<_, ApiError>` return type: surface this in the plan-execution review; the pattern still applies but the substitution table needs another row.
