import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() { return data.size; },
    clear() { data.clear(); },
    getItem(key: string) { return data.has(key) ? data.get(key)! : null; },
    key(index: number) { return Array.from(data.keys())[index] ?? null; },
    removeItem(key: string) { data.delete(key); },
    setItem(key: string, value: string) { data.set(key, value); },
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

function renderBootstrap() {
  return renderWithFullRouter({
    initialEntries: ["/bootstrap"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("BootstrapPage — health check", () => {
  it("stops on the health step when healthz fails", async () => {
    mockCommands({
      healthz: () => { throw { kind: "network", message: "connection refused" }; },
    });

    await renderBootstrap();

    expect(
      await screen.findByText(/Server health check failed: connection refused/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Not logged in/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Login/i })).not.toBeInTheDocument();
  });

  it("does not render a retry button on health failure", async () => {
    mockCommands({
      healthz: () => { throw { kind: "network", message: "down" }; },
    });

    await renderBootstrap();

    expect(await screen.findByText(/Server health check failed/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /retry/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /reload/i })).not.toBeInTheDocument();
  });
});

describe("BootstrapPage — login status", () => {
  it("navigates to / when the user is logged in", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => true });

    const { router } = await renderBootstrap();

    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  });

  it("navigates to /login when the user is not logged in", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => false });

    const { router } = await renderBootstrap();

    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
  });

  it("stops on the login-status step when isLoggedIn throws", async () => {
    mockCommands({
      healthz: () => "ok",
      is_logged_in: () => { throw { kind: "store", message: "broken" }; },
    });

    await renderBootstrap();

    expect(
      await screen.findByText(/Login status check failed: broken/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Login/i })).not.toBeInTheDocument();
  });

  it("fires healthz only once under StrictMode", async () => {
    mockCommands({ healthz: () => "ok", is_logged_in: () => false });

    const { router } = await renderBootstrap();

    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
    const healthzCalls = mockInvoke.mock.calls.filter(
      ([cmd]) => cmd === "healthz",
    );
    expect(healthzCalls).toHaveLength(1);
  });
});
