import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { SettingsPage } from "../../../features/settings/pages/SettingsPage";
import { renderInRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

afterEach(() => {
  cleanup();
});

function renderSettings() {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <SettingsPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

function userViewFixture() {
  return {
    id: 1,
    code: "alice",
    name: "Alice",
    role: "general" as const,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

describe("SettingsPage — update password", () => {
  it("renders the Update password button", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    expect(
      await screen.findByRole("button", { name: /update password/i }),
    ).toBeInTheDocument();
  });

  it("opens the password dialog when the button is clicked", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    expect(screen.getByRole("dialog", { name: /update password/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/new password/i)).toBeInTheDocument();
  });

  it("disables Next when the password field is empty", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    const next = screen.getByRole("button", { name: /^next$/i });
    expect(next).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    expect(next).toBeEnabled();
  });

  it("advances from the password dialog to the confirm dialog", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    // Password dialog gone, confirm dialog present.
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: /update password/i }),
      ).not.toBeInTheDocument(),
    );
    expect(
      screen.getByRole("dialog", { name: /confirm password update/i }),
    ).toBeInTheDocument();
  });

  it("cancels the password dialog and clears the field on re-open", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: /update password/i }),
      ).not.toBeInTheDocument(),
    );
    // Re-open and confirm the field is empty.
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    expect(screen.getByLabelText(/new password/i)).toHaveValue("");
  });

  it("cancels the confirm dialog without calling update_user_credential", async () => {
    const updateCred = vi.fn().mockResolvedValue({
      userCode: "alice",
      passwordHash: "h",
      tokenVersion: 2,
    });
    const logout = vi.fn().mockResolvedValue(undefined);
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    expect(updateCred).not.toHaveBeenCalled();
    expect(logout).not.toHaveBeenCalled();
  });

  it("calls update_user_credential, then logout, then navigates to /login", async () => {
    const updateCred = vi.fn().mockResolvedValue({
      userCode: "alice",
      passwordHash: "h",
      tokenVersion: 2,
    });
    const logout = vi.fn().mockResolvedValue(undefined);
    const calls: string[] = [];
    updateCred.mockImplementation(() => {
      calls.push("update");
      return Promise.resolve({
        userCode: "alice",
        passwordHash: "h",
        tokenVersion: 2,
      });
    });
    logout.mockImplementation(() => {
      calls.push("logout");
      return Promise.resolve(undefined);
    });
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    const { router } = await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^update$/i }));
    await waitFor(() => expect(updateCred).toHaveBeenCalledWith({
      userCode: "alice",
      password: "hunter2",
    }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(calls).toEqual(["update", "logout"]);
  });

  it("keeps the confirm dialog open and shows an error when update fails", async () => {
    const updateCred = vi.fn().mockRejectedValue({
      kind: "http",
      status: 400,
      code: "weak_password",
      message: "too weak",
    });
    const logout = vi.fn();
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "x");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^update$/i }));
    await waitFor(() =>
      expect(
        screen.getByRole("dialog", { name: /confirm password update/i }),
      ).toBeInTheDocument(),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      /weak_password: too weak/,
    );
    expect(logout).not.toHaveBeenCalled();
  });
});
