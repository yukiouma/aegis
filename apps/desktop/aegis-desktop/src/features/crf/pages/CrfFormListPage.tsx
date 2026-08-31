import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  Box,
  CircularProgress,
  IconButton,
  Tooltip,
} from "@aegis/ui/mui";
import { NoteAdd as NoteAddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import {
  useNavigate,
  useParams,
  useSearch,
} from "@tanstack/react-router";

import {
  CrfAssignTakersDrawer,
  CrfFormDrawer,
  CrfFormFilterDrawer,
  type CrfStatusFilter,
  CrfFormTable,
  CrfToolsMenu,
  CrfStatusChip,
  CrfVersionDropdown,
  DeleteCrfFormDialog,
} from "../components";
import {
  useCreateCrfForm,
  useDeleteCrfForm,
  useListCrfForms,
  useListCrfVersions,
  useUpdateCrfForm,
} from "../data/list";
import type { CrfForm } from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";

type DrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CrfForm }
  | null;

/**
 * Splice a new visible-row order into the full row order, preserving the
 * position of rows that aren't in the visible set. Used by `handleReorder`
 * so that dropping a row on a filtered list only repositions the rows the
 * user can see.
 *
 * Defensive guards:
 *   - if `newVisibleIds` runs short of `visibleRows`, the missing slots fall
 *     back to the original row id at that position.
 *   - if `newVisibleIds` is longer than `visibleRows`, only the first
 *     `visibleRows.length` entries are consumed.
 */
export function computeNewFullOrder(
  allRows: CrfForm[],
  newVisibleIds: number[],
  visibleRows: CrfForm[],
): number[] {
  const visibleIds = new Set(visibleRows.map((r) => r.id));
  const out: number[] = [];
  let cursor = 0;
  for (const row of allRows) {
    if (visibleIds.has(row.id)) {
      const id =
        cursor < newVisibleIds.length
          ? newVisibleIds[cursor++]
          : row.id;
      out.push(id);
    } else {
      out.push(row.id);
    }
  }
  return out;
}

