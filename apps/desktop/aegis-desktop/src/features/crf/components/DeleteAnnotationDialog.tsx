import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { Annotation, ApiError } from "../../../shared/api";

interface Props {
  open: boolean;
  row: Annotation | null;
  onClose: () => void;
  onConfirm: (row: Annotation) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

export function DeleteAnnotationDialog({
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
      <DialogTitle>{t("crf.deleteAnnotation.title")}</DialogTitle>
      <DialogContent>
        {row && (
          <>
            <Alert severity="warning" sx={{ mb: 1 }}>
              {t("crf.deleteAnnotation.message")}
            </Alert>
            <Typography variant="body2" sx={{ color: "text.secondary" }}>
              {row.content}
            </Typography>
          </>
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
          {t("crf.deleteAnnotation.submit")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
