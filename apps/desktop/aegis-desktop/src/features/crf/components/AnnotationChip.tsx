import { Chip, Tooltip } from "@aegis/ui/mui";
import type { ChipProps } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
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
  /**
   * Render the chip as MUI-disabled (blocks both `onClick` and
   * `onDelete`) and wrap it in a Tooltip that explains the user
   * lacks permission. Used by `CrfAnnotationArea` when the current
   * viewer is not allowed to edit annotations on this form. The
   * Tooltip host is required because MUI's disabled chips drop
   * hover events; the same `<Tooltip><span><Chip /></span></Tooltip>`
   * pattern is used elsewhere on the page (cf. `CrfDetailPage.tsx`
   * lines 330-369 and 409-466).
   */
  disabled?: boolean;
}

/**
 * The chip body. Doesn't touch `useI18n` so callers don't need an
 * `AegisI18nProvider` for the enabled (default) path. When
 * `disabled` is true, the public `AnnotationChip` routes through
 * `DisabledAnnotationChip` which owns the i18n tooltip title.
 */
export function AnnotationChip(props: Props) {
  if (props.disabled) return <DisabledAnnotationChip {...props} />;
  return <EnabledAnnotationChip {...props} />;
}

function EnabledAnnotationChip({
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
      size="small"
      variant="outlined"
      // `assign: true` flips the chip border to a dotted line so the
      // user can tell at a glance which annotations are "assigned"
      // (vs. just describing the field). MUI's outlined Chip already
      // supplies border-color from the active colour and a 1px width;
      // overriding only `borderStyle` keeps the colour theming intact.
      sx={annotation.assign ? { borderStyle: "dashed" } : undefined}
      // Stable DOM anchor the CrfGlobalSearchPage uses for
      // scrollIntoView when navigating in with ?focus=annotation-<id>.
      data-testid={`crf-annotation-${annotation.id}`}
    />
  );
}

function DisabledAnnotationChip({
  annotation,
  colorIndex,
  onEdit: _onEdit,
  onDelete: _onDelete,
}: Props) {
  const { t } = useI18n();
  // OnDelete is unset so the delete affordance disappears — consistent
  // with `Mui-disabled` blocking the click anyway, and avoids
  // presenting a permanently-disabled delete icon that the user can't
  // use. onEdit is unset for the same reason.
  return (
    <Tooltip
      title={t("crf.detail.tooltip.noPermissionEdit")}
    >
      <span>
        <Chip
          label={annotation.content}
          color={annotationColor(colorIndex)}
          onClick={undefined}
          onDelete={undefined}
          size="small"
          variant="outlined"
          disabled
          sx={annotation.assign ? { borderStyle: "dashed" } : undefined}
          data-testid={`crf-annotation-${annotation.id}`}
        />
      </span>
    </Tooltip>
  );
}