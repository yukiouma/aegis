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

/**
 * Move `sourceId` to `targetId`'s slot in the ordered id sequence, shifting
 * the target and other rows as needed. Returns `null` when the move is a
 * no-op (source === target, or either id is missing from the sequence).
 *
 * The insertion index is the *original* index of the target (computed before
 * source is removed). That index lands on the source in the post-removal
 * array regardless of which side of the target the source started on, so this
 * works for both "drag down" and "drag up" cases:
 *   [1, 2, 3, 4], src=1, tgt=3 → [2, 3, 1, 4]  (target shifts right)
 *   [1, 2, 3, 4], src=4, tgt=2 → [1, 4, 2, 3]  (target stays put)
 */
export function computeReorder(
  orderedIds: readonly number[],
  sourceId: number,
  targetId: number,
): number[] | null {
  if (sourceId === targetId) return null;
  const next = [...orderedIds];
  const srcIdx = next.indexOf(sourceId);
  const tgtIdx = next.indexOf(targetId);
  if (srcIdx < 0 || tgtIdx < 0) return null;
  const [moved] = next.splice(srcIdx, 1);
  next.splice(tgtIdx, 0, moved);
  return next;
}

/**
 * Adapter from a `@dnd-kit/react` `dragend` event to `computeReorder`.
 *
 * Reads the dragged row from `event.operation.source` — NOT
 * `event.operation.target`, which is the drop slot. Respecting
 * `event.canceled` keeps the table stable when the drag is aborted.
 * Returns `null` when there is nothing to reorder.
 */
export function applyReorder(
  orderedIds: readonly number[],
  event: {
    canceled: boolean;
    operation: {
      source: { id: string | number } | null;
      target: { id: string | number } | null;
    };
  },
): number[] | null {
  if (event.canceled) return null;
  const source = event.operation.source;
  const target = event.operation.target;
  if (source == null || target == null) return null;
  const sourceId = Number(source.id);
  const targetId = Number(target.id);
  if (!Number.isFinite(sourceId) || !Number.isFinite(targetId)) return null;
  return computeReorder(orderedIds, sourceId, targetId);
}

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