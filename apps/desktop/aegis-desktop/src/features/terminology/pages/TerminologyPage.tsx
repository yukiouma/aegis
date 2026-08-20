import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
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

  // The selected version lives in the URL (`?versionId=…`) so it
  // survives drilling into a code list and clicking the back arrow.
  // Both `/_authed/_layout/terminology/sdtm` and the `adam` sibling
  // declare the same `versionId` search schema. `strict: false`
  // mirrors `ImportTerminologyPage` and keeps the page renderable in
  // tests that don't register the route.
  const routeSearch = useSearch({ strict: false }) as { versionId?: number };
  const urlVersionId = routeSearch.versionId;

  const [search, setSearch] = useState("");
  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeListView | null>(null);

  const versions = versionsQuery.data ?? [];
  const versionsForKind = versions.filter((v) => v.kind === kind);

  // Effective selection: trust the URL when it points to a known
  // version, otherwise fall back to the first version for this kind.
  // Keeping this derived (instead of mirrored in local state) means
  // the dropdown is automatically correct on remount.
  const selectedVersionId = useMemo<number | null>(() => {
    if (
      urlVersionId != null &&
      versionsForKind.some((v) => v.id === urlVersionId)
    ) {
      return urlVersionId;
    }
    return versionsForKind[0]?.id ?? null;
  }, [urlVersionId, versionsForKind]);

  // Hydrate the URL whenever the effective selection and the URL
  // disagree: this covers the very first visit (no `versionId` in the
  // URL yet) and the stale-version case (the version was deleted or
  // doesn't belong to this kind). `replace: true` keeps the back stack
  // clean — this is just a hydration step, not a user-driven change.
  useEffect(() => {
    if (versionsForKind.length === 0) return;
    const urlIsValid =
      urlVersionId != null &&
      versionsForKind.some((v) => v.id === urlVersionId);
    if (urlIsValid) return;
    const fallback = versionsForKind[0].id;
    const to =
      kind === "sdtm"
        ? "/terminology/sdtm"
        : "/terminology/adam";
    void navigate({
      to,
      replace: true,
      search: { versionId: fallback },
    });
  }, [urlVersionId, versionsForKind, kind, navigate]);

  const setSelectedVersionId = (id: number | null) => {
    const to =
      kind === "sdtm"
        ? "/terminology/sdtm"
        : "/terminology/adam";
    void navigate({ to, search: { versionId: id ?? undefined } });
  };

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
        <ImportButton kind={kind} />
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
          // Forward `versionId` so the detail page's back arrow has it
          // to send the user back to. Without this the URL becomes
          // `/terminology/$kind/codelists/$id` (no search) and the
          // detail page can't preserve the selection on the way back.
          void navigate({
            to: "/terminology/$kind/codelists/$codelistId",
            params: { kind, codelistId: row.id },
            search:
              selectedVersionId != null
                ? { versionId: selectedVersionId }
                : undefined,
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