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
  /**
   * The owning form's `notSubmitted` flag. While the form is
   * marked not-submitted the create-annotation entry points (the
   * item name, the unit value, the option value) no longer open the
   * create dialog — the form's annotations have already been wiped
   * by the cascade, so there is nothing to annotate. The visual
   * affordance (cursor, hover underline) is dropped in the same
   * step so the row doesn't advertise a click that won't fire.
   */
  formNotSubmitted: boolean;
}

export function CrfItemRow({
  itemDetail,
  colorByDomainAnnotationId,
  onCreateAnnotation,
  onEditAnnotation,
  onDeleteAnnotation,
  onClearNotSubmitted,
  formNotSubmitted,
}: Props) {
  const { t } = useI18n();
  const { item, options, units, annotations } = itemDetail;
  // Build the create-annotation handler once per row so the click
  // short-circuits under a single readable guard instead of repeating
  // `if (formNotSubmitted) return` at every call site.
  const createFor = (owner: AnnotationOwner) => {
    if (formNotSubmitted) return;
    onCreateAnnotation(owner);
  };
  // Match MUI's disabled-MenuItem look: drop the pointer cursor and
  // the hover underline so the row doesn't lie about being clickable.
  const clickableSx = formNotSubmitted
    ? undefined
    : {
        cursor: "pointer" as const,
        "&:hover": { textDecoration: "underline" },
      };
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
          sx={clickableSx}
          onClick={() => createFor({ kind: "item", id: item.id })}
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
              sx={clickableSx}
              onClick={() => createFor({ kind: "unit", id: u.unit.id })}
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
                sx={clickableSx}
                onClick={() => createFor({ kind: "option", id: o.option.id })}
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