export function CrfFormListPage() {
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const navigate = useNavigate();
  const { t } = useI18n();
  const routeSearch = useSearch({ strict: false }) as { versionId?: number };
  const selectedVersionId =
    typeof routeSearch.versionId === "number" && routeSearch.versionId > 0
      ? routeSearch.versionId
      : null;

  const versionsQuery = useListCrfVersions(projectCode);
  const versions = versionsQuery.data ?? [];

  // Reconcile ?versionId URL ↔ first version fallback.
  useEffect(() => {
    if (versions.length === 0) return;
    const valid =
      selectedVersionId != null &&
      versions.some((v) => v.id === selectedVersionId);
    if (!valid) {
      navigate({
        to: "/project/$projectCode/crf",
        params: { projectCode },
        search: { versionId: versions[0].id },
        replace: true,
      });
    }
  }, [versions, selectedVersionId, projectCode, navigate]);

  const formsQuery = useListCrfForms(selectedVersionId);
  const allRows = formsQuery.data ?? [];

  // Page-owned filter state (drawer is fully controlled).
  const [searchInput, setSearchInput] = useState("");
  const [statusSelected, setStatusSelected] = useState<CrfStatusFilter[]>([]);
  const [involvedChecked] = useState(false);

  // Inline debounce: 300 ms delay, no max-wait.
  const [debouncedSearch, setDebouncedSearch] = useState("");
  useEffect(() => {
    const handle = setTimeout(() => setDebouncedSearch(searchInput), 300);
    return () => clearTimeout(handle);
  }, [searchInput]);

  const filteredRows = useMemo(() => {
    const q = debouncedSearch.trim().toLowerCase();
    return allRows.filter((r) => {
      // statusSelected + involvedChecked are held but no-op this PR
      void statusSelected;
      void involvedChecked;
      return (
        q === "" ||
        r.code.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q)
      );
    });
  }, [allRows, debouncedSearch, statusSelected, involvedChecked]);

  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CrfForm | null>(null);
  const [assignTakersFor, setAssignTakersFor] = useState<CrfForm | null>(null);
  const [filterOpen, setFilterOpen] = useState(false);

  const createMutation = useCreateCrfForm();
  const updateMutation = useUpdateCrfForm();
  const deleteMutation = useDeleteCrfForm();

  const activeMutationError =
    createMutation.error ??
    updateMutation.error ??
    deleteMutation.error ??
    null;
  const activeMutationPending =
    createMutation.isPending ||
    updateMutation.isPending ||
    deleteMutation.isPending;

  const handleReorder = useCallback(
    (newVisibleIds: number[]) => {
      const oldFullIds = allRows.map((r) => r.id);
      const newFullIds = computeNewFullOrder(allRows, newVisibleIds, filteredRows);
      newFullIds.forEach((id, newIndex) => {
        if (oldFullIds.indexOf(id) !== newIndex) {
          updateMutation.mutate({ id, body: { order: newIndex + 1 } });
        }
      });
    },
    [allRows, filteredRows, updateMutation],
  );

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}><Box
      sx={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        flexWrap: "wrap",
        gap: 2,
      }}
    >
      <Tooltip title={t("crf.toolbar.createVersion")}>
        <IconButton
          size="small"
          aria-label={t("crf.toolbar.createVersion")}
          onClick={() =>
            navigate({
              to: "/project/$projectCode/crf/versions/new",
              params: { projectCode },
            })
          }
        >
          <NoteAddIcon />
        </IconButton>
      </Tooltip>
      <CrfVersionDropdown
        versions={versions}
        value={selectedVersionId}
        onChange={(versionId) =>
          navigate({
            to: "/project/$projectCode/crf",
            params: { projectCode },
            search: { versionId },
          })
        }
        disabled={versions.length === 0}
      />
      <CrfStatusChip />
      <Box sx={{ flexGrow: 1 }} />
      <CrfToolsMenu projectCode={projectCode} versionId={selectedVersionId} />
    </Box>

      {versionsQuery.isError && (
        <Alert severity="error">{errorMessage(versionsQuery.error)}</Alert>
      )}
      {formsQuery.isError && (
        <Alert severity="error">{errorMessage(formsQuery.error)}</Alert>
      )}
      {formsQuery.isFetching && !formsQuery.data && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      <CrfFormTable
        rows={filteredRows}
        loading={formsQuery.isFetching}
        error={formsQuery.error}
        canAddFilter={selectedVersionId != null}
        onAdd={() => setDrawer({ mode: "create" })}
        onFilter={() => setFilterOpen(true)}
        onAssignTakers={(row) => setAssignTakersFor(row)}
        onEdit={(row) => setDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpenDetail={(row) =>
          navigate({
            to: "/project/$projectCode/crf/$formId",
            params: { projectCode, formId: String(row.id) },
            // Carry the selected version onto the detail URL so the
            // back navigation in `CrfDetailPage` can hand it back to
            // the list via `search: (prev) => prev`. Without this the
            // detail page's search is empty and the list falls back
            // to `versions[0]` on remount.
            search: { versionId: selectedVersionId ?? undefined },
          })
        }
        onReorder={handleReorder}
      />

      <CrfFormDrawer
        open={drawer != null}
        mode={drawer?.mode ?? "create"}
        row={drawer?.mode === "edit" ? drawer.row : undefined}
        onClose={() => setDrawer(null)}
        onCreate={(input) => {
          if (selectedVersionId == null) return;
          input.order = allRows.length + 1;
          createMutation.mutate(
            { versionId: selectedVersionId, body: input },
            { onSuccess: () => setDrawer(null) },
          );
        }}
        onUpdate={(id, body) => {
          updateMutation.mutate(
            { id, body },
            { onSuccess: () => setDrawer(null) },
          );
        }}
        mutationError={activeMutationError}
        mutationPending={activeMutationPending}
      />

      <DeleteCrfFormDialog
        open={confirmDelete != null}
        row={confirmDelete}
        onClose={() => setConfirmDelete(null)}
        onConfirm={(row) => {
          deleteMutation.mutate(
            { id: row.id, versionId: row.versionId },
            { onSuccess: () => setConfirmDelete(null) },
          );
        }}
        mutationError={deleteMutation.error}
        mutationPending={deleteMutation.isPending}
      />

      <CrfAssignTakersDrawer
        open={assignTakersFor != null}
        onClose={() => setAssignTakersFor(null)}
      />

      <CrfFormFilterDrawer
        open={filterOpen}
        searchInput={searchInput}
        onSearchInputChange={setSearchInput}
        statusSelected={statusSelected}
        onStatusSelectedChange={setStatusSelected}
        onClear={() => {
          setSearchInput("");
          setStatusSelected([]);
        }}
        onApply={() => setFilterOpen(false)}
      />
    </Box>
  );
}