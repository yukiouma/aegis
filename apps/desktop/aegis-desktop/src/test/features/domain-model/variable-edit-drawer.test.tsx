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
  availableLanguages?: string[];
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
          availableLanguages={props.availableLanguages}
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

  it("seeds one row per language in create mode", () => {
    renderDrawer({
      open: true,
      mode: "create",
      availableLanguages: ["en", "zh-CN"],
    });
    const langInputs = screen.getAllByRole("textbox", { name: /language/i });
    expect(langInputs).toHaveLength(2);
    expect((langInputs[0] as HTMLInputElement).value).toBe("en");
    expect((langInputs[1] as HTMLInputElement).value).toBe("zh-CN");
    const labelInputs = screen.getAllByRole("textbox", { name: /^label$/i });
    expect(labelInputs.length).toBeGreaterThan(0);
    for (const input of labelInputs) {
      expect((input as HTMLInputElement).value).toBe("");
    }
  });

  it("seeds no rows when availableLanguages is empty", () => {
    renderDrawer({
      open: true,
      mode: "create",
      availableLanguages: [],
    });
    expect(screen.queryAllByRole("textbox", { name: /language/i })).toHaveLength(0);
  });

  it("seeds a single row when there is one language", () => {
    renderDrawer({
      open: true,
      mode: "create",
      availableLanguages: ["en"],
    });
    const langInputs = screen.getAllByRole("textbox", { name: /language/i });
    expect(langInputs).toHaveLength(1);
    expect((langInputs[0] as HTMLInputElement).value).toBe("en");
  });

  it("loads row.descriptions in edit mode and ignores availableLanguages", () => {
    const editRow: SdtmVariableView = {
      ...sample,
      descriptions: [
        { lang: "ja", details: { label: "J-Label" } },
        { lang: "fr", details: { label: "F-Label" } },
      ],
    };
    renderDrawer({
      open: true,
      mode: "edit",
      row: editRow,
      availableLanguages: ["en"],
    });
    const langInputs = screen.getAllByRole("textbox", { name: /language/i });
    expect(langInputs).toHaveLength(2);
    expect((langInputs[0] as HTMLInputElement).value).toBe("ja");
    expect((langInputs[1] as HTMLInputElement).value).toBe("fr");
  });

  it("re-seeds descriptions when reopening create mode after closing", async () => {
    const { rerender } = renderDrawer({
      open: true,
      mode: "create",
      availableLanguages: ["en", "zh-CN"],
    });
    expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(2);

    const removeButtons = screen.getAllByRole("button", {
      name: /remove-description/i,
    });
    await userEvent.click(removeButtons[0]);
    expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(1);

    // Close then reopen with the same availableLanguages.
    rerender(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <VariableEditDrawer
            open={false}
            mode="create"
            row={undefined}
            domainId={5}
            initialSequence={3}
            availableLanguages={["en", "zh-CN"]}
            onClose={vi.fn()}
            onCreate={vi.fn()}
            onUpdate={vi.fn()}
            canMutate={true}
            mutationError={null}
            mutationPending={false}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );
    rerender(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <VariableEditDrawer
            open={true}
            mode="create"
            row={undefined}
            domainId={5}
            initialSequence={3}
            availableLanguages={["en", "zh-CN"]}
            onClose={vi.fn()}
            onCreate={vi.fn()}
            onUpdate={vi.fn()}
            canMutate={true}
            mutationError={null}
            mutationPending={false}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    );

    expect(screen.getAllByRole("textbox", { name: /language/i })).toHaveLength(2);
  });

  it("includes auto-seeded rows in the submitted descriptions (create mode)", async () => {
    const onCreate = vi.fn();
    renderDrawer({
      open: true,
      mode: "create",
      availableLanguages: ["en", "zh-CN"],
      onCreate,
    });
    await userEvent.type(
      screen.getByRole("textbox", { name: /name/i }),
      "AETOX",
    );
    await userEvent.click(screen.getByRole("button", { name: /create/i }));

    const submitted = onCreate.mock.calls[0][0] as CreateSdtmVariableInput;
    expect(submitted.descriptions).toEqual([
      { lang: "en", details: { label: "" } },
      { lang: "zh-CN", details: { label: "" } },
    ]);
  });
});