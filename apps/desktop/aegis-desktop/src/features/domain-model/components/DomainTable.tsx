import {
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
  DeleteOutlined as DeleteOutlinedIcon,
  OpenInNew as OpenInNewIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
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
}: DomainTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Paper sx={{ p: 2 }}>
        <Typography color="error">{String(error)}</Typography>
      </Paper>
    );
  }

  if (rows.length === 0) {
    return (
      <Paper sx={{ p: 4, textAlign: "center" }}>
        <Typography>{emptyMessage}</Typography>
      </Paper>
    );
  }

  return (
    <TableContainer component={Paper}>
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell>{t("domainModel.sdtm.col.name")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.description")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.structure")}</TableCell>
            <TableCell>{t("domainModel.sdtm.col.category")}</TableCell>
            <TableCell />
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
                <TableCell sx={{ whiteSpace: "nowrap" }}>
                  <Tooltip title={t("domainModel.sdtm.action.navigate.tooltip")}>
                    <span>
                      <IconButton size="small" disabled aria-label="open-detail">
                        <OpenInNewIcon fontSize="small" />
                      </IconButton>
                    </span>
                  </Tooltip>
                  {canMutate && (
                    <Tooltip title={t("domainModel.sdtm.action.delete.tooltip")}>
                      <IconButton
                        size="small"
                        aria-label="delete-domain"
                        onClick={() => onDelete(row)}
                      >
                        <DeleteOutlinedIcon fontSize="small" />
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