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
  const utils = render(
    <AegisI18nProvider>
      <DomainAnnotationDialog
        open
        mode="create"
        formNotSubmitted={false}
        onClose={() => undefined}
        onSubmit={onSubmit}
        mutationError={null}
        mutationPending={false}
        {...props}
      />
    </AegisI18nProvider>,
  );
  return { onSubmit, ...utils };
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
      notSubmitted: false,
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
      notSubmitted: false,
    });
  });
});
