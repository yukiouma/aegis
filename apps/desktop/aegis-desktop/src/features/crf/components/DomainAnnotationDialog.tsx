import { useEffect, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
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
  DomainAnnotation,
  SdtmDomainView,
} from "../../../shared/api";

export interface DomainAnnotationDialogBody {
  name: string;
  description: string;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  /**
   * Current `notSubmitted` flag of the form that owns this domain
   * annotation. The dialog hides its `Not submit` action while the
   * form is already marked not-submitted — there's nothing left to
   * do — and the page runs the cascade + form update on click.
   */
  formNotSubmitted: boolean;
  onClose: () => void;
  onSubmit: (body: DomainAnnotationDialogBody) => void;
  /**
   * Trigger the form-level cascade: delete every annotation in
   * the form, then PATCH the form's `notSubmitted` flag to true.
   * Wired by the page to `useUpdateOwnerNotSubmitted` for the
   * `{ kind: "form", id }` owner.
   */
  onMarkNotSubmitted: () => void;
  markNotSubmittedPending: boolean;
  markNotSubmittedError: ApiError | null;
  mutationError: ApiError | null;
  mutationPending: boolean;
  /**
   * SDTM domains for the project's resolved SDTMIG version. Empty
   * array disables the autocomplete's option list (the user can
   * still type free-form names via `freeSolo`).
   */
  sdtmDomains: SdtmDomainView[];
  /**
   * Language code used to look up the matching description when
   * the user picks a domain from the autocomplete. `""` / `"en"`
   * / `"zh-CN"` etc. — matches the lang field on
   * `SdtmDomainDescription`.
   */
  sdtmLanguage: string;
}

const EMPTY: DomainAnnotationDialogBody = {
  name: "",
  description: "",
};

/**
 * Look up the description for the given domain name in the project
 * language. Returns `null` when the domain is not in `sdtmDomains`
 * or no description matches the language (per the brainstorming
 * Q3 decision: leave the field empty, not an em-dash, no warning).
 */
function findDescription(
  domains: SdtmDomainView[],
  name: string,
  language: string,
): string | null {
  const upper = name.toUpperCase();
  const match = domains.find((d) => d.name.toUpperCase() === upper);
  if (!match) return null;
  const desc = match.descriptions.find((d) => d.lang === language);
  return desc?.details.description ?? null;
}

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  formNotSubmitted,
  onClose,
  onSubmit,
  onMarkNotSubmitted,
  markNotSubmittedPending,
  markNotSubmittedError,
  mutationError,
  mutationPending,
  sdtmDomains,
  sdtmLanguage,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<DomainAnnotationDialogBody>(EMPTY);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        name: row.name,
        description: row.description,
      });
    } else {
      setBody({
        name: EMPTY.name,
        description: EMPTY.description,
      });
    }
  }, [open, mode, row]);

  const submitDisabled = mutationPending || body.name.trim() === "";
  // The Not submit action is one-way and only meaningful when
  // creating a fresh domain annotation. Hide it in edit mode —
  // the user is editing an existing row, not deciding whether the
  // form needs a flag — and hide it once the form is already
  // not-submitted.
  const markVisible = mode === "create" && !formNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      name: body.name.trim(),
      description: body.description.trim(),
    });
  }

  const domainOptions = sdtmDomains.map((d) => d.name);

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
      <DialogContent>
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
        >
          <Autocomplete
            freeSolo
            options={domainOptions}
            // Case-insensitive startsWith — matches "auto upcase what
            // they enter, filter the domains with the current value".
            filterOptions={(opts, state) =>
              opts.filter((o) =>
                o.toUpperCase().startsWith(state.inputValue.toUpperCase()),
              )
            }
            inputValue={body.name}
            // Typing path: uppercase as the user types.
            onInputChange={(_e, value, reason) => {
              if (reason === "input") {
                setBody((b) => ({ ...b, name: value.toUpperCase() }));
              } else {
                setBody((b) => ({ ...b, name: value }));
              }
            }}
            // Selection path: uppercase the picked name and auto-fill
            // the description in the project's language.
            onChange={(_e, value) => {
              const next = (
                typeof value === "string" ? value : value ?? ""
              ).toUpperCase();
              const desc = next
                ? findDescription(sdtmDomains, next, sdtmLanguage)
                : null;
              setBody((b) => ({
                ...b,
                name: next,
                description: desc ?? (next ? "" : b.description),
              }));
            }}
            renderInput={(params) => (
              <TextField
                {...params}
                size="small"
                label={t("crf.domainDialog.field.name")}
              />
            )}
          />
          <TextField
            size="small"
            label={t("crf.domainDialog.field.description")}
            value={body.description}
            onChange={(e) =>
              setBody((b) => ({ ...b, description: e.target.value }))
            }
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
            data-testid="crf-domain-dialog-not-submit"
          >
            {t("crf.domainDialog.notSubmit")}
          </Button>
        )}
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