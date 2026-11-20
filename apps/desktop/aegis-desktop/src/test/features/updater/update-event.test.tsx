import "@testing-library/jest-dom/vitest";
import { cleanup, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The event subscription can fail — outside a Tauri window the plugin
// has nothing to listen on. A failed subscription must not leave an
// unhandled rejection behind.
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.reject(new Error("event plugin unavailable")),
  emit: vi.fn(),
}));

import { useUpdateStatusEvent } from "../../../features/updater";
import { captureUnhandledRejections } from "../../helpers/unhandled-rejections";

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

afterEach(() => cleanup());

describe("useUpdateStatusEvent", () => {
  it("handles a subscription that fails instead of leaving an unhandled rejection", async () => {
    const captured = captureUnhandledRejections();
    try {
      renderHook(() => useUpdateStatusEvent(), {
        wrapper: wrapperFor(makeClient()),
      });
      // Give the rejected subscription a tick to surface as an unhandled
      // rejection if nothing handled it.
      await new Promise((resolve) => setTimeout(resolve, 20));
      expect(captured.rejections).toEqual([]);
    } finally {
      captured.release();
    }
  });
});
