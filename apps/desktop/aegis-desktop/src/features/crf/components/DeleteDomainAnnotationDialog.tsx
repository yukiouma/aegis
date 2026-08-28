import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, DomainAnnotation } from "../../../shared/api";

interface Props {
  open: boolean;
  row: DomainAnnotation | null;
  onClose: () => void;
  onConfirm: (row: DomainAnnotation) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

export function DeleteDomainAnnotationDialog({
  open,
  row,
  onClose,
  onConfirm,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>{t("crf.deleteDomain.title")}</DialogTitle>
      <DialogContent>
        {row && (
          <Alert severity="warning">
            {t("crf.deleteDomain.message", { name: row.name })}
          </Alert>
        )}
        {mutationError && (
          <Alert severity="error" sx={{ mt: 2 }}>
            {errorMessage(mutationError)}
          </Alert>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="contained"
          color="error"
          disabled={mutationPending || !row}
          onClick={() => row && onConfirm(row)}
        >
          {t("crf.deleteDomain.submit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
