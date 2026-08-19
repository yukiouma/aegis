import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { useCurrentUser } from "../../auth";
import {
  useCreateCodeList,
  useDeleteCodeList,
  useListCodeLists,
  useListTerminologyVersions,
  useUpdateCodeList,
} from "../data";
import type {
  CodeListView,
  CreateCodeListInput,
  TerminologyKind,
  UpdateCodeListInput,
} from "../../../shared/api";
import { CodeListDrawer } from "../components/CodeListDrawer";
import { CodeListTable } from "../components/CodeListTable";
import { ImportButton } from "../components/ImportButton";
import { TermFilterBar } from "../components/TermFilterBar";
import { VersionDropdown } from "../components/VersionDropdown";

type DrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CodeListView }
  | null;

export interface TerminologyPageProps {
  kind: TerminologyKind;
}

export function TerminologyPage({ kind }: TerminologyPageProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const versionsQuery = useListTerminologyVersions();
  const [selectedVersionId, setSelectedVersionId] = useState<number | null>(
    null,
  );
  const [search, setSearch] = useState("");
  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeListView | null>(null);

  const versions = versionsQuery.data ?? [];
  const versionsForKind = versions.filter((v) => v.kind === kind);

  // Initialise the selected version whenever the matching list
  // transitions from empty to non-empty (or when the kind changes).
  useEffect(() => {
    if (selectedVersionId == null && versionsForKind.length > 0) {
      setSelectedVersionId(versionsForKind[0].id);
    } else if (
      selectedVersionId != null &&
      !versionsForKind.some((v) => v.id === selectedVersionId)
    ) {
      // The previously-selected version is no longer in the list
      // (e.g. kind changed). Fall back to the first match.
      setSelectedVersionId(versionsForKind[0]?.id ?? null);
    }
  }, [versionsForKind, selectedVersionId]);

  const codeListsQuery = useListCodeLists(selectedVersionId);
  const createCodeList = useCreateCodeList();
  const updateCodeList = useUpdateCodeList();
  const deleteCodeList = useDeleteCodeList();

  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const rows = codeListsQuery.data ?? [];

  const trimmedQuery = search.trim().toLowerCase();
  const filteredRows = useMemo<CodeListView[]>(() => {
    if (!trimmedQuery) return rows;
    return rows.filter(
      (r) =>
        r.code.toLowerCase().includes(trimmedQuery) ||
        r.name.toLowerCase().includes(trimmedQuery) ||
        r.submissionValue.toLowerCase().includes(trimmedQuery) ||
        r.synonym.toLowerCase().includes(trimmedQuery) ||
        r.definition.toLowerCase().includes(trimmedQuery) ||
        r.nciPreferredTerm.toLowerCase().includes(trimmedQuery),
    );
  }, [rows, trimmedQuery]);

  const mutationLoading =
    createCodeList.isPending ||
    updateCodeList.isPending ||
    deleteCodeList.isPending;

  const error = codeListsQuery.error;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box
        sx={{
          display: "flex",
          gap: 2,
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <TermFilterBar query={search} onQueryChange={setSearch} />
        <VersionDropdown
          kind={kind}
          versions={versions}
          value={selectedVersionId}
          onChange={setSelectedVersionId}
        />
        <ImportButton />
      </Box>

      <CodeListTable
        mode="list"
        rows={filteredRows}
        loading={codeListsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={error}
        canMutate={canMutate}
        onRetry={codeListsQuery.refetch}
        onCreate={() => setDrawer({ mode: "create" })}
        onDelete={(row) => setConfirmDelete(row)}
        onOpen={(row) => {
          void navigate({
            to: "/terminology/$kind/codelists/$codelistId",
            params: { kind, codelistId: row.id },
          });
        }}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codelist.noMatches")
            : t("terminology.codelist.empty")
        }
      />

      <CodeListDrawer
        open={drawer !== null}
        mode={drawer?.mode ?? "create"}
        row={drawer?.mode === "edit" ? drawer.row : undefined}
        versions={versions}
        versionId={selectedVersionId ?? 0}
        onClose={() => setDrawer(null)}
        onCreate={(input: CreateCodeListInput) =>
          createCodeList.mutate(input, {
            onSuccess: () => setDrawer(null),
          })
        }
        onUpdate={(id, body: UpdateCodeListInput) =>
          updateCodeList.mutate(
            { id, body },
            { onSuccess: () => setDrawer(null) },
          )
        }
        canMutate={canMutate}
        mutationError={createCodeList.error ?? updateCodeList.error}
        mutationPending={createCodeList.isPending || updateCodeList.isPending}
      />

      <Dialog
        open={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
      >
        <DialogTitle>
          {t("terminology.action.delete.confirmTitle")}
        </DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("terminology.action.delete.confirmMessage")}
          </DialogContentText>
          {deleteCodeList.isError && (
            <DialogContentText sx={{ mt: 2, color: "error.main" }}>
              {errorMessage(deleteCodeList.error)}
            </DialogContentText>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)} disabled={deleteCodeList.isPending}>
            {t("common.cancel")}
          </Button>
          <Button
            color="error"
            onClick={() => {
              if (!confirmDelete) return;
              deleteCodeList.mutate(
                { id: confirmDelete.id, versionId: confirmDelete.versionId },
                { onSuccess: () => setConfirmDelete(null) },
              );
            }}
            disabled={deleteCodeList.isPending}
          >
            {t("common.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}