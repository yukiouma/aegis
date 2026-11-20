import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { useInstallUpdate, useUpdateStatusEvent } from "../data/update-status";
import { useUpdatePrompt } from "../data/update-prompt";

/**
 * The "a new version is ready" prompt.
 *
 * It mounts in the authenticated shell of the main window, so it never
 * covers the login form and never appears in a workspace window.
 */
export function UpdateReadyDialog() {
  const { t } = useI18n();
  const { ready, shouldPrompt, defer } = useUpdatePrompt();
  const install = useInstallUpdate();
  // One listener for the whole window: the status query it invalidates
  // is shared with the Settings row.
  useUpdateStatusEvent();

  const version = ready?.version ?? "";
  const handleLater = () => {
    void defer();
  };

  return (
    <Dialog
      open={shouldPrompt}
      onClose={handleLater}
      aria-label={t("updater.title", { version })}
    >
      <DialogTitle>{t("updater.title", { version })}</DialogTitle>
      <DialogContent>
        <DialogContentText>{t("updater.message")}</DialogContentText>
        {install.isError && (
          <Alert severity="error" sx={{ mt: 2 }}>
            {t("updater.error.installFailed", {
              message: errorMessage(install.error),
            })}
          </Alert>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={handleLater} disabled={install.isPending}>
          {t("updater.action.later")}
        </Button>
        <Button
          onClick={() => install.mutate()}
          variant="contained"
          disabled={install.isPending}
        >
          {t("updater.action.install")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
