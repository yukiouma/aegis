import "@testing-library/jest-dom/vitest";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The store can fail — a locked or corrupt settings file. The deferral
// is then unknown, and an unknown deferral must not become a prompt.
//
// The failure is injected on `get`, not on `load`: `persist.ts` caches the
// store promise in a module-level singleton, so `load` is only ever
// called once per test file.
const { storeState } = vi.hoisted(() => ({
  storeState: {
    failRead: false,
    failWrite: false,
    values: new Map<string, unknown>(),
  },
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  load: () =>
    Promise.resolve({
      get: (k: string) =>
        storeState.failRead
          ? Promise.reject(new Error("store unavailable"))
          : Promise.resolve(storeState.values.get(k)),
      set: (k: string, v: unknown) => {
        if (storeState.failWrite) {
          return Promise.reject(new Error("store unavailable"));
        }
        storeState.values.set(k, v);
        return Promise.resolve();
      },
      delete: (k: string) => {
        storeState.values.delete(k);
        return Promise.resolve();
      },
      save: () =>
        storeState.failWrite
          ? Promise.reject(new Error("store unavailable"))
          : Promise.resolve(),
    }),
}));

import { useUpdatePrompt } from "../../../features/updater";
import { mockCommands } from "../../helpers/tauri-mock";
import { captureUnhandledRejections } from "../../helpers/unhandled-rejections";

const READY = {
  status: { kind: "ready", version: "0.1.1", notes: null, date: null },
  currentVersion: "0.1.0",
  lastCheckedAt: "2026-11-18T06:05:10Z",
};

function makeClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

function wrapperFor(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

/** Collect the rejections that escape every handler. */

beforeEach(() => {
  storeState.failRead = false;
  storeState.failWrite = false;
  storeState.values.clear();
});
afterEach(() => cleanup());

describe("useUpdatePrompt", () => {
  it("prompts for a ready release that was never deferred", async () => {
    mockCommands({ update_status: () => READY });
    const { result } = renderHook(() => useUpdatePrompt(), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.shouldPrompt).toBe(true));
    expect(result.current.ready?.version).toBe("0.1.1");
  });

  it("does not prompt for a release that was deferred", async () => {
    storeState.values.set("update.deferredVersion", "0.1.1");
    mockCommands({ update_status: () => READY });
    const { result } = renderHook(() => useUpdatePrompt(), {
      wrapper: wrapperFor(makeClient()),
    });
    await waitFor(() => expect(result.current.shouldPrompt).toBe(false));
  });

  it("handles a deferral read that fails instead of leaving an unhandled rejection", async () => {
    storeState.failRead = true;
    const captured = captureUnhandledRejections();
    try {
      mockCommands({ update_status: () => READY });
      const { result } = renderHook(() => useUpdatePrompt(), {
        wrapper: wrapperFor(makeClient()),
      });
      await waitFor(() => expect(result.current.ready?.version).toBe("0.1.1"));
      // Give the rejected read a tick to surface as an unhandled
      // rejection if nothing handled it.
      await new Promise((resolve) => setTimeout(resolve, 20));
      expect(captured.rejections).toEqual([]);
      // An unknown deferral must not become a prompt.
      expect(result.current.shouldPrompt).toBe(false);
    } finally {
      captured.release();
    }
  });

  it("keeps the deferral for this session when the store write fails", async () => {
    storeState.failWrite = true;
    const captured = captureUnhandledRejections();
    try {
      mockCommands({ update_status: () => READY });
      const { result } = renderHook(() => useUpdatePrompt(), {
        wrapper: wrapperFor(makeClient()),
      });
      await waitFor(() => expect(result.current.shouldPrompt).toBe(true));

      await act(async () => {
        await result.current.defer();
      });
      await new Promise((resolve) => setTimeout(resolve, 20));

      expect(captured.rejections).toEqual([]);
      // The deferral still applies to this session even though it could
      // not be written to the store.
      expect(result.current.shouldPrompt).toBe(false);
    } finally {
      captured.release();
    }
  });
});
