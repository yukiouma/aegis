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
  it("disables the Switch on the current user's own row", async () => {
    await renderPage(adminUser, [adminUser, generalUser]);
    await screen.findByText("alice");
    const switches = getSwitches();
    expect(switches[0].disabled).toBe(true); // alice = self
    expect(switches[1].disabled).toBe(false); // bob
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