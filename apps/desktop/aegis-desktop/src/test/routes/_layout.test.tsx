import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderWithFullRouter } from "../file-route-utils";

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
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderRoot(initialEntries: string[] = ["/"]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <AegisI18nProvider>{children}</AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("AppLayout", () => {
  it("renders the Sidebar and the Home page content at /", async () => {
    const { router } = await renderRoot(["/"]);

    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/");
  });

  it("navigates to /settings when the Settings menu item is clicked", async () => {
    const { router } = await renderRoot(["/"]);

    await userEvent.click(screen.getByText("Settings"));

    expect(router.state.location.pathname).toBe("/settings");
    expect(
      screen.getByRole("heading", { level: 4, name: /settings/i }),
    ).toBeInTheDocument();
  });

  it("navigates back to / when the Home menu item is clicked", async () => {
    const { router } = await renderRoot(["/settings"]);

    await userEvent.click(screen.getByText("Home"));

    expect(router.state.location.pathname).toBe("/");
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});
