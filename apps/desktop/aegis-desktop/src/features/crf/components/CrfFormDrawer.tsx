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
  CrfForm,
  CreateCrfFormInput,
  UpdateCrfFormInput,
} from "../../../shared/api";

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: CrfForm;
  onClose: () => void;
  onCreate: (input: CreateCrfFormInput) => void;
  onUpdate: (id: number, input: UpdateCrfFormInput) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY = { code: "", name: "" };

/**
 * Right-anchored drawer for create / edit. Mode = "create" posts
 * a fresh form; mode = "edit" patches the existing row's code +
 * name (other fields are deferred this PR).
 */
export function CrfFormDrawer({
  open,
  mode,
  row,
  onClose,
  onCreate,
  onUpdate,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [code, setCode] = useState(EMPTY.code);
  const [name, setName] = useState(EMPTY.name);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setCode(row.code);
      setName(row.name);
    } else {
      setCode(EMPTY.code);
      setName(EMPTY.name);
    }
  }, [open, mode, row]);

  const submitDisabled =
    mutationPending || code.trim() === "" || name.trim() === "";

  function handleSubmit() {
    if (submitDisabled) return;
    if (mode === "edit" && row) {
      onUpdate(row.id, { code: code.trim(), name: name.trim() });
    } else {
      onCreate({
        code: code.trim(),
        name: name.trim(),
        order: 0,
        notSubmitted: false,
      });
    }
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
              ? "crf.drawer.create.title"
              : "crf.drawer.edit.title",
          )}
        </Typography>
        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("crf.drawer.field.code")}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            required
            slotProps={{ htmlInput: { maxLength: 64 } }}
          />
          <TextField
            size="small"
            label={t("crf.drawer.field.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
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
                ? "crf.drawer.submit.create"
                : "crf.drawer.submit.save",
            )}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}