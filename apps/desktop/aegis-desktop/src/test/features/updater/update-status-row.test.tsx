import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { afterEach, describe, expect, it, vi } from "vitest";

import { UpdateStatusRow } from "../../../features/updater/components/UpdateStatusRow";

function renderRow(props: Parameters<typeof UpdateStatusRow>[0]) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <AegisThemeProvider>
        <AegisI18nProvider>
          <UpdateStatusRow {...props} />
        </AegisI18nProvider>
      </AegisThemeProvider>
    </QueryClientProvider>,
  );
}

const BASE = {
  currentVersion: "0.1.0",
  lastCheckedAt: "2026-11-18T06:05:10Z",
  lastError: null,
  onCheck: () => {},
  onInstall: () => {},
  checking: false,
  installing: false,
  errorMessage: null,
};

afterEach(() => cleanup());

describe("UpdateStatusRow", () => {
  it("shows the ready version and an install button", () => {
    renderRow({
      ...BASE,
      status: { kind: "ready", version: "0.1.1", notes: null, date: null },
    });
    expect(screen.getByText("Update available: 0.1.1")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Update now" }),
    ).toBeInTheDocument();
  });

  it("says nothing is newer only after a check has completed", () => {
    renderRow({ ...BASE, status: { kind: "idle" }, lastCheckedAt: null });
    expect(
      screen.queryByText("Aegis 0.1.0 is up to date."),
    ).not.toBeInTheDocument();
  });

  it("says the app is up to date once a check has completed", () => {
    renderRow({ ...BASE, status: { kind: "idle" } });
    expect(
      screen.getByText("Aegis 0.1.0 is up to date."),
    ).toBeInTheDocument();
  });

  it("shows the download progress", () => {
    renderRow({
      ...BASE,
      status: {
        kind: "downloading",
        version: "0.1.1",
        downloaded: 25,
        total: 100,
      },
    });
    expect(screen.getByText("Downloading update… 25%")).toBeInTheDocument();
  });

  it("shows the current version and a check button", () => {
    renderRow({ ...BASE, status: { kind: "idle" } });
    expect(
      screen.getByText("Installed version: 0.1.0"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Check for updates" }),
    ).toBeInTheDocument();
  });

  it("omits the installed version until it is known", () => {
    renderRow({ ...BASE, status: { kind: "idle" }, currentVersion: "" });
    expect(screen.queryByText(/Installed version/)).not.toBeInTheDocument();
  });

  it("triggers a check when the check button is clicked", async () => {
    const onCheck = vi.fn();
    renderRow({ ...BASE, status: { kind: "idle" }, onCheck });
    await userEvent.click(
      screen.getByRole("button", { name: "Check for updates" }),
    );
    expect(onCheck).toHaveBeenCalled();
  });

  /// A check that failed while a release is held leaves the status at
  /// `Ready` — the download is still installable. The failure is shown
  /// next to it, not in place of it.
  it("shows a failed check next to an available update", () => {
    renderRow({
      ...BASE,
      status: { kind: "ready", version: "0.1.1", notes: null, date: null },
      lastError: "update network failure: offline",
    });
    expect(screen.getByText("Update available: 0.1.1")).toBeInTheDocument();
    expect(
      screen.getByText("Update check failed: update network failure: offline"),
    ).toBeInTheDocument();
  });
});
