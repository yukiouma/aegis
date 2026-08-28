import { useMemo, useState } from "react";
import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  MenuItem,
  Popover,
  Stack,
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
} from "../components";
import { useGetCrfForm } from "../data/list";
import {
  useCrfFormDetail,
  useCreateAnnotation,
  useCreateDomainAnnotation,
  useDeleteAnnotation,
  useDeleteDomainAnnotation,
  useUpdateAnnotation,
  useUpdateDomainAnnotation,
} from "../data/detail";
import type {
  Annotation,
  AnnotationOwner,
  CreateDomainAnnotationInput,
  DomainAnnotation,
  UpdateDomainAnnotationInput,
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

  const openCreateAnnotation = (owner: AnnotationOwner) =>
    setAnnotationDialog({ mode: "create", owner });

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
        {form?.code && <Chip label={form.code} variant="outlined" />}
        <Typography
          variant="h5"
          onMouseEnter={(e) => setFormNameMenuAnchor(e.currentTarget)}
          onMouseLeave={() => setFormNameMenuAnchor(null)}
          sx={{ cursor: "default" }}
          data-testid="crf-form-name"
        >
          {form?.name ?? t("crf.detail.title")}
        </Typography>
        <Popover
          open={Boolean(formNameMenuAnchor)}
          anchorEl={formNameMenuAnchor}
          onClose={() => setFormNameMenuAnchor(null)}
          anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
          disableAutoFocus
          disableEnforceFocus
          slotProps={{
            paper: {
              onMouseLeave: () => setFormNameMenuAnchor(null),
              sx: { minWidth: 200 },
            },
          }}
        >
          <MenuItem
            onClick={() => {
              setFormNameMenuAnchor(null);
              setDomainDialog({ mode: "create" });
            }}
          >
            {t("crf.detail.menu.newDomain")}
          </MenuItem>
          <MenuItem
            onClick={() => {
              setFormNameMenuAnchor(null);
              openCreateAnnotation({ kind: "form", id });
            }}
          >
            {t("crf.detail.menu.newAnnotation")}
          </MenuItem>
        </Popover>
        {/* Domain annotation chips, right of name */}
        {detail && detail.domainAnnotations.length > 0 && (
          <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
            {detail.domainAnnotations.map((d) => (
              <Chip
                key={d.id}
                label={t("crf.detail.domainChip.label", {
                  name: d.name,
                  description: d.description,
                })}
                onClick={() => setDomainDialog({ mode: "edit", row: d })}
                onDelete={() => setConfirmDeleteDomain(d)}
                size="small"
                data-testid={`domain-annotation-chip-${d.id}`}
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
          annotations={detail.formAnnotations}
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
                itemDetail={itemDetail}
                colorByDomainAnnotationId={colorByDomainAnnotationId}
                onCreateAnnotation={openCreateAnnotation}
                onEditAnnotation={(a) => {
                  const owner: AnnotationOwner = a.owner;
                  openEditAnnotation(a, owner);
                }}
                onDeleteAnnotation={(a) => setConfirmDeleteAnnotation(a)}
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
        onClose={() => setDomainDialog(null)}
        onSubmit={(body) => {
          if (domainDialog?.mode === "edit") {
            updateDomain.mutate({
              id: domainDialog.row.id,
              formId: id,
              body: body as UpdateDomainAnnotationInput,
            }, { onSuccess: () => setDomainDialog(null) });
          } else {
            createDomain.mutate({
              formId: id,
              body: body as CreateDomainAnnotationInput,
            }, { onSuccess: () => setDomainDialog(null) });
          }
        }}
        mutationError={activeDomainMutation}
        mutationPending={domainMutationPending}
      />

      <AnnotationDialog
        open={annotationDialog != null}
        mode={annotationDialog?.mode ?? "create"}
        owner={
          annotationDialog ? annotationDialog.owner : { kind: "form", id }
        }
        row={annotationDialog?.mode === "edit" ? annotationDialog.row : undefined}
        availableDomainAnnotations={detail?.domainAnnotations ?? []}
        onClose={() => setAnnotationDialog(null)}
        onSubmit={(body) => {
          if (annotationDialog?.mode === "edit") {
            updateAnnotation.mutate(
              { id: annotationDialog.row.id, formId: id, body },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          } else {
            const owner = annotationDialog?.owner ?? { kind: "form", id };
            createAnnotation.mutate(
              {
                formId: id,
                body: { ...body, owner },
              },
              { onSuccess: () => setAnnotationDialog(null) },
            );
          }
        }}
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
