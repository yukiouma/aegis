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

const codelist = {
  id: 100,
  versionId: 1,
  code: "AE",
  extensible: true,
  name: "Adverse Events",
  submissionValue: "AE",
  synonym: "Adverse Event",
  definition: "Any untoward medical occurrence...",
  nciPreferredTerm: "Adverse Event",
  createdAt: "2026-03-27T00:00:00Z",
  updatedAt: "2026-03-27T00:00:00Z",
};

function makeItem(i: number) {
  return {
    id: i,
    codelistId: 100,
    versionId: 1,
    code: `I${i}`,
    submissionValue: `SV${i}`,
    synonym: "",
    definition: "",
    nciPreferredTerm: "",
    createdAt: "2026-03-27T00:00:00Z",
    updatedAt: "2026-03-27T00:00:00Z",
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

describe("CodeListDetailPage pagination", () => {
  it("loads page 0 of 20, then loads page 1 on intersection", async () => {
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
      get_code_list_by_id: () => codelist,
      list_code_items: (args) => {
        const offset = Number(args?.offset ?? 0);
        const rows = Array.from({ length: 20 }, (_, i) => makeItem(offset + i + 1));
        const nextOffset = offset + 20 < 40 ? offset + 20 : undefined;
        return { items: rows, nextOffset };
      },
    });

    await renderPage(["/terminology/sdtm/codelists/100?versionId=1"]);

    await waitFor(() => expect(screen.getByText("I1")).toBeInTheDocument());
    expect(screen.getByText("I20")).toBeInTheDocument();

    await waitFor(() => expect(observers.length).toBeGreaterThan(0));
    observers[0].cb(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      observers[0] as unknown as IntersectionObserver,
    );

    await waitFor(() => expect(screen.getByText("I21")).toBeInTheDocument());
    // Both pages must remain in the DOM — pagination is append-only.
    expect(screen.getByText("I1")).toBeInTheDocument();
    expect(screen.getByText("I20")).toBeInTheDocument();
    expect(screen.getByText("I40")).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith(
      "list_code_items",
      expect.objectContaining({ offset: 20, limit: 20 }),
    );
  });
});

describe("CodeListDetailPage debounce", () => {
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
      get_code_list_by_id: () => codelist,
      list_code_items: () => ({ items: [], nextOffset: undefined }),
    });

    // Render under real timers first so the router's internal setTimeouts
    // can complete and the placeholder element lands in the DOM.
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await renderPage(["/terminology/sdtm/codelists/100?versionId=1"]);
    await screen.findByPlaceholderText(
      /search by code, submission value/i,
    );

    // Now enable fake timers and exercise the debounced input.
    vi.useFakeTimers({ shouldAdvanceTime: true });
    for (let i = 0; i < 15; i++) {
      await user.keyboard("a");
      await vi.advanceTimersByTimeAsync(200);
    }
    await vi.advanceTimersByTimeAsync(500);

    const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_items");
    expect(calls.length).toBeGreaterThanOrEqual(1);
    expect(calls.length).toBeLessThanOrEqual(5);
    vi.useRealTimers();
  });
});
