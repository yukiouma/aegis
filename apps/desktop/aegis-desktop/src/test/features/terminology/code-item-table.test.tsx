import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { CodeItemTable } from
  "../../../features/terminology/components/CodeItemTable";
import type { CodeItemView } from "../../../shared/api";

afterEach(() => {
  cleanup();
});

function makeRow(overrides: Partial<CodeItemView> = {}): CodeItemView {
  return {
    id: 1,
    codelistId: 1,
    versionId: 1,
    code: "AE01",
    submissionValue: "AE01",
    synonym: "",
    definition: "",
    nciPreferredTerm: "",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function renderTable(props: {
  rows: CodeItemView[];
  canMutate: boolean;
  onEdit?: ReturnType<typeof vi.fn>;
  onDelete?: ReturnType<typeof vi.fn>;
  onCreate?: ReturnType<typeof vi.fn>;
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CodeItemTable
          rows={props.rows}
          loading={false}
          mutationLoading={false}
          error={null}
          canMutate={props.canMutate}
          onRetry={() => {}}
          onCreate={props.onCreate ?? (() => {})}
          onEdit={props.onEdit ?? (() => {})}
          onDelete={props.onDelete ?? (() => {})}
          bottomSlot={props.bottomSlot}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("CodeItemTable — action gating", () => {
  it("shows both edit and delete icons for users who can mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: true });

    expect(
      screen.getByRole("button", { name: /edit code item/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /delete AE01/i }),
    ).toBeInTheDocument();
  });

  it("hides the header create (+) affordance for users who cannot mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: false });

    expect(
      screen.queryByRole("button", { name: /create code item/i }),
    ).not.toBeInTheDocument();
  });

  it("hides edit and delete for users who cannot mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: false });

    expect(
      screen.queryByRole("button", { name: /edit code item/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /delete AE01/i }),
    ).not.toBeInTheDocument();
  });

  it("invokes onEdit when the edit icon is clicked", async () => {
    const onEdit = vi.fn();
    renderTable({ rows: [makeRow()], canMutate: true, onEdit });

    await userEvent.click(screen.getByRole("button", { name: /edit code item/i }));

    expect(onEdit).toHaveBeenCalledTimes(1);
    expect(onEdit).toHaveBeenCalledWith(expect.objectContaining({ code: "AE01" }));
  });

  it("invokes onDelete when the delete icon is clicked", async () => {
    const onDelete = vi.fn();
    renderTable({ rows: [makeRow()], canMutate: true, onDelete });

    await userEvent.click(screen.getByRole("button", { name: /delete AE01/i }));

    expect(onDelete).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledWith(expect.objectContaining({ code: "AE01" }));
  });
});

describe("CodeItemTable — bottomSlot", () => {
  it("renders bottomSlot's output inside the scroll container", () => {
    renderTable({
      rows: [],
      canMutate: false,
      bottomSlot: () => <div data-testid="codeitem-slot">sentinel here</div>,
    });

    const slot = screen.getByTestId("codeitem-slot");
    const paper = slot.closest(".MuiPaper-root");
    expect(paper).not.toBeNull();
    expect(paper).toContainElement(slot);
  });

  it("passes the scroll container element to bottomSlot", () => {
    const captured: Array<HTMLElement | null> = [];
    renderTable({
      rows: [],
      canMutate: false,
      bottomSlot: (el) => {
        captured.push(el);
        return <div data-testid="codeitem-slot" />;
      },
    });

    const nonNull = captured.find((el) => el !== null);
    expect(nonNull).toBeDefined();
    expect(nonNull!.classList.contains("MuiPaper-root")).toBe(true);
  });
});