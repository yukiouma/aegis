import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Stack,
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
  Add,
  Cancel,
  CheckCircle,
  Edit,
  OpenInNew,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { ApiError, ProjectView } from "../api";
import { errorMessage } from "../api/error";

export interface ProjectTableProps {
  rows: ProjectView[];
  loading: boolean;
  error: ApiError | null;
  canEdit: boolean;
  onOpenCreate: () => void;
  onOpenEdit: (code: string) => void;
  onOpenWorkspace: (code: string) => void;
}

/**
 * Renders the project list as a MUI Table. The leader chip arrays
 * distinguish members (outlined) from unblindMembers (filled); the
 * active column uses CheckCircle/Cancel; the operation column gates
 * Add/Edit on `canEdit` and always renders the future OpenInNew as
 * disabled.
 */
export function ProjectTable({
  rows,
  loading,
  error,
  canEdit,
  onOpenCreate,
  onOpenEdit,
  onOpenWorkspace,
}: ProjectTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Alert severity="error">
        {t("project.loadFailed", { message: errorMessage(error) })}
      </Alert>
    );
  }

  const showSpinner = loading && rows.length === 0;

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
              <TableCell>{t("project.field.code")}</TableCell>
              <TableCell>{t("project.field.description")}</TableCell>
              <TableCell>{t("project.col.leaders")}</TableCell>
              <TableCell>{t("project.col.active")}</TableCell>
              <TableCell align="right">
                {canEdit ? (
                  <IconButton
                    aria-label={t("project.add")}
                    onClick={onOpenCreate}
                  >
                    <Add />
                  </IconButton>
                ) : null}
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => {
              const memberLeaders = row.members.leaders;
              const unblindLeaders = row.unblindMembers.leaders;
              const noLeaders =
                memberLeaders.length === 0 && unblindLeaders.length === 0;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>{row.code}</TableCell>
                  <TableCell sx={{ maxWidth: 280 }}>
                    <Typography noWrap>{row.description}</Typography>
                  </TableCell>
                  <TableCell>
                    <Stack
                      direction="row"
                      spacing={0.5}
                      sx={{ flexWrap: "wrap", gap: 0.5 }}
                    >
                      {memberLeaders.map((u) => (
                        <Chip
                          key={`m-${u.code}`}
                          variant="outlined"
                          size="small"
                          label={u.name}
                        />
                      ))}
                      {unblindLeaders.map((u) => (
                        <Chip
                          key={`u-${u.code}`}
                          variant="filled"
                          size="small"
                          label={u.name}
                        />
                      ))}
                      {noLeaders && <span>—</span>}
                    </Stack>
                  </TableCell>
                  <TableCell>
                    <Tooltip
                      title={t(row.active ? "project.active" : "project.inactive")}
                    >
                      <span>
                        {row.active ? (
                          <CheckCircle color="success" />
                        ) : (
                          <Cancel color="disabled" />
                        )}
                      </span>
                    </Tooltip>
                  </TableCell>
                  <TableCell align="right">
                    <Stack
                      direction="row"
                      spacing={0.5}
                      sx={{ justifyContent: "flex-end" }}
                    >
                      {canEdit && (
                        <IconButton
                          aria-label={t("project.edit")}
                          onClick={() => onOpenEdit(row.code)}
                        >
                          <Edit />
                        </IconButton>
                      )}
                      <IconButton
                        aria-label={t("project.open")}
                        onClick={() => onOpenWorkspace(row.code)}
                      >
                        <OpenInNew />
                      </IconButton>
                    </Stack>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{t("project.empty")}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}