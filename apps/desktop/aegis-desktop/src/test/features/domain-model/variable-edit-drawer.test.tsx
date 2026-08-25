import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type {
  CreateSdtmVariableInput,
  SdtmVariableView,
  UpdateSdtmVariableInput,
} from "../../../shared/api";
import { VariableEditDrawer } from "../../../features/domain-model/components/VariableEditDrawer";

const sample: SdtmVariableView = {
  id: 11,
  domainId: 5,
  name: "AESEV",
  variableType: "Character",
  variableCore: "Req",
  variableRole: "Record Qualifier",
  variableSequence: 2,
  descriptions: [{ lang: "en", details: { label: "Severity" } }],
  createdAt: "",
  updatedAt: "",
};

function renderDrawer(props: {
  open: boolean;
  mode: "create" | "edit";
  row?: SdtmVariableView;
  domainId?: number;
  initialSequence?: number;
  onClose?: () => void;
  onCreate?: (i: CreateSdtmVariableInput) => void;
  onUpdate?: (id: number, b: UpdateSdtmVariableInput) => void;
  mutationError?: unknown;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <VariableEditDrawer
          open={props.open}
          mode={props.mode}
          row={props.row}
          domainId={props.domainId ?? 5}
          initialSequence={props.initialSequence ?? 3}
          onClose={props.onClose ?? vi.fn()}
          onCreate={props.onCreate ?? vi.fn()}
          onUpdate={props.onUpdate ?? vi.fn()}
          canMutate={true}
          mutationError={(props.mutationError ?? null) as never}
          mutationPending={false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("VariableEditDrawer", () => {
  afterEach(cleanup);

  it("does not render a variableSequence field in create mode", async () => {
    const onCreate = vi.fn();
    renderDrawer({ open: true, mode: "create", onCreate });
    await userEvent.type(
      screen.getByRole("textbox", { name: /name/i }),
      "AETERM",
    );
    await userEvent.click(screen.getByRole("button", { name: /create/i }));
    expect(onCreate).toHaveBeenCalledOnce();
    const input = onCreate.mock.calls[0][0] as CreateSdtmVariableInput;
    expect(input.variableSequence).toBe(3);
    expect(input.domainId).toBe(5);
    expect(input.variableType).toBe("Character");
    expect(input.variableCore).toBe("Req");
    expect(input.variableRole).toBeUndefined();
  });

  it("does not send variableSequence in update mode", async () => {
    const onUpdate = vi.fn();
    renderDrawer({ open: true, mode: "edit", row: sample, onUpdate });
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    expect(onUpdate).toHaveBeenCalledOnce();
    const [id, body] = onUpdate.mock.calls[0];
    expect(id).toBe(11);
    expect(body.variableSequence).toBeUndefined();
  });

  it("renders mutation error inline", () => {
    renderDrawer({
      open: true,
      mode: "edit",
      row: sample,
      mutationError: new Error("save failed"),
    });
    expect(screen.getByText(/save failed/)).toBeInTheDocument();
  });
});