import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The deferred version lives in the settings store, so the dialog reads
// it from the same `settings.bin` mock the settings feature uses.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () =>
    Promise.resolve({
      get: (k: string) => Promise.resolve(store.get(k)),
      set: (k: string, v: unknown) => {
        store.set(k, v);
        return Promise.resolve();
      },
      delete: (k: string) => {
        store.delete(k);
        return Promise.resolve();
      },
      save: () => Promise.resolve(),
    }),
}));

import { UpdateReadyDialog } from "../../../features/updater/components/UpdateReadyDialog";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

function makeClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

function renderDialog() {
  const client = makeClient();
  return render(
    <QueryClientProvider client={client}>
      <AegisThemeProvider>
        <AegisI18nProvider>
          <UpdateReadyDialog />
        </AegisI18nProvider>
      </AegisThemeProvider>
    </QueryClientProvider>,
  );
}

const READY_STATUS = {
  status: { kind: "ready", version: "0.1.1", notes: null, date: null },
  currentVersion: "0.1.0",
  lastCheckedAt: "2026-11-18T06:05:10Z",
};

beforeEach(() => {
  store.clear();
  mockInvoke.mockReset();
});
afterEach(() => cleanup());

describe("UpdateReadyDialog", () => {
  it("offers Update now and Later for a ready release", async () => {
    mockCommands({ update_status: () => READY_STATUS });
    renderDialog();
    expect(
      await screen.findByText("Aegis 0.1.1 is ready to install"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Update now" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Later" })).toBeInTheDocument();
  });

  it("does not open for a release the user already deferred", async () => {
    store.set("update.deferredVersion", "0.1.1");
    mockCommands({ update_status: () => READY_STATUS });
    renderDialog();
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("update_status"));
    expect(
      screen.queryByText("Aegis 0.1.1 is ready to install"),
    ).not.toBeInTheDocument();
  });

  it("records the deferral when Later is clicked", async () => {
    mockCommands({ update_status: () => READY_STATUS });
    renderDialog();
    await userEvent.click(await screen.findByRole("button", { name: "Later" }));
    await waitFor(() =>
      expect(store.get("update.deferredVersion")).toBe("0.1.1"),
    );
  });

  it("does not open when the check failed", async () => {
    mockCommands({
      update_status: () => ({
        status: { kind: "failed", message: "release server unreachable" },
        currentVersion: "0.1.0",
        lastCheckedAt: "2026-11-18T06:05:10Z",
      }),
    });
    renderDialog();
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("update_status"));
    expect(
      screen.queryByText("Aegis 0.1.1 is ready to install"),
    ).not.toBeInTheDocument();
  });
});
