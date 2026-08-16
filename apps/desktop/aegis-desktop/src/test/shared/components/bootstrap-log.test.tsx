import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { BootstrapLog, useBootstrapLog } from "../../../shared/components/BootstrapLog";

afterEach(() => {
  cleanup();
});

/** Drives the hook from a button so the test exercises the real API. */
function Harness() {
  const { entries, push } = useBootstrapLog();
  const { setLocale } = useI18n();
  return (
    <>
      <button onClick={() => push("info", "bootstrap.log.healthCheck.start")}>
        add-info
      </button>
      <button onClick={() => push("success", "login.log.login.ok")}>
        add-success
      </button>
      <button
        onClick={() => push("error", "login.log.login.failed", { message: "boom" })}
      >
        add-error
      </button>
      <button onClick={() => setLocale("zh-CN")}>to-zh</button>
      <BootstrapLog entries={entries} />
    </>
  );
}

function renderHarness() {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <Harness />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("BootstrapLog", () => {
  it("renders nothing when there are no entries", () => {
    renderHarness();
    expect(screen.queryByTestId("bootstrap-log")).not.toBeInTheDocument();
  });

  it("appends entries in order and keeps them", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));

    const rows = screen.getByTestId("bootstrap-log").children;
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Checking server health...");
    expect(rows[1]).toHaveTextContent("Login succeeded. Entering the app...");
  });

  it("tags each entry with its level", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));
    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("bootstrap-log-info")).toBeInTheDocument();
    expect(screen.getByTestId("bootstrap-log-success")).toBeInTheDocument();
    expect(screen.getByTestId("bootstrap-log-error")).toBeInTheDocument();
  });

  it("interpolates params into the message", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("bootstrap-log-error")).toHaveTextContent(
      "Login failed: boom",
    );
  });

  it("re-translates existing entries when the locale changes", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    expect(screen.getByTestId("bootstrap-log")).toHaveTextContent(
      "Checking server health...",
    );

    await act(async () => {
      await userEvent.click(screen.getByText("to-zh"));
    });

    expect(screen.getByTestId("bootstrap-log")).toHaveTextContent(
      "正在检查服务器健康状态……",
    );
  });
});
