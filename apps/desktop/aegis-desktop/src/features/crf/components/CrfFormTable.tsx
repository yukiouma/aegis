import {
  Box,
  Chip,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
} from "@aegis/ui/mui";
import {
  Add as AddIcon,
  AssignmentInd as AssignmentIndIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  FilterList as FilterListIcon,
  Launch as LaunchIcon,
  PendingActions as PendingActionsIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import type { CrfForm } from "../../../shared/api";

interface Props {
  rows: CrfForm[];
  loading: boolean;
  error: unknown;
  canAddFilter: boolean;
  onAdd: () => void;
  onFilter: () => void;
  onAssignTakers: (row: CrfForm) => void;
  onEdit: (row: CrfForm) => void;
  onDelete: (row: CrfForm) => void;
  onOpenDetail: (row: CrfForm) => void;
}

export function CrfFormTable({
  rows,
  loading,
  error,
  canAddFilter,
  onAdd,
  onFilter,
  onAssignTakers,
  onEdit,
  onDelete,
  onOpenDetail,
}: Props) {
  const { t } = useI18n();
  return (
    <TableContainer component={Paper}>
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell>{t("crf.table.column.code")}</TableCell>
            <TableCell>{t("crf.table.column.name")}</TableCell>
            <TableCell>{t("crf.table.column.taker")}</TableCell>
            <TableCell>{t("crf.table.column.status")}</TableCell>
            <TableCell align="right">
              <Tooltip title={t("crf.table.action.addForm")}>
                <IconButton
                  size="small"
                  aria-label={t("crf.table.action.addForm")}
                  onClick={onAdd}
                >
                  <AddIcon />
                </IconButton>
              </Tooltip>
              <Tooltip title={t("crf.table.action.filter")}>
                <IconButton
                  size="small"
                  aria-label={t("crf.table.action.filter")}
                  onClick={onFilter}
                  disabled={!canAddFilter}
                >
                  <FilterListIcon />
                </IconButton>
              </Tooltip>
            </TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.length === 0 && !loading && !error && (
            <TableRow>
              <TableCell colSpan={5} align="center">
                <Box sx={{ py: 3, color: "text.secondary" }}>
                  {t("common.noData")}
                </Box>
              </TableCell>
            </TableRow>
          )}
          {rows.map((row) => (
            <TableRow key={row.id} hover>
              <TableCell>{row.code}</TableCell>
              <TableCell>{row.name}</TableCell>
              <TableCell />
              <TableCell>
                <Chip
                  icon={<PendingActionsIcon />}
                  label={t("crf.toolbar.statusPending")}
                  size="small"
                  color="warning"
                  variant="outlined"
                />
              </TableCell>
              <TableCell align="right">
                <Tooltip title={t("crf.table.action.assignTakers")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.assignTakers")}
                    onClick={() => onAssignTakers(row)}
                  >
                    <AssignmentIndIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.edit")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.edit")}
                    onClick={() => onEdit(row)}
                  >
                    <EditIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.delete")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.delete")}
                    onClick={() => onDelete(row)}
                  >
                    <DeleteIcon />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t("crf.table.action.openDetail")}>
                  <IconButton
                    size="small"
                    aria-label={t("crf.table.action.openDetail")}
                    onClick={() => onOpenDetail(row)}
                  >
                    <LaunchIcon />
                  </IconButton>
                </Tooltip>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
}