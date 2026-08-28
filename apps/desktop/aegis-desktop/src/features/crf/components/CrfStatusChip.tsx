import { Chip } from "@aegis/ui/mui";
import { PendingActions as PendingActionsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

/**
 * Placeholder status chip. The status API is not ready; renders
 * a literal "Pending" label with a pending-actions glyph.
 */
export function CrfStatusChip() {
  const { t } = useI18n();
  return (
    <Chip
      icon={<PendingActionsIcon />}
      label={t("crf.toolbar.statusPending")}
      color="warning"
      variant="outlined"
      size="small"
    />
  );
}