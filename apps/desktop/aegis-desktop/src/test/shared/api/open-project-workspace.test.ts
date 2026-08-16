import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockShow = vi.fn();
const mockSetFocus = vi.fn();

// `vi.mock` is hoisted to the top of the file, before any `const`/`let`.
// `vi.hoisted` makes a value available inside the factory without tripping
// the temporal dead zone check.
const { WebviewWindowMock, getByLabelMock } = vi.hoisted(() => {
  const show = vi.fn();
  const setFocus = vi.fn();
  const Ctor = vi.fn().mockImplementation(() => ({ show, setFocus }));
  (Ctor as unknown as { getByLabel: ReturnType<typeof vi.fn> }).getByLabel =
    vi.fn();
  return {
    WebviewWindowMock: Ctor,
    getByLabelMock: (Ctor as unknown as { getByLabel: ReturnType<typeof vi.fn> })
      .getByLabel,
    show,
    setFocus,
  };
});

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: WebviewWindowMock,
}));

import { api } from "../../../shared/api";

beforeEach(() => {
  WebviewWindowMock.mockClear();
  getByLabelMock.mockReset();
  mockShow.mockReset();
  mockSetFocus.mockReset();

  getByLabelMock.mockResolvedValue(null);
  WebviewWindowMock.mockImplementation(() => ({
    show: mockShow,
    setFocus: mockSetFocus,
  }));
  mockShow.mockResolvedValue(undefined);
  mockSetFocus.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.clearAllMocks();
});

describe("api.openProjectWorkspace", () => {
  it("creates a new maximized window when no window with that label exists", async () => {
    await api.openProjectWorkspace("DEMO-001");

    expect(getByLabelMock).toHaveBeenCalledWith("project:DEMO-001");
    expect(WebviewWindowMock).toHaveBeenCalledWith("project:DEMO-001", {
      url: "/project/DEMO-001",
      title: "DEMO-001",
      width: 1100,
      height: 720,
      minWidth: 720,
      minHeight: 480,
      maximized: true,
    });
  });

  it("focuses the existing window instead of creating a duplicate", async () => {
    const existing = { show: mockShow, setFocus: mockSetFocus };
    getByLabelMock.mockResolvedValue(existing);

    await api.openProjectWorkspace("DEMO-001");

    expect(WebviewWindowMock).not.toHaveBeenCalled();
    expect(mockShow).toHaveBeenCalledTimes(1);
    expect(mockSetFocus).toHaveBeenCalledTimes(1);
  });
});