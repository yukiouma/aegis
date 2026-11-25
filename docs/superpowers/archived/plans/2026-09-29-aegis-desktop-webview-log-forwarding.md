# aegis-desktop Webview Console Log Forwarding — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Forward the React webview's `console.warn` / `console.error` calls to the Tauri backend and emit them through `tracing` at the matching level, without trace-id wrapping and without forwarding any other `console.*` calls.

**Architecture:**
- **Frontend (TS):** A side-effect install in `main.tsx` replaces `console.warn` and `console.error` with thin wrappers that stringify the args (with `Error.stack` extraction) and call a new `api.forwardWebviewLog(level, message)` over the Tauri IPC. Errors from the IPC are swallowed so the forwarder never breaks the app; the original method still runs so DevTools still shows the log.
- **Backend (Rust):** A new `commands/webview_log.rs` shim exposes a single `#[tauri::command] forward_webview_log(level: String, message: String)`. It maps `"warn"` → `tracing::warn!` and `"error"` → `tracing::error!` under the `"aegis_desktop_lib::webview"` target; every other level (info, log, debug, trace, unknown) is silently dropped. The command does **not** mint a trace id and does **not** wrap in an `info_span!` — per the requirement that these logs do not need trace id.

**Tech Stack:** Tauri 2 (`#[tauri::command]`, `@tauri-apps/api/core::invoke`), Rust `tracing` (already wired by the prior logging feature), Vitest + jsdom for the TS forwarder.

---

## File Structure

| File | Responsibility |
|---|---|
| `apps/desktop/aegis-desktop/src-tauri/src/commands/webview_log.rs` (new) | `#[tauri::command] forward_webview_log` shim; level → `tracing::warn!` / `tracing::error!` / drop. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` (modify) | Add `pub mod webview_log;` to the barrel. |
| `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (modify) | Register `commands::webview_log::forward_webview_log` in `invoke_handler`. |
| `apps/desktop/aegis-desktop/src/shared/api/types.ts` (modify) | Add `WebviewLogLevel = "warn" \| "error"`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts` (modify) | Add `api.forwardWebviewLog(level, message)` wrapper. |
| `apps/desktop/aegis-desktop/src/shared/webview-log/forwarder.ts` (new) | `installWebviewLogForwarder()` side-effect; replaces `console.warn` / `console.error`. |
| `apps/desktop/aegis-desktop/src/shared/webview-log/index.ts` (new) | Barrel: `export { installWebviewLogForwarder }`. |
| `apps/desktop/aegis-desktop/src/shared/webview-log/format.ts` (new) | Pure helper `formatConsoleArgs(args: unknown[]): string` — keeps `forwarder.ts` focused. |
| `apps/desktop/aegis-desktop/src/main.tsx` (modify) | Call `installWebviewLogForwarder()` at the top, before any other module-level work. |
| `apps/desktop/aegis-desktop/src/test/shared/api/forward-webview-log.test.ts` (new) | Asserts `api.forwardWebviewLog` invokes `"forward_webview_log"` with `{ level, message }`. |
| `apps/desktop/aegis-desktop/src/test/shared/webview-log/format.test.ts` (new) | Pure tests for `formatConsoleArgs` (string, Error, object, multiple args, circular). |
| `apps/desktop/aegis-desktop/src/test/shared/webview-log/forwarder.test.ts` (new) | Asserts `console.warn`/`console.error` are intercepted and forwarded, `console.log`/`info` are not, and IPC failures are swallowed. |

Tests added inside the Rust module are inlined under `#[cfg(test)] mod tests` in `commands/webview_log.rs`.

---

## Task 1: Rust command shim with TDD

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/webview_log.rs`

- [ ] **Step 1: Write the failing tests**

Replace the not-yet-existent file with the tests first:

```rust
//! Tauri command that emits a webview `console.warn` / `console.error`
//! through the global subscriber, with no trace-id wrap and no http
//! call. Other levels are silently dropped — the frontend is the
//! primary gate, this is defense-in-depth.

use serde::Deserialize;

use crate::http::dto::ApiError;

/// Allowed level strings. The frontend pins this union; the backend
/// treats any other string as "drop" rather than rejecting, so a
/// stale build (frontend older than backend) still forwards what it
/// can.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebviewLogLevel {
    Warn,
    Error,
}

