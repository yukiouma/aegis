import { useState } from "react";
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
import { useInitializeProjectFolder } from "../data/project-branches";
import { errorMessage } from "../../../shared/api/error";
import type {
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
  alreadyExists: "project.branches.initialize.report.reason.alreadyExists",
  unresolvedField:
    "project.branches.initialize.report.reason.unresolvedField",
  malformedTemplate:
    "project.branches.initialize.report.reason.malformedTemplate",
  invalidName: "project.branches.initialize.report.reason.invalidName",
  noReference: "project.branches.initialize.report.reason.noReference",
  referenceFailed:
    "project.branches.initialize.report.reason.referenceFailed",
  migrationFailed:
    "project.branches.initialize.report.reason.migrationFailed",
};

export interface InitializeProjectFolderDialogProps {
  open: boolean;
  /** The project code — the context's `study`. */
  projectCode: string;
  /** The branch the folder is being initialized for, when the dialog
   *  was opened from a branch row. */
  branch?: ProjectBranchView;
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
  onClose,
  onInitialized,
}: InitializeProjectFolderDialogProps) {
  const { t } = useI18n();
  const structures = useListTemplateStructures();
  const initialize = useInitializeProjectFolder();
  const [structureId, setStructureId] = useState<number | null>(null);
  const [report, setReport] = useState<InitReport | null>(null);

  const selected = structures.data?.find((s) => s.id === structureId);

  async function handleConfirm() {
    if (!selected) return;
    try {
      const result = await initialize.mutateAsync({
        structureId: selected.id,
        study: projectCode,
        branch: branch?.branch ?? null,
      });
      setReport(result);
      onInitialized(result);
    } catch {
      // The error is rendered inside the dialog, which stays open.
    }
  }

  function handleClose() {
    setReport(null);
    setStructureId(null);
    initialize.reset();
    onClose();
  }

  return (
    <Dialog open={open} onClose={handleClose}>
      <DialogTitle>{t("project.branches.initialize.title")}</DialogTitle>
      <DialogContent>
        {branch && (
          <Typography variant="body2" sx={{ mb: 2 }}>
            {branch.branch}
          </Typography>
        )}

        {report ? (
          <Stack spacing={2} data-testid="initialize-report">
            <Typography variant="body2" data-testid="initialize-report-root">
              {report.root}
            </Typography>
            <Box sx={{ display: "flex", gap: 2 }}>
              <Typography
                variant="body2"
                data-testid="initialize-report-created"
              >
                {t("project.branches.initialize.report.created", {
                  count: report.created.length,
                })}
              </Typography>
              <Typography
                variant="body2"
                data-testid="initialize-report-skipped"
              >
                {t("project.branches.initialize.report.skipped", {
                  count: report.skipped.length,
                })}
              </Typography>
            </Box>
            {report.skipped.length > 0 && (
              <List dense data-testid="initialize-report-skipped-list">
                {report.skipped.map((item, index) => (
                  <ListItem key={index}>
                    <Typography variant="body2">{item.path}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {t(SKIP_REASON_KEYS[item.reason], {
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
              label={t("project.branches.initialize.structure")}
              value={structureId ?? ""}
              onChange={(e) => {
                const raw = e.target.value;
                setStructureId(raw === "" ? null : Number(raw));
              }}
              helperText={
                structureId === null
                  ? t("project.branches.initialize.required")
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
            {initialize.isError && (
              <Alert severity="error" sx={{ mt: 2 }}>
                {t("project.branches.initialize.failed", {
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
            <Button onClick={handleClose} disabled={initialize.isPending}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="contained"
              disabled={!selected || initialize.isPending}
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
