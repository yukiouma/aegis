import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { UserListPage } from "../../pages/UserList";
import type { UserView } from "../../api";
import { mockCommands, httpError } from "../tauri-mock";
import { renderInRouter } from "../file-route-utils";
import { TestQueryProvider } from "../test-query-provider";

const rootUser: UserView = {
  id: 1,
  code: "root",
  name: "Root",
  role: "root",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const adminUser: UserView = {
  id: 2,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const generalUser: UserView = {
  id: 3,
  code: "bob",
  name: "Bob",
  role: "general",
  active: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

async function renderPage(current: UserView, list: UserView[]) {
  mockCommands({
    current_user: () => current,
    list_users: () => list,
    update_user: () => list.find((u) => u.code === "bob") ?? generalUser,
  });
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <UserListPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

/** Switch inputs in row order. */
function getSwitches(): HTMLInputElement[] {
  return Array.from(
    document.querySelectorAll('input[type="checkbox"]'),
  ) as HTMLInputElement[];
}

describe("UserListPage — root filter", () => {
  it("does NOT render users whose role is root", async () => {
    await renderPage(adminUser, [rootUser, adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.queryByText("root")).not.toBeInTheDocument();
  });
});

describe("UserListPage — role gate", () => {
  it("renders nothing for a general user", async () => {
    const { container } = await renderPage(generalUser, [adminUser]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("current_user");
    });
    expect(container.textContent).not.toContain("alice");
  });

  it("renders the table for an admin user", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("renders the table for a root user (viewing other non-root users)", async () => {
    await renderPage(rootUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    expect(screen.getByText("bob")).toBeInTheDocument();
  });
});

describe("UserListPage — toggle calls update_user", () => {
  it("calls update_user with { code, body: { active: !prev } }", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("bob");
    const switches = getSwitches();
    const bobSwitch = switches[1]; // alice (self, disabled) is first
    await userEvent.click(bobSwitch);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { active: true },
      });
    });
  });
});

describe("UserListPage — self-disable", () => {
  it("disables both the Switch and the Select on the current user's row", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const switches = getSwitches();
    expect(switches[0].disabled).toBe(true); // alice Switch

    const selects = screen.getAllByRole("combobox");
    const aliceSelect = selects.find((s) => s.textContent === "Admin");
    expect(aliceSelect).toBeDefined();
    expect(aliceSelect).toHaveAttribute("aria-disabled", "true");
  });
});

describe("UserListPage — role change", () => {
  /** Find the Select on a row whose visible text equals `label`. */
  function selectWithLabel(label: string): HTMLElement {
    const selects = screen.getAllByRole("combobox");
    const match = selects.find((s) => s.textContent === label);
    if (!match) throw new Error(`Select with label "${label}" not found`);
    return match;
  }

  it("calls update_user with { code, body: { role } } when a role is picked", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("bob");
    await userEvent.click(selectWithLabel("General"));
    await userEvent.click(screen.getByRole("option", { name: "Admin" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { role: "admin" },
      });
    });
  });
});

describe("UserListPage — error surfaces", () => {
  it("renders an Alert when list_users fails", async () => {
    mockCommands({
      current_user: () => adminUser,
      list_users: () => Promise.reject(httpError(500, "boom", "boom")),
    });
    await renderInRouter(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <UserListPage />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });
});

describe("UserListPage — search", () => {
  it("renders a TextField with a Search icon in place of the heading", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    expect(input).toBeInTheDocument();
    // The Search icon is wired via InputAdornment with position="start".
    // MUI renders an svg inside the adornment — assert presence loosely
    // (no role="img" on MUI icons in v9).
    expect(input.parentElement?.querySelector("svg")).not.toBeNull();
  });

  it("filters rows by code substring (case-insensitive)", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "BO");
    // Only bob (code "bob") remains — alice (code "alice") does not
    // contain "bo".
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("filters rows by name substring", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "bob");
    // "bob" matches generalUser's code AND name.
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });

  it("shows 'no matches' empty state when query yields zero rows", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "xyz");
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    expect(screen.queryByText("bob")).not.toBeInTheDocument();
    expect(screen.getByText(/no matching users/i)).toBeInTheDocument();
  });

  it("clearing the query restores the full list", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const input = screen.getByPlaceholderText(/search by name or code/i);
    await userEvent.type(input, "bob");
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    await userEvent.clear(input);
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("bob")).toBeInTheDocument();
  });
});