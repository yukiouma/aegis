import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

afterEach(() => cleanup());
beforeEach(() => {
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() {
      return 0;
    },
  });
});

function renderRoot(initialEntries: string[]) {
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

describe("/project/DEMO-001/dashboard — authenticated", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the sidebar with the project code as title", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(await screen.findByTestId("sidebar")).toBeInTheDocument();
    expect(await screen.findByText("DEMO-001")).toBeInTheDocument();
  });

  it("renders the Dashboard heading with the project code", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(
      await screen.findByRole("heading", {
        name: /Dashboard — DEMO-001/,
      }),
    ).toBeInTheDocument();
  });

  it("renders the focus-main footer Button", async () => {
    await renderRoot(["/project/DEMO-001/dashboard"]);
    expect(
      await screen.findByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();
  });
});

describe("/project/DEMO-001/configuration — authenticated", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("renders the Configuration heading with the project code", async () => {
    await renderRoot(["/project/DEMO-001/configuration"]);
    expect(
      await screen.findByRole("heading", {
        name: /Configuration — DEMO-001/,
      }),
    ).toBeInTheDocument();
  });
});

describe("/project/DEMO-001 — authenticated redirect", () => {
  beforeEach(() => {
    mockCommands({ is_logged_in: () => true });
  });

  it("redirects bare /project/<code> to /project/<code>/dashboard", async () => {
    const { router } = await renderRoot(["/project/DEMO-001"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe(
        "/project/DEMO-001/dashboard",
      ),
    );
  });
});

describe("/project/DEMO-001/dashboard — unauthenticated", () => {
  it("redirects to /login when not logged in", async () => {
    mockCommands({ is_logged_in: () => false });
    const { router } = await renderRoot(["/project/DEMO-001/dashboard"]);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });
});