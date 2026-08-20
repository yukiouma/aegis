import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { CodeListTable } from
  "../../../features/terminology/components/CodeListTable";
import type { CodeListView } from "../../../shared/api";

afterEach(() => {
  cleanup();
});

function makeRow(overrides: Partial<CodeListView> = {}): CodeListView {
  return {
    id: 1,
    versionId: 1,
    code: "AE",
    extensible: false,
    name: "Adverse Events",
    submissionValue: "AE",
    synonym: "",
    definition: "",
    nciPreferredTerm: "",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function renderTable(props: {
  rows: CodeListView[];
  canMutate: boolean;
  onOpen?: ReturnType<typeof vi.fn>;
  onDelete?: ReturnType<typeof vi.fn>;
  onCreate?: ReturnType<typeof vi.fn>;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CodeListTable
          mode="list"
          rows={props.rows}
          loading={false}
          mutationLoading={false}
          error={null}
          canMutate={props.canMutate}
          onRetry={() => {}}
          onCreate={props.onCreate ?? (() => {})}
          onDelete={props.onDelete ?? (() => {})}
          onOpen={props.onOpen ?? (() => {})}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("CodeListTable — action gating", () => {
  it("shows both launch (open) and delete icons for users who can mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: true });

    expect(
      screen.getByRole("button", { name: /open AE/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /delete AE/i }),
    ).toBeInTheDocument();
  });

  it("hides the header create (+) affordance for users who cannot mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: false });

    expect(
      screen.queryByRole("button", { name: /create code list/i }),
    ).not.toBeInTheDocument();
  });

  it("shows the launch (open) icon but hides delete for users who cannot mutate", () => {
    renderTable({ rows: [makeRow()], canMutate: false });

    // Open is no longer gated — viewers can drill into a code list.
    expect(
      screen.getByRole("button", { name: /open AE/i }),
    ).toBeInTheDocument();
    // Delete stays gated behind canMutate.
    expect(
      screen.queryByRole("button", { name: /delete AE/i }),
    ).not.toBeInTheDocument();
  });

  it("invokes onOpen when the launch icon is clicked (viewer role)", async () => {
    const onOpen = vi.fn();
    renderTable({ rows: [makeRow()], canMutate: false, onOpen });

    await userEvent.click(screen.getByRole("button", { name: /open AE/i }));

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ code: "AE" }));
  });

  it("invokes onDelete when the delete icon is clicked (manager role)", async () => {
    const onDelete = vi.fn();
    renderTable({ rows: [makeRow()], canMutate: true, onDelete });

    await userEvent.click(screen.getByRole("button", { name: /delete AE/i }));

    expect(onDelete).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledWith(expect.objectContaining({ code: "AE" }));
  });
});