import { Badge, Box, Chip, Stack, Tooltip, Typography } from "@aegis/ui/mui";
import { RadioButtonUnchecked as RadioButtonUncheckedIcon } from "@aegis/ui/icons";

import type { Annotation, AnnotationOwner, CrfItemDetail } from "../../../shared/api";
import { sanitizeHtml } from "../../../shared/sanitize";
import { useI18n } from "@aegis/ui/i18n";
import { AnnotationChip } from "./AnnotationChip";
import { NotSubmittedChip } from "./NotSubmittedChip";

interface Props {
  itemDetail: CrfItemDetail;
  colorByDomainAnnotationId: Map<number, number>;
  /**
   * Whether the current viewer is allowed to create / update
   * annotations on this form. When `false`, every row-level
   * create-annotation entry point (item name, option value, unit
   * value) is gated: the pointer cursor / hover underline fall
   * away and the click is a no-op. Derived by the page from the
   * same RBAC flags used by the form-name hover menu.
   */
  canEditAnnotations: boolean;
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
  /**
   * Number of mission issues with `state: opened` for this item's
   * code (the same scope as `targetItem`). The page derives this
   * from the cached issues list. Drives the count Badge on the
   * item code chip — `0` hides the badge (MUI's default `showZero:
   * false` behavior).
   */
  openIssueCount: number;
  /**
   * Open the mission-issue dialog scoped to this item. Wired by
   * the page to `setIssueDialog({ scope: { kind: "item", ... } })`.
   */
  onOpenIssues: () => void;
  /**
   * Whether a mission exists for the owning form. When false the
   * code chip is disabled with a tooltip — there's no scope to open
   * an issue against. The Badge still renders (with `invisible`)
   * so the chip doesn't reflow when the mission shows up.
   */
  missionExists: boolean;
  /**
   * Whether the current viewer is allowed to open the mission-issue
   * dialog when its scope has zero opened issues. When `false` AND
   * `openIssueCount === 0`, the item code chip is disabled with the
   * no-issues-to-view tooltip. The page derives this from the same
   * RBAC flags used by the form-name hover menu.
   */
  canOpenEmptyIssueDialog: boolean;
}

export function CrfItemRow({
  itemDetail,
  colorByDomainAnnotationId,
  canEditAnnotations,
  onCreateAnnotation,
  onEditAnnotation,
  onDeleteAnnotation,
  onClearNotSubmitted,
  formNotSubmitted,
  itemNotSubmitted,
  noDomainAnnotations,
  openIssueCount,
  onOpenIssues,
  missionExists,
  canOpenEmptyIssueDialog,
}: Props) {
  const { t } = useI18n();
  const { item, options, units, annotations } = itemDetail;
  // Label items are static text — they don't carry a captured
  // variable, so the code chip and every create-annotation entry
  // point are no-ops for them. Treat `kind === "label"` as a
  // row-wide block alongside the existing guards.
  const isLabel = item.kind === "label";
  // Collapse the row-level "no new annotations" guards into one.
  // When any of these is true every create-annotation entry point on
  // this row must short-circuit — the form cascade has wiped every
  // annotation, the item cascade has wiped this row's annotations,
  // there is no domain annotation to assign a new annotation to,
  // the item is a static label, or the current viewer doesn't have
  // permission to edit annotations on this form (QC / task-unrelated).
  const rowBlocked =
    formNotSubmitted || itemNotSubmitted || noDomainAnnotations ||
    isLabel || !canEditAnnotations;
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
        {!isLabel && (() => {
          // Three gate reasons, ranked by informativeness:
          //   1. no mission at all -> "no mission exists" tooltip wins
          //   2. mission exists but zero opened issues AND the viewer
          //      can't open the empty-issue dialog -> "no issues to view"
          //   3. otherwise, the chip is enabled
          const noMission = !missionExists;
          const noIssuesToView =
            missionExists &&
            openIssueCount === 0 &&
            !canOpenEmptyIssueDialog;
          const chipDisabled = noMission || noIssuesToView;
          const chipTitle = noMission
            ? t("crf.missionIssue.tooltip.noMission")
            : noIssuesToView
              ? t("crf.detail.tooltip.noIssueToView")
              : "";
          return (
            <Tooltip
              title={chipTitle}
              disableHoverListener={!chipDisabled}
              disableFocusListener={!chipDisabled}
              disableTouchListener={!chipDisabled}
            >
              <span>
                <Badge
                  color="error"
                  badgeContent={openIssueCount}
                  invisible={!missionExists}
                  overlap="circular"
                >
                  <Chip
                    sx={{ width: 92 }}
                    label={item.code}
                    variant="outlined"
                    size="small"
                    onClick={onOpenIssues}
                    disabled={chipDisabled}
                    data-testid={`crf-item-code-${item.id}`}
                  />
                </Badge>
              </span>
            </Tooltip>
          );
        })()}
        <Typography
          variant="subtitle1"
          sx={clickableSx}
          onClick={() => createFor({ kind: "item", id: item.id })}
          data-testid={`crf-item-name-${item.id}`}
          // `item.name` arrives verbatim from als-resolver and may
          // contain inline markup (`<br />`, `<b>…</b>`) that the
          // user expects to render. Sanitize before injecting —
          // the backend stores it raw, so we cannot assume the
          // input is safe.
          dangerouslySetInnerHTML={{ __html: sanitizeHtml(item.name) }}
        />
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
              disabled={!canEditAnnotations}
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
                  disabled={!canEditAnnotations}
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
                    disabled={!canEditAnnotations}
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
