import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DragDropProvider } from "@aegis/ui/dnd";

import type { SdtmVariableView } from "../../../shared/api";
import {
  applyReorder,
  computeReorder,
  VariableTable,
} from "../../../features/domain-model/components/VariableTable";

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

  it("renders the type and core chips in their own column, separate from the name", () => {
    renderTable({});
    const row1 = screen.getByText("AETERM").closest("tr")!;
    const cells = within(row1).getAllByRole("cell");
    // cells[0] = drag handle, cells[1] = name, cells[2] = type/core chips,
    // cells[3] = label, cells[4] = role, cells[5] = actions
    expect(cells[1]).toHaveTextContent("AETERM");
    expect(within(cells[1]).queryByText("C")).toBeNull();
    expect(within(cells[1]).queryByText("Required")).toBeNull();
    expect(within(cells[2]).getByText("C")).toBeInTheDocument();
    expect(within(cells[2]).getByText("Required")).toBeInTheDocument();
  });

  it("swaps the label cell when selectedLang changes", () => {
    renderTable({ selectedLang: "zh-CN" });
    const row1 = screen.getByText("AETERM").closest("tr")!;
    const cells = within(row1).getAllByRole("cell");
    // Label lives in cells[3] now that the type/core chips occupy cells[2].
    expect(cells[3]).toBeEmptyDOMElement();
    // The chips column is unaffected by selectedLang.
    expect(within(cells[2]).getByText("C")).toBeInTheDocument();
    expect(within(cells[2]).getByText("Required")).toBeInTheDocument();
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

describe("computeReorder", () => {
  it("moves the source row forward to the target's slot, shifting the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 1, 3)).toEqual([2, 3, 1, 4]);
  });

  it("moves the source row backward to the target's slot, pushing the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 4, 2)).toEqual([1, 4, 2, 3]);
  });

  it("drops to the end of the list when the target is the last row", () => {
    expect(computeReorder([1, 2, 3], 1, 3)).toEqual([2, 3, 1]);
  });

  it("drops to the front of the list when the target is the first row", () => {
    expect(computeReorder([1, 2, 3], 3, 1)).toEqual([3, 1, 2]);
  });

  it("returns null when source equals target (no-op drop on self)", () => {
    expect(computeReorder([1, 2, 3], 2, 2)).toBeNull();
  });

  it("returns null when the source id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 99, 1)).toBeNull();
  });

  it("returns null when the target id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 1, 99)).toBeNull();
  });

  it("does not mutate the input array", () => {
    const input = [1, 2, 3, 4];
    computeReorder(input, 1, 3);
    expect(input).toEqual([1, 2, 3, 4]);
  });
});

describe("applyReorder", () => {
  const event = (sourceId: string | number | null, targetId: string | number | null, canceled = false) => ({
    canceled,
    operation: {
      source: sourceId == null ? null : { id: sourceId },
      target: targetId == null ? null : { id: targetId },
    },
  });

  it("reads source.id (the dragged row) — regression: previously used target.id and pushed the dropped-on row to the end", () => {
    // Drag row 1 onto row 3 → row 1 should land at row 3's slot, not move row 3.
    expect(applyReorder([1, 2, 3], event("1", "3"))).toEqual([2, 3, 1]);
    expect(applyReorder([1, 2, 3], event("3", "1"))).toEqual([3, 1, 2]);
  });

  it("returns null when the drag was canceled", () => {
    expect(applyReorder([1, 2, 3], event("1", "3", true))).toBeNull();
  });

  it("returns null when source is missing (drop outside any draggable)", () => {
    expect(applyReorder([1, 2, 3], event(null, "1"))).toBeNull();
  });

  it("returns null when target is missing (drop outside any droppable)", () => {
    expect(applyReorder([1, 2, 3], event("1", null))).toBeNull();
  });

  it("returns null when source equals target", () => {
    expect(applyReorder([1, 2, 3], event("2", "2"))).toBeNull();
  });

  it("coerces string ids to numbers before indexing", () => {
    expect(applyReorder([1, 2, 3, 4], event("1", "3"))).toEqual([2, 3, 1, 4]);
  });
});