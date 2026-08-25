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
import type { SdtmDomainView } from "../../../shared/api";

export interface DeleteDomainDialogProps {
  open: boolean;
  row: SdtmDomainView | null;
  onClose: () => void;
  onConfirm: (row: SdtmDomainView) => void;
  pending: boolean;
  error: unknown;
}

export function DeleteDomainDialog({
  open,
  row,
  onClose,
  onConfirm,
  pending,
  error,
}: DeleteDomainDialogProps) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t("domainModel.sdtm.delete.confirmTitle")}</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {t("domainModel.sdtm.delete.confirmMessage")}
        </DialogContentText>
        {error && (
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