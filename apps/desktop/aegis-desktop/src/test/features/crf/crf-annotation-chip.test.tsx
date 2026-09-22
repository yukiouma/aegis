import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import {
  AnnotationChip,
  annotationColor,
} from "../../../features/crf/components/AnnotationChip";
import { CrfAnnotationArea } from "../../../features/crf/components/CrfAnnotationArea";

// When `disabled={true}` is passed, AnnotationChip calls useI18n to
// render the disabled-state tooltip title. Tests in the
// `AnnotationChip — disabled prop` block wrap their render with
// AegisI18nProvider so the hook has a context. The default (enabled)
// path doesn't touch i18n, so existing tests don't need a provider.

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
    const { container } = render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={vi.fn()}
        onDelete={onDelete}
      />,
    );
    // MUI's Chip renders its delete affordance as an element with the
    // `.MuiChip-deleteIcon` class — that's the only deleteIcon prop
    // we override here is the default MUI icon.
    const deleteIcon = container.querySelector(".MuiChip-deleteIcon");
    expect(deleteIcon).not.toBeNull();
    fireEvent.click(deleteIcon!);
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

  it("uses a solid border when assign is false and a dashed border when assign is true", () => {
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
    ).toBe("dashed");
    unmount();
  });
});

describe("AnnotationChip — disabled prop", () => {
  const baseAnnotation = {
    id: 1,
    domainAnnotationId: 50,
    content: "annotation text",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-02T00:00:00Z",
  };

  // Disabled path uses useI18n for the tooltip title; wrap with the
  // provider so the hook has a context.
  function renderDisabled(ui: React.ReactElement) {
    return render(<AegisI18nProvider>{ui}</AegisI18nProvider>);
  }

  it("renders the chip unchanged when disabled is omitted or false", () => {
    render(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
      />,
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).not.toHaveClass("Mui-disabled");
  });

  it("renders with Mui-disabled when disabled is true", () => {
    renderDisabled(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
        disabled={true}
      />,
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).toHaveClass("Mui-disabled");
  });

  it("does not render the delete icon when disabled is true", () => {
    // When `disabled` is true the implementation unsets onDelete so the
    // delete affordance disappears entirely (rather than rendering a
    // permanently-disabled delete icon). This is the same behaviour
    // as MUI's own disabled chip with no onDelete supplied.
    const { container } = renderDisabled(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={() => undefined}
        onDelete={() => undefined}
        disabled={true}
      />,
    );
    expect(container.querySelector(".MuiChip-deleteIcon")).toBeNull();
  });

  it("does not call onEdit when the disabled chip is clicked", () => {
    const onEdit = vi.fn();
    renderDisabled(
      <AnnotationChip
        annotation={baseAnnotation}
        colorIndex={0}
        onEdit={onEdit}
        onDelete={() => undefined}
        disabled={true}
      />,
    );
    fireEvent.click(screen.getByText("annotation text"));
    expect(onEdit).not.toHaveBeenCalled();
  });
});

describe("CrfAnnotationArea — canEditAnnotations", () => {
  const baseAnnotation = {
    id: 1,
    domainAnnotationId: 50,
    content: "annotation text",
    assign: false,
    owner: { kind: "form" as const, id: 11 },
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-02T00:00:00Z",
  };

  it("renders chips enabled when canEditAnnotations is true", () => {
    render(
      <CrfAnnotationArea
        annotations={[baseAnnotation]}
        colorByDomainAnnotationId={new Map([[50, 0]])}
        canEditAnnotations={true}
        onEdit={() => undefined}
        onDelete={() => undefined}
      />,
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).not.toHaveClass("Mui-disabled");
  });

  it("renders chips disabled when canEditAnnotations is false", () => {
    render(
      <AegisI18nProvider>
        <CrfAnnotationArea
          annotations={[baseAnnotation]}
          colorByDomainAnnotationId={new Map([[50, 0]])}
          canEditAnnotations={false}
          onEdit={() => undefined}
          onDelete={() => undefined}
        />
      </AegisI18nProvider>,
    );
    const chip = screen.getByText("annotation text").closest(".MuiChip-root")!;
    expect(chip).toHaveClass("Mui-disabled");
  });

  it("does not call onEdit when a disabled chip is clicked", () => {
    const onEdit = vi.fn();
    render(
      <AegisI18nProvider>
        <CrfAnnotationArea
          annotations={[baseAnnotation]}
          colorByDomainAnnotationId={new Map([[50, 0]])}
          canEditAnnotations={false}
          onEdit={onEdit}
          onDelete={() => undefined}
        />
      </AegisI18nProvider>,
    );
    fireEvent.click(screen.getByText("annotation text"));
    expect(onEdit).not.toHaveBeenCalled();
  });
});
