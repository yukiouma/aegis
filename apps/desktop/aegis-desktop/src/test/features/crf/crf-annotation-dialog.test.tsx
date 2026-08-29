import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { AnnotationDialog } from "../../../features/crf/components/AnnotationDialog";
import type {
  AnnotationOwner,
  DomainAnnotation,
} from "../../../shared/api";

afterEach(() => cleanup());

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
];

function renderDialog(
  props: Partial<React.ComponentProps<typeof AnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <AnnotationDialog
        open
        mode="create"
        owner={owner}
        ownerNotSubmitted={false}
        availableDomainAnnotations={domainAnnotations}
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

describe("AnnotationDialog", () => {
  it("submit is disabled until content is non-empty", () => {
    renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Content/i), {
      target: { value: "note" },
    });
    expect(submit).not.toBeDisabled();
  });

  it("edit mode disables the domain annotation select and preserves assign", () => {
    const { onSubmit } = renderDialog({
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
    // Domain annotation Select is disabled in edit mode (MUI uses
    // aria-disabled on the combobox role when FormControl is disabled).
    const combobox = screen.getByRole("combobox");
    expect(combobox).toHaveAttribute("aria-disabled", "true");
    // Content is pre-filled
    expect(screen.getByDisplayValue("old note")).toBeInTheDocument();
    // Assign checkbox is checked (the first checkbox in the dialog;
    // the second one is the new `Not submitted` flag)
    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes[0]).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      domainAnnotationId: 50,
      content: "old note",
      assign: true,
      notSubmitted: false,
    });
  });
});
