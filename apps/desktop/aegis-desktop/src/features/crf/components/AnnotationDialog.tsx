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
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  Annotation,
  AnnotationOwner,
  ApiError,
  DomainAnnotation,
} from "../../../shared/api";

export interface AnnotationDialogBody {
  domainAnnotationId: number;
  content: string;
  assign: boolean;
  notSubmitted: boolean;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  owner: AnnotationOwner;
  /**
   * The current `notSubmitted` flag of the annotation's owner
   * (form / item / option / unit). The dialog seeds its
   * `Not submitted` checkbox from this so the user can see
   * the current state and toggle it; the page runs the
   * cascade-delete + owner update when the new value differs.
   */
  ownerNotSubmitted: boolean;
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
  notSubmitted: false,
};

export function AnnotationDialog({
  open,
  mode,
  owner: _owner,
  ownerNotSubmitted,
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
        notSubmitted: ownerNotSubmitted,
      });
    } else {
      setBody({
        domainAnnotationId: availableDomainAnnotations[0]?.id ?? 0,
        content: "",
        assign: false,
        notSubmitted: ownerNotSubmitted,
      });
    }
  }, [open, mode, row, availableDomainAnnotations, ownerNotSubmitted]);

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
            ? "crf.annotationDialog.create.title"
            : "crf.annotationDialog.edit.title",
        )}
      </DialogTitle>
      <DialogContent>
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
        >
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
          <FormControlLabel
            control={
              <Checkbox
                checked={body.notSubmitted}
                onChange={(e) =>
                  setBody((b) => ({ ...b, notSubmitted: e.target.checked }))
                }
              />
            }
            label={t("crf.annotationDialog.field.notSubmitted")}
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
              ? "crf.annotationDialog.submit.create"
              : "crf.annotationDialog.submit.save",
          )}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
