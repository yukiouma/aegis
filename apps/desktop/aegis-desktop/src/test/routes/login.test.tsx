import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { httpError, mockCommands, mockInvoke } from "../tauri-mock";

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

function renderLogin() {
  return renderWithFullRouter({
    initialEntries: ["/login"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

async function chooseAccount() {
  await userEvent.click(screen.getByRole("radio", { name: /Account and password/i }));
  await screen.findByLabelText(/^Account$/i);
}

describe("LoginPage — method step layout", () => {
  it("selects Domain by default", async () => {
    await renderLogin();
    const domainRadio = screen.getByRole("radio", { name: /Domain information/i });
    const accountRadio = screen.getByRole("radio", { name: /Account and password/i });
    expect(domainRadio).toBeChecked();
    expect(accountRadio).not.toBeChecked();
  });

  it("hides the account fields when Domain is selected", async () => {
    await renderLogin();
    expect(screen.queryByLabelText(/^Account$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^Password$/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeInTheDocument();
  });

  it("shows the account fields when Account is selected", async () => {
    await renderLogin();
    await chooseAccount();
    expect(screen.getByLabelText(/^Account$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Password$/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeInTheDocument();
  });

  it("disables Login until both account fields are filled", async () => {
    await renderLogin();
    await chooseAccount();
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeEnabled();
  });

  it("swaps the fields back off when Domain is re-selected", async () => {
    await renderLogin();
    await chooseAccount();
    await userEvent.click(screen.getByRole("radio", { name: /Domain information/i }));
    expect(screen.queryByLabelText(/^Account$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^Password$/i)).not.toBeInTheDocument();
  });
});

describe("LoginPage — domain login", () => {
  it("calls login_domain and navigates home on success", async () => {
    mockCommands({ login_domain: () => undefined, is_logged_in: () => true });

    const { router } = await renderLogin();
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login_domain");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("logs a notImplemented failure and stops", async () => {
    mockCommands({
      login_domain: () => {
        throw { kind: "notImplemented", detail: "loginDomain requires Windows" };
      },
    });

    await renderLogin();
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: loginDomain requires Windows/i),
    ).toBeInTheDocument();
  });
});

describe("LoginPage — account login", () => {
  it("calls login and navigates home on success", async () => {
    mockCommands({ login: () => undefined, is_logged_in: () => true });

    const { router } = await renderLogin();
    await chooseAccount();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login", { code: "alice", password: "secret" });
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("offers a Register button when the account is not found", async () => {
    mockCommands({
      login: () => { throw httpError(404, "not_found", "no such user"); },
    });

    const { router } = await renderLogin();
    await chooseAccount();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "ghost");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(await screen.findByTestId("bootstrap-log-error")).toHaveTextContent(
      "No account matches these credentials.",
    );
    expect(screen.getByText(/You can register a new one/i)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /Register/i }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/register"));
  });

  it("shows the contact-administrator hint for an inactive account", async () => {
    mockCommands({
      login: () => { throw httpError(403, "user_inactive", "inactive"); },
    });

    await renderLogin();
    await chooseAccount();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "bob");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/not active yet\. Please contact your administrator/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
  });

  it("logs the message and stops for any other failure", async () => {
    mockCommands({
      login: () => { throw httpError(401, "invalid_credentials", "bad password"); },
    });

    const { router } = await renderLogin();
    await chooseAccount();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: invalid_credentials: bad password/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/login");
  });

  it("clears the failure outcome when the user switches back to Domain", async () => {
    mockCommands({
      login: () => { throw httpError(404, "not_found", "no such user"); },
    });

    await renderLogin();
    await chooseAccount();
    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(await screen.findByText(/You can register a new one/i)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("radio", { name: /Domain information/i }));

    expect(screen.queryByText(/You can register a new one/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(screen.getByTestId("bootstrap-log-error")).toHaveTextContent(
      "No account matches these credentials.",
    );
  });
});