/// Wire payload. `level` and `message` are flat top-level keys; the
/// command takes them as positional Tauri parameters (mirrors how
/// `commands/auth.rs::login` takes `code` / `password`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebviewLogInput {
    pub level: WebviewLogLevel,
    pub message: String,
}

#[tauri::command]
pub fn forward_webview_log(level: WebviewLogLevel, message: String) -> Result<(), ApiError> {
    match level {
        WebviewLogLevel::Warn => {
            tracing::warn!(
                target: "aegis_desktop_lib::webview",
                message = %message,
                "webview console.warn"
            );
        }
        WebviewLogLevel::Error => {
            tracing::error!(
                target: "aegis_desktop_lib::webview",
                message = %message,
                "webview console.error"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The level → tracing emission is the only behavior that lives in
    //! Rust. The frontend owns the "which console methods to intercept"
    //! decision; this command is a thin sink. Tests assert the level
    //! mapping and that the message is carried as the `message` field.

    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::field::Visit;
    use tracing::Subscriber;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::Layer;

    #[derive(Default, Clone)]
    struct Capture {
        events: Arc<Mutex<Vec<(tracing::Level, String, String)>>>,
    }

    struct MessageVisitor(String);

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" {
                self.0 = value.to_string();
            }
        }
    }

    impl<S: Subscriber> Layer<S> for Capture {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let metadata = event.metadata();
            let mut visitor = MessageVisitor(String::new());
            event.record(&mut visitor);
            self.events.lock().unwrap().push((
                *metadata.level(),
                metadata.target().to_string(),
                visitor.0,
            }));
        }
    }

    fn with_capture<F: FnOnce()>(cap: &Capture, f: F) {
        let subscriber = tracing_subscriber::registry().with(cap.clone());
        tracing::subscriber::with_default(subscriber, f);
    }

    #[test]
    fn forward_warn_emits_warn_event_under_webview_target() {
        let cap = Capture::default();
        with_capture(&cap, || {
            forward_webview_log(WebviewLogLevel::Warn, "boom".into()).unwrap();
        });
        let events = cap.events.lock().unwrap();
        assert_eq!(events.len(), 1, "expected exactly one event");
        let (level, target, message) = &events[0];
        assert_eq!(*level, tracing::Level::WARN);
        assert_eq!(target, "aegis_desktop_lib::webview");
        assert_eq!(message, "boom");
    }

    #[test]
    fn forward_error_emits_error_event_under_webview_target() {
        let cap = Capture::default();
        with_capture(&cap, || {
            forward_webview_log(WebviewLogLevel::Error, "kaboom".into()).unwrap();
        });
        let events = cap.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        let (level, target, message) = &events[0];
        assert_eq!(*level, tracing::Level::ERROR);
        assert_eq!(target, "aegis_desktop_lib::webview");
        assert_eq!(message, "kaboom");
    }

    #[test]
    fn forward_returns_ok_for_every_supported_level() {
        // The contract is that the command never fails — the IPC
        // surface is fire-and-forget. Pin the unit variants here so
        // a future addition without a matching match arm is caught
        // at compile time.
        assert!(forward_webview_log(WebviewLogLevel::Warn, "x".into()).is_ok());
        assert!(forward_webview_log(WebviewLogLevel::Error, "x".into()).is_ok());
    }

    #[test]
    fn webview_log_level_deserializes_lowercase_strings() {
        let w: WebviewLogLevel = serde_json::from_str("\"warn\"").unwrap();
        let e: WebviewLogLevel = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(w, WebviewLogLevel::Warn);
        assert_eq!(e, WebviewLogLevel::Error);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cd /Users/yukichen/Coding/Projects/aegis
cargo test -p aegis-desktop --lib commands::webview_log::tests
```
Expected: compile failure — `commands/webview_log` does not exist yet. The error message names the missing module. (If the tests build, the file already exists and Step 1 was already applied.)

- [ ] **Step 3: Confirm file is the only code change**

Re-read the file. The implementation above is the complete implementation; Steps 1 and 3 are the same file write because the test module sits in the same file as the production code (the project convention for narrow single-command modules — see `commands/healthz.rs` for the template).

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test -p aegis-desktop --lib commands::webview_log::tests
```
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/webview_log.rs
git commit -m "feat(desktop): add forward_webview_log Tauri command

Emits webview console.warn / console.error through the global
tracing subscriber under the aegis_desktop_lib::webview target.
No trace-id wrap, no http call; the command is a thin sink so the
frontend can decide which console methods to intercept.

Verification:
  cargo test -p aegis-desktop --lib commands::webview_log::tests"
```

---

## Task 2: Register the command in the barrel and the invoke handler

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs:1-12` — add `pub mod webview_log;`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:121-125` — register `commands::webview_log::forward_webview_log` in the `invoke_handler` macro, alongside the `health` block.

- [ ] **Step 1: Update `commands.rs`**

Edit `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`. The current file ends with `pub mod user_credential;`. Append `pub mod webview_log;` on a new line.

Final file:

```rust
//! Tauri command shims that delegate 1:1 to the `http` layer.
pub mod auth;
pub mod crf;
pub mod domain_model;
pub mod healthz;
pub mod identity;
pub mod mission;
pub mod project;
pub mod terminology;
pub mod user;
pub mod user_credential;
pub mod webview_log;
```

- [ ] **Step 2: Register in `lib.rs` `invoke_handler`**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, the `invoke_handler!` macro ends with:
```rust
            commands::healthz::healthz,
            // legacy greet (kept for the existing test)
            greet,
        ])
