import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { SdtmVariableView } from "../../../shared/api";

export interface DeleteVariableDialogProps {
  open: boolean;
  row: SdtmVariableView | null;
  onClose: () => void;
  onConfirm: (row: SdtmVariableView) => void;
  pending: boolean;
  error: unknown;
}

export function DeleteVariableDialog({
  open,
  row,
  onClose,
  onConfirm,
  pending,
  error,
}: DeleteVariableDialogProps) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>
        {t("domainModel.sdtm.variable.delete.confirmTitle")}
      </DialogTitle>
      <DialogContent>
        <DialogContentText>
          {t("domainModel.sdtm.variable.delete.confirmMessage")}
        </DialogContentText>
        {Boolean(error) && (
          <DialogContentText sx={{ mt: 2, color: "error.main" }}>
            {errorMessage(error)}
          </DialogContentText>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={pending}>
          {t("common.cancel")}
        </Button>
        <Button
          color="error"
          onClick={() => row && onConfirm(row)}
          disabled={pending || !row}
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}