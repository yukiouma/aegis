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