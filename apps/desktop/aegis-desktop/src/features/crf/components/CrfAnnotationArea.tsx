import { Box, Stack } from "@aegis/ui/mui";

import type { Annotation } from "../../../shared/api";
import { AnnotationChip } from "./AnnotationChip";

interface Props {
  annotations: Annotation[];
  colorByDomainAnnotationId: Map<number, number>;
  /**
   * Whether the current viewer is allowed to edit annotations on
   * this form. When `false`, every chip is rendered with
   * `disabled={true}` so MUI blocks `onClick` and `onDelete` — see
   * `AnnotationChip` for the Tooltip wrapper. The page derives this
   * from the same RBAC flags used by the form-name hover menu.
   */
  canEditAnnotations: boolean;
  onEdit: (annotation: Annotation) => void;
  onDelete: (annotation: Annotation) => void;
}

/**
 * Renders the form-level annotation chips. The list lives directly
 * under the header, above the item rows.
 */
export function CrfAnnotationArea({
  annotations,
  colorByDomainAnnotationId,
  canEditAnnotations,
  onEdit,
  onDelete,
}: Props) {
  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1, height: "20px" }}>
      {annotations.length === 0 ? null : <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
        {annotations.map((a) => (
          <AnnotationChip
            key={a.id}
            annotation={a}
            colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
            disabled={!canEditAnnotations}
            onEdit={() => onEdit(a)}
            onDelete={() => onDelete(a)}
          />
        ))}
      </Stack>}

    </Box>
  );
}