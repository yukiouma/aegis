# aegis-desktop Tauri / Impl Command Split — Design

**Date:** 2026-10-09
**Status:** Approved (awaiting implementation plan)
**Scope:** Refactor 73 `#[tauri::command]` shims across `commands/crf/*`, `commands/domain_model/*`, `commands/mission.rs`, `commands/project.rs`, `commands/terminology/*` so each is split into a thin Tauri wrapper and a plain `pub async fn xxx_impl`. Introduce `SharedAppState`, `RequestContext`, and `Caller` types so a future LLM-agent integration can drive the same code path without Tauri's runtime.

Out of scope for this PR: `commands/auth.rs`, `commands/user.rs`, `commands/user_credential.rs`, `commands/identity.rs`, `commands/healthz.rs`, `commands/webview_log.rs`, and the legacy `greet`. They stay on today's `State<'_, HttpClient>` + `State<'_, TraceIdGenerator>` shape until a follow-up PR extends the pattern.

## Motivation

Every command today is a `#[tauri::command]` that takes `State<'_, HttpClient>` and `State<'_, TraceIdGenerator>`, mints a trace id, opens a `tracing::info_span!` carrying that id, scopes the inner http call into a `tokio::task_local! TRACE_ID` so the outbound request picks up `X-Trace-ID`, and emits `enter` / `success` / `failed` log lines around it. The *logic* of what the command does lives inside a function whose state parameter is tauri-typed — making the same operation unreachable from any non-Tauri entry point.

A future LLM-agent integration wants to invoke the same operation: same http call, same logging, same span layout — but *not* through `#[tauri::command]` and *not* through `tauri::State<'_, …>`. The required decoupling is to split each command into:

1. A **plain `pub async fn xxx_impl`** that takes a domain-agnostic state handle and a request context, and owns the existing `info_span!` + `TRACE_ID.scope` + `enter` / `success` / `failed` machinery.
2. A **`#[tauri::command] pub async fn xxx`** wrapper that pulls Tauri's managed state, builds the `RequestContext { trace_id, caller }`, and delegates to the impl.

After the split, 73 mechanical in-place edits land in one PR.

## Approach

- Introduce a new `src-tauri/src/state.rs` with three types: `Caller` enum, `RequestContext` struct, `SharedAppState` struct that wraps all four currently-separately-managed items (`HttpClient`, `TraceIdGenerator`, `LogSubmitter`, `LogGuard`).
- During the transition, `.setup` keeps the original four `app.manage(...)` calls for the benefit of legacy commands (which take `State<'_, HttpClient>` etc.), and adds a fifth `app.manage(SharedAppState::new(...))` for the refactored commands. Once auth / user / user_credential / identity / healthz are migrated in a follow-up PR, the four originals can be removed.
- For each of the 73 in-scope commands, add a `pub async fn xxx_impl(&SharedAppState, &RequestContext, …)` sibling. The Tauri wrapper becomes a ~6-line delegate that constructs a `RequestContext` (freshly-minted trace id, `Caller::User`) and forwards.

The split is **mechanical, not behavioural** — the `_impl` body is the existing command body with `generator.client_side()` and `&client` swapped for the new abstractions. No http-module signature changes; no logging changes; no domain logic changes.

## Architecture

```
src-tauri/src/
├── lib.rs                         # modified — see "Wiring" below
├── state.rs                       # NEW — SharedAppState, RequestContext, Caller
├── trace_id_setup.rs              # unchanged
├── http.rs                        # unchanged
├── http/
│   ├── client.rs                  # unchanged — TRACE_ID and X-Trace-ID stay
│   └── ...                        # all 13 modules — unchanged
├── system.rs                      # unchanged
├── commands.rs                    # unchanged
└── commands/
    ├── auth.rs                    # out of scope — kept as-is for now
    ├── identity.rs                # out of scope
    ├── user.rs                    # out of scope
    ├── user_credential.rs         # out of scope
    ├── healthz.rs                 # out of scope
    ├── webview_log.rs             # out of scope (not a tauri command in any
    │                              #   sense — no state, no _impl shape)
    ├── mission.rs                 # refactored (9 commands)
    ├── project.rs                 # refactored (4 commands)
    ├── crf/                       # refactored (29 commands across 7 submodules)
    ├── domain_model/              # refactored (15 commands across 3 submodules)
    └── terminology/               # refactored (16 commands across 4 submodules)
```

