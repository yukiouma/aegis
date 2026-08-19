import { useState } from "react";
import { IconButton, Snackbar, Tooltip } from "@aegis/ui/mui";
import { Add as AddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

/**
 * Placeholder Import button. Opens a "coming soon" snackbar; the
 * real `ImportTerminology` page is a follow-up feature.
 */
export function ImportButton() {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);

  return (
    <>
      <Tooltip title={t("terminology.importComingSoon")}>
        <IconButton
          aria-label={t("terminology.importComingSoon")}
          onClick={() => setOpen(true)}
        >
          <AddIcon />
        </IconButton>
      </Tooltip>
      <Snackbar
        open={open}
        autoHideDuration={3000}
        onClose={() => setOpen(false)}
        message={t("terminology.importComingSoon")}
      />
    </>
  );
}