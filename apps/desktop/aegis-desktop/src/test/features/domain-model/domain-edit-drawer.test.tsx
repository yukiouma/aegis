import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type {
  CreateSdtmDomainInput,
  SdtmDomainView,
  UpdateSdtmDomainInput,
} from "../../../shared/api";
import { DomainEditDrawer } from "../../../features/domain-model/components/DomainEditDrawer";

const sample: SdtmDomainView = {
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

function renderDrawer(props: {
  mode?: "create" | "edit";
  row?: SdtmDomainView;
  versionId?: number;
  onUpdate?: (id: number, b: UpdateSdtmDomainInput) => void;
  onCreate?: (input: CreateSdtmDomainInput) => void;
  pending?: boolean;
  error?: unknown;
}) {
  const row = props.row ?? sample;
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DomainEditDrawer
          open={true}
          mode={props.mode ?? "edit"}
          row={row}
          versionId={props.versionId}
          onClose={vi.fn()}
          onUpdate={props.onUpdate ?? vi.fn()}
          onCreate={props.onCreate ?? vi.fn()}
          canMutate={true}
          mutationError={(props.error ?? null) as never}
          mutationPending={props.pending ?? false}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("DomainEditDrawer", () => {
  afterEach(cleanup);

  it("submits the edited name and category via onUpdate", async () => {
    const onUpdate = vi.fn();
    renderDrawer({ row: sample, onUpdate });
    const nameInput = screen.getByRole("textbox", { name: /^code$/i });
    await userEvent.clear(nameInput);
    await userEvent.type(nameInput, "AEMOD");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    expect(onUpdate).toHaveBeenCalledOnce();
    const [id, body] = onUpdate.mock.calls[0];
    expect(id).toBe(7);
    expect(body.name).toBe("AEMOD");
    expect(body.category).toBe("Events");
    expect(Array.isArray(body.descriptions)).toBe(true);
  });

  it("renders mutation error inline", () => {
    renderDrawer({ row: sample, error: new Error("save failed") });
    expect(screen.getByText(/save failed/)).toBeInTheDocument();
  });

  it("disables submit while pending", () => {
    renderDrawer({ row: sample, pending: true });
    expect(screen.getByRole("button", { name: /save/i })).toBeDisabled();
  });

  it("renders the Create title and submit label in create mode", () => {
    renderDrawer({ mode: "create", versionId: 5 });
    expect(screen.getByText("Create domain")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Create$/ })).toBeInTheDocument();
  });

  it("submits the new domain via onCreate in create mode", async () => {
    const onCreate = vi.fn();
    renderDrawer({ mode: "create", versionId: 5, onCreate });
    const nameInput = screen.getByRole("textbox", { name: /^code$/i });
    await userEvent.type(nameInput, "AE");
    await userEvent.click(screen.getByRole("button", { name: /^Create$/ }));
    expect(onCreate).toHaveBeenCalledOnce();
    expect(onCreate).toHaveBeenCalledWith({
      versionId: 5,
      name: "AE",
      category: "Special Purpose",
      descriptions: [],
    });
  });

  it("disables the create submit button when versionId is undefined", () => {
    renderDrawer({ mode: "create" });
    expect(screen.getByRole("button", { name: /^Create$/ })).toBeDisabled();
  });
});