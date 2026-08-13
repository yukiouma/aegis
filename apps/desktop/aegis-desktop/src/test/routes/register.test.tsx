import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../file-route-utils";
import { httpError, mockCommands, mockInvoke } from "../tauri-mock";

const IDENTITY = {
  domain: "corp.example",
  hostMachine: "ws-001",
  sid: "S-1-5-21-1234",
  userid: "alice",
};

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

function renderRegister() {
  return renderWithFullRouter({
    initialEntries: ["/register"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

async function fillAndSubmit() {
  await userEvent.type(await screen.findByLabelText(/User name/i), "Alice");
  await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
  await userEvent.click(screen.getByRole("button", { name: /Register/i }));
}

describe("RegisterPage — identity", () => {
  it("fills the four identity fields and disables them", async () => {
    mockCommands({ get_domain_user_info: () => IDENTITY });

    await renderRegister();

    const userCode = await screen.findByLabelText(/User code/i);
    expect(userCode).toHaveValue("alice");
    expect(userCode).toBeDisabled();

    expect(screen.getByLabelText(/^Domain$/i)).toHaveValue("corp.example");
    expect(screen.getByLabelText(/^Domain$/i)).toBeDisabled();
    expect(screen.getByLabelText(/Hostname/i)).toHaveValue("ws-001");
    expect(screen.getByLabelText(/Hostname/i)).toBeDisabled();
    expect(screen.getByLabelText(/SID/i)).toHaveValue("S-1-5-21-1234");
    expect(screen.getByLabelText(/SID/i)).toBeDisabled();

    expect(screen.getByLabelText(/User name/i)).toBeEnabled();
    expect(screen.getByLabelText(/^Password$/i)).toBeEnabled();
  });

  it("logs the failure and renders no form when the identity lookup fails", async () => {
    mockCommands({
      get_domain_user_info: () => {
        throw { kind: "notImplemented", detail: "requires Windows" };
      },
    });

    await renderRegister();

    expect(
      await screen.findByText(/Could not read domain user information: requires Windows/i),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText(/User name/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
  });
});

describe("RegisterPage — submission", () => {
  it("disables Register until both editable fields are filled", async () => {
    mockCommands({ get_domain_user_info: () => IDENTITY });

    await renderRegister();
    await screen.findByLabelText(/User name/i);

    expect(screen.getByRole("button", { name: /Register/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/User name/i), "Alice");
    expect(screen.getByRole("button", { name: /Register/i })).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/^Password$/i), "secret");
    expect(screen.getByRole("button", { name: /Register/i })).toBeEnabled();
  });

  it("sends the full input, built from the identity", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => ({}),
    });

    await renderRegister();
    await fillAndSubmit();

    expect(mockInvoke).toHaveBeenCalledWith("register_user", {
      userCode: "alice",
      userName: "Alice",
      domainName: "corp.example",
      hostname: "ws-001",
      sid: "S-1-5-21-1234",
      password: "secret",
    });
  });

  it("replaces the form with the contact-administrator hint on success", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => ({}),
    });

    await renderRegister();
    await fillAndSubmit();

    expect(
      await screen.findByText(/Contact your administrator to activate your account/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Register/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/User name/i)).not.toBeInTheDocument();
  });

  it("logs the failure message and keeps the form on failure", async () => {
    mockCommands({
      get_domain_user_info: () => IDENTITY,
      register_user: () => {
        throw httpError(409, "already_exists", "user exists");
      },
    });

    await renderRegister();
    await fillAndSubmit();

    expect(
      await screen.findByText(/Registration failed: already_exists: user exists/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Contact your administrator to activate your account/i),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Register/i })).toBeInTheDocument();
  });
});
