import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider, useI18n } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { SplashLog, useSplashLog } from "../../components/SplashLog";

afterEach(() => {
  cleanup();
});

/** Drives the hook from a button so the test exercises the real API. */
function Harness() {
  const { entries, push } = useSplashLog();
  const { setLocale } = useI18n();
  return (
    <>
      <button onClick={() => push("info", "splash.log.healthCheck.start")}>
        add-info
      </button>
      <button onClick={() => push("success", "splash.log.login.ok")}>
        add-success
      </button>
      <button
        onClick={() => push("error", "splash.log.login.failed", { message: "boom" })}
      >
        add-error
      </button>
      <button onClick={() => setLocale("zh-CN")}>to-zh</button>
      <SplashLog entries={entries} />
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

describe("SplashLog", () => {
  it("renders an empty container when there are no entries", () => {
    renderHarness();
    expect(screen.getByTestId("splash-log")).toBeEmptyDOMElement();
  });

  it("appends entries in order and keeps them", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));

    const rows = screen.getByTestId("splash-log").children;
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Checking server health...");
    expect(rows[1]).toHaveTextContent("Login succeeded. Entering the app...");
  });

  it("tags each entry with its level", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    await userEvent.click(screen.getByText("add-success"));
    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("splash-log-info")).toBeInTheDocument();
    expect(screen.getByTestId("splash-log-success")).toBeInTheDocument();
    expect(screen.getByTestId("splash-log-error")).toBeInTheDocument();
  });

  it("interpolates params into the message", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-error"));

    expect(screen.getByTestId("splash-log-error")).toHaveTextContent(
      "Login failed: boom",
    );
  });

  it("re-translates existing entries when the locale changes", async () => {
    renderHarness();

    await userEvent.click(screen.getByText("add-info"));
    expect(screen.getByTestId("splash-log")).toHaveTextContent(
      "Checking server health...",
    );

    await act(async () => {
      await userEvent.click(screen.getByText("to-zh"));
    });

    expect(screen.getByTestId("splash-log")).toHaveTextContent(
      "正在检查服务器健康状态……",
    );
  });
});
