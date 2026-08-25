import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type {
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
  row: SdtmDomainView;
  onUpdate?: (id: number, b: UpdateSdtmDomainInput) => void;
  pending?: boolean;
  error?: unknown;
}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DomainEditDrawer
          open={true}
          row={props.row}
          onClose={vi.fn()}
          onUpdate={props.onUpdate ?? vi.fn()}
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
});