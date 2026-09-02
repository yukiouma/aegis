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
  /**
   * The item's own `notSubmitted` flag. When the item itself is
   * marked not-submitted the cascade has already wiped the
   * item's annotations AND the annotations on its options and
   * units, so every create-annotation entry point in this row
   * must be blocked. The page surfaces the same flag on the
   * `NotSubmittedChip` so the user can flip the item back to
   * submitted without going through the form-level chip.
   */
  itemNotSubmitted: boolean;
  /**
   * Whether the form has any domain annotations at all. An
   * annotation needs a domain annotation to belong to, so when
   * the form has none, every create-annotation entry point must
   * be blocked. The page also gates the `New annotation` menu
   * item on the same condition; this prop keeps the row in sync.
   */
  noDomainAnnotations: boolean;
}

export function CrfItemRow({
  itemDetail,
  colorByDomainAnnotationId,
  onCreateAnnotation,
  onEditAnnotation,
  onDeleteAnnotation,
  onClearNotSubmitted,
  formNotSubmitted,
  itemNotSubmitted,
  noDomainAnnotations,
}: Props) {
  const { t } = useI18n();
  const { item, options, units, annotations } = itemDetail;
  // Label items are static text — they don't carry a captured
  // variable, so the code chip and every create-annotation entry
  // point are no-ops for them. Treat `kind === "label"` as a
  // row-wide block alongside the existing guards.
  const isLabel = item.kind === "label";
  // Collapse the three "no new annotations" guards into one. When
  // any of these is true every create-annotation entry point on
  // this row must short-circuit — the form cascade has wiped
  // every annotation, the item cascade has wiped this row's
  // annotations, there is no domain annotation to assign a
  // new annotation to, or the item is a static label.
  const rowBlocked =
    formNotSubmitted || itemNotSubmitted || noDomainAnnotations || isLabel;
  // Build the create-annotation handler once per row so the click
  // short-circuits under a single readable guard instead of
  // repeating the conditions at every call site.
  const createFor = (owner: AnnotationOwner) => {
    if (rowBlocked) return;
    onCreateAnnotation(owner);
  };
  // Match MUI's disabled-MenuItem look: drop the pointer cursor and
  // the hover underline so the row doesn't lie about being clickable.
  const clickableSx = rowBlocked
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
      data-testid={`crf-item-${item.id}`}
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
        {/* Label items have no captured variable — hide the code
            chip so the row reads as static text rather than as a
            field that can be annotated. */}
        {!isLabel && (
          <Chip
            sx={{ width: 85 }}
            label={item.code}
            variant="outlined"
            size="small"
          />
        )}
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
