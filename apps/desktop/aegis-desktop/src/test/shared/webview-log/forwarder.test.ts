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