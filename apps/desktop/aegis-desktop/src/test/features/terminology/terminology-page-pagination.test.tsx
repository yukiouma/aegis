import "@testing-library/jest-dom/vitest";
import {
  cleanup,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

const versions = [
  {
    id: 1,
    kind: "sdtm" as const,
    name: "2026-01-01",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];

function makeRow(i: number) {
  return {
    id: i,
    versionId: 1,
    code: `C${i}`,
    extensible: false,
    name: `Name ${i}`,
    submissionValue: `SV${i}`,
    synonym: "",
    definition: "",
    nciPreferredTerm: "",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

function renderPage(initialEntries: string[]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("TerminologyPage pagination", () => {
  it("loads page 0 of 20, then loads page 1 on intersection", async () => {
    // Replace the noop IntersectionObserver with one that records callbacks.
    const observers: Array<{ cb: IntersectionObserverCallback }> = [];
    const fakeObserver = class {
      cb: IntersectionObserverCallback;
      constructor(cb: IntersectionObserverCallback) {
        this.cb = cb;
        observers.push(this);
      }
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
      takeRecords(): IntersectionObserverEntry[] {
        return [];
      }
    };
    (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver =
      fakeObserver;

    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_terminology_versions: () => versions,
      list_code_lists: (args) => {
        const offset = Number(args?.offset ?? 0);
        const rows = Array.from({ length: 20 }, (_, i) => makeRow(offset + i + 1));
        const nextOffset = offset + 20 < 40 ? offset + 20 : undefined;
        return { codelists: rows, nextOffset };
      },
    });

    await renderPage(["/terminology/sdtm?versionId=1"]);

    await waitFor(() => expect(screen.getByText("C1")).toBeInTheDocument());
    expect(screen.getByText("C20")).toBeInTheDocument();
    expect(screen.queryByText("C21")).not.toBeInTheDocument();

    // Sentinel mounts an observer after the first page renders.
    await waitFor(() => expect(observers.length).toBeGreaterThan(0));
    // Fire the first observer to request the next page.
    observers[0].cb(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      observers[0] as unknown as IntersectionObserver,
    );

    await waitFor(() => expect(screen.getByText("C21")).toBeInTheDocument());
    expect(mockInvoke).toHaveBeenCalledWith(
      "list_code_lists",
      expect.objectContaining({ offset: 20, limit: 20 }),
    );

    // No third page — second response had nextOffset = undefined.
    observers[0].cb(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      observers[0] as unknown as IntersectionObserver,
    );
    await waitFor(() =>
      expect(
        mockInvoke.mock.calls.filter((c) => c[0] === "list_code_lists"),
      ).toHaveLength(2),
    );
  });
});

describe("TerminologyPage debounce", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("debounces continuous typing to at most one request per second", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_terminology_versions: () => versions,
      list_code_lists: () => ({ codelists: [], nextOffset: undefined }),
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await renderPage(["/terminology/sdtm?versionId=1"]);

    await screen.findByPlaceholderText(/search by code, name/i);

    // Continuous typing for ~3 s in 200 ms steps → at most a handful of
    // list_code_lists calls (one per maxWaitMs = 1000 ms).
    for (let i = 0; i < 15; i++) {
      await user.keyboard("a");
      await vi.advanceTimersByTimeAsync(200);
    }
    // Final trailing fire happens ~300 ms after the last keystroke.
    await vi.advanceTimersByTimeAsync(500);

    const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_lists");
    expect(calls.length).toBeGreaterThanOrEqual(1);
    expect(calls.length).toBeLessThanOrEqual(5);
  });

  it("resets offset when the fragment changes", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_terminology_versions: () => versions,
      list_code_lists: (args) => {
        const offset = Number(args?.offset ?? 0);
        const fragment = String(args?.fragment ?? "");
        const rows = fragment
          ? [makeRow(101)]
          : Array.from({ length: 20 }, (_, i) => makeRow(offset + i + 1));
        return { codelists: rows, nextOffset: undefined };
      },
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await renderPage(["/terminology/sdtm?versionId=1"]);

    await screen.findByText("C1");

    const input = screen.getByPlaceholderText(/search by code, name/i);
    await user.type(input, "AE");
    await vi.advanceTimersByTimeAsync(400);

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_lists");
      const lastCall = calls[calls.length - 1]!;
      expect(lastCall[1]).toMatchObject({ fragment: "AE", offset: 0, limit: 20 });
    });
  });
});
