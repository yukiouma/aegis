import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import type { SdtmDomainView } from "../../../shared/api";
import { DomainTable } from "../../../features/domain-model/components/DomainTable";

function wrap(ui: React.ReactNode) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale="en">{ui}</AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const row: SdtmDomainView = {
  id: 1,
  versionId: 1,
  name: "AE",
  category: "Events",
  descriptions: [
    { lang: "en", details: { description: "Adverse Events", structure: "One per AE" } },
    { lang: "zh", details: { description: "不良事件", structure: "每条记录一个AE" } },
  ],
  createdAt: "",
  updatedAt: "",
};

function renderTable(props: Partial<React.ComponentProps<typeof DomainTable>> = {}) {
  return wrap(
    <DomainTable
      rows={[row]}
      loading={false}
      error={null}
      canMutate={false}
      selectedLang="en"
      onRetry={() => {}}
      onDelete={() => {}}
      emptyMessage="empty"
      {...props}
    />,
  );
}

describe("DomainTable", () => {
  afterEach(cleanup);

  it("renders the English description when selectedLang='en'", () => {
    renderTable();
    expect(screen.getByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("One per AE")).toBeInTheDocument();
    expect(screen.getByText("Events")).toBeInTheDocument();
  });

  it("renders the Chinese description when selectedLang='zh'", () => {
    renderTable({ selectedLang: "zh" });
    expect(screen.getByText("不良事件")).toBeInTheDocument();
  });

  it("renders empty Description and Structure cells when the selected language is not present", () => {
    renderTable({ selectedLang: "ja" });
    const rowEl = screen.getByRole("row", { name: /AE/ });
    const cells = within(rowEl).getAllByRole("cell");
    // cells[0] = Name, cells[1] = Description, cells[2] = Structure, cells[3] = Category, cells[4] = Operations
    expect(cells[1].textContent).toBe("");
    expect(cells[2].textContent).toBe("");
  });

  it("renders the delete icon only when canMutate=true", () => {
    renderTable({ canMutate: true });
    expect(screen.getByRole("button", { name: /delete/i })).toBeInTheDocument();
  });

  it("hides the delete icon when canMutate=false", () => {
    renderTable({ canMutate: false });
    expect(screen.queryByRole("button", { name: /delete/i })).not.toBeInTheDocument();
  });

  it("calls onDelete when the delete icon is clicked", async () => {
    const onDelete = vi.fn();
    renderTable({ canMutate: true, onDelete });
    await userEvent.click(screen.getByRole("button", { name: /delete/i }));
    expect(onDelete).toHaveBeenCalledWith(row);
  });

  it("renders the empty message when rows is empty", () => {
    wrap(
      <DomainTable
        rows={[]}
        loading={false}
        error={null}
        canMutate={false}
        selectedLang="en"
        onRetry={() => {}}
        onDelete={() => {}}
        emptyMessage="No matches."
      />,
    );
    expect(screen.getByText("No matches.")).toBeInTheDocument();
  });

  it("keeps the open-detail button disabled when onNavigate is not provided", () => {
    renderTable();
    const btn = screen.getByRole("button", { name: /open-detail/i });
    expect(btn).toBeDisabled();
  });

  it("enables the open-detail button and calls onNavigate when provided", async () => {
    const onNavigate = vi.fn();
    renderTable({ onNavigate });
    const btn = screen.getByRole("button", { name: /open-detail/i });
    expect(btn).not.toBeDisabled();
    await userEvent.click(btn);
    expect(onNavigate).toHaveBeenCalledWith(row);
  });
});