The five in-scope leaf modules (`crf`, `domain_model`, `mission`, `project`, `terminology`) gain a `pub async fn xxx_impl` next to each of their existing 73 `#[tauri::command]` shims. The `http/` layer is unchanged.

## New types — `src-tauri/src/state.rs`

```rust
use crate::http::client::HttpClient;
// `LogGuard`, `LogSubmitter`, `TraceIdGenerator` all live in the
// `logging_utils` workspace crate — re-exported from `crate::logging_utils`.
// `crate::trace_id_setup` only owns `load_or_create` and
// `DEVICE_PREFIX_FILE_NAME`; the type itself comes from `logging_utils`.
use crate::logging_utils::{LogGuard, LogSubmitter, TraceIdGenerator};

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Caller {
    User,
    Agent,
    System,
}

impl std::fmt::Display for Caller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Caller::User => f.write_str("user"),
            Caller::Agent => f.write_str("agent"),
            Caller::System => f.write_str("system"),
        }
    }
}

/// Carries the per-invocation metadata that a command body needs but
/// that does NOT belong in any tauri-typed state. Today the only
/// caller of every refactored command is the React frontend
/// (Tauri shim → `_impl`), so `trace_id` is freshly minted per
/// invocation and `caller` is hardcoded to `User`.
pub struct RequestContext {
    pub trace_id: String,
    pub caller: Caller,
}

/// Aggregates the four items currently managed separately, so each
/// `#[tauri::command]` resolves a single `State<'_, SharedAppState>`
/// rather than two `State<'_, _>` parameters and lifts the rest into
/// managed state for their `Drop` side effects.
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
    ) -> Self;

    pub fn http_client(&self) -> &HttpClient;
    pub fn trace_id_generator(&self) -> &TraceIdGenerator;
    pub fn log_submitter(&self) -> &LogSubmitter;
    // No `log_guard()` accessor — it's held only for its Drop side
    // effect, and `Drop` is invoked when SharedAppState is dropped.
}
```

Three deliberate choices:

- **`Caller` derives `Serialize`** so a future integration can embed it directly in a JSON log envelope (e.g. when an agent's call is forwarded to a remote log sink). The `rename_all = "snake_case"` makes the wire spelling `user` / `agent` / `system` so it dovetails with future server-side parsing without a second mapping pass.
- **Accessor methods, not `pub` fields.** Lets us swap in `Arc<dyn …>` for tests later, or evolve one field into a computed one (e.g. cache a parsed JWT sub once) without rewriting every call site. Cost: a few lines per field. Worth it.
- **`LogGuard` is not exposed.** It's just a brand around a `WorkerGuard` (per the [2026-09-29-aegis-desktop-logging-design.md](./2026-09-29-aegis-desktop-logging-design.md) spec) held for its `Drop`. No one reads it; nobody should.

## Wiring — `.setup` in `src-tauri/src/lib.rs`

Today the setup closure ([lib.rs:127-217](apps/desktop/aegis-desktop/src-tauri/src/lib.rs#L127-L217)) builds the four pieces in this order:

```
HttpClient  →  app.manage(client)
TraceIdGenerator (= generator)  →  app.manage(generator)
LogSubmitter (= submitter)  →  app.manage(submitter)
LogGuard (= log_guard)  →  app.manage(log_guard)
```

After the refactor, the four originals stay — they continue to back the legacy `commands/auth.rs`, `commands/user.rs`, `commands/user_credential.rs`, `commands/identity.rs`, `commands/healthz.rs` shims, which still take `State<'_, HttpClient>` + `State<'_, TraceIdGenerator>`. A fifth line is added:

```rust
let shared = SharedAppState::new(
    client.clone(),         // HttpClient is Clone (wraps Arc<reqwest::Client>)
    generator.clone(),      // TraceIdGenerator is Clone
    submitter.clone(),      // LogSubmitter must be Clone — verify in impl
    log_guard,              // moved (no clone needed; the original manage call
                            // below this one is removed in this PR)
);
app.manage(shared);
```

If `LogSubmitter` is not `Clone`, the spec needs revision: the legacy `app.manage(submitter)` line is dropped (`SharedAppState` owns the only live copy) and any current consumer of `State<'_, LogSubmitter>` is identified. The implementation should check `LogSubmitter` for a `Clone` impl during planning; if absent, file an issue on that dependency first.

