import { useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  List,
  ListItem,
  MenuItem,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import type { TranslationKey } from "@aegis/ui/i18n";

import { useListTemplateStructures } from "../../templates";
import { useInitializeProjectFolder } from "../data/project-folder";
import { errorMessage, toApiError } from "../../../shared/api/error";
import type {
  ApiError,
  InitReport,
  ProjectBranchView,
  SkipReason,
} from "../../../shared/api";

/**
 * The translation key for each skip reason.
 *
 * A map rather than a template literal: `TranslationKey` is a union of
 * the catalogue's keys, so a computed key would not typecheck — and the
 * map makes a missing translation a compile error rather than a blank
 * label at runtime.
 */
const SKIP_REASON_KEYS: Record<SkipReason, TranslationKey> = {
  alreadyExists: "scaffold.initialize.report.reason.alreadyExists",
  unresolvedField:
    "scaffold.initialize.report.reason.unresolvedField",
  malformedTemplate:
    "scaffold.initialize.report.reason.malformedTemplate",
  invalidName: "scaffold.initialize.report.reason.invalidName",
  noReference: "scaffold.initialize.report.reason.noReference",
  referenceFailed:
    "scaffold.initialize.report.reason.referenceFailed",
  migrationFailed:
    "scaffold.initialize.report.reason.migrationFailed",
};

export interface InitializeProjectFolderDialogProps {
  open: boolean;
  /** The project code — the context's `study`. */
  projectCode: string;
  /** The branch the folder is being initialized for, when the dialog
   *  was opened from a branch row. */
  branch?: ProjectBranchView;
  /**
   * Awaited after the user confirms a template and before the folder
   * is materialized. Rejecting aborts the whole operation — nothing
   * is written to disk — and closes the dialog, so the caller can
   * show the error where the user can act on it.
   */
  beforeInit?: () => Promise<void>;
  /**
   * Called when the folder could not be materialized. When omitted,
   * the dialog keeps itself open and renders the error inline, so
   * the user can retry.
   */
  onInitError?: (error: ApiError) => void;
  onClose: () => void;
  /** Called with the report when the folder has been generated. */
  onInitialized: (report: InitReport) => void;
}

/**
 * "Initialize project folder" dialog.
 *
 * Picking a template runs the `initialize_project_folder` command,
 * which materialises the structure on the desktop machine's disk. The
 * command's report — what was created, what was skipped and why —
 * stays in the dialog, because a skipped node is a normal outcome and
 * the user needs to see the reason.
 */
export function InitializeProjectFolderDialog({
  open,
  projectCode,
  branch,
  beforeInit,
  onInitError,
  onClose,
  onInitialized,
}: InitializeProjectFolderDialogProps) {
  const { t } = useI18n();
  const structures = useListTemplateStructures();
  const initialize = useInitializeProjectFolder();
  const [structureId, setStructureId] = useState<number | null>(null);
  const [report, setReport] = useState<InitReport | null>(null);
  const [preparing, setPreparing] = useState(false);
  // A ref, not state: two clicks in the same tick would both read a
  // stale `preparing`, and `beforeInit` must run exactly once.
  const confirming = useRef(false);

  const selected = structures.data?.find((s) => s.id === structureId);

  async function handleConfirm() {
    if (!selected || confirming.current) return;
    confirming.current = true;
    setPreparing(true);
    try {
      if (beforeInit) {
        try {
          await beforeInit();
        } catch {
          // The caller's own mutation state holds the error. The init
          // command never runs, so nothing is written to disk.
          onClose();
          return;
        }
      }
      const result = await initialize.mutateAsync({
        structureId: selected.id,
        study: projectCode,
        branch: branch?.branch ?? null,
      });
      setReport(result);
      onInitialized(result);
    } catch (e) {
      if (onInitError) {
        onInitError(toApiError(e));
      }
      // Otherwise the inline Alert renders initialize.error, as before.
    } finally {
      confirming.current = false;
      setPreparing(false);
    }
  }

  function handleClose() {
    setReport(null);
    setStructureId(null);
    initialize.reset();
    onClose();
  }

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="lg" fullWidth>
      <DialogTitle>{t("scaffold.initialize.title")}</DialogTitle>
      <DialogContent>
        {branch && (
          <Typography variant="body2" color="primary" sx={{ mb: 2 }}>
            {branch.branch}
          </Typography>
        )}

        {report ? (
          <Stack spacing={2} data-testid="initialize-report">
            <Box sx={{ display: "flex", gap: 2 }}>
              <Typography
                variant="body2"
                data-testid="initialize-report-created"
              >
                {t("scaffold.initialize.report.created", {
                  count: report.created.length,
                })}
              </Typography>
              <Typography
                variant="body2"
                data-testid="initialize-report-skipped"
              >
                {t("scaffold.initialize.report.skipped", {
                  count: report.skipped.length,
                })}
              </Typography>
            </Box>
            {report.skipped.length > 0 && (
              <List dense data-testid="initialize-report-skipped-list">
                {report.skipped.map((item, index) => (
                  <ListItem key={index}>
                    <Typography variant="body2" color="primary">
                      {item.path}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      : {t(SKIP_REASON_KEYS[item.reason], {
                        detail: item.detail ?? "",
                      })}
                    </Typography>
                  </ListItem>
                ))}
              </List>
            )}
          </Stack>
        ) : (
          <>
            <TextField
              select
              fullWidth
              size="small"
              label={t("scaffold.initialize.structure")}
              value={structureId ?? ""}
              onChange={(e) => {
                const raw = e.target.value;
                setStructureId(raw === "" ? null : Number(raw));
              }}
              helperText={
                structureId === null
                  ? t("scaffold.initialize.required")
                  : undefined
              }
              data-testid="initialize-folder-structure"
            >
              {(structures.data ?? []).map((s) => (
                <MenuItem key={s.id} value={s.id}>
                  {s.name}
                </MenuItem>
              ))}
            </TextField>
            {initialize.isError && !onInitError && (
              <Alert severity="error" sx={{ mt: 2 }}>
                {t("scaffold.initialize.failed", {
                  message: errorMessage(initialize.error),
                })}
              </Alert>
            )}
          </>
        )}
      </DialogContent>
      <DialogActions>
        {report ? (
          <Button onClick={handleClose}>{t("common.close")}</Button>
        ) : (
          <>
            <Button
              onClick={handleClose}
              disabled={preparing || initialize.isPending}
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="contained"
              disabled={!selected || preparing || initialize.isPending}
              onClick={() => void handleConfirm()}
              data-testid="initialize-folder-confirm"
            >
              {t("common.confirm")}
            </Button>
          </>
        )}
      </DialogActions>
    </Dialog>
  );
}