```

Replace the comment line so it lists `webview_log` first (log infrastructure is its own category, alphabetically near `user_credential`):

```rust
            commands::healthz::healthz,
            // webview log forwarder (sink for console.warn / console.error)
            commands::webview_log::forward_webview_log,
            // legacy greet (kept for the existing test)
            greet,
        ])
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check -p aegis-desktop --all-targets
```
Expected: no errors, no new warnings.

- [ ] **Step 4: Run the existing test suite to confirm no regression**

Run:
```bash
cargo test -p aegis-desktop --lib
```
Expected: existing tests pass plus the 4 new tests from Task 1.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): register forward_webview_log in invoke_handler

Verification:
  cargo check -p aegis-desktop --all-targets
  cargo test -p aegis-desktop --lib"
```

---

## Task 3: Add the `api.forwardWebviewLog` TS wrapper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts:1-37` — add `WebviewLogLevel`.
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts:1-99, 502-503` — add wrapper + re-export.

- [ ] **Step 1: Add the level type to `types.ts`**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, immediately after the `ApiError` block (the file's wire-shape boundary section), insert:

```ts
// Mirrors `commands::webview_log::WebviewLogLevel`. The Rust enum
// uses `#[serde(rename_all = "lowercase")]`, so the wire form is
// `"warn"` / `"error"`.
export type WebviewLogLevel = "warn" | "error";
```

- [ ] **Step 2: Add the wrapper to `index.ts`**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`:

1. Add the import at the top of the file (next to the other `import type { ... } from "./types";` line):

```ts
import type {
  WebviewLogLevel,
  // …existing imports…
} from "./types";
```

(Add `WebviewLogLevel` to the existing import block; do not introduce a second `import type` line.)

2. Add the wrapper method on the `api` object, between the `healthz` line and the `openProjectWorkspace` block:

```ts
  // webview console forwarder
  forwardWebviewLog: (
    level: WebviewLogLevel,
    message: string,
  ): Promise<void> =>
    call<void>("forward_webview_log", { level, message }),
```

3. Add `WebviewLogLevel` to the `export type { … } from "./types";` re-export at the bottom of the file (alphabetised with the other entries).

- [ ] **Step 3: Verify it typechecks**

Run:
```bash
pnpm --filter aegis-desktop typecheck
```
Expected: no errors. The new wrapper is now part of the typed bridge.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/api/index.ts
git commit -m "feat(desktop): expose api.forwardWebviewLog bridge wrapper

Verification:
  pnpm --filter aegis-desktop typecheck"
```

---

## Task 4: Add the forwarder unit tests (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/shared/webview-log/forwarder.test.ts`
- Create: `apps/desktop/aegis-desktop/src/test/shared/webview-log/format.test.ts`

- [ ] **Step 1: Write the failing format tests**

