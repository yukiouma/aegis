import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  CreateDomainAnnotationInput,
  DomainAnnotation,
  UpdateDomainAnnotationInput,
} from "../../../shared/api";

type SubmitBody = CreateDomainAnnotationInput | UpdateDomainAnnotationInput;

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  onClose: () => void;
  onSubmit: (body: SubmitBody) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: SubmitBody = { name: "", description: "" };

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  onClose,
  onSubmit,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setName(row.name);
      setDescription(row.description);
    } else {
      setName(EMPTY.name as string);
      setDescription(EMPTY.description as string);
    }
  }, [open, mode, row]);

  const submitDisabled = mutationPending || name.trim() === "";

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({ name: name.trim(), description: description.trim() });
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
      <DialogContent
        sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
      >
        <TextField
          size="small"
          label={t("crf.domainDialog.field.name")}
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <TextField
          size="small"
          label={t("crf.domainDialog.field.description")}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          multiline
          minRows={2}
        />
        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}
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
