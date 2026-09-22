import { useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent } from "react";
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
  MenuList,
  Popover,
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
  SdtmDomainView,
} from "../../../shared/api";
import { useListSdtmVariables } from "../../domain-model/data";

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
  /**
   * SDTM domains for the project's resolved SDTMIG version. The
   * dialog matches the currently-selected domain annotation's
   * `name` against this list (case-insensitive) to decide which
   * SDTM domain's variables to offer in the `@`-mention
   * dropdown. Empty array disables the dropdown entirely.
   */
  sdtmDomains: SdtmDomainView[];
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
  sdtmDomains,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<AnnotationDialogBody>(EMPTY);

  // --- @-mention state ---
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const [mentionRange, setMentionRange] =
    useState<{ start: number; end: number } | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

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
    // Reseeding also wipes the active mention.
    setAnchorEl(null);
    setMentionRange(null);
  }, [open, mode, row, availableDomainAnnotations]);

  const submitDisabled =
    mutationPending ||
    body.content.trim() === "" ||
    body.domainAnnotationId === 0;
  // The Not submit action is one-way and only meaningful when
  // creating a fresh annotation. Hide it in edit mode — the user is
  // editing an existing row, not deciding whether the owner needs a
  // flag — and hide it once the owner is already not-submitted.
  const markVisible = mode === "create" && !ownerNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  // --- SDTM domain lookup for the current domain annotation ---
  const selectedDomainName = useMemo(() => {
    const da = availableDomainAnnotations.find(
      (d) => d.id === body.domainAnnotationId,
    );
    return da?.name?.toUpperCase() ?? null;
  }, [availableDomainAnnotations, body.domainAnnotationId]);
  const selectedDomain = useMemo(
    () =>
      sdtmDomains.find(
        (d) => d.name.toUpperCase() === selectedDomainName,
      ) ?? null,
    [sdtmDomains, selectedDomainName],
  );
  const variablesQuery = useListSdtmVariables(
    open && selectedDomain ? selectedDomain.id : null,
  );

  const fragment = mentionRange
    ? body.content.slice(mentionRange.start + 1, mentionRange.end)
    : "";
  const filteredVariables = useMemo(() => {
    const all = variablesQuery.data ?? [];
    const q = fragment.toUpperCase();
    if (!q) return all;
    return all.filter((v) => v.name.toUpperCase().startsWith(q));
  }, [variablesQuery.data, fragment]);

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
    });
  }

  // --- @-mention detection ---
  function handleContentChange(e: ChangeEvent<HTMLInputElement>) {
    const value = e.target.value;
    const caret = e.target.selectionStart ?? value.length;
    setBody((b) => ({ ...b, content: value }));

    // Walk backwards from caret to the previous whitespace.
    const before = value.slice(0, caret);
    const lastWs = Math.max(
      before.lastIndexOf(" "),
      before.lastIndexOf("\n"),
      before.lastIndexOf("\t"),
    );
    const head = before.slice(lastWs + 1);
    const atIdx = head.lastIndexOf("@");
    if (atIdx >= 0) {
      const start = lastWs + 1 + atIdx;
      // Reject @ that is mid-word (e.g. "foo@bar").
      if (atIdx === 0 || /\s/.test(head[atIdx - 1] ?? "")) {
        setMentionRange({ start, end: caret });
        setAnchorEl(e.currentTarget);
        return;
      }
    }
    setMentionRange(null);
    setAnchorEl(null);
  }

  function insertVariable(name: string) {
    if (!mentionRange) return;
    const before = body.content.slice(0, mentionRange.start);
    const after = body.content.slice(mentionRange.end);
    // Inserted text is always the variable name — language-independent.
    const inserted = name;
    const next = before + inserted + after;
    setBody((b) => ({ ...b, content: next }));
    setMentionRange(null);
    setAnchorEl(null);
    const caret = (before + inserted).length;
    queueMicrotask(() => {
      inputRef.current?.setSelectionRange(caret, caret);
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
            onChange={handleContentChange}
            inputRef={inputRef}
            slotProps={{
              htmlInput: {
                "data-testid": "crf-annotation-dialog-content",
              },
            }}
          />
          {/* @-mention Popover. Anchored to the content TextField. */}
          <Popover
            open={Boolean(anchorEl) && mentionRange !== null}
            anchorEl={anchorEl}
            anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
            slotProps={{ paper: { sx: { minWidth: 240, maxHeight: 240 } } }}
            data-testid="crf-variable-popover"
          >
            <MenuList>
              {filteredVariables.length === 0 ? (
                <MenuItem disabled>
                  {t("crf.annotationDialog.variable.noMatch")}
                </MenuItem>
              ) : (
                filteredVariables.map((v) => (
                  <MenuItem
                    key={v.id}
                    onClick={() => insertVariable(v.name)}
                    data-testid={`crf-variable-${v.id}`}
                  >
                    {v.name}
                  </MenuItem>
                ))
              )}
            </MenuList>
          </Popover>
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