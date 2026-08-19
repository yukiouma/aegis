import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { SettingsPage } from "../../../features/settings/pages/SettingsPage";
import { renderInRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

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
  // SettingsPage now reads the current user via useCurrentUser, so the
  // test renderer must satisfy the Tauri `current_user` command.
  mockCommands({
    current_user: () => ({
      id: 1,
      code: "alice",
      name: "Alice",
      role: "general",
      active: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    }),
  });
});

afterEach(() => {
  cleanup();
});

function renderSettings(defaultLocale: "en" | "zh-CN" = "en") {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider defaultLocale={defaultLocale}>
          <SettingsPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("SettingsPage", () => {
  it("renders English copy by default", async () => {
    await renderSettings();

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent(
      "Settings",
    );
    // Theme dropdown label interpolates the current theme name, so it
    // reads "Theme: Light" out of the box (default mode = 'light').
    expect(screen.getByLabelText(/Theme: Light/)).toBeInTheDocument();
    expect(screen.getByLabelText("Language")).toHaveTextContent("English");
  });

  it("renders Simplified Chinese copy when the default locale is zh-CN", async () => {
    await renderSettings("zh-CN");

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent("设置");
    expect(screen.getByLabelText(/主题：浅色/)).toBeInTheDocument();
    expect(screen.getByLabelText("语言")).toHaveTextContent("简体中文");
  });

  it("switches locale, headings, and theme label when the user picks zh-CN", async () => {
    await renderSettings("en");

    await userEvent.click(screen.getByLabelText("Language"));
    await userEvent.click(
      screen.getByRole("option", { name: "Simplified Chinese" }),
    );

    expect(screen.getByRole("heading", { level: 4 })).toHaveTextContent("设置");
    expect(screen.getByLabelText(/主题：浅色/)).toBeInTheDocument();
  });
});
