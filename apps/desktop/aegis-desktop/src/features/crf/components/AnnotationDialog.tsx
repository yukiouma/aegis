import { useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";
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
  Stack,
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
  /**
   * Resolved `CrfItem.code` for the annotation's owner. `null` for
   * non-item owners (form/option/unit) and for item owners whose
   * item is not in the cached form detail. Drives the SUPP button's
   * `<itemCode> in SUPP<domainCode>` draft; `null`/`undefined`
   * disables the button when nothing useful can be drafted.
   */
  ownerItemCode?: string | null;
}

const EMPTY: AnnotationDialogBody = {
  domainAnnotationId: 0,
  content: "",
  assign: false,
};

export function AnnotationDialog({
  open,
  mode,
  owner,
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
  ownerItemCode = null,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<AnnotationDialogBody>(EMPTY);

  // --- @-mention state ---
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const [mentionRange, setMentionRange] =
    useState<{ start: number; end: number } | null>(null);
  // Index of the keyboard-highlighted variable inside `filteredVariables`.
  // Resets to 0 whenever the filtered list changes (new fragment, new
  // fetch result, dropdown re-opens).
  const [highlightIndex, setHighlightIndex] = useState(0);
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
  // SUPP gate. `null` selectedDomainName means no domain annotation
  // is selected — nothing meaningful to draft. For item owners we
  // additionally need the resolved item code: without it there's no
  // "ITEMCODE in SUPPXX" left half to write.
  const suppDisabled =
    !selectedDomainName ||
    (owner.kind === "item" && !ownerItemCode);
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

  // Reset the keyboard highlight whenever the filtered list changes
  // (user typed more letters, the variables query resolved, the
  // dropdown re-opened after Escape). Without this the highlight would
  // drift past the end of the new list.
  useEffect(() => {
    setHighlightIndex(0);
  }, [filteredVariables]);

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
    });
  }

  // SUPP quick-draft. Replaces the entire content with the common
  // " in SUPP<domainCode>" / "<itemCode> in SUPP<domainCode>" pattern.
  // `suppDisabled` gates the button in the JSX, but the handler
  // re-checks so a programmatic click (test, future keyboard
  // shortcut) can't slip through.
  function handleSuppClick() {
    if (suppDisabled) return;
    const domainCode = selectedDomainName!;
    const next =
      owner.kind === "item" && ownerItemCode
        ? `${ownerItemCode} in SUPP${domainCode}`
        : ` in SUPP${domainCode}`;
    setBody((b) => ({ ...b, content: next }));
    // Close any open @-mention dropdown — clicking SUPP replaces
    // the field wholesale, so a stale `@fragment` mention would be
    // orphaned.
    setMentionRange(null);
    setAnchorEl(null);
    queueMicrotask(() => {
      inputRef.current?.setSelectionRange(next.length, next.length);
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

  // --- @-mention keyboard navigation ---
  // The content field keeps focus while the Popover is open; intercept
  // Arrow / Enter / Escape here so the caret does not move out from
  // under the user.
  function handleContentKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (!mentionRange) return;
    if (filteredVariables.length === 0) {
      // Only Escape is meaningful when the list is empty.
      if (e.key === "Escape") {
        e.preventDefault();
        setMentionRange(null);
        setAnchorEl(null);
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlightIndex((i) => (i + 1) % filteredVariables.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlightIndex(
        (i) => (i - 1 + filteredVariables.length) % filteredVariables.length,
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      const picked = filteredVariables[highlightIndex];
      if (picked) insertVariable(picked.name);
    } else if (e.key === "Escape") {
      e.preventDefault();
      // Close the dropdown but leave the `@fragment` text in the field
      // — the user explicitly cancelled the menu, not the typing.
      setMentionRange(null);
      setAnchorEl(null);
    }
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
          <Stack direction="row" spacing={1} alignItems="center">
            <TextField
              size="small"
              label={t("crf.annotationDialog.field.content")}
              value={body.content}
              onChange={handleContentChange}
              onKeyDown={handleContentKeyDown}
              inputRef={inputRef}
              sx={{ flexGrow: 1 }}
              slotProps={{
                htmlInput: {
                  "data-testid": "crf-annotation-dialog-content",
                },
              }}
            />
            {/* SUPP quick-draft. Replaces `body.content` with the
                common " in SUPP<domainCode>" / "<itemCode> in
                SUPP<domainCode>" pattern. `data-testid` is the only
                identifier — no label, no tooltip — see spec
                2026-09-23-crf-annotation-supp-draft-design.md. */}
            <Button
              size="small"
              variant="outlined"
              onClick={handleSuppClick}
              disabled={suppDisabled}
              data-testid="crf-annotation-dialog-supp"
            >
              SUPP
            </Button>
          </Stack>
          {/* @-mention Popover. Anchored to the content TextField. */}
          <Popover
            open={Boolean(anchorEl) && mentionRange !== null}
            anchorEl={anchorEl}
            anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
            slotProps={{ paper: { sx: { minWidth: 240, maxHeight: 240 } } }}
            // The TextField keeps focus while the Popover is open — the
            // user types more letters to filter and uses Arrow / Enter /
            // Escape to drive the menu. Without these flags MUI's Popover
            // would steal focus to the first MenuItem on open, swallow
            // further keystrokes, and capture Escape.
            disableAutoFocus
            disableEnforceFocus
            data-testid="crf-variable-popover"
          >
            <MenuList>
              {filteredVariables.length === 0 ? (
                <MenuItem disabled>
                  {t("crf.annotationDialog.variable.noMatch")}
                </MenuItem>
              ) : (
                filteredVariables.map((v, idx) => (
                  <MenuItem
                    key={v.id}
                    selected={idx === highlightIndex}
                    onClick={() => insertVariable(v.name)}
                    data-testid={`crf-variable-${v.id}`}
                    data-highlighted={idx === highlightIndex ? "true" : null}
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