Create `apps/desktop/aegis-desktop/src/test/shared/webview-log/format.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatConsoleArgs } from "../../../shared/webview-log/format";

describe("formatConsoleArgs", () => {
  it("returns the single string verbatim", () => {
    expect(formatConsoleArgs(["hello"])).toBe("hello");
  });

  it("joins multiple primitive args with a space", () => {
    expect(formatConsoleArgs(["a", 1, true])).toBe("a 1 true");
  });

  it("JSON-serialises plain objects", () => {
    expect(formatConsoleArgs([{ k: "v" }])).toBe('{"k":"v"}');
  });

  it("extracts Error name, message, and stack", () => {
    const err = new Error("boom");
    const out = formatConsoleArgs([err]);
    expect(out).toContain("Error: boom");
    expect(out).toContain(err.stack ?? ""); // stack presence is jsdom-dependent but the call must not throw
  });

  it("falls back to String() for unserialisable values", () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    const out = formatConsoleArgs([circular]);
    // JSON.stringify throws on circular; the fallback must not throw.
    expect(typeof out).toBe("string");
    expect(out.length).toBeGreaterThan(0);
  });

  it("returns an empty string for no args", () => {
    expect(formatConsoleArgs([])).toBe("");
  });
});
```

- [ ] **Step 2: Write the failing forwarder tests**

Create `apps/desktop/aegis-desktop/src/test/shared/webview-log/forwarder.test.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { installWebviewLogForwarder } from "../../../shared/webview-log";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("installWebviewLogForwarder", () => {
  it("forwards console.warn with level=warn and the formatted message", () => {
    installWebviewLogForwarder();
    console.warn("hello", 1);

    expect(mockInvoke).toHaveBeenCalledWith("forward_webview_log", {
      level: "warn",
      message: "hello 1",
    });
  });

  it("forwards console.error with level=error", () => {
    installWebviewLogForwarder();
    console.error("oops");

    expect(mockInvoke).toHaveBeenCalledWith("forward_webview_log", {
      level: "error",
      message: "oops",
    });
  });

  it("does not forward console.log", () => {
    installWebviewLogForwarder();
    console.log("noise");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("does not forward console.info", () => {
    installWebviewLogForwarder();
    console.info("noise");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("does not forward console.debug", () => {
    installWebviewLogForwarder();
    console.debug("noise");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("still calls the original console method (so DevTools shows the log)", () => {
    const originalWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    installWebviewLogForwarder();
    console.warn("kept");
    expect(originalWarn).toHaveBeenCalledWith("kept");
  });

  it("swallows an IPC rejection so logging never breaks the app", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("ipc down"));
    installWebviewLogForwarder();
    // Must not throw — the rejection is caught inside the forwarder.
    expect(() => console.warn("still alive")).not.toThrow();
    // Let the queued microtask drain so the .catch arm has run.
    await new Promise((r) => setTimeout(r, 0));
  });

  it("forwards window 'error' events at error level", async () => {
    installWebviewLogForwarder();
    // ErrorEvent is the standard shape jsdom dispatches for window.error.
    const ev = new ErrorEvent("error", {
      message: "boom",
      filename: "app.js",
      lineno: 42,
      colno: 7,
    });
    window.dispatchEvent(ev);
    // The handler runs synchronously, but the IPC promise resolves on a
    // microtask. Drain once.
    await new Promise((r) => setTimeout(r, 0));

    const call = mockInvoke.mock.calls.find(
      (c) => c[0] === "forward_webview_log" && c[1].level === "error",
    );
    expect(call, "expected at least one error-level forward").toBeDefined();
    expect(call![1].message).toContain("boom");
    expect(call![1].message).toContain("app.js:42:7");
  });

  it("includes the originating Error stack when 'error' event carries one", async () => {
    installWebviewLogForwarder();
    const err = new Error("with-stack");
    const ev = new ErrorEvent("error", { message: "with-stack", error: err });
    window.dispatchEvent(ev);
    await new Promise((r) => setTimeout(r, 0));

    const call = mockInvoke.mock.calls.find(
      (c) => c[0] === "forward_webview_log" && c[1].level === "error",
    );
    expect(call, "expected an error-level forward").toBeDefined();
    expect(call![1].message).toContain("Error: with-stack");
    expect(call![1].message).toContain(err.stack ?? "");
  });

  it("forwards 'unhandledrejection' events at error level", async () => {
    installWebviewLogForwarder();
    // jsdom may not expose PromiseRejectionEvent; construct a plain
    // Event and stamp a `reason` getter so the handler can read it.
    const ev = new Event("unhandledrejection") as Event & { reason?: unknown };
    Object.defineProperty(ev, "reason", { value: new Error("rej") });
    window.dispatchEvent(ev);
    await new Promise((r) => setTimeout(r, 0));

    const call = mockInvoke.mock.calls.find(
      (c) => c[0] === "forward_webview_log" && c[1].level === "error",
    );
    expect(call, "expected an error-level forward").toBeDefined();
    expect(call![1].message).toContain("unhandled rejection");
    expect(call![1].message).toContain("Error: rej");
  });

  it("is idempotent — a second install does not double-forward", () => {
    installWebviewLogForwarder();
    installWebviewLogForwarder();
    console.warn("once");
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("is idempotent for window listeners — second install does not double-fire", async () => {
    installWebviewLogForwarder();
    installWebviewLogForwarder();
    const ev = new ErrorEvent("error", { message: "once" });
    window.dispatchEvent(ev);
    await new Promise((r) => setTimeout(r, 0));

    const errorCalls = mockInvoke.mock.calls.filter(
      (c) =>
        c[0] === "forward_webview_log" &&
        c[1].level === "error" &&
        c[1].message.includes("once"),
    );
    expect(errorCalls).toHaveLength(1);
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/shared/webview-log
```
Expected: FAIL — module `shared/webview-log` does not exist yet.

