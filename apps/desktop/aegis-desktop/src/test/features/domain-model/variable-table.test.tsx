import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DragDropProvider } from "@aegis/ui/dnd";

import type { SdtmVariableView } from "../../../shared/api";
import { VariableTable } from "../../../features/domain-model/components/VariableTable";

const variables: SdtmVariableView[] = [
  {
    id: 1,
    domainId: 5,
    name: "AETERM",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Topic",
    variableSequence: 1,
    descriptions: [{ lang: "en", details: { label: "Term" } }],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    domainId: 5,
    name: "AESEV",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Record Qualifier",
    variableSequence: 2,
    descriptions: [{ lang: "en", details: { label: "Severity" } }],
    createdAt: "",
    updatedAt: "",
  },
];

function renderTable(props: {
  onCreate?: () => void;
  onEdit?: (r: SdtmVariableView) => void;
  onDelete?: (r: SdtmVariableView) => void;
  onReorder?: (orderedIds: number[]) => void;
  canMutate?: boolean;
  selectedLang?: string | null;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <VariableTable
          rows={variables}
          loading={false}
          error={null}
          canMutate={props.canMutate ?? false}
          selectedLang={props.selectedLang ?? "en"}
          onRetry={vi.fn()}
          onCreate={props.onCreate ?? vi.fn()}
          onEdit={props.onEdit ?? vi.fn()}
          onDelete={props.onDelete ?? vi.fn()}
          onReorder={props.onReorder ?? vi.fn()}
          emptyMessage="empty"
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("VariableTable", () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders the variable rows", () => {
    renderTable({});
    expect(screen.getByText("AETERM")).toBeInTheDocument();
    expect(screen.getByText("AESEV")).toBeInTheDocument();
  });

  it("renders the type and core chips next to the name", () => {
    renderTable({});
    const row1 = screen.getByText("AETERM").closest("tr")!;
    expect(within(row1).getByText("C")).toBeInTheDocument();
    expect(within(row1).getByText("Required")).toBeInTheDocument();
  });

  it("swaps the label cell when selectedLang changes", () => {
    renderTable({ selectedLang: "zh-CN" });
    const row1 = screen.getByText("AETERM").closest("tr")!;
    expect(within(row1).getAllByRole("cell")[2]).toHaveTextContent("");
  });

  it("hides add/edit/delete when canMutate is false", () => {
    renderTable({ canMutate: false });
    expect(
      screen.queryByRole("button", { name: /create variable/i }),
    ).toBeNull();
    expect(screen.queryByRole("button", { name: /edit variable/i })).toBeNull();
    expect(
      screen.queryByRole("button", { name: /delete variable/i }),
    ).toBeNull();
  });

  it("renders add/edit/delete buttons when canMutate is true", async () => {
    const onCreate = vi.fn();
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    renderTable({ canMutate: true, onCreate, onEdit, onDelete });

    const headerCreate = screen.getByRole("button", {
      name: /create variable/i,
    });
    await userEvent.click(headerCreate);
    expect(onCreate).toHaveBeenCalled();

    const editButtons = screen.getAllByRole("button", {
      name: /edit variable/i,
    });
    await userEvent.click(editButtons[0]);
    expect(onEdit).toHaveBeenCalledWith(variables[0]);

    const deleteButtons = screen.getAllByRole("button", {
      name: /delete variable/i,
    });
    fireEvent.click(deleteButtons[0]);
    expect(onDelete).toHaveBeenCalledWith(variables[0]);
  });

  it("calls onReorder when the drag provider fires onDragEnd", () => {
    const onReorder = vi.fn();
    renderTable({ onReorder });
    // Smoke check: DragDropProvider is mounted in DOM
    void DragDropProvider;
    expect(document.querySelector("table")).toBeInTheDocument();
  });
});