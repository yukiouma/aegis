import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { CrfStatusPieCard } from "../../../features/project-workspace/components/CrfStatusPieCard";
import { renderWithQueryClient } from "../../helpers/render-with-query-client";
import { httpError, mockCommands } from "../../helpers/tauri-mock";
import type { CrfForm, CrfVersion } from "../../../shared/api";

const version: CrfVersion = {
  id: 7,
  projectCode: "DEMO-001",
  name: "CRF v1.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

function form(overrides: Partial<CrfForm> = {}): CrfForm {
  return {
    id: 1,
    versionId: 7,
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

function renderCard() {
  return renderWithQueryClient(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CrfStatusPieCard version={version} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

afterEach(() => cleanup());

describe("CrfStatusPieCard", () => {
  it("titles the card with the CRF version name", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    renderCard();
    expect(await screen.findByText("CRF v1.0")).toBeInTheDocument();
  });

  it("splits the forms into Approved and Pending by the approved flag", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({
        forms: [
          form({ id: 1, approved: true }),
          form({ id: 2, approved: true }),
          form({ id: 3, approved: false }),
        ],
      }),
    });
    renderCard();
    const approved = await screen.findByTestId("crf-status-count-approved");
    expect(approved).toHaveTextContent("2");
    expect(approved).toHaveTextContent("67%");
    const pending = screen.getByTestId("crf-status-count-pending");
    expect(pending).toHaveTextContent("1");
    expect(pending).toHaveTextContent("33%");
    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.getByText("Pending")).toBeInTheDocument();
  });

  it("counts by the approved flag only and ignores notSubmitted", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({
        forms: [
          form({ id: 1, approved: true, notSubmitted: true }),
          form({ id: 2, approved: false, notSubmitted: true }),
        ],
      }),
    });
    renderCard();
    expect(
      await screen.findByTestId("crf-status-count-approved"),
    ).toHaveTextContent("1");
    expect(screen.getByTestId("crf-status-count-pending")).toHaveTextContent("1");
  });

  it("gives every status row an icon as well as its label", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    renderCard();
    expect(
      await screen.findByTestId("crf-status-icon-approved"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("crf-status-icon-pending")).toBeInTheDocument();
  });

  it("shows the empty state when the version has no forms", async () => {
    mockCommands({ list_crf_forms_by_version: () => ({ forms: [] }) });
    renderCard();
    expect(
      await screen.findByText("No forms in this version"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("crf-status-count-approved"),
    ).not.toBeInTheDocument();
  });

  it("shows an error inside the card when the forms query fails", async () => {
    mockCommands({
      list_crf_forms_by_version: () => {
        throw httpError(500, "internal_error", "boom");
      },
    });
    renderCard();
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("internal_error: boom");
  });
});