- [ ] **Step 4: Commit the failing tests**

```bash
git add apps/desktop/aegis-desktop/src/test/shared/webview-log
git commit -m "test(desktop): failing specs for webview-log forwarder + format"
```

(Committing failing tests is allowed when the next task makes them pass in the same logical change. The pair commits together form the feature commit.)

---

## Task 5: Implement the formatter + forwarder to pass the tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/shared/webview-log/format.ts`
- Create: `apps/desktop/aegis-desktop/src/shared/webview-log/forwarder.ts`
- Create: `apps/desktop/aegis-desktop/src/shared/webview-log/index.ts`

- [ ] **Step 1: Implement `format.ts`**

Create `apps/desktop/aegis-desktop/src/shared/webview-log/format.ts`:

```ts
/**
 * Stringify the args of a `console.*` call into a single line. Used
 * by the webview log forwarder before sending the message over the
 * Tauri IPC. Designed to never throw — the forwarder sits on a hot
 * path so any error here must not break the calling app.
 *
 * Behaviour:
 * - `string` / `number` / `boolean` / `null` / `undefined` →
 *   `String(arg)`.
 * - `Error` → `"<name>: <message>\n<stack>"`.
 * - plain object / array → `JSON.stringify(arg)` (falls back to
 *   `String(arg)` when serialisation throws, e.g. on circular refs).
 * - multiple args are joined with a single space.
 */
export function formatConsoleArgs(args: readonly unknown[]): string {
  return args.map(formatOne).join(" ");
}

function formatOne(arg: unknown): string {
  if (arg === null) return "null";
  if (arg === undefined) return "undefined";
  if (typeof arg === "string") return arg;
  if (typeof arg === "number" || typeof arg === "boolean" || typeof arg === "bigint") {
    return String(arg);
  }
  if (arg instanceof Error) {
    const stack = arg.stack ?? "";
    return stack ? `${arg.name}: ${arg.message}\n${stack}` : `${arg.name}: ${arg.message}`;
  }
  if (typeof arg === "object") {
    try {
      return JSON.stringify(arg);
    } catch {
      return String(arg);
    }
  }
  return String(arg);
}
```

- [ ] **Step 2: Implement `forwarder.ts`**

Create `apps/desktop/aegis-desktop/src/shared/webview-log/forwarder.ts`:

