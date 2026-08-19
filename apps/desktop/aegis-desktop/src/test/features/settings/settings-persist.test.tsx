import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, act } from "@testing-library/react";
import { AegisThemeProvider, useThemeMode } from "@aegis/ui/theme";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";

// Mock the store BEFORE importing the module under test so the
// singleton store handle is constructed against the mock loader.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () => Promise.resolve({
    get: (k: string) => Promise.resolve(store.get(k)),
    set: (k: string, v: unknown) => { store.set(k, v); return Promise.resolve(); },
    save: () => Promise.resolve(),
  }),
}));

// In-memory pub-sub for events. Captures listeners so tests can fire
// payloads directly.
type Handler = (e: { payload: unknown }) => void;
const handlers: Handler[] = [];
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_name: string, h: Handler) => {
    handlers.push(h);
    return Promise.resolve(() => {
      const idx = handlers.indexOf(h);
      if (idx >= 0) handlers.splice(idx, 1);
    });
  },
  emit: vi.fn(),
}));

// Now import the module under test — its top-level singleton is built
// against the mocked loader.
import {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "../../../features/settings";

function HydrateProbe() {
  useHydrateSettingsFromStore();
  return null;
}

function ListenProbe() {
  useListenForSettingsChanges();
  return null;
}

function ThemeProbe({ label }: { label: string }) {
  const { mode } = useThemeMode();
  return <span data-testid={label}>{mode}</span>;
}

function LocaleProbe({ label }: { label: string }) {
  const { locale } = useI18n();
  return <span data-testid={label}>{locale}</span>;
}

beforeEach(() => {
  store.clear();
  handlers.length = 0;
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() { return 0; },
  });
});
afterEach(() => cleanup());

describe("useHydrateSettingsFromStore", () => {
  it("calls setMode and setLocale when the store has values that differ", async () => {
    store.set("theme", "dark");
    store.set("locale", "zh-CN");

    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <HydrateProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("zh-CN");
    });
  });

  it("leaves providers alone when the store is empty", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <HydrateProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    // Wait one tick so the hook's effect runs.
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.getByTestId("mode").textContent).toBe("light");
    expect(screen.getByTestId("locale").textContent).toBe("en");
  });
});

describe("useListenForSettingsChanges", () => {
  it("calls setMode and setLocale when an event fires", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ListenProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    // Wait for the listen() effect to register its handler.
    await waitFor(() => expect(handlers.length).toBe(1));

    await act(async () => {
      handlers[0]({ payload: { theme: "dark", locale: "zh-CN" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("zh-CN");
    });
  });

  it("only applies the keys present in the payload", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ListenProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => expect(handlers.length).toBe(1));

    await act(async () => {
      handlers[0]({ payload: { theme: "dark" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("dark");
      expect(screen.getByTestId("locale").textContent).toBe("en");
    });
  });

  it("applies a new character theme ID broadcast from another window", async () => {
    render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ListenProbe />
          <ThemeProbe label="mode" />
          <LocaleProbe label="locale" />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    await waitFor(() => expect(handlers.length).toBe(1));

    await act(async () => {
      handlers[0]({ payload: { theme: "totoro" } });
    });

    await waitFor(() => {
      expect(screen.getByTestId("mode").textContent).toBe("totoro");
    });
    expect(screen.getByTestId("locale").textContent).toBe("en");
  });
});

describe("persistSettings", () => {
  it("writes both keys when both are provided", async () => {
    await persistSettings({ theme: "dark", locale: "zh-CN" });
    expect(store.get("theme")).toBe("dark");
    expect(store.get("locale")).toBe("zh-CN");
  });

  it("writes only the patch key when only one is provided", async () => {
    store.set("locale", "en");
    await persistSettings({ theme: "dark" });
    expect(store.get("theme")).toBe("dark");
    expect(store.get("locale")).toBe("en"); // unchanged
  });
});
