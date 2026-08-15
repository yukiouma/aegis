import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { mockCommands, mockInvoke } from "../tauri-mock";
import { TestQueryProvider } from "../test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderRoot(initialEntries: string[] = ["/projects"]) {
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

describe("/projects — authenticated", () => {
  beforeEach(() => {
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
      list_projects: () => [],
    });
  });

  it("renders the Sidebar and the Project page at /projects", async () => {
    const { router } = await renderRoot(["/projects"]);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    // The page is rendered (table/list view is reachable). We assert on
    // path + sidebar rather than a heading, because the heading is
    // intentionally absent from the current page design.
    expect(router.state.location.pathname).toBe("/projects");
  });

  it("renders a Projects link in the Sidebar", async () => {
    await renderRoot(["/"]);
    expect(await screen.findByText("Projects")).toBeInTheDocument();
  });

  it("navigates from /settings to /projects when Projects is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);
    await userEvent.click(await screen.findByText("Projects"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/projects"),
    );
  });
});

describe("/projects — unauthenticated", () => {
  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/projects"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});