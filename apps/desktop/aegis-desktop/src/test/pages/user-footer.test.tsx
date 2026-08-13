import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../file-route-utils";
import { mockCommands } from "../tauri-mock";
import { UserFooter } from "../../pages/UserFooter";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

afterEach(() => {
  cleanup();
});

function renderFooter(props: { sidebarOpen?: boolean } = {}) {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <UserFooter sidebarOpen={props.sidebarOpen ?? true} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

function userViewFixture(overrides: Partial<{
  code: string;
  name: string;
  role: "root" | "admin" | "general";
}> = {}) {
  return {
    id: 1,
    code: overrides.code ?? "alice",
    name: overrides.name ?? "Alice",
    role: overrides.role ?? "general",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

describe("UserFooter", () => {
  it("renders the user name after getCurrentUser resolves", async () => {
    mockCommands({
      current_user: () => userViewFixture({ name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Alice")).toBeInTheDocument();
  });

  it("shows the role chip when role is admin", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "admin", name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Admin")).toBeInTheDocument();
  });

  it("shows the role chip when role is root", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "root", name: "Alice" }),
    });
    await renderFooter();
    expect(await screen.findByText("Root")).toBeInTheDocument();
  });

  it("does not show a role chip when role is general", async () => {
    mockCommands({
      current_user: () => userViewFixture({ role: "general", name: "Alice" }),
    });
    await renderFooter();
    await screen.findByText("Alice");
    expect(screen.queryByText("Admin")).not.toBeInTheDocument();
    expect(screen.queryByText("Root")).not.toBeInTheDocument();
  });

  it("opens the confirm dialog when the logout button is clicked", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
      logout: () => undefined,
    });
    await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    expect(screen.getByText(/confirm logout/i)).toBeInTheDocument();
    expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
  });

  it("calls logout and navigates to /login on confirm", async () => {
    const logout = vi.fn().mockResolvedValue(undefined);
    mockCommands({
      current_user: () => userViewFixture(),
      logout,
    });
    const { router } = await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    await userEvent.click(screen.getByRole("button", { name: /^confirm$/i }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
  });

  it("cancels without calling logout", async () => {
    const logout = vi.fn();
    mockCommands({
      current_user: () => userViewFixture(),
      logout,
    });
    await renderFooter();
    await userEvent.click(await screen.findByRole("button", { name: /log out/i }));
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    expect(logout).not.toHaveBeenCalled();
    // MUI Dialog animates out, so wait for the title to disappear.
    await waitFor(() =>
      expect(screen.queryByText(/confirm logout/i)).not.toBeInTheDocument(),
    );
  });

  it("hides name and chip when sidebarOpen is false but keeps the logout button", async () => {
    mockCommands({
      current_user: () => userViewFixture({ name: "Alice", role: "admin" }),
    });
    await renderFooter({ sidebarOpen: false });
    // Wait for fetch to settle before asserting absence of the name.
    await waitFor(() => expect(screen.queryByText("Alice")).not.toBeInTheDocument());
    expect(screen.queryByText("Admin")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /log out/i })).toBeInTheDocument();
  });
});