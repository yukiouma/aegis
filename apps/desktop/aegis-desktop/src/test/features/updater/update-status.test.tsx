import "@testing-library/jest-dom/vitest";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// In-memory pub-sub for events, so the test can fire the payload
// directly — the same pattern as
// src/test/features/settings/settings-persist.test.tsx.
type Handler = (e: { payload: unknown }) => void;
const handlers: Handler[] = [];
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_name: string, h: Handler) => {
    handlers.push(h);
    return Promise.resolve(() => {
      const idx = handlers.indexOf(h);
      if (idx >= 0) handlers.splice(idx, 1);
    });
  },
  emit: vi.fn(),
}));

import {
  useCheckForUpdates,
  useInstallUpdate,
  useUpdateReadyEvent,
  useUpdateStatus,
} from "../../../features/updater";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

function makeClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

function wrapperFor(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

beforeEach(() => {
  mockInvoke.mockReset();
  handlers.length = 0;
});
afterEach(() => cleanup());

describe("the updater hooks", () => {
  it("reads the status through the update_status command", async () => {
    mockCommands({
      update_status: () => ({
        status: { kind: "ready", version: "0.1.1", notes: null, date: null },
        currentVersion: "0.1.0",
        lastCheckedAt: "2026-11-18T06:05:10Z",
      }),
    });
    const { result } = renderHook(() => useUpdateStatus(), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.data?.status.kind).toBe("ready"));
    expect(result.current.data?.currentVersion).toBe("0.1.0");
  });

  it("checkForUpdates posts to the check_for_updates command", async () => {
    mockCommands({
      check_for_updates: () => ({
        status: { kind: "idle" },
        currentVersion: "0.1.0",
        lastCheckedAt: null,
      }),
    });
    const { result } = renderHook(() => useCheckForUpdates(), {
      wrapper: wrapperFor(makeClient()),
    });
    await act(async () => {
      await result.current.mutateAsync();
    });
    expect(mockInvoke).toHaveBeenCalledWith("check_for_updates");
  });

  it("installReadyUpdate posts to the install_ready_update command", async () => {
    mockCommands({ install_ready_update: () => undefined });
    const { result } = renderHook(() => useInstallUpdate(), {
      wrapper: wrapperFor(makeClient()),
    });
    await act(async () => {
      await result.current.mutateAsync();
    });
    expect(mockInvoke).toHaveBeenCalledWith("install_ready_update");
  });

  it("invalidates the status query when the ready event fires", async () => {
    mockCommands({
      update_status: () => ({
        status: { kind: "idle" },
        currentVersion: "0.1.0",
        lastCheckedAt: null,
      }),
    });
    function Probe() {
      useUpdateReadyEvent();
      return null;
    }
    const client = makeClient();
    renderHook(() => Probe(), { wrapper: wrapperFor(client) });
    await waitFor(() => expect(handlers.length).toBe(1));
    const invalidate = vi.spyOn(client, "invalidateQueries");
    await act(async () => {
      handlers[0]({ payload: { version: "0.1.1" } });
    });
    expect(invalidate).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: ["updater", "status"] }),
    );
  });
});
