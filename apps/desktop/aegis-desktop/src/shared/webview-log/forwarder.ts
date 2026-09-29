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
 * on the symbol stored on globalThis, so a fresh module instance
 * after HMR sees the sentinel and short-circuits.
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