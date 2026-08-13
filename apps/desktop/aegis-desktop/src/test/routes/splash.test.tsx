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

function renderSplash() {
  return renderWithFullRouter({
    initialEntries: ["/splash"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

/** Health check passes, then land on the method step. */
async function onMethodStep() {
  await screen.findByText(/Server is healthy/i);
}

/** Switch to the Account radio and return when the form fields are ready. */
async function chooseAccount() {
  await userEvent.click(screen.getByRole("radio", { name: /Account and password/i }));
  // The textfield label is exactly "Account"; the radio label is
  // "Account and password", which also matches `/Account/i`. Anchor
  // the regex to the textfield label only.
  await screen.findByLabelText(/^Account$/i);
}

describe("SplashPage — health check", () => {
  it("logs success and advances to the method step", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();

    expect(await screen.findByText(/Server is healthy: ok/i)).toBeInTheDocument();
    expect(
      await screen.findByRole("radio", { name: /Account and password/i }),
    ).toBeInTheDocument();
  });

  it("stops on the health step when healthz fails", async () => {
    mockCommands({
      healthz: () => {
        throw { kind: "network", message: "connection refused" };
      },
    });

    await renderSplash();

    expect(
      await screen.findByText(/Server health check failed: connection refused/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("radio", { name: /Account and password/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Login/i })).not.toBeInTheDocument();
  });
});

describe("SplashPage — method step layout", () => {
  it("selects Domain by default", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await onMethodStep();

    const domainRadio = screen.getByRole("radio", { name: /Domain information/i });
    const accountRadio = screen.getByRole("radio", { name: /Account and password/i });
    expect(domainRadio).toBeChecked();
    expect(accountRadio).not.toBeChecked();
  });

  it("hides the account fields when Domain is selected", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await onMethodStep();

    expect(screen.queryByLabelText(/^Account$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^Password$/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeInTheDocument();
  });

  it("shows the account fields when Account is selected", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await onMethodStep();
    await chooseAccount();

    expect(screen.getByLabelText(/^Account$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Password$/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeInTheDocument();
  });

  it("disables Login until both account fields are filled", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await onMethodStep();
    await chooseAccount();

    expect(screen.getByRole("button", { name: /^Login$/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    expect(screen.getByRole("button", { name: /^Login$/i })).toBeEnabled();
  });

  it("swaps the fields back off when Domain is re-selected", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await onMethodStep();
    await chooseAccount();

    await userEvent.click(screen.getByRole("radio", { name: /Domain information/i }));

    expect(screen.queryByLabelText(/^Account$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^Password$/i)).not.toBeInTheDocument();
  });
});

describe("SplashPage — domain login", () => {
  it("calls login_domain and navigates home on success", async () => {
    mockCommands({
      healthz: () => "ok",
      login_domain: () => undefined,
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await onMethodStep();

    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login_domain");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("logs a notImplemented failure and stops", async () => {
    mockCommands({
      healthz: () => "ok",
      login_domain: () => {
        throw { kind: "notImplemented", detail: "loginDomain requires Windows" };
      },
      is_logged_in: () => true,
    });

    await renderSplash();
    await onMethodStep();

    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: loginDomain requires Windows/i),
    ).toBeInTheDocument();
  });
});

describe("SplashPage — account login", () => {
  it("calls login and navigates home on success", async () => {
    mockCommands({ healthz: () => "ok", login: () => undefined, is_logged_in: () => true });

    const { router } = await renderSplash();
    await onMethodStep();
    await chooseAccount();

    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(mockInvoke).toHaveBeenCalledWith("login", {
      code: "alice",
      password: "secret",
    });
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("offers a Register button when the account is not found", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(404, "not_found", "no such user");
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await onMethodStep();
    await chooseAccount();

    await userEvent.type(screen.getByLabelText(/^Account$/i), "ghost");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(await screen.findByTestId("splash-log-error")).toHaveTextContent(
      "No account matches these credentials.",
    );
    expect(screen.getByText(/You can register a new one/i)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /Register/i }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/register"));
  });

  it("shows the contact-administrator hint for an inactive account", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(403, "user_inactive", "inactive");
      },
      is_logged_in: () => true,
    });

    await renderSplash();
    await onMethodStep();
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
      healthz: () => "ok",
      login: () => {
        throw httpError(401, "invalid_credentials", "bad password");
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await onMethodStep();
    await chooseAccount();

    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: invalid_credentials: bad password/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/splash");
  });

  it("clears the failure outcome when the user switches back to Domain", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(404, "not_found", "no such user");
      },
      is_logged_in: () => true,
    });

    await renderSplash();
    await onMethodStep();
    await chooseAccount();

    await userEvent.type(screen.getByLabelText(/^Account$/i), "alice");
    await userEvent.type(screen.getByLabelText(/^Password$/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/You can register a new one/i),
    ).toBeInTheDocument();

    // Switching the method is the only way out of the failure state
    // without a Back button — the same handlers reset `outcome`.
    await userEvent.click(screen.getByRole("radio", { name: /Domain information/i }));

    expect(
      screen.queryByText(/You can register a new one/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    // The log entry stays — that is the audit trail.
    expect(screen.getByTestId("splash-log-error")).toHaveTextContent(
      "No account matches these credentials.",
    );
  });
});
