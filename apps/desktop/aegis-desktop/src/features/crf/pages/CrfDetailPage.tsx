import { useMemo, useState } from "react";
import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  MenuItem,
  MenuList,
  Popover,
  Stack,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams } from "@tanstack/react-router";

import {
  AnnotationDialog,
  CrfAnnotationArea,
  CrfItemRow,
  CrfToolsMenu,
  DeleteAnnotationDialog,
  DeleteDomainAnnotationDialog,
  DomainAnnotationDialog,
  NotSubmittedChip,
} from "../components";
import { annotationColor } from "../components/AnnotationChip";
import { useGetCrfForm } from "../data/list";
import {
  useCrfFormDetail,
  useCreateAnnotation,
  useCreateDomainAnnotation,
  useDeleteAnnotation,
  useDeleteDomainAnnotation,
  useUpdateAnnotation,
  useUpdateDomainAnnotation,
  useUpdateOwnerNotSubmitted,
} from "../data/detail";
import type {
  Annotation,
  AnnotationOwner,
  CrfFormDetail,
  DomainAnnotation,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";

type DomainDialogState =
  | { mode: "create" }
  | { mode: "edit"; row: DomainAnnotation }
  | null;

type AnnotationDialogState =
  | { mode: "create"; owner: AnnotationOwner }
  | { mode: "edit"; row: Annotation; owner: AnnotationOwner }
  | null;

/**
 * Look up the cached `notSubmitted` flag for an owner so the
 * dialog can seed its checkbox from the source of truth and the
 * cascade mutation can decide whether to run. Returns `null` when
 * the cache is empty / stale — callers fall back to `false` for
 * display and skip the cascade so we never delete annotations
 * whose state we can't verify.
 */
function readOwnerNotSubmitted(
  detail: CrfFormDetail | undefined,
  owner: AnnotationOwner,
): boolean | null {
  if (!detail) return null;
  if (owner.kind === "form") return detail.form.notSubmitted;
  if (owner.kind === "item") {
    const found = detail.items.find((i) => i.item.id === owner.id);
    return found ? found.item.notSubmitted : null;
  }
  if (owner.kind === "option") {
    for (const item of detail.items) {
      const opt = item.options.find((o) => o.option.id === owner.id);
      if (opt) return opt.option.notSubmitted;
    }
    return null;
  }
  for (const item of detail.items) {
    const u = item.units.find((uu) => uu.unit.id === owner.id);
    if (u) return u.unit.notSubmitted;
  }
  return null;
}

/**
 * Order annotations the same way the form's `domainAnnotations` list
 * is ordered — within a single owner (form / item / option / unit),
 * chips for the first domain annotation appear first, then the
 * second, and so on. Annotations whose domain annotation is not in
 * the map (orphaned, e.g. the server returned a domain annotation
 * the page hasn't seen yet) fall to the end.
 */
function sortByDomainAnnotationOrder<
  T extends { domainAnnotationId: number; id: number },
>(
  annotations: T[],
  indexByDomainAnnotationId: Map<number, number>,
): T[] {
  const fallback = Number.MAX_SAFE_INTEGER;
  return [...annotations].sort((a, b) => {
    const ai =
      indexByDomainAnnotationId.get(a.domainAnnotationId) ?? fallback;
    const bi =
      indexByDomainAnnotationId.get(b.domainAnnotationId) ?? fallback;
    if (ai !== bi) return ai - bi;
    // Stable tie-breaker: keep insertion order within a single
    // domain annotation. `Array.sort` is stable in modern engines,
    // so this only matters if we later add a non-stable sort.
    return a.id - b.id;
  });
}

export function CrfDetailPage() {
  const { t } = useI18n();
  const { projectCode, formId } = useParams({ strict: false }) as {
    projectCode: string;
    formId?: string;
  };
  const navigate = useNavigate();
  const id =
    formId != null && Number.isFinite(Number(formId)) && Number(formId) > 0
      ? Number(formId)
      : null;

  const query = useGetCrfForm(id);
  const detailQuery = useCrfFormDetail(id);

  const createDomain = useCreateDomainAnnotation();
  const updateDomain = useUpdateDomainAnnotation();
  const deleteDomain = useDeleteDomainAnnotation();
  const createAnnotation = useCreateAnnotation();
  const updateAnnotation = useUpdateAnnotation();
  const deleteAnnotation = useDeleteAnnotation();
  const updateOwnerNotSubmitted = useUpdateOwnerNotSubmitted();

  const [domainDialog, setDomainDialog] = useState<DomainDialogState>(null);
  const [annotationDialog, setAnnotationDialog] =
    useState<AnnotationDialogState>(null);
  const [confirmDeleteDomain, setConfirmDeleteDomain] =
    useState<DomainAnnotation | null>(null);
  const [confirmDeleteAnnotation, setConfirmDeleteAnnotation] =
    useState<Annotation | null>(null);
  const [formNameMenuAnchor, setFormNameMenuAnchor] =
    useState<HTMLElement | null>(null);

  const colorByDomainAnnotationId = useMemo(() => {
    const map = new Map<number, number>();
    detailQuery.data?.domainAnnotations.forEach((d, i) => map.set(d.id, i));
    return map;
  }, [detailQuery.data]);

  const back = () =>
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: (prev: Record<string, unknown>) => prev,
    });

  if (id == null) {
    return (
      <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
          <IconButton aria-label={t("crf.detail.back")} onClick={back}>
            <ArrowBackIcon />
          </IconButton>
          <Typography variant="h4">{t("crf.detail.title")}</Typography>
        </Box>
        <Alert severity="error">{t("common.invalidId")}</Alert>
      </Box>
    );
  }

  const form = query.data;
  const detail = detailQuery.data;
  // An annotation needs a domain annotation to belong to, so when
  // the form has none, every create-annotation entry point must
  // be blocked. The "New domain annotation" path stays open — it's
  // how the first one is created.
  const noDomainAnnotations =
    (detail?.domainAnnotations.length ?? 0) === 0;

  const activeDomainMutation =
    createDomain.error ?? updateDomain.error ?? deleteDomain.error ?? null;
  const domainMutationPending =
    createDomain.isPending || updateDomain.isPending || deleteDomain.isPending;

  const activeAnnotationMutation =
    createAnnotation.error ??
    updateAnnotation.error ??
    deleteAnnotation.error ??
    null;
  const annotationMutationPending =
    createAnnotation.isPending ||
    updateAnnotation.isPending ||
    deleteAnnotation.isPending;

  const openCreateAnnotation = (owner: AnnotationOwner) => {
    // Defensive: when the form is marked not-submitted, the cascade
    // has already wiped every annotation (and every domain
    // annotation) on the form, so there is nothing to hang a new
    // annotation on. The MenuItem in the header and the item/unit/
    // option click handlers also gate themselves, but a future caller
    // (or a hot-reload flicker) shouldn't be able to slip through.
    if (form?.notSubmitted) return;
    // An annotation needs a domain annotation to belong to, so
    // block creation until at least one exists. The "New domain
    // annotation" path stays open — that's how the first one is
    // created.
    if (noDomainAnnotations) return;
    setAnnotationDialog({ mode: "create", owner });
  };

  const openEditAnnotation = (row: Annotation, owner: AnnotationOwner) =>
    setAnnotationDialog({ mode: "edit", row, owner });

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          flexWrap: "wrap",
          gap: 2,
        }}
      >
        <IconButton aria-label={t("crf.detail.back")} onClick={back}>
          <ArrowBackIcon />
        </IconButton>
        {form?.code && <Chip size="small" label={form.code} variant="outlined" />}
        <Typography
          variant="h5"
          onClick={(e) =>
            setFormNameMenuAnchor((prev) => (prev ? null : e.currentTarget))
          }
          aria-haspopup="menu"
          aria-expanded={Boolean(formNameMenuAnchor)}
          sx={{ cursor: "pointer" }}
          data-testid="crf-form-name"
        >
          {form?.name ?? t("crf.detail.title")}
        </Typography>
        {form?.notSubmitted && (
          <NotSubmittedChip
            onDelete={() =>
              updateOwnerNotSubmitted.mutate({
                formId: id,
                owner: { kind: "form", id },
                notSubmitted: false,
              })
            }
          />
        )}
        <Popover
          open={Boolean(formNameMenuAnchor)}
          anchorEl={formNameMenuAnchor}
          onClose={() => setFormNameMenuAnchor(null)}
          anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
          slotProps={{
            paper: {
              sx: { minWidth: 200 },
            },
          }}
        >
          {/* `MenuItem` requires a `MenuListContext` to register itself
              for keyboard navigation. Wrap in `MenuList` so MUI doesn't
              warn in development and the Popover behaves as a real menu
              for assistive tech. */}
          <MenuList>
            <Tooltip
              title={
                form?.notSubmitted
                  ? t("crf.detail.menu.disabledWhenNotSubmitted")
                  : ""
              }
              disableHoverListener={!form?.notSubmitted}
              disableFocusListener={!form?.notSubmitted}
              disableTouchListener={!form?.notSubmitted}
            >
              {/* `span` wrapper is required because MUI's disabled
                  MenuItem doesn't forward refs / props to a Tooltip
                  host — wrapping lets the tooltip track hover even
                  when the menu item itself is aria-disabled. */}
              <span>
                <MenuItem
                  disabled={Boolean(form?.notSubmitted)}
                  onClick={() => {
                    setFormNameMenuAnchor(null);
                    setDomainDialog({ mode: "create" });
                  }}
                >
                  {t("crf.detail.menu.newDomain")}
                </MenuItem>
              </span>
            </Tooltip>
            <Tooltip
              title={
                form?.notSubmitted
                  ? t("crf.detail.menu.disabledWhenNotSubmitted")
                  : noDomainAnnotations
                    ? t("crf.detail.menu.disabledWhenNoDomainAnnotations")
                    : ""
              }
              disableHoverListener={
                !form?.notSubmitted && !noDomainAnnotations
              }
              disableFocusListener={
                !form?.notSubmitted && !noDomainAnnotations
              }
              disableTouchListener={
                !form?.notSubmitted && !noDomainAnnotations
              }
            >
              <span>
                <MenuItem
                  disabled={
                    Boolean(form?.notSubmitted) || noDomainAnnotations
                  }
                  onClick={() => {
                    setFormNameMenuAnchor(null);
                    openCreateAnnotation({ kind: "form", id });
                  }}
                >
                  {t("crf.detail.menu.newAnnotation")}
                </MenuItem>
              </span>
            </Tooltip>
          </MenuList>
        </Popover>
        {/* Domain annotation chips, right of name. Their colour cycles
            with the position in `domainAnnotations` so the user can see
            which annotation colour a chip produces — matches the cycle
            applied to the per-domain annotation chips below. */}
        {detail && detail.domainAnnotations.length > 0 && (
          <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
            {detail.domainAnnotations.map((d, i) => (
              <Chip
                key={d.id}
                label={t("crf.detail.domainChip.label", {
                  name: d.name,
                  description: d.description,
                })}
                color={annotationColor(i)}
                onClick={() => setDomainDialog({ mode: "edit", row: d })}
                onDelete={() => setConfirmDeleteDomain(d)}
                size="small"
                data-testid={`domain-annotation-chip-${d.id}`}
                variant="outlined"
              />
            ))}
          </Stack>
        )}
        <Box sx={{ flexGrow: 1 }} />
        <CrfToolsMenu projectCode={projectCode} />
      </Box>

      {query.isFetching && !form && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      {query.isError && (
        <Alert severity="error">{errorMessage(query.error)}</Alert>
      )}
      {detailQuery.isError && (
        <Alert severity="error">
          {t("crf.detail.loadFailed", {
            message: errorMessage(detailQuery.error),
          })}
        </Alert>
      )}
      {detailQuery.isFetching && !detail && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      {/* Form-level annotation chips */}
      {detail && (
        <CrfAnnotationArea
          annotations={sortByDomainAnnotationOrder(
            detail.formAnnotations,
            colorByDomainAnnotationId,
          )}
          colorByDomainAnnotationId={colorByDomainAnnotationId}
          onEdit={(a) => openEditAnnotation(a, { kind: "form", id })}
          onDelete={(a) => setConfirmDeleteAnnotation(a)}
        />
      )}

      {/* Item list */}
      {detail && (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {detail.items.length === 0 ? (
            <Alert severity="info">{t("crf.detail.empty")}</Alert>
          ) : (
            detail.items.map((itemDetail) => (
              <CrfItemRow
                key={itemDetail.item.id}
                itemDetail={{
                  ...itemDetail,
                  annotations: sortByDomainAnnotationOrder(
                    itemDetail.annotations,
                    colorByDomainAnnotationId,
                  ),
                  options: itemDetail.options.map((opt) => ({
                    ...opt,
                    annotations: sortByDomainAnnotationOrder(
                      opt.annotations,
                      colorByDomainAnnotationId,
                    ),
                  })),
                  units: itemDetail.units.map((u) => ({
                    ...u,
                    annotations: sortByDomainAnnotationOrder(
                      u.annotations,
                      colorByDomainAnnotationId,
                    ),
                  })),
                }}
                colorByDomainAnnotationId={colorByDomainAnnotationId}
                onCreateAnnotation={openCreateAnnotation}
                onEditAnnotation={(a) => {
                  const owner: AnnotationOwner = a.owner;
                  openEditAnnotation(a, owner);
                }}
                onDeleteAnnotation={(a) => setConfirmDeleteAnnotation(a)}
                onClearNotSubmitted={(owner) =>
                  updateOwnerNotSubmitted.mutate({
                    formId: id,
                    owner,
                    notSubmitted: false,
                  })
                }
                formNotSubmitted={Boolean(form?.notSubmitted)}
                itemNotSubmitted={Boolean(itemDetail.item.notSubmitted)}
                noDomainAnnotations={noDomainAnnotations}
              />
            ))
          )}
        </Box>
      )}

      {/* Dialogs */}
      <DomainAnnotationDialog
        open={domainDialog != null}
        mode={domainDialog?.mode ?? "create"}
        row={domainDialog?.mode === "edit" ? domainDialog.row : undefined}
        formNotSubmitted={form?.notSubmitted ?? false}
        onClose={() => setDomainDialog(null)}
        onSubmit={(body) => {
          if (domainDialog?.mode === "edit") {
            updateDomain.mutate({
              id: domainDialog.row.id,
              formId: id,
              body: { name: body.name, description: body.description },
            }, { onSuccess: () => setDomainDialog(null) });
          } else {
            createDomain.mutate({
              formId: id,
              body: { name: body.name, description: body.description },
            }, { onSuccess: () => setDomainDialog(null) });
          }
        }}
        // The form-level `Not submit` action — runs the cascade
        // (delete every annotation in the form, then PATCH the
        // form's notSubmitted=true). Closes the dialog on success
        // regardless of which dialog was open.
        onMarkNotSubmitted={() =>
          updateOwnerNotSubmitted.mutate(
            {
              formId: id,
              owner: { kind: "form", id },
              notSubmitted: true,
            },
            { onSuccess: () => setDomainDialog(null) },
          )
        }
        markNotSubmittedPending={updateOwnerNotSubmitted.isPending}
        markNotSubmittedError={updateOwnerNotSubmitted.error}
        mutationError={activeDomainMutation}
        mutationPending={domainMutationPending}
      />

      <AnnotationDialog
        open={annotationDialog != null}
        mode={annotationDialog?.mode ?? "create"}
        owner={
          annotationDialog ? annotationDialog.owner : { kind: "form", id }
        }
        ownerNotSubmitted={
          readOwnerNotSubmitted(
            detail,
            annotationDialog
              ? annotationDialog.owner
              : { kind: "form", id },
          ) ?? false
        }
        row={annotationDialog?.mode === "edit" ? annotationDialog.row : undefined}
        availableDomainAnnotations={detail?.domainAnnotations ?? []}
        onClose={() => setAnnotationDialog(null)}
        onSubmit={(body) => {
          if (annotationDialog?.mode === "edit") {
            updateAnnotation.mutate(
              {
                id: annotationDialog.row.id,
                formId: id,
                body: {
                  content: body.content,
                  assign: body.assign,
                },
              },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          } else {
            const owner = annotationDialog?.owner ?? { kind: "form", id };
            createAnnotation.mutate(
              {
                formId: id,
                body: {
                  domainAnnotationId: body.domainAnnotationId,
                  content: body.content,
                  assign: body.assign,
                  owner,
                },
              },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          }
        }}
        // The annotation-level `Not submit` action — runs the
        // cascade against the annotation's owner (form → all,
        // item → item+options+units, option/unit → own), then
        // PATCHes the owner's notSubmitted=true. Closes the
        // dialog on success regardless of create vs edit.
        onMarkNotSubmitted={() =>
          updateOwnerNotSubmitted.mutate(
            {
              formId: id,
              owner: annotationDialog?.owner ?? { kind: "form", id },
              notSubmitted: true,
            },
            { onSuccess: () => setAnnotationDialog(null) },
          )
        }
        markNotSubmittedPending={updateOwnerNotSubmitted.isPending}
        markNotSubmittedError={updateOwnerNotSubmitted.error}
        mutationError={activeAnnotationMutation}
        mutationPending={annotationMutationPending}
      />

      <DeleteDomainAnnotationDialog
        open={confirmDeleteDomain != null}
        row={confirmDeleteDomain}
        onClose={() => setConfirmDeleteDomain(null)}
        onConfirm={(row) =>
          deleteDomain.mutate(
            { id: row.id, formId: id },
            { onSuccess: () => setConfirmDeleteDomain(null) },
          )
        }
        mutationError={deleteDomain.error}
        mutationPending={deleteDomain.isPending}
      />

      <DeleteAnnotationDialog
        open={confirmDeleteAnnotation != null}
        row={confirmDeleteAnnotation}
        onClose={() => setConfirmDeleteAnnotation(null)}
        onConfirm={(row) =>
          deleteAnnotation.mutate(
            { id: row.id, formId: id },
            { onSuccess: () => setConfirmDeleteAnnotation(null) },
          )
        }
        mutationError={deleteAnnotation.error}
        mutationPending={deleteAnnotation.isPending}
      />
    </Box>
  );
}