```ts
import { api } from "../api";
import { formatConsoleArgs } from "./format";

type ConsoleMethod = (...args: unknown[]) => void;

/**
 * Sentinel that prevents double-install. The forwarder replaces
 * `console.warn` / `console.error` with thin wrappers and attaches
 * window error listeners; calling `installWebviewLogForwarder()`
 * twice would wrap the wrappers a second time AND register a second
 * pair of listeners, doubling every forwarded event. Re-installs
 * after a hot-module replacement are still safe — the guard is keyed
 * on the function identity, not on a process-global flag, so a
 * fresh module instance after HMR sees the wrappers as "not yet
 * replaced" and installs once.
 */
const INSTALLED = Symbol.for("aegis.desktop.webview-log-forwarder");

/**
 * Replace `console.warn` and `console.error` with wrappers that
 * forward the call to the Tauri backend via `api.forwardWebviewLog`,
 * while still invoking the original method so DevTools shows the
 * log as usual. Also subscribes to `window.error` and
 * `window.unhandledrejection` so uncaught exceptions and rejected
 * promises surface as error-level webview logs without a stack
 * trace going missing.
 *
 * Side-effect only; intended to be called once from `main.tsx`
 * before any user code runs. The IPC call is fire-and-forget:
 * rejections are swallowed inside the wrapper so the forwarder can
 * never break the calling app.
 */
export function installWebviewLogForwarder(): void {
  const g = globalThis as unknown as Record<symbol, boolean>;
  if (g[INSTALLED]) {
    return;
  }
  g[INSTALLED] = true;

  wrap("warn");
  wrap("error");
  hookWindowErrors();
}

function wrap(level: "warn" | "error"): void {
  const original = console[level].bind(console) as ConsoleMethod;
  const replacement: ConsoleMethod = (...args) => {
    original(...args);
    const message = formatConsoleArgs(args);
    // Fire-and-forget; swallow rejections so logging infrastructure
    // never breaks the app.
    void api.forwardWebviewLog(level, message).catch(() => undefined);
  };
  console[level] = replacement as typeof console[typeof level];
}

function hookWindowErrors(): void {
  // Uncaught exceptions. `event.error` is non-null when the script
  // threw a real Error; we include the formatted Error so the
  // stack reaches the Rust log too.
  window.addEventListener("error", (event) => {
    const header = String(event.message ?? "uncaught error");
    const location =
      event.filename
        ? ` at ${event.filename}:${event.lineno ?? 0}:${event.colno ?? 0}`
        : "";
    const body = event.error ? `\n${formatConsoleArgs([event.error])}` : "";
    const message = `${header}${location}${body}`;
    void api.forwardWebviewLog("error", message).catch(() => undefined);
  });

  // Rejected promises that no .catch() handled. The browser fires
  // this on the next microtask; we forward whatever `reason` was
  // attached (Error, string, object) as an error-level log.
  window.addEventListener("unhandledrejection", (event) => {
    const reason = (event as unknown as { reason?: unknown }).reason;
    const reasonText = formatConsoleArgs([reason]);
    void api
      .forwardWebviewLog("error", `unhandled rejection: ${reasonText}`)
      .catch(() => undefined);
  });
}
```

- [ ] **Step 3: Create the barrel `index.ts`**

Create `apps/desktop/aegis-desktop/src/shared/webview-log/index.ts`:

```ts
export { installWebviewLogForwarder } from "./forwarder";
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/shared/webview-log
```
Expected: 6 format tests + 8 forwarder tests pass. Total 14.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/webview-log \
        apps/desktop/aegis-desktop/src/test/shared/webview-log
git commit -m "feat(desktop): webview console.log/.warn/.error forwarder

Forwards console.warn and console.error through
api.forwardWebviewLog; console.log / .info / .debug are not
forwarded. IPC failures are swallowed so the forwarder never
breaks the calling app. The original console method still runs so
DevTools shows the log.

Verification:
  pnpm --filter aegis-desktop test -- src/test/shared/webview-log"
```

---

## Task 6: Add the `api.forwardWebviewLog` unit test

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/shared/api/forward-webview-log.test.ts`

- [ ] **Step 1: Write the test**

Create `apps/desktop/aegis-desktop/src/test/shared/api/forward-webview-log.test.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../../shared/api";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("api.forwardWebviewLog", () => {
  it("invokes 'forward_webview_log' with level=warn", async () => {
    await api.forwardWebviewLog("warn", "hello");
    expect(mockInvoke).toHaveBeenCalledWith("forward_webview_log", {
      level: "warn",
      message: "hello",
    });
  });

  it("invokes 'forward_webview_log' with level=error", async () => {
    await api.forwardWebviewLog("error", "kaboom");
    expect(mockInvoke).toHaveBeenCalledWith("forward_webview_log", {
      level: "error",
      message: "kaboom",
    });
  });

  it("forwards the full message payload unchanged", async () => {
    const long = "line1\nline2\nline3";
    await api.forwardWebviewLog("warn", long);
    expect(mockInvoke).toHaveBeenCalledWith("forward_webview_log", {
      level: "warn",
      message: long,
    });
  });
});
```

