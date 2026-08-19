import {
  Alert,
  Box,
  Button,
  Chip,
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
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, CodeListView } from "../../../shared/api";
import { DescriptionsCell } from "./DescriptionsCell";

/**
 * `mode: "list"` — used by the Terminology list page. Renders the
 * `+` header button and per-row edit / delete / open icons (each
 * gated on `canMutate`).
 *
 * `mode: "single"` — used by the detail page to render exactly one
 * row with edit only; the header `+` button is never rendered.
 */
export type CodeListTableProps =
  | {
      mode: "list";
      rows: CodeListView[];
      loading: boolean;
      mutationLoading: boolean;
      error: ApiError | null;
      canMutate: boolean;
      onRetry: () => void;
      onCreate: () => void;
      onEdit: (row: CodeListView) => void;
      onDelete: (row: CodeListView) => void;
      onOpen: (row: CodeListView) => void;
      emptyMessage?: string;
    }
  | {
      mode: "single";
      rows: CodeListView[];
      loading: boolean;
      mutationLoading: boolean;
      error: ApiError | null;
      canMutate: boolean;
      onRetry: () => void;
      onEdit: (row: CodeListView) => void;
      emptyMessage?: string;
    };

export function CodeListTable(props: CodeListTableProps) {
  const { t } = useI18n();
  const isList = props.mode === "list";
  const showSpinner = props.loading && props.rows.length === 0;
  const emptyMessage =
    ("emptyMessage" in props ? props.emptyMessage : undefined) ??
    t("terminology.codelist.empty");

  if (props.error && props.rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codelist.loadFailed", {
            message: errorMessage(props.error),
          })}
        </Alert>
        <Box>
          <Button onClick={props.onRetry}>{t("common.retry")}</Button>
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
              <TableCell>{t("terminology.codelist.field.code")}</TableCell>
              <TableCell>{t("terminology.codelist.field.name")}</TableCell>
              <TableCell>
                {t("terminology.codelist.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codelist.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: isList ? 140 : 64 }} align="right">
                {isList && props.canMutate ? (
                  <Tooltip
                    title={t("terminology.codelist.create.title")}
                  >
                    <IconButton
                      size="small"
                      aria-label={t("terminology.codelist.create.title")}
                      onClick={props.onCreate}
                      disabled={props.mutationLoading}
                    >
                      <AddIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                ) : null}
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {props.rows.map((row) => {
              const disabled = props.mutationLoading;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <span>{row.code}</span>
                      {row.extensible && (
                        <Tooltip title={t("terminology.extensible")}>
                          <Chip label="EXT" size="small" />
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>{row.name}</TableCell>
                  <TableCell>{row.submissionValue}</TableCell>
                  <TableCell>
                    <DescriptionsCell
                      synonym={row.synonym}
                      definition={row.definition}
                      nciPreferredTerm={row.nciPreferredTerm}
                    />
                  </TableCell>
                  <TableCell align="right">
                    <Box
                      sx={{
                        display: "flex",
                        gap: 0.5,
                        justifyContent: "flex-end",
                      }}
                    >
                      {props.canMutate && (
                        <Tooltip
                          title={t("terminology.codelist.edit.title")}
                        >
                          <IconButton
                            size="small"
                            aria-label={t("terminology.codelist.edit.title")}
                            onClick={() => props.onEdit(row)}
                            disabled={disabled}
                          >
                            <EditIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      )}
                      {isList && props.canMutate && (
                        <Tooltip
                          title={t("terminology.codelist.field.code")}
                        >
                          <IconButton
                            size="small"
                            aria-label={`open ${row.code}`}
                            onClick={() => props.onOpen(row)}
                            disabled={disabled}
                          >
                            <LaunchIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      )}
                      {isList && props.canMutate && (
                        <Tooltip
                          title={t(
                            "terminology.action.delete.confirmTitle",
                          )}
                        >
                          <IconButton
                            size="small"
                            color="error"
                            aria-label={`delete ${row.code}`}
                            onClick={() => props.onDelete(row)}
                            disabled={disabled}
                          >
                            <DeleteIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      )}
                    </Box>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && props.rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{emptyMessage}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}