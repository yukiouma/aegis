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
}: CodeItemTableProps) {
  const { t } = useI18n();
  const showSpinner = loading && rows.length === 0;
  const msg = emptyMessage ?? t("terminology.codeitem.empty");

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

      <TableContainer component={Paper}>
        <Table size="small">
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
                  <TableCell>{row.code}</TableCell>
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
      </TableContainer>
    </Box>
  );
}