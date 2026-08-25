import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { SdtmDomainView } from "../../../shared/api";
import { DomainHeaderTable } from "../../../features/domain-model/components/DomainHeaderTable";

function renderHeader(props: {
  domain?: SdtmDomainView;
  canMutate?: boolean;
  error?: unknown;
  onEdit?: () => void;
  onBack?: () => void;
  selectedLang?: string | null;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DomainHeaderTable
          domain={props.domain}
          loading={false}
          error={props.error ?? null}
          canMutate={props.canMutate ?? false}
          selectedLang={props.selectedLang ?? "en"}
          onEdit={props.onEdit ?? vi.fn()}
          onBack={props.onBack ?? vi.fn()}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const sampleDomain: SdtmDomainView = {
  id: 7,
  versionId: 5,
  name: "AE",
  category: "Events",
  descriptions: [
    {
      lang: "en",
      details: { description: "Adverse Events", structure: "One per AE" },
    },
  ],
  createdAt: "",
  updatedAt: "",
};

describe("DomainHeaderTable", () => {
  afterEach(cleanup);

  it("renders the domain metadata when loaded", () => {
    renderHeader({ domain: sampleDomain });
    expect(screen.getByText("AE")).toBeInTheDocument();
    expect(screen.getByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("One per AE")).toBeInTheDocument();
    expect(screen.getByText("Events")).toBeInTheDocument();
  });

  it("falls back to empty strings for missing selected-lang description", () => {
    renderHeader({ domain: sampleDomain, selectedLang: "zh-CN" });
    const cells = screen.getAllByRole("cell");
    // cells[0] = back, cells[1] = name, cells[2] = description, cells[3] = structure, cells[4] = category, cells[5] = edit
    expect(cells[2]).toBeEmptyDOMElement();
    expect(cells[3]).toBeEmptyDOMElement();
  });

  it("hides the edit icon when canMutate is false", () => {
    renderHeader({ domain: sampleDomain, canMutate: false });
    expect(screen.queryByRole("button", { name: /edit/i })).toBeNull();
  });

  it("renders the edit icon and fires onEdit when canMutate", async () => {
    const onEdit = vi.fn();
    renderHeader({ domain: sampleDomain, canMutate: true, onEdit });
    const editButton = screen.getByRole("button", { name: /edit/i });
    await userEvent.click(editButton);
    expect(onEdit).toHaveBeenCalledOnce();
  });

  it("fires onBack when the back button is clicked", async () => {
    const onBack = vi.fn();
    renderHeader({ domain: sampleDomain, onBack });
    const backButton = screen.getByRole("button", { name: /back/i });
    await userEvent.click(backButton);
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("shows the error alert with back button when error and no domain", () => {
    renderHeader({ error: new Error("boom") });
    expect(screen.getByText(/boom/)).toBeInTheDocument();
  });
});