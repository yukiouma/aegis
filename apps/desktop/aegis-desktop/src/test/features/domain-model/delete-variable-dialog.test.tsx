import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { SdtmVariableView } from "../../../shared/api";
import { DeleteVariableDialog } from "../../../features/domain-model/components/DeleteVariableDialog";

const sampleVariable: SdtmVariableView = {
  id: 11,
  domainId: 5,
  name: "AESEV",
  variableType: "Character",
  variableCore: "Req",
  variableRole: "Record Qualifier",
  variableSequence: 2,
  descriptions: [],
  createdAt: "",
  updatedAt: "",
};

function renderDialog(props: {
  open: boolean;
  row?: SdtmVariableView | null;
  pending?: boolean;
  error?: unknown;
  onClose?: () => void;
  onConfirm?: (row: SdtmVariableView) => void;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DeleteVariableDialog
          open={props.open}
          row={props.row ?? null}
          onClose={props.onClose ?? vi.fn()}
          onConfirm={props.onConfirm ?? vi.fn()}
          pending={props.pending ?? false}
          error={props.error ?? null}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("DeleteVariableDialog", () => {
  afterEach(cleanup);

  it("does not render content when closed", () => {
    renderDialog({ open: false });
    expect(screen.queryByText(/Delete variable/)).toBeNull();
  });

  it("renders the confirm message when open", () => {
    renderDialog({ open: true, row: sampleVariable });
    expect(screen.getByText(/Delete variable\?/)).toBeInTheDocument();
    expect(screen.getByText(/This cannot be undone/)).toBeInTheDocument();
  });

  it("fires onConfirm with the row when Confirm clicked", async () => {
    const onConfirm = vi.fn();
    renderDialog({ open: true, row: sampleVariable, onConfirm });
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));
    expect(onConfirm).toHaveBeenCalledWith(sampleVariable);
  });

  it("disables both buttons while pending", () => {
    renderDialog({ open: true, row: sampleVariable, pending: true });
    expect(screen.getByRole("button", { name: /cancel/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled();
  });

  it("renders the error in error color when provided", () => {
    renderDialog({
      open: true,
      row: sampleVariable,
      error: new Error("boom"),
    });
    expect(screen.getByText(/boom/)).toBeInTheDocument();
  });
});