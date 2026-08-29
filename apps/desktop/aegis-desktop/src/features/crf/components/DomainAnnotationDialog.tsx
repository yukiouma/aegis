import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, DomainAnnotation } from "../../../shared/api";

export interface DomainAnnotationDialogBody {
  name: string;
  description: string;
  notSubmitted: boolean;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  /**
   * Current `notSubmitted` flag of the form that owns this domain
   * annotation. Seeds the dialog's `Not submitted` checkbox so the
   * user can see the current state and toggle it; the page runs the
   * cascade-delete + form update when the new value differs.
   */
  formNotSubmitted: boolean;
  onClose: () => void;
  onSubmit: (body: DomainAnnotationDialogBody) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: DomainAnnotationDialogBody = {
  name: "",
  description: "",
  notSubmitted: false,
};

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  formNotSubmitted,
  onClose,
  onSubmit,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<DomainAnnotationDialogBody>(EMPTY);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        name: row.name,
        description: row.description,
        notSubmitted: formNotSubmitted,
      });
    } else {
      setBody({
        name: EMPTY.name,
        description: EMPTY.description,
        notSubmitted: formNotSubmitted,
      });
    }
  }, [open, mode, row, formNotSubmitted]);

  const submitDisabled = mutationPending || body.name.trim() === "";

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      name: body.name.trim(),
      description: body.description.trim(),
      notSubmitted: body.notSubmitted,
    });
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
    >
      <DialogTitle>
        {t(
          mode === "create"
            ? "crf.domainDialog.create.title"
            : "crf.domainDialog.edit.title",
        )}
      </DialogTitle>
      <DialogContent>
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
        >
          <TextField
            size="small"
            label={t("crf.domainDialog.field.name")}
            value={body.name}
            onChange={(e) =>
              setBody((b) => ({ ...b, name: e.target.value }))
            }
          />
          <TextField
            size="small"
            label={t("crf.domainDialog.field.description")}
            value={body.description}
            onChange={(e) =>
              setBody((b) => ({ ...b, description: e.target.value }))
            }
          />
          <FormControlLabel
            control={
              <Checkbox
                checked={body.notSubmitted}
                onChange={(e) =>
                  setBody((b) => ({
                    ...b,
                    notSubmitted: e.target.checked,
                  }))
                }
              />
            }
            label={t("crf.domainDialog.field.notSubmitted")}
          />
          {mutationError && (
            <Alert severity="error">{errorMessage(mutationError)}</Alert>
          )}
        </Box>

      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="contained"
          onClick={handleSubmit}
          disabled={submitDisabled}
        >
          {t(
            mode === "create"
              ? "crf.domainDialog.submit.create"
              : "crf.domainDialog.submit.save",
          )}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
