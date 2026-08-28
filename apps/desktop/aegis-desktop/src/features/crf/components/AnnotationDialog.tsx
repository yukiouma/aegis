import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Drawer,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  Annotation,
  AnnotationOwner,
  ApiError,
  CreateAnnotationInput,
  DomainAnnotation,
  UpdateAnnotationInput,
} from "../../../shared/api";

export interface AnnotationDialogBody {
  domainAnnotationId: number;
  content: string;
  assign: boolean;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  owner: AnnotationOwner;
  row?: Annotation;
  availableDomainAnnotations: DomainAnnotation[];
  onClose: () => void;
  /**
   * Called with the dialog body. The page composes the full
   * CreateAnnotationInput by merging the owner at the call site.
   */
  onSubmit: (body: AnnotationDialogBody) => void;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const EMPTY: AnnotationDialogBody = {
  domainAnnotationId: 0,
  content: "",
  assign: false,
};

export function AnnotationDialog({
  open,
  mode,
  owner: _owner,
  row,
  availableDomainAnnotations,
  onClose,
  onSubmit,
  mutationError,
  mutationPending,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<AnnotationDialogBody>(EMPTY);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        domainAnnotationId: row.domainAnnotationId,
        content: row.content,
        assign: row.assign,
      });
    } else {
      setBody({
        domainAnnotationId: availableDomainAnnotations[0]?.id ?? 0,
        content: "",
        assign: false,
      });
    }
  }, [open, mode, row, availableDomainAnnotations]);

  const submitDisabled =
    mutationPending ||
    body.content.trim() === "" ||
    body.domainAnnotationId === 0;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
    });
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
              ? "crf.annotationDialog.create.title"
              : "crf.annotationDialog.edit.title",
          )}
        </Typography>
        <Stack spacing={2}>
          <FormControl size="small" disabled={mode === "edit"}>
            <InputLabel id="annotation-domain-annotation-label">
              {t("crf.annotationDialog.field.domainAnnotation")}
            </InputLabel>
            <Select
              labelId="annotation-domain-annotation-label"
              label={t("crf.annotationDialog.field.domainAnnotation")}
              value={body.domainAnnotationId || ""}
              onChange={(e) =>
                setBody((b) => ({
                  ...b,
                  domainAnnotationId: Number(e.target.value) || 0,
                }))
              }
              required
            >
              {availableDomainAnnotations.length === 0 && (
                <MenuItem value="" disabled>
                  {t("crf.annotationDialog.domainAnnotation.none")}
                </MenuItem>
              )}
              {availableDomainAnnotations.map((d) => (
                <MenuItem key={d.id} value={d.id}>
                  {d.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <TextField
            size="small"
            label={t("crf.annotationDialog.field.content")}
            value={body.content}
            onChange={(e) =>
              setBody((b) => ({ ...b, content: e.target.value }))
            }
            multiline
            minRows={3}
            required
          />
          <FormControlLabel
            control={
              <Checkbox
                checked={body.assign}
                onChange={(e) =>
                  setBody((b) => ({ ...b, assign: e.target.checked }))
                }
              />
            }
            label={t("crf.annotationDialog.field.assign")}
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
                ? "crf.annotationDialog.submit.create"
                : "crf.annotationDialog.submit.save",
            )}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}
