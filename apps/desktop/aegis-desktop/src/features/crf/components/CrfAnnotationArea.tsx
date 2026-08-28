import { Box, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

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
  const { t } = useI18n();
  if (annotations.length === 0) return null;
  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <Typography variant="subtitle2" sx={{ color: "text.secondary" }}>
        {t("crf.detail.formAnnotationsHeading")}
      </Typography>
      <Stack direction="row" flexWrap="wrap" gap={1}>
        {annotations.map((a) => (
          <AnnotationChip
            key={a.id}
            annotation={a}
            colorIndex={colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1}
            onEdit={() => onEdit(a)}
            onDelete={() => onDelete(a)}
          />
        ))}
      </Stack>
    </Box>
  );
}
