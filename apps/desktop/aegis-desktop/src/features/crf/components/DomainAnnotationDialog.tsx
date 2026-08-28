import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  Stack,
  TextField,
  Typography,
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
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t(
            mode === "create"
              ? "crf.domainDialog.create.title"
              : "crf.domainDialog.edit.title",
          )}
        </Typography>
        <Stack spacing={2}>
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
        </Stack>
        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}
        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
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
        </Box>
      </Box>
    </Drawer>
  );
}
