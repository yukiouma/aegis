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
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useCallback, useState, type ReactNode } from "react";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, CodeListView } from "../../../shared/api";
import { DescriptionsCell } from "./DescriptionsCell";

/**
 * `mode: "list"` — used by the Terminology list page. Renders the
 * `+` header button and per-row launch + delete icons (each gated
 * on `canMutate`). The edit affordance lives on the detail page's
 * codelist header, not on the table.
 */
export type CodeListTableProps = {
  mode: "list";
  rows: CodeListView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  canMutate: boolean;
  onRetry: () => void;
  onCreate: () => void;
  onDelete: (row: CodeListView) => void;
  onOpen: (row: CodeListView) => void;
  emptyMessage?: string;
  /** Rendered inside the scroll container, after the Table. Receives the
   *  scroll container element so callers can wire IntersectionObserver
   *  roots that observe scroll-within-table. */
  bottomSlot?: (scrollEl: HTMLElement | null) => ReactNode;
};

export function CodeListTable(props: CodeListTableProps) {
  const { t } = useI18n();
  const showSpinner = props.loading && props.rows.length === 0;
  const emptyMessage = props.emptyMessage ?? t("terminology.codelist.empty");

  // Capture the TableContainer's DOM element via a callback ref so we can
  // hand it to `bottomSlot`. The state update forces a re-render when the
  // element is committed, after which `bottomSlot` receives a non-null value.
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const containerRefCallback = useCallback((node: HTMLDivElement | null) => {
    setScrollEl(node);
  }, []);

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

      <TableContainer
        component={Paper}
        ref={containerRefCallback}
        sx={{ maxHeight: "calc(100vh - 120px)" }}
      >
        <Table size="small" stickyHeader>
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
              <TableCell sx={{ width: 110 }} align="right">
                {props.canMutate ? (
                  <Tooltip title={t("terminology.codelist.create.title")}>
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
                      <Tooltip title={t("terminology.codelist.field.code")}>
                        <IconButton
                          size="small"
                          aria-label={`open ${row.code}`}
                          onClick={() => props.onOpen(row)}
                          disabled={disabled}
                        >
                          <LaunchIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                      {props.canMutate && (
                        <Tooltip
                          title={t("terminology.action.delete.confirmTitle")}
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
        {props.bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}