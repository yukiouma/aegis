import {
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
  OpenInNew as OpenInNewIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { errorMessage } from "../../../shared/api/error";
import type { SdtmDomainView } from "../../../shared/api";

export interface DomainTableProps {
  rows: SdtmDomainView[];
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  onRetry: () => void;
  onDelete: (row: SdtmDomainView) => void;
  emptyMessage: string;
  /**
   * Optional click handler for the open-detail button. When provided the
   * button is enabled; when omitted it stays disabled with a "coming soon"
   * tooltip so older callers don't need to wire navigation just to render
   * the table.
   */
  onNavigate?: (row: SdtmDomainView) => void;
  onCreate?: () => void;
}

const cellEllipsis = {
  whiteSpace: "nowrap" as const,
  overflow: "hidden",
  textOverflow: "ellipsis",
  maxWidth: 360,
};

export function DomainTable({
  rows,
  loading,
  error,
  canMutate,
  selectedLang,
  onRetry,
  onDelete,
  emptyMessage,
  onNavigate,
  onCreate,
}: DomainTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Paper sx={{ p: 2 }}>
        <Typography color="error">
          {t("domainModel.sdtm.loadFailed", {
            message: errorMessage(error),
          })}
        </Typography>
        <Button onClick={onRetry} sx={{ mt: 1 }}>
          {t("common.retry")}
        </Button>
      </Paper>
    );
  }

  if (rows.length === 0) {
    if (loading) {
      return (
        <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
          <CircularProgress />
        </Box>
      );
    }
    return (
      <Paper sx={{ p: 4, textAlign: "center" }}>
        <Typography>{emptyMessage}</Typography>
      </Paper>
    );
  }

  return (
    <TableContainer component={Paper} sx={{ maxHeight: "calc(100vh - 120px)" }}>
      <Table size="small" stickyHeader>
        <TableHead >
          <TableRow>
            <TableCell>{t("domainModel.sdtm.col.name")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.description")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.structure")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.category")}</TableCell>
            <TableCell align="right">
              {canMutate && onCreate && (
                <Tooltip title={t("domainModel.sdtm.create.tooltip")}>
                  <IconButton
                    size="small"
                    aria-label={t("domainModel.sdtm.create.tooltip")}
                    onClick={onCreate}
                  >
                    <AddIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              )}
            </TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((row) => {
            const d = selectedLang
              ? row.descriptions.find((x) => x.lang === selectedLang)
              : undefined;
            const description = d?.details.description ?? "";
            const structure = d?.details.structure ?? "";
            return (
              <TableRow key={row.id}>
                <TableCell>{row.name}</TableCell>
                <TableCell sx={cellEllipsis} title={description}>
                  {description}
                </TableCell>
                <TableCell sx={cellEllipsis} title={structure}>
                  {structure}
                </TableCell>
                <TableCell>{row.category}</TableCell>
                <TableCell sx={{ whiteSpace: "nowrap" }} align="right">
                  <Tooltip title={t("domainModel.sdtm.action.navigate.tooltip")}>
                    <span>
                      <IconButton
                        size="small"
                        disabled={!onNavigate}
                        aria-label="open-detail"
                        onClick={() => onNavigate?.(row)}
                      >
                        <OpenInNewIcon fontSize="small" />
                      </IconButton>
                    </span>
                  </Tooltip>
                  {canMutate && (
                    <Tooltip title={t("domainModel.sdtm.action.delete.tooltip")}>
                      <IconButton
                        size="small"
                        aria-label="delete-domain"
                        color="error"
                        onClick={() => onDelete(row)}
                      >
                        <DeleteIcon fontSize="small" />
                      </IconButton>
                    </Tooltip>
                  )}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </TableContainer>
  );
}