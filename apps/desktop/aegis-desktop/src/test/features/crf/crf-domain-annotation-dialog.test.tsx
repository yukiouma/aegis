import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { DomainAnnotationDialog } from "../../../features/crf/components/DomainAnnotationDialog";
import type { SdtmDomainView } from "../../../shared/api";

afterEach(() => cleanup());

const sdtmDomains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Adverse Events", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 101,
    versionId: 5,
    name: "AESI",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "AESIs", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 102,
    versionId: 5,
    name: "AG",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Agent", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 103,
    versionId: 5,
    name: "VS",
    category: "Findings",
    descriptions: [
      // Note: only EN — the test asserts zh-CN fallback empties the description.
      { lang: "en", details: { description: "Vital Signs", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
];

function renderDialog(
  props: Partial<React.ComponentProps<typeof DomainAnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const onMarkNotSubmitted = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <DomainAnnotationDialog
        open
        mode="create"
        formNotSubmitted={false}
        onClose={() => undefined}
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        markNotSubmittedPending={false}
        markNotSubmittedError={null}
        mutationError={null}
        mutationPending={false}
        sdtmDomains={sdtmDomains}
        sdtmLanguage="en"
        {...props}
      />
    </AegisI18nProvider>,
  );
  return { onSubmit, onMarkNotSubmitted, ...utils };
}

describe("DomainAnnotationDialog", () => {
  it("submit is disabled while name is empty", () => {
    const { onSubmit } = renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getAllByLabelText(/Name/i)[0]!, {
      target: { value: "AE" },
    });
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    expect(onSubmit).toHaveBeenCalledWith({
      name: "AE",
      description: "",
    });
  });

  it("edit mode pre-fills from row", () => {
    const onSubmit = vi.fn();
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "AE",
        description: "Adverse Events",
        createdAt: "",
        updatedAt: "",
      },
      onSubmit,
    });
    // The Autocomplete uppercases typed input, so "renamed" → "RENAMED".
    fireEvent.change(screen.getAllByLabelText(/Name/i)[0]!, {
      target: { value: "renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "RENAMED",
      description: "Adverse Events",
    });
  });

  it("renders the Not submit button and triggers onMarkNotSubmitted", () => {
    const { onMarkNotSubmitted } = renderDialog();
    const notSubmit = screen.getByTestId("crf-domain-dialog-not-submit");
    expect(notSubmit).toBeInTheDocument();
    fireEvent.click(notSubmit);
    expect(onMarkNotSubmitted).toHaveBeenCalledTimes(1);
  });

  it("hides the Not submit button when the form is already not-submitted", () => {
    renderDialog({ formNotSubmitted: true });
    expect(
      screen.queryByTestId("crf-domain-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  it("hides the Not submit button in edit mode", () => {
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "AE",
        description: "Adverse Events",
        createdAt: "",
        updatedAt: "",
      },
    });
    expect(
      screen.queryByTestId("crf-domain-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  // --- New: Autocomplete + description auto-fill ---

  it("auto-uppercases typed name input", () => {
    renderDialog();
    const nameInput = screen.getAllByLabelText(/Name/i)[0]!;
    fireEvent.change(nameInput, { target: { value: "ae" } });
    expect(nameInput).toHaveValue("AE");
  });

  it("free-form typing leaves the description unchanged when no domain matches", () => {
    renderDialog();
    fireEvent.change(screen.getAllByLabelText(/Name/i)[0]!, {
      target: { value: "ZZ" },
    });
    fireEvent.change(screen.getAllByLabelText(/Description/i)[0]!, {
      target: { value: "custom" },
    });
    expect(screen.getAllByLabelText(/Description/i)[0]!).toHaveValue("custom");
  });

  it("picking a matching domain auto-fills the description in the project's language", () => {
    renderDialog();
    const nameInput = screen.getAllByLabelText(/Name/i)[0]!;
    fireEvent.change(nameInput, { target: { value: "AE" } });
    // Open the Autocomplete dropdown and pick the AE option.
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "AE" }));
    expect(screen.getAllByLabelText(/Description/i)[0]!).toHaveValue("Adverse Events");
  });

  it("picking a domain with no description in the project language leaves the description empty", () => {
    renderDialog({ sdtmLanguage: "zh-CN" });
    const nameInput = screen.getAllByLabelText(/Name/i)[0]!;
    fireEvent.change(nameInput, { target: { value: "VS" } });
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "VS" }));
    expect(screen.getAllByLabelText(/Description/i)[0]!).toHaveValue("");
  });

  it("in edit mode, picking a different domain re-fills the description", () => {
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "old",
        description: "old description",
        createdAt: "",
        updatedAt: "",
      },
    });
    const nameInput = screen.getAllByLabelText(/Name/i)[0]!;
    fireEvent.change(nameInput, { target: { value: "AESI" } });
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "AESI" }));
    expect(screen.getAllByLabelText(/Description/i)[0]!).toHaveValue("AESIs");
  });
});