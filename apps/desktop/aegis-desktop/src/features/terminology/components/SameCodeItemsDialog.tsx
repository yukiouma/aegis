import { useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogContent,
  DialogTitle,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { Close as CloseIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  CodeItemView,
  CodeListView,
  TerminologyKind,
} from "../../../shared/api";
import {
  useGetCodeListsByIds,
  useListCodeItemsByVersionAndCode,
} from "../data";

interface JoinedRow {
  item: CodeItemView;
  codelist: CodeListView | undefined;
  isCurrent: boolean;
}

export interface SameCodeItemsDialogProps {
  open: boolean;
  code: string | null;
  versionId: number;
  currentCodelistId: number;
  kind: TerminologyKind;
  onClose: () => void;
}

/**
 * Lists every code item that shares the same `code` value across the
 * given terminology version, regardless of which codelist it lives in.
 * Each row carries its owning codelist's `code` and `submissionValue`
 * and is a click-target that navigates to that codelist's detail page.
 *
 * Owns its own queries and navigation glue — the parent page only
 * controls `open` / `code` / `onClose`.
 */
export function SameCodeItemsDialog({
  open,
  code,
  versionId,
  currentCodelistId,
  kind,
  onClose,
}: SameCodeItemsDialogProps) {
  const { t } = useI18n();
  const navigate = useNavigate();

  const enabled = open && !!code;
  const itemsQuery = useListCodeItemsByVersionAndCode(
    versionId,
    enabled ? code : null,
  );

  const codelistIds = useMemo(() => {
    if (!itemsQuery.data) return [];
    const ids = itemsQuery.data.items.map((i) => i.codelistId);
    return Array.from(new Set(ids));
  }, [itemsQuery.data]);

  const codelistQueries = useGetCodeListsByIds(codelistIds);

  const rows = useMemo<JoinedRow[]>(() => {
    if (!itemsQuery.data) return [];
    return itemsQuery.data.items.map((item) => {
      const idx = codelistIds.indexOf(item.codelistId);
      const codelist = codelistQueries[idx]?.data;
      return {
        item,
        codelist,
        isCurrent: item.codelistId === currentCodelistId,
      };
    });
  }, [itemsQuery.data, codelistQueries, codelistIds, currentCodelistId]);

  const handleRowClick = (codelistId: number) => {
    onClose();
    void navigate({
      to: "/terminology/$kind/codelists/$codelistId",
      params: { kind, codelistId },
      search: { versionId },
    });
  };

  const title = code
    ? t("terminology.codeitem.sameCode.dialogTitle", { code })
    : "";

  return (
    <Dialog open={open && !!code} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Box component="span" sx={{ flex: 1 }}>
          {title}
        </Box>
        <Tooltip title={t("common.close")}>
          <IconButton
            size="small"
            aria-label={t("common.close")}
            onClick={onClose}
          >
            <CloseIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </DialogTitle>
      <DialogContent dividers>
        {itemsQuery.isLoading ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : itemsQuery.isError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {t("terminology.codeitem.loadFailed", {
                message: errorMessage(itemsQuery.error),
              })}
            </Alert>
            <Box>
              <Button onClick={() => void itemsQuery.refetch()}>
                {t("common.retry")}
              </Button>
            </Box>
          </Box>
        ) : rows.length === 0 ? (
          <Typography
            color="text.secondary"
            sx={{ py: 4, textAlign: "center" }}
          >
            {t("terminology.codeitem.sameCode.empty")}
          </Typography>
        ) : (
          <TableContainer
            component={Paper}
            sx={{ maxHeight: "calc(100vh - 220px)" }}
          >
            <Table size="small" stickyHeader>
              <TableHead>
                <TableRow>
                  <TableCell>{t("terminology.codeitem.field.code")}</TableCell>
                  <TableCell>
                    {t("terminology.codeitem.field.submissionValue")}
                  </TableCell>
                  <TableCell>{t("terminology.codelist.field.code")}</TableCell>
                  <TableCell>
                    {t("terminology.codelist.field.submissionValue")}
                  </TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows.map(({ item, codelist, isCurrent }) => (
                  <TableRow
                    key={item.id}
                    hover
                    onClick={() => handleRowClick(item.codelistId)}
                    sx={{
                      cursor: "pointer",
                      bgcolor: isCurrent ? "action.hover" : undefined,
                    }}
                  >
                    <TableCell>{item.code}</TableCell>
                    <TableCell>{item.submissionValue}</TableCell>
                    <TableCell>{codelist?.code ?? "—"}</TableCell>
                    <TableCell>{codelist?.submissionValue ?? "—"}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        )}
      </DialogContent>
    </Dialog>
  );
}
