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
});
