import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { AnnotationDialog } from "../../../features/crf/components/AnnotationDialog";
import type {
  AnnotationOwner,
  DomainAnnotation,
  SdtmDomainView,
  SdtmVariableView,
} from "../../../shared/api";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { mockCommands, mockInvoke } from "../../../test/helpers/tauri-mock";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

afterEach(() => cleanup());
beforeEach(() => {
  mockInvoke.mockReset();
  mockCommands({
    list_sdtm_variables_by_domain: () => ({ variables }),
  });
});

const owner: AnnotationOwner = { kind: "form", id: 11 };

const domainAnnotations: DomainAnnotation[] = [
  {
    id: 50,
    formId: 11,
    name: "AE",
    description: "Adverse Events",
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 51,
    formId: 11,
    name: "VS",
    description: "Vital Signs",
    createdAt: "",
    updatedAt: "",
  },
  // A free-form annotation that does NOT match any SDTM domain.
  {
    id: 52,
    formId: 11,
    name: "ZZ",
    description: "Custom",
    createdAt: "",
    updatedAt: "",
  },
];

const sdtmDomains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 101,
    versionId: 5,
    name: "VS",
    category: "Findings",
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

const variables: SdtmVariableView[] = [
  {
    id: 1,
    domainId: 100,
    name: "AETERM",
    variableType: "Character",
    variableCore: "Req",
    variableSequence: 1,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    domainId: 100,
    name: "AESEV",
    variableType: "Character",
    variableCore: "Exp",
    variableSequence: 2,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 3,
    domainId: 100,
    name: "AGE",
    variableType: "Numeric",
    variableCore: "Req",
    variableSequence: 3,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 4,
    domainId: 100,
    name: "DOMAIN",
    variableType: "Character",
    variableCore: "Req",
    variableSequence: 4,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

function SeedAndDialog({
  onSubmit,
  onMarkNotSubmitted,
  dialogProps,
}: {
  onSubmit: ReturnType<typeof vi.fn>;
  onMarkNotSubmitted: ReturnType<typeof vi.fn>;
  dialogProps: Partial<React.ComponentProps<typeof AnnotationDialog>>;
}) {
  return (
    <AegisI18nProvider>
      <AnnotationDialog
        open
        mode="create"
        owner={owner}
        ownerNotSubmitted={false}
        availableDomainAnnotations={domainAnnotations}
        onClose={() => undefined}
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        markNotSubmittedPending={false}
        markNotSubmittedError={null}
        mutationError={null}
        mutationPending={false}
        sdtmDomains={sdtmDomains}
        {...dialogProps}
      />
    </AegisI18nProvider>
  );
}

function mountWithSeed(
  props: Partial<React.ComponentProps<typeof AnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const onMarkNotSubmitted = vi.fn();
  const utils = render(
    <TestQueryProvider>
      <SeedAndDialog
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        dialogProps={props}
      />
    </TestQueryProvider>,
  );
  return { onSubmit, onMarkNotSubmitted, ...utils };
}

describe("AnnotationDialog", () => {
  it("submit is disabled until content is non-empty", () => {
    mountWithSeed();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Content/i), {
      target: { value: "note" },
    });
    expect(submit).not.toBeDisabled();
  });

  it("edit mode disables the domain annotation select and preserves assign", () => {
    const { onSubmit } = mountWithSeed({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old note",
        assign: true,
        owner,
        createdAt: "",
        updatedAt: "",
      },
    });
    const combobox = screen.getByRole("combobox");
    expect(combobox).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByDisplayValue("old note")).toBeInTheDocument();
    const assign = screen.getByRole("checkbox");
    expect(assign).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      domainAnnotationId: 50,
      content: "old note",
      assign: true,
    });
  });

  it("renders the Not submit button and triggers onMarkNotSubmitted", () => {
    const { onMarkNotSubmitted } = mountWithSeed();
    const notSubmit = screen.getByTestId("crf-annotation-dialog-not-submit");
    expect(notSubmit).toBeInTheDocument();
    fireEvent.click(notSubmit);
    expect(onMarkNotSubmitted).toHaveBeenCalledTimes(1);
  });

  it("hides the Not submit button when the owner is already not-submitted", () => {
    mountWithSeed({ ownerNotSubmitted: true });
    expect(
      screen.queryByTestId("crf-annotation-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  it("hides the Not submit button in edit mode", () => {
    mountWithSeed({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old note",
        assign: true,
        owner,
        createdAt: "",
        updatedAt: "",
      },
    });
    expect(
      screen.queryByTestId("crf-annotation-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  // --- New: @-mention behavior ---

  it("typing @ opens a dropdown with the full variable list for the matching SDTM domain", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    await waitFor(() => {
      expect(screen.getByTestId("crf-variable-1")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-2")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-4")).toBeInTheDocument();
    });
  });

  it("filters the variable list to startsWith the typed fragment (uppercased)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@a", selectionStart: 2 } });
    await waitFor(() => {
      expect(screen.getByTestId("crf-variable-1")).toBeInTheDocument(); // AETERM
      expect(screen.getByTestId("crf-variable-2")).toBeInTheDocument(); // AESEV
      expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument(); // AGE
    });
    expect(screen.queryByTestId("crf-variable-4")).not.toBeInTheDocument(); // DOMAIN
  });

  it("clicking a variable inserts the variable's name (replacing @fragment)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@age", selectionStart: 4 } });
    await waitFor(() => screen.getByTestId("crf-variable-3"));
    fireEvent.click(screen.getByTestId("crf-variable-3"));
    await waitFor(() => {
      expect(screen.getByLabelText(/Content/i)).toHaveValue("AGE");
    });
  });

  it("does NOT open the dropdown when @ is mid-word", () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, {
      target: { value: "foo@bar", selectionStart: 7 },
    });
    expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument();
  });

  it("does NOT open the dropdown when the picked domain annotation has no SDTM match", async () => {
    // Switch the picked domain annotation to the free-form "ZZ".
    mountWithSeed();
    const select = screen.getByRole("combobox");
    fireEvent.mouseDown(select);
    const zzOption = await screen.findByRole("option", { name: "ZZ" });
    fireEvent.click(zzOption);
    // Now type @ in content.
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument();
  });

  it("shows a 'No variables match' disabled item when the fragment has no match", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, {
      target: { value: "@zzz", selectionStart: 4 },
    });
    await waitFor(() => {
      expect(
        screen.getByText(/No variables match/i),
      ).toBeInTheDocument();
    });
  });

  // --- Keyboard navigation ---

  it("ArrowDown moves the highlight to the next variable (wrapping)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    // First item highlighted by default
    expect(screen.getByTestId("crf-variable-1")).toHaveAttribute(
      "data-highlighted",
      "true",
    );
    // ArrowDown → second item
    fireEvent.keyDown(content, { key: "ArrowDown" });
    expect(screen.getByTestId("crf-variable-2")).toHaveAttribute(
      "data-highlighted",
      "true",
    );
    expect(screen.getByTestId("crf-variable-1")).not.toHaveAttribute(
      "data-highlighted",
    );
    // Two more → wrap to first
    fireEvent.keyDown(content, { key: "ArrowDown" });
    fireEvent.keyDown(content, { key: "ArrowDown" });
    fireEvent.keyDown(content, { key: "ArrowDown" });
    expect(screen.getByTestId("crf-variable-1")).toHaveAttribute(
      "data-highlighted",
      "true",
    );
  });

  it("ArrowUp moves the highlight to the previous variable (wrapping)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    // ArrowUp from first → last (wrap)
    fireEvent.keyDown(content, { key: "ArrowUp" });
    expect(screen.getByTestId("crf-variable-4")).toHaveAttribute(
      "data-highlighted",
      "true",
    );
  });

  it("Enter inserts the highlighted variable", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    // Move to AESEV (second)
    fireEvent.keyDown(content, { key: "ArrowDown" });
    fireEvent.keyDown(content, { key: "Enter" });
    await waitFor(() => {
      expect(screen.getByLabelText(/Content/i)).toHaveValue("AESEV");
    });
  });

  it("Escape closes the dropdown but keeps the @ symbol in the text field", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@a", selectionStart: 2 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    fireEvent.keyDown(content, { key: "Escape" });
    // Dropdown closes — the variable items are gone.
    await waitFor(() =>
      expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument(),
    );
    // The @-fragment remains in the field; the user kept what they typed.
    expect(screen.getByLabelText(/Content/i)).toHaveValue("@a");
  });

  it("typing more letters after Escape re-opens the dropdown", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@a", selectionStart: 2 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    fireEvent.keyDown(content, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument(),
    );
    // User keeps typing — mention reopens.
    fireEvent.change(content, { target: { value: "@ag", selectionStart: 3 } });
    await waitFor(() => screen.getByTestId("crf-variable-3"));
    // AGE (id=3) is the only variable starting with "AG".
    expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument();
    expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument();
  });

  // --- Focus retention while the Popover is open ---
  // MUI's Popover steals focus on open by default. With focus stolen,
  // typing and ESC never reach the content TextField's handlers — the
  // user is typing into a MenuItem instead. The Popover must therefore
  // be told to leave focus on the TextField.

  it("focus stays on the content TextField after the dropdown opens", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i) as HTMLInputElement;
    content.focus();
    expect(document.activeElement).toBe(content);
    fireEvent.change(content, { target: { value: "@", selectionStart: 1 } });
    await waitFor(() => screen.getByTestId("crf-variable-1"));
    // Popover is open, but the TextField must still be the active
    // element — otherwise typing goes nowhere useful.
    expect(document.activeElement).toBe(content);
  });

  // --- SUPP quick-draft button ---

  it("renders the SUPP button when the dialog is open and a domain annotation is selected", () => {
    mountWithSeed();
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeInTheDocument();
  });

  it("disables the SUPP button when no domain annotation is selected", () => {
    // availableDomainAnnotations has three entries, but in create
    // mode the dialog defaults `body.domainAnnotationId` to
    // `availableDomainAnnotations[0].id`, so simulate "no selection"
    // by passing an empty list.
    mountWithSeed({ availableDomainAnnotations: [] });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeDisabled();
  });

  it("disables the SUPP button for an item owner with no resolved item code", () => {
    mountWithSeed({ owner: { kind: "item", id: 99 }, ownerItemCode: null });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).toBeDisabled();
  });

  it("enables the SUPP button for an item owner with a resolved item code", () => {
    mountWithSeed({ owner: { kind: "item", id: 99 }, ownerItemCode: "LBCLSIG" });
    expect(
      screen.getByTestId("crf-annotation-dialog-supp"),
    ).not.toBeDisabled();
  });

  it("drafts ' in SUPPXX' for a form-level owner (AE)", () => {
    const { onSubmit } = mountWithSeed({ owner: { kind: "form", id: 11 } });
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(" in SUPPAE");
    // onSubmit must NOT have been triggered — the click only sets body.
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("drafts '<itemCode> in SUPPXX' for an item-level owner (LBCLSIG/VS)", () => {
    const { onSubmit } = mountWithSeed({
      owner: { kind: "item", id: 99 },
      ownerItemCode: "LBCLSIG",
    });
    // The dialog defaults the picked domain annotation to
    // availableDomainAnnotations[0] (AE, id=50) in create mode.
    // Switch it to VS (id=51) so the SUPP draft lands on "VS".
    fireEvent.mouseDown(screen.getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "VS" }));
    fireEvent.click(screen.getByTestId("crf-annotation-dialog-supp"));
    expect(screen.getByLabelText(/Content/i)).toHaveValue(
      "LBCLSIG in SUPPVS",
    );
    expect(onSubmit).not.toHaveBeenCalled();
  });
});