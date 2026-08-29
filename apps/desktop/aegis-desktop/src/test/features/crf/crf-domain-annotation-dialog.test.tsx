import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { DomainAnnotationDialog } from "../../../features/crf/components/DomainAnnotationDialog";

afterEach(() => cleanup());

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
    fireEvent.change(screen.getByLabelText(/Name/i), {
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
    fireEvent.change(screen.getByLabelText(/Name/i), {
      target: { value: "Renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "Renamed",
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
    // The Not submit action is a "decide whether the form needs
    // the flag" affordance. In edit mode the user is changing an
    // existing domain annotation's name / description, not making
    // that decision — so the button must be hidden even when the
    // form is currently submitted.
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
});
