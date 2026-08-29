import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, DomainAnnotation } from "../../../shared/api";

export interface DomainAnnotationDialogBody {
  name: string;
  description: string;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  /**
   * Current `notSubmitted` flag of the form that owns this domain
   * annotation. The dialog hides its `Not submit` action while the
   * form is already marked not-submitted — there's nothing left to
   * do — and the page runs the cascade + form update on click.
   */
  formNotSubmitted: boolean;
  onClose: () => void;
  onSubmit: (body: DomainAnnotationDialogBody) => void;
  /**
   * Trigger the form-level cascade: delete every annotation in
   * the form, then PATCH the form's `notSubmitted` flag to true.
   * Wired by the page to `useUpdateOwnerNotSubmitted` for the
   * `{ kind: "form", id }` owner.
   */
  onMarkNotSubmitted: () => void;
  markNotSubmittedPending: boolean;
  markNotSubmittedError: ApiError | null;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: DomainAnnotationDialogBody = {
  name: "",
  description: "",
};

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  formNotSubmitted,
  onClose,
  onSubmit,
  onMarkNotSubmitted,
  markNotSubmittedPending,
  markNotSubmittedError,
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
      });
    } else {
      setBody({
        name: EMPTY.name,
        description: EMPTY.description,
      });
    }
  }, [open, mode, row]);

  const submitDisabled = mutationPending || body.name.trim() === "";
  // The Not submit action is one-way: only show it when the form
  // is currently submitted (notSubmitted === false). Once the form
  // is marked not-submitted the user can no longer toggle it from
  // this dialog.
  const markVisible = !formNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      name: body.name.trim(),
      description: body.description.trim(),
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
          {(mutationError ?? markNotSubmittedError) && (
            <Alert severity="error">
              {errorMessage(mutationError ?? markNotSubmittedError!)}
            </Alert>
          )}
        </Box>

      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending || markNotSubmittedPending}>
          {t("common.cancel")}
        </Button>
        {markVisible && (
          <Button
            variant="outlined"
            color="warning"
            onClick={onMarkNotSubmitted}
            disabled={markDisabled}
            data-testid="crf-domain-dialog-not-submit"
          >
            {t("crf.domainDialog.notSubmit")}
          </Button>
        )}
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
