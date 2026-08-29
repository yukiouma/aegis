import { Box, Chip, Stack, Typography } from "@aegis/ui/mui";
import { RadioButtonUnchecked as RadioButtonUncheckedIcon } from "@aegis/ui/icons";

import type { Annotation, AnnotationOwner, CrfItemDetail } from "../../../shared/api";
import { useI18n } from "@aegis/ui/i18n";
import { AnnotationChip } from "./AnnotationChip";
import { NotSubmittedChip } from "./NotSubmittedChip";

interface Props {
  itemDetail: CrfItemDetail;
  colorByDomainAnnotationId: Map<number, number>;
  /**
   * Open the new-annotation dialog for the given owner. The page
   * holds the dialog state so the caller's owner kind/id stays in
   * one place.
   */
  onCreateAnnotation: (owner: Annotation["owner"]) => void;
  onEditAnnotation: (annotation: Annotation) => void;
  onDeleteAnnotation: (annotation: Annotation) => void;
  /**
   * Clear the owner-level `notSubmitted` flag back to `false`.
   * Wired by the page to `useUpdateOwnerNotSubmitted`. No
   * cascade — only the `false → true` transition deletes
   * annotations; going back to `false` just lifts the flag.
   */
  onClearNotSubmitted: (owner: AnnotationOwner) => void;
}

export function CrfItemRow({
  itemDetail,
  colorByDomainAnnotationId,
  onCreateAnnotation,
  onEditAnnotation,
  onDeleteAnnotation,
  onClearNotSubmitted,
}: Props) {
  const { t } = useI18n();
  const { item, options, units, annotations } = itemDetail;
  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        gap: 1,
        p: 2,
        border: 1,
        borderColor: "divider",
        borderRadius: 1,
      }}
      data-testid={`crf-item-row-${item.id}`}
    >
      <Box
        sx={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 1,
          flexWrap: "wrap",
        }}
      >
        <Chip label={item.code} variant="outlined" size="small" />
        <Typography
          variant="subtitle1"
          sx={{
            cursor: "pointer",
            "&:hover": { textDecoration: "underline" },
          }}
          onClick={() => onCreateAnnotation({ kind: "item", id: item.id })}
          data-testid={`crf-item-name-${item.id}`}
        >
          {item.name}
        </Typography>
        {item.notSubmitted && (
          <NotSubmittedChip
            onDelete={() =>
              onClearNotSubmitted({ kind: "item", id: item.id })
            }
          />
        )}
        <Stack
          direction="row"
          spacing={1}
          sx={{ flexWrap: "wrap", flexGrow: 1 }}
        >
          {annotations.map((a) => (
            <AnnotationChip
              key={a.id}
              annotation={a}
              colorIndex={
                colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1
              }
              onEdit={() => onEditAnnotation(a)}
              onDelete={() => onDeleteAnnotation(a)}
            />
          ))}
        </Stack>
        {/* Unit on the right side */}
        {units.map((u) => (
          <Box
            key={u.unit.id}
            sx={{ display: "flex", alignItems: "center", gap: 1 }}
          >
            <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
              {u.annotations.map((a) => (
                <AnnotationChip
                  key={a.id}
                  annotation={a}
                  colorIndex={
                    colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1
                  }
                  onEdit={() => onEditAnnotation(a)}
                  onDelete={() => onDeleteAnnotation(a)}
                />
              ))}
            </Stack>
            <Typography
              variant="body2"
              sx={{
                cursor: "pointer",
                "&:hover": { textDecoration: "underline" },
              }}
              onClick={() =>
                onCreateAnnotation({ kind: "unit", id: u.unit.id })
              }
              data-testid={`crf-unit-${u.unit.id}`}
            >
              {t("crf.detail.unitLabel", { value: u.unit.value })}
            </Typography>
            {u.unit.notSubmitted && (
              <NotSubmittedChip
                onDelete={() =>
                  onClearNotSubmitted({ kind: "unit", id: u.unit.id })
                }
              />
            )}
          </Box>
        ))}
      </Box>
      {options.length > 0 && (
        <Box sx={{ mt: 3, display: "flex", flexDirection: "column", gap: 1 }}>
          {options.map((o) => (
            <Box
              key={o.option.id}
              sx={{ display: "flex", gap: 1 }}
            >
              <RadioButtonUncheckedIcon fontSize="small" />
              <Typography
                variant="body2"
                sx={{
                  cursor: "pointer",
                  "&:hover": { textDecoration: "underline" },
                }}
                onClick={() =>
                  onCreateAnnotation({ kind: "option", id: o.option.id })
                }
                data-testid={`crf-option-${o.option.id}`}
              >
                {o.option.value}
              </Typography>
              {o.option.notSubmitted && (
                <NotSubmittedChip
                  onDelete={() =>
                    onClearNotSubmitted({ kind: "option", id: o.option.id })
                  }
                />
              )}
              <Stack direction="row" spacing={1}>
                {o.annotations.map((a) => (
                  <AnnotationChip
                    key={a.id}
                    annotation={a}
                    colorIndex={
                      colorByDomainAnnotationId.get(a.domainAnnotationId) ?? -1
                    }
                    onEdit={() => onEditAnnotation(a)}
                    onDelete={() => onDeleteAnnotation(a)}
                  />
                ))}
              </Stack>
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
}
