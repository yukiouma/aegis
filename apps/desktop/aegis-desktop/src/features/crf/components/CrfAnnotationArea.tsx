import { Box, Stack } from "@aegis/ui/mui";

import type { Annotation } from "../../../shared/api";
import { AnnotationChip } from "./AnnotationChip";

interface Props {
  annotations: Annotation[];
  colorByDomainAnnotationId: Map<number, number>;
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
            onEdit={() => onEdit(a)}
            onDelete={() => onDelete(a)}
          />
        ))}
      </Stack>}

    </Box>
  );
}
