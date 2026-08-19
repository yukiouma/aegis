import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  CodeListView,
  CreateCodeListInput,
  TerminologyVersionView,
  UpdateCodeListInput,
} from "../../../shared/api";

export interface CodeListDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  row?: CodeListView;
  versions: TerminologyVersionView[];
  versionId: number;
  onClose: () => void;
  onCreate: (input: CreateCodeListInput) => void;
  onUpdate: (id: number, body: UpdateCodeListInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY_FIELDS = {
  code: "",
  extensible: false,
  name: "",
  submissionValue: "",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
};

export function CodeListDrawer({
  open,
  mode,
  row,
  versions,
  versionId,
  onClose,
  onCreate,
  onUpdate,
  canMutate,
  mutationError,
  mutationPending,
}: CodeListDrawerProps) {
  const { t } = useI18n();
  const [code, setCode] = useState(EMPTY_FIELDS.code);
  const [extensible, setExtensible] = useState(EMPTY_FIELDS.extensible);
  const [name, setName] = useState(EMPTY_FIELDS.name);
  const [submissionValue, setSubmissionValue] = useState(
    EMPTY_FIELDS.submissionValue,
  );
  const [synonym, setSynonym] = useState(EMPTY_FIELDS.synonym);
  const [definition, setDefinition] = useState(EMPTY_FIELDS.definition);
  const [nciPreferredTerm, setNciPreferredTerm] = useState(
    EMPTY_FIELDS.nciPreferredTerm,
  );
  // `selectedVersionId` only matters in create mode; in edit mode
  // the version is locked to the row's own version.
  const [selectedVersionId, setSelectedVersionId] = useState<number>(
    versionId,
  );

  useEffect(() => {
    if (mode === "edit" && row) {
      setCode(row.code);
      setExtensible(row.extensible);
      setName(row.name);
      setSubmissionValue(row.submissionValue);
      setSynonym(row.synonym);
      setDefinition(row.definition);
      setNciPreferredTerm(row.nciPreferredTerm);
      setSelectedVersionId(row.versionId);
    } else if (mode === "create") {
      setCode("");
      setExtensible(false);
      setName("");
      setSubmissionValue("");
      setSynonym("");
      setDefinition("");
      setNciPreferredTerm("");
      setSelectedVersionId(versionId);
    }
  }, [mode, row, versionId, open]);

  const title =
    mode === "create"
      ? t("terminology.codelist.create.title")
      : t("terminology.codelist.edit.title");

  const submitDisabled = !canMutate || code.trim() === "" || mutationPending;

  function handleSubmit() {
    if (!canMutate) return;
    if (code.trim() === "") return;
    if (mode === "create") {
      onCreate({
        versionId: selectedVersionId,
        code: code.trim(),
        extensible,
        name,
        submissionValue,
        synonym,
        definition,
        nciPreferredTerm,
      });
    } else if (row) {
      onUpdate(row.id, {
        code: code.trim(),
        extensible,
        name,
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
          <Alert severity="info">{t("terminology.codelist.readOnly")}</Alert>
        )}

        {mode === "create" && (
          <FormControl size="small" fullWidth>
            <InputLabel id="version-select-label">
              {t("terminology.version.helper")}
            </InputLabel>
            <Select
              labelId="version-select-label"
              value={selectedVersionId}
              label={t("terminology.version.helper")}
              onChange={(e) => setSelectedVersionId(Number(e.target.value))}
              disabled={!canMutate}
            >
              {versions.map((v) => (
                <MenuItem key={v.id} value={v.id}>
                  {`${v.kind.toUpperCase()} — ${v.name}`}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        )}

        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("terminology.codelist.field.code")}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            disabled={!canMutate}
            required
          />
          <FormControlLabel
            control={
              <Switch
                size="small"
                checked={extensible}
                onChange={(e) => setExtensible(e.target.checked)}
                disabled={!canMutate}
              />
            }
            label={t("terminology.codelist.field.extensible")}
          />
          <TextField
            size="small"
            label={t("terminology.codelist.field.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={!canMutate}
          />
          <TextField
            size="small"
            label={t("terminology.codelist.field.submissionValue")}
            value={submissionValue}
            onChange={(e) => setSubmissionValue(e.target.value)}
            disabled={!canMutate}
          />
          <TextField
            size="small"
            label={t("terminology.codelist.field.synonym")}
            value={synonym}
            onChange={(e) => setSynonym(e.target.value)}
            disabled={!canMutate}
            multiline
            minRows={2}
          />
          <TextField
            size="small"
            label={t("terminology.codelist.field.definition")}
            value={definition}
            onChange={(e) => setDefinition(e.target.value)}
            disabled={!canMutate}
            multiline
            minRows={3}
          />
          <TextField
            size="small"
            label={t("terminology.codelist.field.nciPreferredTerm")}
            value={nciPreferredTerm}
            onChange={(e) => setNciPreferredTerm(e.target.value)}
            disabled={!canMutate}
          />
        </Stack>

        {mutationError && (
          <Alert severity="error">
            {t("terminology.codelist.loadFailed", {
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
              ? t("terminology.codelist.action.create")
              : t("terminology.codelist.action.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}