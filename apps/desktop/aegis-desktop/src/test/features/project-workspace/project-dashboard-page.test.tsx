import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { httpError, mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import type { CrfForm, CrfVersion } from "../../../shared/api";

const versionA: CrfVersion = {
  id: 1,
  projectCode: "DEMO-001",
  name: "CRF v1.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const versionB: CrfVersion = {
  id: 2,
  projectCode: "DEMO-001",
  name: "CRF v2.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

function form(overrides: Partial<CrfForm> = {}): CrfForm {
  return {
    id: 1,
    versionId: 1,
    code: "FORM",
    name: "Form",
    order: 1,
    notSubmitted: false,
    approved: false,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function renderDashboard() {
  return renderWithFullRouter({
    initialEntries: ["/project/DEMO-001/dashboard"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

afterEach(() => cleanup());

describe("project dashboard — CRF status charts", () => {
  it("renders one card per CRF version", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA, versionB] }),
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    await renderDashboard();
    expect(await screen.findByText("CRF v1.0")).toBeInTheDocument();
    expect(screen.getByText("CRF v2.0")).toBeInTheDocument();
  });

  it("renders the CRF status section heading", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA] }),
      list_crf_forms_by_version: () => ({ forms: [] }),
    });
    await renderDashboard();
    expect(
      await screen.findByRole("heading", { name: "CRF form status" }),
    ).toBeInTheDocument();
  });

  it("shows the empty state when the project has no CRF versions", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [] }),
    });
    await renderDashboard();
    expect(
      await screen.findByText("No CRF versions yet"),
    ).toBeInTheDocument();
  });

  it("isolates a per-version failure to its own card", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA, versionB] }),
      list_crf_forms_by_version: (args) => {
        if (Number(args?.versionId) === 1) {
          throw httpError(500, "internal_error", "boom");
        }
        return { forms: [form({ versionId: 2, approved: true })] };
      },
    });
    await renderDashboard();
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("internal_error: boom");
    expect(screen.getByText("CRF v2.0")).toBeInTheDocument();
    expect(screen.getByTestId("crf-status-count-approved")).toHaveTextContent("1");
  });
});