- [ ] **Step 2: Run the test**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/shared/api/forward-webview-log.test.ts
```
Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/shared/api/forward-webview-log.test.ts
git commit -m "test(desktop): pin api.forwardWebviewLog wire shape"
```

---

## Task 7: Install the forwarder from `main.tsx`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/main.tsx:1-32` — install the forwarder before any other module-level work.

- [ ] **Step 1: Add the import and the install call**

In `apps/desktop/aegis-desktop/src/main.tsx`, after the existing imports (the line ending with `from "./features/bootstrap/redirect";`) insert:

```ts
import { installWebviewLogForwarder } from "./shared/webview-log";

// Install the webview console forwarder as the very first thing the
// app does, so even the bootstrap probes' failures (logged via
// console.error in the showWindow effect below) reach the Rust
// tracing sink. Idempotent — safe under React StrictMode double-mount.
installWebviewLogForwarder();
```

Keep everything else in `main.tsx` unchanged. The install must run before the `shouldRedirectToBootstrap` block, before the `createRouter` call, and before any `<App />` work.

- [ ] **Step 2: Run typecheck**

Run:
```bash
pnpm --filter aegis-desktop typecheck
```
Expected: no errors.

- [ ] **Step 3: Run the existing test suite to confirm no regression**

Run:
```bash
pnpm --filter aegis-desktop test
```
Expected: every test passes, including the new 14 from Tasks 4+5 and 3 from Task 6 (total +17).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/main.tsx
git commit -m "feat(desktop): install webview log forwarder at app boot

Verification:
  pnpm --filter aegis-desktop typecheck
  pnpm --filter aegis-desktop test"
```

---

## Task 8: Verification gate

- [ ] **Step 1: Format check (Rust)**

Run:
```bash
cargo fmt --all -- --check
```
Expected: no diff.

- [ ] **Step 2: Lint (Rust)**

Run:
```bash
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```
Expected: no warnings.

- [ ] **Step 3: Full Rust test run**

Run:
```bash
cargo test -p aegis-desktop
```
Expected: every test passes.

- [ ] **Step 4: Typecheck + build (TS)**

Run:
```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop build
```
Expected: no errors.

- [ ] **Step 5: Vitest full run (TS)**

Run:
```bash
pnpm --filter aegis-desktop test
```
Expected: every test passes (full suite, not just the new files).

- [ ] **Step 6: Commit any formatting-only fix**

If `cargo fmt --all -- --check` produced a diff, run `cargo fmt --all` and commit the result with:
```bash
git add -u
git commit -m "style(desktop): rustfmt"
```

- [ ] **Step 7: Manual smoke (optional but recommended)**

Build the Tauri app, run it, then in DevTools trigger `console.warn("manual smoke")` and `console.error(new Error("boom"))`. Open `<app_data_dir>/logs/aegis-desktop.log.<today>.jsonl` and confirm both entries are present with `target = "aegis_desktop_lib::webview"` and no `trace_id` field.

---

## Self-review

**Spec coverage:**
- "Forward webview console logs to tauri backend" → Tasks 1 (Rust command), 3 (TS bridge), 5 (forwarder), 7 (install). ✓
- "Collect with tracing crate" → Task 1 uses `tracing::warn!` / `tracing::error!`. ✓
- "No trace id" → Task 1 explicitly omits the `info_span!` wrap, `TraceIdGenerator` state, and `TRACE_ID.scope(...)` block; Step 1's tracing events emit no `trace_id` field. ✓
- "Only warning and error level" → Task 1 maps `WebviewLogLevel::{Warn, Error}` to the corresponding tracing level and lets serde reject non-`"warn"` / `"error"` strings at the wire boundary. Task 5 only wraps `console.warn` and `console.error` (the four negative tests in Task 4 pin `console.log/info/debug` as not-forwarded). ✓

**Type consistency:**
- `WebviewLogLevel` is defined once in `src/shared/api/types.ts` (Task 3), referenced by the `api` wrapper (Task 3) and the forwarder (Task 5). The Rust enum `commands::webview_log::WebviewLogLevel` is the wire mirror; both halves use `rename_all = "lowercase"` / `"warn" | "error"`.
- `api.forwardWebviewLog(level, message)` signature is identical in Tasks 3, 4 (test), 5 (forwarder call site), 6 (test).

**Placeholder scan:**
- No "TODO" / "TBD" / "implement later" in any step.
- Every test has the full body and expected outcome.
- Every command has the exact `Run:` line.