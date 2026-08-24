import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

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

describe("AppLayout (authenticated)", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the Sidebar and the Home page content at /", async () => {
    const { router } = await renderRoot(["/"]);

    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/");
  });

  it("navigates to /settings when the Settings menu item is clicked", async () => {
    const { router } = await renderRoot(["/"]);

    await userEvent.click(screen.getByText("Settings"));

    expect(router.state.location.pathname).toBe("/settings");
    expect(
      screen.getByRole("heading", { level: 4, name: /settings/i }),
    ).toBeInTheDocument();
  });

  it("navigates back to / when the Home menu item is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);

    await userEvent.click(screen.getByText("Home"));

    expect(router.state.location.pathname).toBe("/");
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});

describe("AppLayout (role-based menu visibility)", () => {
  function mockUser(role: "root" | "admin" | "general") {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role,
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
    });
  }

  it("shows the Knowledge Base entry for a general (non-manager) user", async () => {
    mockUser("general");
    await renderRoot(["/"]);

    // Use findBy* to wait for the current_user query to settle, since
    // the menu is recomputed from `role` once the query resolves.
    expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();
    // Management is gated behind admin/root — must not be present.
    expect(screen.queryByText("Management")).not.toBeInTheDocument();
  });

  it("shows the Knowledge Base entry for an admin user", async () => {
    mockUser("admin");
    await renderRoot(["/"]);

    expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("shows the Knowledge Base entry for a root user", async () => {
    mockUser("root");
    await renderRoot(["/"]);

    expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("still surfaces Knowledge Base when current_user has not yet resolved", async () => {
    // Without mocking current_user the query errors out, so role is
    // undefined. The menu falls into the non-manager branch, which now
    // includes Knowledge Base for everyone.
    mockCommands({ is_logged_in: () => true });
    await renderRoot(["/"]);

    expect(screen.getByText("Knowledge Base")).toBeInTheDocument();
    expect(screen.queryByText("Management")).not.toBeInTheDocument();
  });
});

describe("AppLayout (unauthenticated)", () => {
  it("redirects / to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });

    const { router } = await renderRoot(["/"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });

  it("redirects /settings to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });

    const { router } = await renderRoot(["/settings"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
  });

  it("redirects to /login when the login check itself fails", async () => {
    mockCommands({
      is_logged_in: () => {
        throw { kind: "store", message: "auth.bin is locked" };
      },
    });

    const { router } = await renderRoot(["/"]);

    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
  });

  it("does not guard /login itself", async () => {
    mockCommands({ is_logged_in: () => false });

    const { router } = await renderRoot(["/login"]);

    expect(router.state.location.pathname).toBe("/login");
    expect(
      await screen.findByRole("radio", { name: /Domain information/i }),
    ).toBeInTheDocument();
  });
});
