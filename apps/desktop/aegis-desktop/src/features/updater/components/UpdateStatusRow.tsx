import {
  Alert,
  Box,
  Button,
  LinearProgress,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { UpdateStatus } from "../../../shared/api/types";

export interface UpdateStatusRowProps {
  status: UpdateStatus | undefined;
  currentVersion: string;
  lastCheckedAt: string | null;
  lastError: string | null;
  onCheck: () => void;
  onInstall: () => void;
  checking: boolean;
  installing: boolean;
  errorMessage: string | null;
}

/**
 * The quiet, persistent indicator: the current version, what the
 * updater is doing, and the two actions.
 */
export function UpdateStatusRow({
  status,
  currentVersion,
  lastCheckedAt,
  lastError,
  onCheck,
  onInstall,
  checking,
  installing,
  errorMessage,
}: UpdateStatusRowProps) {
  const { t } = useI18n();

  const percent =
    status?.kind === "downloading" && status.total
      ? Math.round((status.downloaded / status.total) * 100)
      : 0;

  // The failure of the last check. A check that failed while a release
  // is held leaves the status at `ready` — the download is still
  // installable — so the failure is shown next to it rather than in
  // place of it.
  const failureMessage =
    status?.kind === "failed" ? status.message : lastError;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      {currentVersion !== "" && (
        <Typography variant="body2">
          {t("updater.status.current", { version: currentVersion })}
        </Typography>
      )}

      {status?.kind === "checking" && (
        <Typography variant="body2">{t("updater.status.checking")}</Typography>
      )}

      {status?.kind === "downloading" && (
        <>
          <Typography variant="body2">
            {t("updater.status.downloading", { percent })}
          </Typography>
          <LinearProgress variant="determinate" value={percent} />
        </>
      )}

      {status?.kind === "ready" && (
        <>
          <Typography variant="body2">
            {t("updater.status.ready", { version: status.version })}
          </Typography>
          <Box>
            <Button variant="contained" onClick={onInstall} disabled={installing}>
              {t("updater.action.install")}
            </Button>
          </Box>
        </>
      )}

      {status?.kind === "idle" && lastCheckedAt !== null && (
        <Typography variant="body2">
          {t("updater.status.upToDate", { version: currentVersion })}
        </Typography>
      )}

      {failureMessage !== null && (
        <Typography variant="body2" color="error">
          {t("updater.status.failed", { message: failureMessage })}
        </Typography>
      )}

      {errorMessage !== null && <Alert severity="error">{errorMessage}</Alert>}

      <Box>
        <Button variant="outlined" onClick={onCheck} disabled={checking}>
          {t("updater.action.check")}
        </Button>
      </Box>
    </Box>
  );
}
