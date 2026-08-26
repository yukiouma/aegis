import { useMemo, useState } from "react";
import {
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
  DragIndicator as DragIndicatorIcon,
  Edit as EditIcon,
} from "@aegis/ui/icons";
import {
  DragDropProvider,
  useDraggable,
  useDroppable,
} from "@aegis/ui/dnd";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { SdtmVariableView, SdtmVariableCore } from "../../../shared/api";

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
 * `event.canceled` keeps the table stable when the drag is aborted. Returns
 * `null` when there is nothing to reorder.
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

export interface VariableTableProps {
  rows: SdtmVariableView[];
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  onRetry: () => void;
  onCreate: () => void;
  onEdit: (row: SdtmVariableView) => void;
  onDelete: (row: SdtmVariableView) => void;
  onReorder: (orderedIds: number[]) => void;
  emptyMessage: string;
}

const TYPE_CHIP: Record<SdtmVariableView["variableType"], string> = {
  Numeric: "N",
  Character: "C",
};

const cellEllipsis = {
  whiteSpace: "nowrap" as const,
  overflow: "hidden",
  textOverflow: "ellipsis",
  maxWidth: 360,
};

interface DraggableRowProps {
  row: SdtmVariableView;
  canMutate: boolean;
  selectedLang: string | null;
  onEdit: (r: SdtmVariableView) => void;
  onDelete: (r: SdtmVariableView) => void;
}

function variableCoreColor(core: SdtmVariableCore): "error" | "warning" | "success" | "info" {
  switch (core) {
    case "Exp":
      return "warning";
    case "Req":
      return "error";
    case "Perm":
      return "success";
    default:
      return "info";
  }
}

function DraggableRow({
  row,
  canMutate,
  selectedLang,
  onEdit,
  onDelete,
}: DraggableRowProps) {
  const { t } = useI18n();
  const draggable = useDraggable({ id: String(row.id), type: "variable" });
  const droppable = useDroppable({ id: String(row.id), accept: "variable" });
  const label =
    selectedLang == null
      ? ""
      : (row.descriptions.find((d) => d.lang === selectedLang)?.details.label ??
        "");
  const role = row.variableRole ?? "—";
  return (
    <TableRow
      ref={(el: HTMLTableRowElement | null) => {
        if (el && draggable.ref) draggable.ref(el);
        if (el && droppable.ref) droppable.ref(el);
      }}
    >
      <TableCell sx={{ width: 40 }}>
        <DragIndicatorIcon
          fontSize="small"
          sx={{ cursor: "grab", opacity: 0.6 }}
          aria-label={`drag-${row.name}`}
        />
      </TableCell>
      <TableCell>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <span>{row.name}</span>
        </Box>
      </TableCell>
      <TableCell>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <Chip size="small" label={TYPE_CHIP[row.variableType]} variant={row.variableType === "Character" ? "filled" : "outlined"} />
          <Chip
            color={variableCoreColor(row.variableCore)}
            variant="outlined"
            size="small"
            label={t(`domainModel.sdtm.variable.core.${row.variableCore}`)}
          />
        </Box>
      </TableCell>
      <TableCell sx={cellEllipsis} title={label}>
        {label}
      </TableCell>
      <TableCell>{role}</TableCell>
      <TableCell sx={{ whiteSpace: "nowrap" }} align="right">
        {canMutate && (
          <>
            <Tooltip title={t("domainModel.sdtm.variable.editTitle")}>
              <IconButton
                size="small"
                aria-label={`edit variable ${row.name}`}
                onClick={() => onEdit(row)}
              >
                <EditIcon fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip
              title={t("domainModel.sdtm.variable.delete.confirmTitle")}
            >
              <IconButton
                size="small"
                aria-label={`delete variable ${row.name}`}
                color="error"
                onClick={() => onDelete(row)}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </>
        )}
      </TableCell>
    </TableRow>
  );
}

export function VariableTable({
  rows,
  loading,
  error,
  canMutate,
  selectedLang,
  onRetry,
  onCreate,
  onEdit,
  onDelete,
  onReorder,
  emptyMessage,
}: VariableTableProps) {
  const { t } = useI18n();
  const [internalOrder, setInternalOrder] = useState<number[] | null>(null);

  const orderedIds = useMemo(() => {
    if (internalOrder) return internalOrder;
    return rows.map((r) => r.id);
  }, [rows, internalOrder]);

  if (error) {
    return (
      <Paper sx={{ p: 2 }}>
        <Typography color="error">
          {t("domainModel.sdtm.detail.variablesLoadFailed", {
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
    <DragDropProvider
      onDragEnd={(event) => {
        const next = applyReorder(orderedIds, event);
        if (next == null) return;
        setInternalOrder(next);
        onReorder(next);
      }}
    >
      <TableContainer component={Paper} sx={{ maxHeight: "calc(100vh - 200px)" }}>
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell />
              <TableCell>{t("domainModel.sdtm.detail.col.name")}</TableCell>
              <TableCell></TableCell>
              <TableCell>{t("domainModel.sdtm.detail.col.label")}</TableCell>
              <TableCell>{t("domainModel.sdtm.detail.col.role")}</TableCell>
              <TableCell align="right">
                {canMutate && (
                  <Tooltip
                    title={t("domainModel.sdtm.variable.create.tooltip")}
                  >
                    <IconButton
                      size="small"
                      aria-label={t(
                        "domainModel.sdtm.variable.create.tooltip",
                      )}
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
            {orderedIds.map((id) => {
              const row = rows.find((r) => r.id === id);
              if (!row) return null;
              return (
                <DraggableRow
                  key={row.id}
                  row={row}
                  canMutate={canMutate}
                  selectedLang={selectedLang}
                  onEdit={onEdit}
                  onDelete={onDelete}
                />
              );
            })}
          </TableBody>
        </Table>
      </TableContainer>
    </DragDropProvider>
  );
}