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
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  owner: AnnotationOwner;
  /**
   * Current `notSubmitted` flag of the annotation's owner
   * (form / item / option / unit). The dialog hides its
   * `Not submit` action while the owner is already marked
   * not-submitted — there's nothing left to do — and the
   * page runs the cascade + owner update on click.
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
  /**
   * Trigger the owner-level cascade: delete every annotation
   * attached to the owner (form / item / option / unit),
   * then PATCH the owner's `notSubmitted` flag to true.
   * Wired by the page to `useUpdateOwnerNotSubmitted`.
   */
  onMarkNotSubmitted: () => void;
  markNotSubmittedPending: boolean;
  markNotSubmittedError: ApiError | null;
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
  ownerNotSubmitted,
  row,
  availableDomainAnnotations,
  onClose,
  onSubmit,
  onMarkNotSubmitted,
  markNotSubmittedPending,
  markNotSubmittedError,
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
  // The Not submit action is one-way: only show it when the
  // owner is currently submitted (notSubmitted === false).
  const markVisible = !ownerNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
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
            data-testid="crf-annotation-dialog-not-submit"
          >
            {t("crf.annotationDialog.notSubmit")}
          </Button>
        )}
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
