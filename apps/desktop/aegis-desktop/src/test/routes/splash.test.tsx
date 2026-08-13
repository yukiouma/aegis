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

/** Health check passes, then advance to the credentials step. */
async function advanceToCredentials(method: "account" | "domain") {
  await screen.findByText(/Server is healthy/i);
  // The default selection is Domain, so the Account path has to switch
  // the radio explicitly. The Domain path can just continue.
  if (method === "account") {
    await userEvent.click(screen.getByRole("radio", { name: /Account and password/i }));
  }
  await userEvent.click(screen.getByRole("button", { name: /Continue/i }));
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
    expect(screen.queryByRole("button", { name: /Continue/i })).not.toBeInTheDocument();
  });
});

describe("SplashPage — account login", () => {
  it("calls login and navigates home on success", async () => {
    mockCommands({ healthz: () => "ok", login: () => undefined, is_logged_in: () => true });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "alice");
    await userEvent.type(screen.getByLabelText(/Password/i), "secret");
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
      get_domain_user_info: () => {
        throw { kind: "notImplemented", detail: "requires Windows" };
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "ghost");
    await userEvent.type(screen.getByLabelText(/Password/i), "pw");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    // The log line and the alert hint share a prefix, so assert on each
    // distinctly rather than with one ambiguous text query.
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
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "bob");
    await userEvent.type(screen.getByLabelText(/Password/i), "pw");
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
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "alice");
    await userEvent.type(screen.getByLabelText(/Password/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: invalid_credentials: bad password/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/splash");
  });
});

describe("SplashPage — domain login", () => {
  it("selects Domain by default", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await screen.findByText(/Server is healthy/i);

    const domainRadio = screen.getByRole("radio", { name: /Domain information/i });
    const accountRadio = screen.getByRole("radio", { name: /Account and password/i });
    expect(domainRadio).toBeChecked();
    expect(accountRadio).not.toBeChecked();
  });

  it("calls login_domain with no arguments and navigates home", async () => {
    mockCommands({
      healthz: () => "ok",
      login_domain: () => undefined,
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("domain");

    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
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
    await advanceToCredentials("domain");

    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    expect(
      await screen.findByText(/Login failed: loginDomain requires Windows/i),
    ).toBeInTheDocument();
  });
});

describe("SplashPage — Back button", () => {
  it("returns from the credentials step to the method step", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    await renderSplash();
    await advanceToCredentials("domain");

    // We're on the credentials step.
    await userEvent.click(screen.getByRole("button", { name: /^Back$/i }));

    // The method radio is back; the Login button is gone.
    expect(
      screen.getByRole("radio", { name: /Domain information/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Login$/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Continue/i })).toBeInTheDocument();
  });

  it("clears a failed login outcome when Back is pressed", async () => {
    mockCommands({
      healthz: () => "ok",
      login: () => {
        throw httpError(404, "not_found", "no such user");
      },
      is_logged_in: () => true,
    });

    const { router } = await renderSplash();
    await advanceToCredentials("account");

    await userEvent.type(screen.getByLabelText(/Account/i), "alice");
    await userEvent.type(screen.getByLabelText(/Password/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /^Login$/i }));

    // A `not_found` failure shows both the Alert and the Register button.
    expect(
      await screen.findByText(/You can register a new one/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Register/i })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /^Back$/i }));

    // Returning to the method step must clear the inline outcome — the
    // log entry stays because that is the audit trail.
    expect(
      screen.queryByText(/You can register a new one/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(screen.getByTestId("splash-log-error")).toHaveTextContent(
      "No account matches these credentials.",
    );

    // Continue forward again and the credentials step is reachable.
    await userEvent.click(screen.getByRole("button", { name: /Continue/i }));
    expect(screen.getByLabelText(/Account/i)).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/splash");
  });
});
