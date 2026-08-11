import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";
import { DocumentLangSync } from "../DocumentLangSync";

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
  document.documentElement.lang = "en";
});

afterEach(() => {
  cleanup();
});

function Switcher() {
  const { setLocale } = useI18n();
  return <button onClick={() => setLocale("zh-CN")}>set-zh-CN</button>;
}

describe("DocumentLangSync", () => {
  it("mirrors the initial locale onto <html lang>", () => {
    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <DocumentLangSync />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe("zh-CN");
  });

  it("updates <html lang> when the active locale changes", async () => {
    render(
      <AegisI18nProvider>
        <DocumentLangSync />
        <Switcher />
      </AegisI18nProvider>,
    );

    expect(document.documentElement.lang).toBe("en");

    await userEvent.click(screen.getByText("set-zh-CN"));

    expect(document.documentElement.lang).toBe("zh-CN");
  });
});
