import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Box,
  CircularProgress,
} from "@aegis/ui/mui";
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
  CrfGlobalSearchButton,
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

export function CrfFormListPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const navigate = useNavigate();
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
      <CrfGlobalSearchButton projectCode={projectCode} />
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
          })
        }
      />

      <CrfFormDrawer
        open={drawer != null}
        mode={drawer?.mode ?? "create"}
        row={drawer?.mode === "edit" ? drawer.row : undefined}
        onClose={() => setDrawer(null)}
        onCreate={(input) => {
          if (selectedVersionId == null) return;
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