Construction **must** preserve the existing init order: client → generator → submitter → log_guard. Comments in `lib.rs` documenting each step's ordering invariant (`device_prefix` is read from `generator` *before* the generator is moved; `submitter.handle()` is handed to `init_tracing` after the submitter exists) are preserved verbatim.

## Per-command refactor — the shape

For each of the 73 commands, the new shape is:

```rust
// src-tauri/src/commands/crf/domain_annotation.rs (and 72 other files)

#[tauri::command]
pub async fn create_crf_domain_annotation(
    state: tauri::State<'_, SharedAppState>,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_crf_domain_annotation_impl(state.inner(), &req_ctx, form_id, body).await
}

pub async fn create_crf_domain_annotation_impl(
    state: &SharedAppState,
    req_ctx: &RequestContext,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_crf_domain_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::create(&state.http_client(), form_id, body).await
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

The `_impl` body is **byte-equal** to today's command body, with two mechanical substitutions and one added field:

| Today's body part                            | New body part                              |
|----------------------------------------------|--------------------------------------------|
| `let trace_id = generator.client_side();`    | (no minting; trace_id comes from `req_ctx`)|
| `&client` and `client.` inside the call      | `&state.http_client()` and `state.http_client()` |
| `command = "login"` (the span's `command`)   | `caller = %req_ctx.caller` (new field)     |

The span's `trace_id = %trace_id` keeps the same field name; the field's source moves from a local `String` to `req_ctx.trace_id`. `TRACE_ID.scope(req_ctx.trace_id.clone(), async { … }).await` is unchanged — the task-local is still a `String` and the http client still reads it inside `send()`.

`TRACE_ID` keeps living at `src-tauri/src/http/client.rs:22-24` as a `tokio::task_local!`. Its importers shift: today every command file has `use crate::http::client::TRACE_ID;` (already true for the 73 refactored files). Post-refactor, only `_impl` functions import it; the tauri wrapper does not.

## Caller semantics — today and tomorrow

- **Today:** every wrapper hardcodes `Caller::User`. The `caller` field is logged into the `info_span!`, so it surfaces in JSON logs via the configured `tracing-subscriber` (see [2026-09-29-aegis-desktop-logging-design.md](./2026-09-29-aegis-desktop-logging-design.md)).
- **Tomorrow (out of scope for this PR):** a future agent integration runs `_impl` directly — for example, an Axum-style IPC listening for agent requests — and constructs its own `RequestContext` with `Caller::Agent`. The `_impl` and the http module do not need to change.
- **No caller-based gating in this PR.** The `Caller::Agent` and `Caller::System` variants exist so the *type* is in place; command-level authorization is a future concern and explicitly deferred. Adding filtering now would expand the surface area of the refactor.

## Testing

This PR does not add new tests. Its contribution is *enabling* future tests:

- `_impl` accepts `&SharedAppState`, which is constructible in unit tests via `SharedAppState::new(...)` — no `tauri::State`, no `tauri::test::mock_app()`. The existing comment at `commands/crf/form.rs:254` complaining about `tauri::State` becoming obsolete for the refactored commands is the documentation that this enablement actually happens.
- Existing http-module tests (`http/client.rs::tests`, `commands/auth.rs::tests`) remain green untouched. The wrap is transparent to them — the http fns don't see `tauri::State` or `TraceIdGenerator` anyway.
- The wrap pattern (mint → span → enter → `TRACE_ID.scope` → success / failed → instrument) is covered by the existing logging PR's manual smoke test. The mechanical nature of this refactor guarantees the same template produces the same observable output.

## What stays the same

- **Every `http/<module>.rs` file** — unchanged. The http layer already takes `&HttpClient` and knows nothing about Tauri or trace-id minting.
- **`TRACE_ID` `tokio::task_local!` declaration** at `http/client.rs:22-24` — unchanged.
- **`ApiError` enum and `ErrorBody` shape** — unchanged.
- **Out-of-scope command files**: `commands/auth.rs`, `commands/user.rs`, `commands/user_credential.rs`, `commands/identity.rs`, `commands/healthz.rs`, `commands/webview_log.rs`. They keep their current `State<'_, HttpClient>` + `State<'_, TraceIdGenerator>` shape until a follow-up PR extends the pattern.
- **The 3 outliers** — `forward_webview_log` (no state, observability only), `get_domain_user_info` (sync, OS-only), legacy `greet` — unchanged.
- **Frontend (`src/features/**`)** — unchanged. The Tauri command names are preserved 1:1, so the `invoke<…>("…")` calls in `src/api/index.ts` continue to resolve.
- **`lib.rs::generate_handler!` list** — unchanged at the surface level (same function names registered).
- **`device_prefix` file lifecycle**, **log file lifecycle**, **token store lifecycle**, **log submitter spawn** — unchanged. The refactor only moves ownership of the four items into a struct; the underlying lifecycles are identical.

## Key Decisions Locked In

- **Type name**: `SharedAppState`. The motivating paragraph uses `AppState`; the example code uses `SharedAppState`. The example wins (it appears with the actual struct fields and trait usage) — `Shared*` better reflects the fact that `tauri::State<T>` is itself the sharing wrapper.
- **`SharedAppState` wraps**: `HttpClient` + `TraceIdGenerator` + `LogSubmitter` + `LogGuard`. All four currently-managed items, so `.setup` documents a single aggregation point.
- **Accessors**: methods (`state.http_client()`, etc.) rather than `pub` fields. Future-proofs for `Arc<dyn …>` swaps without rewriting call sites.
- **Command layout**: co-located — tauri wrapper and `_impl` in the same file. 1:1 mapping of file → command → `_impl`. No parallel `impls/` module tree.
- **Migration scope**: 5 modules, 73 commands. Out-of-scope modules stay on their current shape until a follow-up PR.
- **Caller enum**: defined today, populated with `Caller::User` everywhere. Future agent integration introduces the `Caller::Agent` path.
- **`_impl` is `pub`**: required for the future agent path. Lives in `commands/<module>/<x>.rs` next to its wrapper.
- **No new tests**: enables future tests; doesn't add them in this PR.
- **No macro**: the per-command boilerplate stays verbose. A follow-up can introduce a macro if duplication becomes painful; deferring it keeps the refactor mechanical and grep-friendly.
- **No caller-based gating**: deferred. Only the *type* is added; filters that gate commands by caller are out of scope.

## Trade-offs / things we accept

- **Co-located wrappers + impls make each command file slightly larger.** Each command goes from ~25 lines to ~35 lines in one file, plus a ~6-line wrapper. The alternative — a parallel `impls/<module>/` tree — doubles the file count and forces a navigation cost every time a reader wants both halves. Co-location is the right trade-off at this size; revisit only if a single file passes ~200 lines.
- **No macro.** The same span / `TRACE_ID.scope` / `enter` / `success` / `failed` block is repeated 73 times. A macro would shrink per-command code from ~25 to ~5 lines, but adds a build-time abstraction that complicates stepping through the code in a debugger and increases the on-boarding surface for new commands. Mechanical duplication is acceptable for this PR; a follow-up can introduce a macro if it proves painful.
- **Out-of-scope commands keep their old shape** and the four legacy `app.manage(...)` calls remain during the transition. After this PR lands, `app.manage(generator)`, `app.manage(client)`, `app.manage(log_submitter)`, `app.manage(log_guard)` all still happen, *plus* the new `app.manage(SharedAppState)`. The legacy calls are still used by `auth` / `user` / `user_credential` / `identity` / `healthz` today. **Follow-up cleanup:** once those modules are migrated in a future PR, the original four `app.manage(...)` calls can be removed.

  During the transition:
  - `app.manage(client)` registers an `HttpClient` for use by old commands.
  - `app.manage(SharedAppState::new(client_clone, generator_clone, submitter_clone, log_guard))` registers a `SharedAppState` for use by new commands.
  - `HttpClient` and `TraceIdGenerator` are both `Clone` (verified — they wrap `Arc`s internally); cloning them into the struct is cheap.
  - `LogSubmitter` Clone status must be confirmed during implementation planning; if absent, file a follow-up on the dependency.

  **Alternative considered and rejected**: ship a separate PR first to migrate `auth` / `user` / `identity` / `healthz` ahead of the bulk one, so `.setup` ends up with a single `app.manage(SharedAppState)` from day one. Rejected because it compounds the review surface — the small-batch PR would be effectively the same shape plus the bulk PR. Net change is identical, review cost is doubled.
- **`TraceIdGenerator` is cloned once for SharedAppState.** The two copies (one in legacy managed state, one inside `SharedAppState`) mint ids with the same wire format because they both carry the same device prefix. Trace ids minted by old commands and new commands share the same middle segment, preserving the join-on-trace-id property the existing observability wiring relies on.
- **`req_ctx.trace_id.clone()` allocates a `String` per command call.** Already true today (`let trace_id = generator.client_side();` followed by `TRACE_ID.scope(trace_id, …)` — the local is moved into the scope, then dropped). The `.clone()` is functionally identical; the difference is syntactic. If this matters later, switch `RequestContext::trace_id` to a `Cow<'static, str>` once we need non-mint callers.

## Verification gate, before any PR

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
cargo check --workspace

# Manual smoke (after build)
pnpm --filter aegis-desktop tauri build
# Launch the built .exe, log in, exercise each refactored module
# (crf, domain_model, mission, project, terminology), and confirm:
#   - one JSON log line per command carries the new `caller: "user"` field
#   - one JSON request log line per HTTP call
#   - tail the device-prefix file matches the middle segment of every trace id
#   - legacy commands (auth, user, etc.) still mint trace ids from the
#     standalone generator, with the same wire format
```

Pass criteria:
- All four `cargo` gates green.
- Manual smoke produces JSON log lines whose `caller` field is `"user"` for every refactored command.
- Pre-existing test suite (`login_persists_tokens`, etc.) green without any test edits.
- No new fields in any wire DTO (the refactor is Rust-internal; the frontend needs zero changes).

## File changes summary

- **New**: `src-tauri/src/state.rs` (~80 lines: 3 types + impl block + small unit tests for `Caller` `Serialize` / `Display`).
- **Modified**: `src-tauri/src/lib.rs` — one new `app.manage(SharedAppState::new(...))` call inside `.setup`. Existing comments preserved verbatim; no comments deleted.
- **Modified**: `src-tauri/src/commands/crf/{version,form,item,option,unit,annotation,domain_annotation}.rs` — 29 commands refactored.
- **Modified**: `src-tauri/src/commands/domain_model/{version,domain,variable}.rs` — 15 commands refactored.
- **Modified**: `src-tauri/src/commands/mission.rs` — 9 commands refactored.
- **Modified**: `src-tauri/src/commands/project.rs` — 4 commands refactored.
- **Modified**: `src-tauri/src/commands/terminology/{version,code_list,code_item,import}.rs` — 16 commands refactored.
- **Total commands refactored**: 73.
- **Unchanged**: 9 commands in `auth` / `user` / `user_credential` / `identity` / `healthz`; 1 in `webview_log`; 1 legacy `greet`.
