import { Chip } from "@aegis/ui/mui";
import type { ChipProps } from "@aegis/ui/mui";
import type { Annotation } from "../../../shared/api";

/**
 * Map an index (the position of the owning domain annotation in the
 * form's domain-annotation list) to a Chip color. Cycles every 4
 * domain annotations. A negative index (the owning domain annotation
 * is not in the loaded list) falls back to the default colour.
 */
export function annotationColor(index: number): ChipProps["color"] {
  if (index < 0) return "default";
  const palette: ChipProps["color"][] = ["info", "warning", "success", "error"];
  return palette[index % palette.length];
}

interface Props {
  annotation: Annotation;
  /**
   * Index of the owning domain annotation in the form's
   * `domainAnnotations` array, or -1 if not found. Negative falls
   * through to the default palette slot.
   */
  colorIndex: number;
  onEdit: () => void;
  onDelete: () => void;
}

export function AnnotationChip({
  annotation,
  colorIndex,
  onEdit,
  onDelete,
}: Props) {
  return (
    <Chip
      label={annotation.content}
      color={annotationColor(colorIndex)}
      onClick={onEdit}
      onDelete={onDelete}
      deleteIcon={
        <span data-testid="annotation-chip-delete" aria-hidden>
          ×
        </span>
      }
      size="small"
    />
  );
}
