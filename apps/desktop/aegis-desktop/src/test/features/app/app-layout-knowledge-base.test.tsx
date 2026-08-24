import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

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
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => {
  cleanup();
});

function renderRoot(initialEntries: string[] = ["/"]) {
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

describe("AppLayout — Knowledge Base menu entry", () => {
  it("renders the Knowledge Base label in the sidebar", async () => {
    mockCommands({ is_logged_in: () => true });
    await renderRoot(["/"]);
    expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();
  });

  it("expands Knowledge Base on click and shows Metadata sub-menu", async () => {
    mockCommands({ is_logged_in: () => true });
    await renderRoot(["/"]);
    await userEvent.click(await screen.findByText("Knowledge Base"));
    expect(screen.getByText("Metadata")).toBeInTheDocument();
  });

  it("navigates to /metadata when the Metadata sub-menu is clicked", async () => {
    mockCommands({ is_logged_in: () => true });
    const { router } = await renderRoot(["/"]);
    await userEvent.click(await screen.findByText("Knowledge Base"));
    await userEvent.click(screen.getByText("Metadata"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/metadata"),
    );
  });

  it("shows Knowledge Base after Management for an admin user", async () => {
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
    });
    await renderRoot(["/"]);
    // All three labels must be present.
    expect(await screen.findByText("Management")).toBeInTheDocument();
    expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();
    expect(await screen.findByText("Settings")).toBeInTheDocument();
  });
});