import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../file-route-utils";
import { HomePage } from "../../pages/home";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

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

function renderHome(defaultLocale: "en" | "zh-CN" = "en") {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <HomePage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("HomePage", () => {
  it("renders the welcome heading", async () => {
    await renderHome();
    expect(
      screen.getByRole("heading", { level: 4, name: /home/i }),
    ).toBeInTheDocument();
  });
});