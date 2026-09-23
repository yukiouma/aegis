import { Chip } from "@aegis/ui/mui";
import {
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { CrfForm } from "../../../shared/api";

/**
 * Aggregate approval chip for a CRF version. Derives its label
 * and color from the supplied forms:
 *   - empty list          → "Pending" (warning)
 *   - all approved        → "Approved" (success)
 *   - mixed / any pending → "Pending" (warning)
 *
 * The list page passes the unfiltered `allRows` so the chip
 * reflects the version, not the user's filter.
 */
export function CrfStatusChip({ forms }: { forms: CrfForm[] }) {
  const { t } = useI18n();
  const allApproved = forms.length > 0 && forms.every((f) => f.approved);
  if (allApproved) {
    return (
      <Chip
        icon={<VerifiedIcon />}
        label={t("crf.toolbar.statusApproved")}
        color="success"
        variant="outlined"
        size="small"
      />
    );
  }
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