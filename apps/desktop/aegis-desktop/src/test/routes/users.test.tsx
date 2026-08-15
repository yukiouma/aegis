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

function renderRoot(initialEntries: string[] = ["/users"]) {
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

const adminUser = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const rootUser = {
  ...adminUser,
  id: 99,
  code: "root",
  name: "Root",
  role: "root" as const,
};
const generalUser = {
  ...adminUser,
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general" as const,
};

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

describe("/users — sidebar gating", () => {
  it("shows the Management entry for an admin user", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => adminUser,
      list_users: () => [],
    });
    await renderRoot(["/users"]);
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("shows the Management entry for a root user", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => rootUser,
      list_users: () => [],
    });
    await renderRoot(["/users"]);
    expect(await screen.findByText("Management")).toBeInTheDocument();
  });

  it("hides the Management entry for a general user", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => generalUser,
    });
    await renderRoot(["/users"]);
    await screen.findByTestId("sidebar");
    expect(screen.queryByText("Management")).not.toBeInTheDocument();
  });

  it("expands the Users submenu when Management is clicked", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => adminUser,
      list_users: () => [],
    });
    await renderRoot(["/"]);
    expect(screen.queryByText("Users")).not.toBeInTheDocument();
    await userEvent.click(await screen.findByText("Management"));
    expect(await screen.findByText("Users")).toBeInTheDocument();
  });
});

describe("/users — routing", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => adminUser,
      list_users: () => [],
    });
  });

  it("renders the Sidebar and the Users page at /users", async () => {
    const { router } = await renderRoot(["/users"]);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/users");
  });

  it("navigates from /settings to /users when Users submenu is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);
    await userEvent.click(await screen.findByText("Management"));
    await userEvent.click(await screen.findByText("Users"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/users"),
    );
  });

  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/users"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});