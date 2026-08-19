import { useMemo, useState } from "react";
import {
  getRouteApi,
  useNavigate,
} from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Edit as EditIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { useCurrentUser } from "../../auth";
import {
  useCreateCodeItem,
  useDeleteCodeItem,
  useGetCodeList,
  useListCodeItems,
  useListTerminologyVersions,
  useUpdateCodeItem,
  useUpdateCodeList,
} from "../data";
import type {
  CodeItemView,
  CreateCodeItemInput,
  TerminologyKind,
  UpdateCodeItemInput,
  UpdateCodeListInput,
} from "../../../shared/api";
import { CodeItemDrawer } from "../components/CodeItemDrawer";
import { CodeItemTable } from "../components/CodeItemTable";
import { CodeListDrawer } from "../components/CodeListDrawer";
import { TermFilterBar } from "../components/TermFilterBar";

// Single route under `/_authed/_layout/terminology/$kind/...`
// services both SDTM and ADaM; the `$kind` param tells the page
// which terminology it belongs to.
const routeApi = getRouteApi(
  "/_authed/_layout/terminology/$kind/codelists/$codelistId",
);

type ItemDrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: CodeItemView }
  | null;

export function CodeListDetailPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();
  const kind = params.kind as TerminologyKind;
  const codelistId = Number(params.codelistId);
  const versionIdFromUrl = search.versionId;

  const currentUser = useCurrentUser();
  const versionsQuery = useListTerminologyVersions();
  const codelistQuery = useGetCodeList(codelistId);
  const itemsQuery = useListCodeItems(codelistId);

  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const codelist = codelistQuery.data;

  const versionId = codelist?.versionId ?? versionIdFromUrl ?? 0;
  const backLink = `/terminology/${kind}`;

  const [search2, setSearch2] = useState("");
  const [editCodelistDrawerOpen, setEditCodelistDrawerOpen] = useState(false);
  const [itemDrawer, setItemDrawer] = useState<ItemDrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<CodeItemView | null>(null);

  const updateCodelist = useUpdateCodeList();
  const createItem = useCreateCodeItem();
  const updateItem = useUpdateCodeItem();
  const deleteItem = useDeleteCodeItem();

  const items = itemsQuery.data ?? [];
  const trimmedQuery = search2.trim().toLowerCase();
  const filteredItems = useMemo<CodeItemView[]>(() => {
    if (!trimmedQuery) return items;
    return items.filter(
      (it) =>
        it.code.toLowerCase().includes(trimmedQuery) ||
        it.submissionValue.toLowerCase().includes(trimmedQuery) ||
        it.synonym.toLowerCase().includes(trimmedQuery) ||
        it.definition.toLowerCase().includes(trimmedQuery) ||
        it.nciPreferredTerm.toLowerCase().includes(trimmedQuery),
    );
  }, [items, trimmedQuery]);

  const mutationLoading =
    updateCodelist.isPending ||
    createItem.isPending ||
    updateItem.isPending ||
    deleteItem.isPending;

  const error = codelist ? null : (codelistQuery.error ?? itemsQuery.error);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <TableContainer component={Paper}>
        <Table size="small">
          <TableBody>
            <TableRow>
              <TableCell sx={{ width: 48 }}>
                <Tooltip title={t("common.back")}>
                  <span>
                    <IconButton
                      onClick={() => navigate({ to: backLink })}
                      disabled={!backLink}
                      aria-label={t("common.back")}
                    >
                      <ArrowBackIcon />
                    </IconButton>
                  </span>
                </Tooltip>
              </TableCell>
              {error && !codelist ? (
                <TableCell colSpan={4}>
                  <Alert severity="error">
                    {t("terminology.codeitem.loadFailed", {
                      message: errorMessage(error),
                    })}
                  </Alert>
                  <Box sx={{ mt: 1 }}>
                    <Button onClick={() => navigate({ to: backLink })}>
                      {t("common.back")}
                    </Button>
                  </Box>
                </TableCell>
              ) : codelist ? (
                <>
                  <TableCell>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                        {codelist.code}
                      </Typography>
                      {codelist.extensible && (
                        <Tooltip title={t("terminology.extensible")}>
                          <Chip label="EXT" size="small" />
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" color="textSecondary">
                      {codelist.name}
                    </Typography>
                  </TableCell>
                  <TableCell sx={{ color: "text.secondary" }}>
                    <Typography variant="body2">
                      {codelist.submissionValue || "—"}
                    </Typography>
                  </TableCell>
                  <TableCell sx={{ width: 64 }} align="right">
                    {canMutate && (
                      <Tooltip title={t("terminology.codelist.edit.title")}>
                        <IconButton
                          size="small"
                          aria-label={t("terminology.codelist.edit.title")}
                          onClick={() => setEditCodelistDrawerOpen(true)}
                          disabled={mutationLoading}
                        >
                          <EditIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                    )}
                  </TableCell>
                </>
              ) : null}
            </TableRow>
          </TableBody>
        </Table>
      </TableContainer>


      <TermFilterBar
        query={search2}
        onQueryChange={setSearch2}
        placeholder={t("terminology.codeitem.search.placeholder")}
      />

      <CodeItemTable
        rows={filteredItems}
        loading={itemsQuery.isLoading}
        mutationLoading={mutationLoading}
        error={itemsQuery.error}
        canMutate={canMutate}
        onRetry={itemsQuery.refetch}
        onCreate={() => setItemDrawer({ mode: "create" })}
        onEdit={(row) => setItemDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        emptyMessage={
          trimmedQuery
            ? t("terminology.codeitem.noMatches")
            : t("terminology.codeitem.empty")
        }
      />

      {codelist && (
        <CodeListDrawer
          open={editCodelistDrawerOpen}
          mode="edit"
          row={codelist}
          versions={versionsQuery.data ?? []}
          versionId={codelist.versionId}
          onClose={() => setEditCodelistDrawerOpen(false)}
          onCreate={() => {
            /* unreachable in edit mode */
          }}
          onUpdate={(_id, body: UpdateCodeListInput) =>
            updateCodelist.mutate(
              { id: codelist.id, body },
              { onSuccess: () => setEditCodelistDrawerOpen(false) },
            )
          }
          canMutate={canMutate}
          mutationError={updateCodelist.error}
          mutationPending={updateCodelist.isPending}
        />
      )}

      <CodeItemDrawer
        open={itemDrawer !== null}
        mode={itemDrawer?.mode ?? "create"}
        row={itemDrawer?.mode === "edit" ? itemDrawer.row : undefined}
        codelistId={codelistId}
        versionId={versionId}
        onClose={() => setItemDrawer(null)}
        onCreate={(input: CreateCodeItemInput) =>
          createItem.mutate(input, {
            onSuccess: () => setItemDrawer(null),
          })
        }
        onUpdate={(id, body: UpdateCodeItemInput) =>
          updateItem.mutate(
            { id, body },
            { onSuccess: () => setItemDrawer(null) },
          )
        }
        canMutate={canMutate}
        mutationError={createItem.error ?? updateItem.error}
        mutationPending={createItem.isPending || updateItem.isPending}
      />

      <Dialog
        open={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
      >
        <DialogTitle>
          {t("terminology.codeitem.action.delete.confirmTitle")}
        </DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("terminology.codeitem.action.delete.confirmMessage")}
          </DialogContentText>
          {deleteItem.isError && (
            <DialogContentText sx={{ mt: 2, color: "error.main" }}>
              {errorMessage(deleteItem.error)}
            </DialogContentText>
          )}
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setConfirmDelete(null)}
            disabled={deleteItem.isPending}
          >
            {t("common.cancel")}
          </Button>
          <Button
            color="error"
            onClick={() => {
              if (!confirmDelete) return;
              deleteItem.mutate(
                { id: confirmDelete.id, codelistId: confirmDelete.codelistId },
                { onSuccess: () => setConfirmDelete(null) },
              );
            }}
            disabled={deleteItem.isPending}
          >
            {t("common.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}