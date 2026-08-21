import {
  Alert,
  Box,
  Button,
  CircularProgress,
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
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useCallback, useState, type ReactNode } from "react";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, CodeItemView } from "../../../shared/api";
import { DescriptionsCell } from "./DescriptionsCell";

export interface CodeItemTableProps {
  rows: CodeItemView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  canMutate: boolean;
  onRetry: () => void;
  onCreate: () => void;
  onEdit: (row: CodeItemView) => void;
  onDelete: (row: CodeItemView) => void;
  emptyMessage?: string;
  /** Rendered inside the scroll container, after the Table. Receives the
   *  scroll container element so callers can wire IntersectionObserver
   *  roots that observe scroll-within-table. */
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
  /** Click handler for the code cell. When provided, the code renders
   *  with a pointer cursor, hover underline, and tooltip. When omitted,
   *  the cell renders identically to a non-interactive cell. */
  onCodeClick?: (row: CodeItemView) => void;
}

export function CodeItemTable({
  rows,
  loading,
  mutationLoading,
  error,
  canMutate,
  onRetry,
  onCreate,
  onEdit,
  onDelete,
  emptyMessage,
  bottomSlot,
  onCodeClick,
}: CodeItemTableProps) {
  const { t } = useI18n();
  const showSpinner = loading && rows.length === 0;
  const msg = emptyMessage ?? t("terminology.codeitem.empty");

  // Capture the TableContainer's DOM element via a callback ref so we can
  // hand it to `bottomSlot`. The state update forces a re-render when the
  // element is committed, after which `bottomSlot` receives a non-null value.
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const containerRefCallback = useCallback((node: HTMLDivElement | null) => {
    setScrollEl(node);
  }, []);

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codeitem.loadFailed", {
            message: errorMessage(error),
          })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      <TableContainer
        component={Paper}
        ref={containerRefCallback}
        sx={{ maxHeight: "calc(100vh - 200px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codeitem.field.code")}</TableCell>
              <TableCell>
                {t("terminology.codeitem.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codeitem.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 110 }} align="right">
                {canMutate ? (
                  <Tooltip title={t("terminology.codeitem.create.title")}>
                    <IconButton
                      size="small"
                      aria-label={t("terminology.codeitem.create.title")}
                      onClick={onCreate}
                      disabled={mutationLoading}
                    >
                      <AddIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                ) : null}
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => {
              const disabled = mutationLoading;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>
                    <Tooltip
                      title={t("terminology.codeitem.codeClick.tooltip")}
                      disableInteractive
                    >
                      <Box
                        component="span"
                        onClick={
                          onCodeClick ? () => onCodeClick(row) : undefined
                        }
                        sx={{
                          cursor: onCodeClick ? "pointer" : "default",
                          "&:hover": onCodeClick
                            ? { textDecoration: "underline" }
                            : undefined,
                          display: "inline-block",
                        }}
                      >
                        {row.code}
                      </Box>
                    </Tooltip>
                  </TableCell>
                  <TableCell>{row.submissionValue}</TableCell>
                  <TableCell>
                    <DescriptionsCell
                      synonym={row.synonym}
                      definition={row.definition}
                      nciPreferredTerm={row.nciPreferredTerm}
                    />
                  </TableCell>
                  <TableCell align="right">
                    {canMutate && (
                      <Box
                        sx={{
                          display: "flex",
                          gap: 0.5,
                          justifyContent: "flex-end",
                        }}
                      >
                        <Tooltip title={t("terminology.codeitem.edit.title")}>
                          <IconButton
                            size="small"
                            aria-label={t("terminology.codeitem.edit.title")}
                            onClick={() => onEdit(row)}
                            disabled={disabled}
                          >
                            <EditIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                        <Tooltip
                          title={t(
                            "terminology.codeitem.action.delete.confirmTitle",
                          )}
                        >
                          <IconButton
                            size="small"
                            color="error"
                            aria-label={`delete ${row.code}`}
                            onClick={() => onDelete(row)}
                            disabled={disabled}
                          >
                            <DeleteIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      </Box>
                    )}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{msg}</Typography>
          </Box>
        )}
        {bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}