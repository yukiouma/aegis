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
  CodeItemView,
  CreateCodeItemInput,
  UpdateCodeItemInput,
} from "../../../shared/api";

export interface CodeItemDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  row?: CodeItemView;
  codelistId: number;
  versionId: number;
  onClose: () => void;
  onCreate: (input: CreateCodeItemInput) => void;
  onUpdate: (id: number, body: UpdateCodeItemInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY_FIELDS = {
  code: "",
  submissionValue: "",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
};

export function CodeItemDrawer({
  open,
  mode,
  row,
  codelistId,
  versionId,
  onClose,
  onCreate,
  onUpdate,
  canMutate,
  mutationError,
  mutationPending,
}: CodeItemDrawerProps) {
  const { t } = useI18n();
  const [code, setCode] = useState(EMPTY_FIELDS.code);
  const [submissionValue, setSubmissionValue] = useState(
    EMPTY_FIELDS.submissionValue,
  );
  const [synonym, setSynonym] = useState(EMPTY_FIELDS.synonym);
  const [definition, setDefinition] = useState(EMPTY_FIELDS.definition);
  const [nciPreferredTerm, setNciPreferredTerm] = useState(
    EMPTY_FIELDS.nciPreferredTerm,
  );

  useEffect(() => {
    if (mode === "edit" && row) {
      setCode(row.code);
      setSubmissionValue(row.submissionValue);
      setSynonym(row.synonym);
      setDefinition(row.definition);
      setNciPreferredTerm(row.nciPreferredTerm);
    } else if (mode === "create") {
      setCode("");
      setSubmissionValue("");
      setSynonym("");
      setDefinition("");
      setNciPreferredTerm("");
    }
  }, [mode, row, open]);

  const title =
    mode === "create"
      ? t("terminology.codeitem.create.title")
      : t("terminology.codeitem.edit.title");

  const submitDisabled = !canMutate || code.trim() === "" || mutationPending;

  function handleSubmit() {
    if (!canMutate) return;
    if (code.trim() === "") return;
    if (mode === "create") {
      onCreate({
        codelistId,
        versionId,
        code: code.trim(),
        submissionValue,
        synonym,
        definition,
        nciPreferredTerm,
      });
    } else if (row) {
      onUpdate(row.id, {
        code: code.trim(),
        submissionValue,
        synonym,
        definition,
        nciPreferredTerm,
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
        <Typography variant="h6">{title}</Typography>
        {!canMutate && (
          <Alert severity="info">{t("terminology.codeitem.readOnly")}</Alert>
        )}

        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("terminology.codeitem.field.code")}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            disabled={!canMutate}
            required
          />
          <TextField
            size="small"
            label={t("terminology.codeitem.field.submissionValue")}
            value={submissionValue}
            onChange={(e) => setSubmissionValue(e.target.value)}
            disabled={!canMutate}
          />
          <TextField
            size="small"
            label={t("terminology.codeitem.field.synonym")}
            value={synonym}
            onChange={(e) => setSynonym(e.target.value)}
            disabled={!canMutate}
            multiline
            minRows={2}
          />
          <TextField
            size="small"
            label={t("terminology.codeitem.field.definition")}
            value={definition}
            onChange={(e) => setDefinition(e.target.value)}
            disabled={!canMutate}
            multiline
            minRows={3}
          />
          <TextField
            size="small"
            label={t("terminology.codeitem.field.nciPreferredTerm")}
            value={nciPreferredTerm}
            onChange={(e) => setNciPreferredTerm(e.target.value)}
            disabled={!canMutate}
          />
        </Stack>

        {mutationError && (
          <Alert severity="error">
            {t("terminology.codeitem.loadFailed", {
              message: errorMessage(mutationError),
            })}
          </Alert>
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
            {mode === "create"
              ? t("terminology.codeitem.action.create")
              : t("terminology.codeitem.action.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}