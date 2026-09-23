import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Badge,
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
import {
  ArrowBack as ArrowBackIcon,
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";

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
import {
  MissionIssueDialog,
  useAppendComment,
  useCreateIssue,
  useIsProjectLeader,
  useListIssuesByMission,
  useListMissionsByProject,
  usePatchIssueState,
  type IssueScope,
} from "../../mission";
import { useCurrentUser } from "../../auth";
import { useUserNameMap } from "../../user";
import { annotationColor } from "../components/AnnotationChip";
import { useGetCrfForm, useSetCrfFormApproved } from "../data/list";
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
import { useSdtmContext } from "../data/sdtm";
import type {
  Annotation,
  AnnotationOwner,
  CrfFormDetail,
  DomainAnnotation,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { queryKeys } from "../../../shared/query/keys";

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
 * Look up the `CrfItem.code` for an item-level annotation owner so
 * the AnnotationDialog's SUPP button can draft `<code> in SUPP<xx>`
 * content. Returns `null` for non-item owners and for item owners
 * whose item isn't in the cached form detail — `null` is the same
 * "nothing useful to draft" signal the dialog already treats as
 * "disable the button".
 */
function readOwnerItemCode(
  detail: CrfFormDetail | undefined,
  owner: AnnotationOwner,
): string | null {
  if (!detail || owner.kind !== "item") return null;
  const found = detail.items.find((i) => i.item.id === owner.id);
  return found ? found.item.code : null;
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
  const sdtmContext = useSdtmContext(projectCode);

  // `focus` carries `kind-id` from the global-search row click
  // (e.g. "item-21"). When the detail query resolves we scroll the
  // matching `data-testid` into view. Falls back to the existing
  // `domain-annotation-chip-<id>` testid because domain-annotation
  // chips use that prefix; `scrollIntoView` walks up to the nearest
  // scrollable ancestor so the `Box` wrapping `detail.items`
  // (around line 400) is the container.
  const routeSearch = useSearch({ strict: false }) as {
    versionId?: number;
    focus?: string;
  };
  const focus = routeSearch.focus;
  useEffect(() => {
    if (!focus || !detailQuery.data) return;
    const [kind, idStr] = focus.split("-");
    if (!kind || !idStr) return;
    // Defer the scroll one frame so the freshly rendered items Box
    // has had a chance to lay out. Without this the rows can be at
    // 0×0 when `scrollIntoView` measures them and the page stays
    // stuck at the top.
    const raf = requestAnimationFrame(() => {
      const el =
        document.querySelector(`[data-testid="crf-${kind}-${idStr}"]`) ??
        document.querySelector(
          `[data-testid="domain-annotation-chip-${idStr}"]`,
        );
      el?.scrollIntoView({ block: "start", behavior: "smooth" });
    });
    return () => cancelAnimationFrame(raf);
  }, [focus, detailQuery.data]);

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

  // Mission-issue wiring. The form → mission lookup mirrors
  // CrfMissionAssignDrawer: filter the project's `crf` missions by
  // `missionCode === form.code`. The mission is shared between the
  // dialog's title and the per-chip dot badge — derive once here.
  // `useIsProjectLeader` returns `boolean | null` (null = still
  // loading). The RBAC booleans therefore default to `false`
  // while project data is resolving — controls flip on once the
  // check resolves to `true`.
  const isProjectLeader = useIsProjectLeader(projectCode);
  const resolveName = useUserNameMap();
  const missionList =
    useListMissionsByProject(projectCode, "crf").data ?? [];
  const formMission = useMemo(
    () => missionList.find((m) => m.missionCode === form?.code) ?? null,
    [missionList, form?.code],
  );
  const issuesQuery = useListIssuesByMission(formMission?.id ?? null);
  const openIssueCountByTarget = useMemo(() => {
    const map = new Map<string | null, number>();
    for (const i of issuesQuery.data ?? []) {
      if (i.state !== "opened") continue;
      map.set(
        i.targetItem ?? null,
        (map.get(i.targetItem ?? null) ?? 0) + 1,
      );
    }
    return map;
  }, [issuesQuery.data]);

  const currentUser = useCurrentUser().data;
  const isMissionQc = !!formMission?.assignees.some(
    (a) => a.userCode === currentUser?.code && a.role === "qc",
  );
  const isMissionDev = !!formMission?.assignees.some(
    (a) => a.userCode === currentUser?.code && a.role === "dev",
  );
  const canCreate = isProjectLeader === true || isMissionQc;
  const canActOnIssue = isProjectLeader === true || isMissionQc;
  const canComment = canActOnIssue || isMissionDev;
  // Two flat RBAC flags that drive the page-level gates. `canEditAnnotations`
  // controls the form-name hover menu's create entries, the header
  // domain-annotation chips, the form-level annotation chips (via
  // CrfAnnotationArea), and the CrfItemRow click affordances.
  // `canOpenEmptyIssueDialog` controls the form / item code chip's
  // `disabled` when its scope has zero opened issues — the spec lets
  // leader and QC through, blocks DEV and task-unrelated. Both default
  // to `false` when `isProjectLeader === null` (initial fetch) —
  // matches today's `canCreate` behavior.
  const canEditAnnotations = isProjectLeader === true || isMissionDev;
  const canOpenEmptyIssueDialog = isProjectLeader === true || isMissionQc;
  // Only project leader and mission DEV can clear the
  // [NOT SUBMITTED] flag on a form / item / option / unit. Mission
  // QC (the reviewer) and unrelated users see the chip as a
  // read-only status indicator — the delete affordance is dropped
  // entirely so they can't accidentally re-mark the owner as
  // submitted.
  const canClearNotSubmitted = isProjectLeader === true || isMissionDev;

  const createIssue = useCreateIssue();
  const patchIssueState = usePatchIssueState();
  const appendComment = useAppendComment();
  const setApproved = useSetCrfFormApproved();
  const qc = useQueryClient();

  // Approval chip — QC-only, blocked while open issues exist on
  // the form (cached count used for visual feedback only; the
  // authoritative gate lives in the Tauri command and
  // re-fetches the live count before calling the server).
  const openIssueCount = openIssueCountByTarget.get(null) ?? 0;
  const approvedDisabled =
    setApproved.isPending ||
    !isMissionQc ||
    (form !== undefined && form.approved === false && openIssueCount > 0);
  const approvedTooltipReason = !isMissionQc
    ? t("crf.toolbar.approveDisabled.notQc")
    : form !== undefined && form.approved === false && openIssueCount > 0
      ? t("crf.toolbar.approveDisabled.openIssues", { count: openIssueCount })
      : "";
  const handleToggleApproved = () => {
    if (!form || !formMission) return;
    setApproved.mutate({
      id: form.id,
      approved: !form.approved,
      missionId: formMission.id,
    });
  };

  // Surface a closable Alert when the Tauri command rejects
  // set_approved (e.g. open issues blocking approval). Closing
  // re-fetches the mission's issues so the chip's disabled state
  // stays in sync with the fresh count.
  const [approvalError, setApprovalError] = useState<string | null>(null);
  useEffect(() => {
    if (setApproved.isError && setApproved.error) {
      setApprovalError(errorMessage(setApproved.error));
    }
  }, [setApproved.isError, setApproved.error]);
  const dismissApprovalError = () => {
    setApprovalError(null);
    setApproved.reset();
    if (formMission) {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(formMission.id),
      });
    }
  };

  const [issueDialog, setIssueDialog] = useState<
    | { scope: IssueScope; missionId: number }
    | null
  >(null);

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
        {form?.code && (
          <Badge
            color="error"
            badgeContent={openIssueCountByTarget.get(null) ?? 0}
            invisible={!formMission}
            overlap="circular"
          >
            <Chip
              sx={{ minWidth: 70 }}
              size="small"
              label={form.code}
              variant="outlined"
              // No `onClick` when the chip should not open the
              // issue dialog (no mission / zero opened issues for a
              // viewer without permission) — silently drops the
              // click so the chip keeps the same outlined style as
              // the always-enabled path.
              onClick={
                formMission && canOpenEmptyIssueDialog
                  || (formMission && (openIssueCountByTarget.get(null) ?? 0) > 0)
                  ? () =>
                      setIssueDialog({
                        scope: { kind: "form" },
                        missionId: formMission!.id,
                      })
                  : undefined
              }
              // Stable anchor for `?focus=form-<id>` from the global
              // search page. Sits next to the form-name Typography so
              // scrolling here lands the user on the form header.
              data-testid={`crf-form-${id}`}
            />
          </Badge>
        )}
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
            // QC / task-unrelated users see the chip but can't
            // clear the flag — `onDelete` is omitted so MUI drops
            // the delete icon entirely.
            onDelete={
              canClearNotSubmitted
                ? () =>
                    updateOwnerNotSubmitted.mutate({
                      formId: id,
                      owner: { kind: "form", id },
                      notSubmitted: false,
                    })
                : undefined
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
                  : !canEditAnnotations
                    ? t("crf.detail.tooltip.noPermissionEdit")
                    : ""
              }
              disableHoverListener={
                !!form?.notSubmitted || !canEditAnnotations
              }
              disableFocusListener={
                !!form?.notSubmitted || !canEditAnnotations
              }
              disableTouchListener={
                !!form?.notSubmitted || !canEditAnnotations
              }
            >
              {/* `span` wrapper is required because MUI's disabled
                  MenuItem doesn't forward refs / props to a Tooltip
                  host — wrapping lets the tooltip track hover even
                  when the menu item itself is aria-disabled. */}
              <span>
                <MenuItem
                  disabled={Boolean(form?.notSubmitted) || !canEditAnnotations}
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
                    : !canEditAnnotations
                      ? t("crf.detail.tooltip.noPermissionEdit")
                      : ""
              }
              disableHoverListener={
                !!form?.notSubmitted ||
                noDomainAnnotations ||
                !canEditAnnotations
              }
              disableFocusListener={
                !!form?.notSubmitted ||
                noDomainAnnotations ||
                !canEditAnnotations
              }
              disableTouchListener={
                !!form?.notSubmitted ||
                noDomainAnnotations ||
                !canEditAnnotations
              }
            >
              <span>
                <MenuItem
                  disabled={
                    Boolean(form?.notSubmitted) ||
                    noDomainAnnotations ||
                    !canEditAnnotations
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
                onClick={
                  canEditAnnotations
                    ? () => setDomainDialog({ mode: "edit", row: d })
                    : undefined
                }
                onDelete={
                  canEditAnnotations
                    ? () => setConfirmDeleteDomain(d)
                    : undefined
                }
                size="small"
                data-testid={`domain-annotation-chip-${d.id}`}
                variant="outlined"
              />
            ))}
          </Stack>
        )}
        <Box sx={{ flexGrow: 1 }} />
        {form && (
          <Tooltip
            title={approvedTooltipReason}
            disableHoverListener={!approvedDisabled || !approvedTooltipReason}
          >
            <span>
              <Chip
                icon={
                  form.approved ? <VerifiedIcon /> : <PendingActionsIcon />
                }
                label={
                  form.approved
                    ? t("crf.toolbar.statusApproved")
                    : t("crf.toolbar.statusPending")
                }
                color={form.approved ? "success" : "warning"}
                variant="outlined"
                size="small"
                onClick={approvedDisabled ? undefined : handleToggleApproved}
                sx={
                  approvedDisabled
                    ? { opacity: 0.5, cursor: "not-allowed" }
                    : undefined
                }
                data-testid="crf-approval-toggle"
              />
            </span>
          </Tooltip>
        )}
        <CrfToolsMenu projectCode={projectCode} versionId={routeSearch.versionId ?? null} />
      </Box>

      {approvalError && (
        <Alert
          severity="warning"
          onClose={dismissApprovalError}
          data-testid="crf-approval-error"
        >
          {approvalError}
        </Alert>
      )}

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
          canEditAnnotations={canEditAnnotations}
          onEdit={(a) => openEditAnnotation(a, { kind: "form", id })}
          onDelete={(a) => setConfirmDeleteAnnotation(a)}
        />
      )}

      {/* Item list */}
      {detail && (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1, maxHeight: "calc(100vh - 160px)", overflowY: "auto" }}>
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
                canEditAnnotations={canEditAnnotations}
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
                openIssueCount={
                  openIssueCountByTarget.get(itemDetail.item.code) ?? 0
                }
                onOpenIssues={() =>
                  formMission &&
                  setIssueDialog({
                    scope: {
                      kind: "item",
                      itemId: itemDetail.item.id,
                      itemCode: itemDetail.item.code,
                    },
                    missionId: formMission.id,
                  })
                }
                missionExists={Boolean(formMission)}
                canOpenEmptyIssueDialog={canOpenEmptyIssueDialog}
                canClearNotSubmitted={canClearNotSubmitted}
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
        sdtmDomains={sdtmContext.domains}
        sdtmLanguage={sdtmContext.language}
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
        ownerItemCode={readOwnerItemCode(
          detail,
          annotationDialog
            ? annotationDialog.owner
            : { kind: "form", id },
        )}
        row={annotationDialog?.mode === "edit" ? annotationDialog.row : undefined}
        availableDomainAnnotations={detail?.domainAnnotations ?? []}
        sdtmDomains={sdtmContext.domains}
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

      {issueDialog && formMission && (
        <MissionIssueDialog
          open
          scope={issueDialog.scope}
          mission={formMission}
          resolveName={resolveName}
          issues={(issuesQuery.data ?? []).filter((i) => {
            if (issueDialog.scope.kind === "form") return i.targetItem == null;
            return i.targetItem === issueDialog.scope.itemCode;
          })}
          currentUser={currentUser}
          canCreate={canCreate}
          canActOnIssue={canActOnIssue}
          canComment={canComment}
          createPending={createIssue.isPending}
          createError={createIssue.error}
          onCreate={(description) =>
            createIssue.mutate(
              {
                missionId: issueDialog.missionId,
                body: {
                  description,
                  targetItem:
                    issueDialog.scope.kind === "item"
                      ? issueDialog.scope.itemCode
                      : undefined,
                },
              },
              { onSuccess: () => undefined },
            )
          }
          patchPending={patchIssueState.isPending}
          onPatchState={(issueId, next) =>
            patchIssueState.mutate({
              missionId: issueDialog.missionId,
              issueId,
              next,
            })
          }
          commentPending={appendComment.isPending}
          commentError={appendComment.error}
          onAppendComment={(issueId, content) =>
            appendComment.mutate({
              missionId: issueDialog.missionId,
              issueId,
              body: { content },
            })
          }
          onClose={() => setIssueDialog(null)}
        />
      )}
    </Box>
  );
}
