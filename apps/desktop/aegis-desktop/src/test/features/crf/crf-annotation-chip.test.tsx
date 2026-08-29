import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  AnnotationChip,
  annotationColor,
} from "../../../features/crf/components/AnnotationChip";

afterEach(() => cleanup());

describe("annotationColor", () => {
  it("cycles info -> warning -> success -> error -> info", () => {
    expect(annotationColor(0)).toBe("info");
    expect(annotationColor(1)).toBe("warning");
    expect(annotationColor(2)).toBe("success");
    expect(annotationColor(3)).toBe("error");
    expect(annotationColor(4)).toBe("info");
    expect(annotationColor(-1)).toBe("default");
  });
});

describe("AnnotationChip", () => {
  const baseAnnotation = {
    id: 100,
    domainAnnotationId: 50,
    content: "form-level note",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "",
    updatedAt: "",
  };

  it("renders the annotation content", () => {
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText("form-level note")).toBeInTheDocument();
  });

  it("clicking the chip body calls onEdit", () => {
    const onEdit = vi.fn();
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={onEdit}
        onDelete={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByText("form-level note"));
    expect(onEdit).toHaveBeenCalledTimes(1);
  });

  it("clicking the delete icon calls onDelete", () => {
    const onDelete = vi.fn();
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={onDelete}
      />,
    );
    fireEvent.click(screen.getByTestId("annotation-chip-delete"));
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it("applies the colour for the supplied index", () => {
    // The header domain-annotation chips share this same `annotationColor`
    // palette, so this test guards both call sites. The colour class is
    // attached to the chip's root element, not the label span that
    // `getByText` returns — so walk up to the chip first.
    const cases: Array<[number, string]> = [
      [0, "MuiChip-colorInfo"],
      [1, "MuiChip-colorWarning"],
      [2, "MuiChip-colorSuccess"],
      [3, "MuiChip-colorError"],
    ];
    for (const [colorIndex, className] of cases) {
      const { unmount } = render(
        <AnnotationChip
          annotation={baseAnnotation}
          colorIndex={colorIndex}
          onEdit={vi.fn()}
          onDelete={vi.fn()}
        />,
      );
      const chip = screen.getByText("form-level note").closest(".MuiChip-root");
      expect(chip).not.toBeNull();
      expect(chip).toHaveClass(className);
      unmount();
    }
  });

  it("uses a solid border when assign is false and a dotted border when assign is true", () => {
    const { rerender, unmount } = render(
      <AnnotationChip
        annotation={{ ...baseAnnotation, assign: false }}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    const root = screen.getByText("form-level note").closest(".MuiChip-root");
    expect(root).not.toBeNull();
    // Default MUI outlined Chip renders a solid border.
    expect(getComputedStyle(root as Element).borderStyle).toBe("solid");

    rerender(
      <AnnotationChip
        annotation={{ ...baseAnnotation, assign: true }}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(
      getComputedStyle(screen.getByText("form-level note").closest(".MuiChip-root") as Element)
        .borderStyle,
    ).toBe("dotted");
    unmount();
  });
});
