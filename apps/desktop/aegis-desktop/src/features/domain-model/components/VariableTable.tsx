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
import type { SdtmVariableView } from "../../../shared/api";

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
          <Chip size="small" label={TYPE_CHIP[row.variableType]} />
          <Chip
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
      onDragEnd={(event: {
        operation: { target: { id: string | number } | null };
      }) => {
        const targetId =
          event.operation.target == null
            ? null
            : Number(event.operation.target.id);
        if (targetId == null || Number.isNaN(targetId)) return;
        const next = orderedIds.filter((id) => id !== targetId);
        const insertAt = next.length;
        next.splice(insertAt, 0, targetId);
        setInternalOrder(next);
        onReorder(next);
      }}
    >
      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell />
              <TableCell>{t("domainModel.sdtm.detail.col.name")}</TableCell>
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