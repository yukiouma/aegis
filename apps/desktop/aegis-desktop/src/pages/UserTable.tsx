import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  FormControlLabel,
  Paper,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { ApiError, UserView } from "../api";
import { errorMessage } from "../api/error";

export interface UserTableProps {
  rows: UserView[];
  loading: boolean;
  mutationLoading: boolean;
  error: ApiError | null;
  selfCode: string | null;
  onToggle: (code: string, nextActive: boolean) => void;
  onRetry: () => void;
}

/**
 * Renders the user list as a MUI Table. The Switch in the active
 * column is disabled on the row matching `selfCode` (cannot
 * deactivate yourself) and on every row while a mutation is in
 * flight.
 */
export function UserTable({
  rows,
  loading,
  mutationLoading,
  error,
  selfCode,
  onToggle,
  onRetry,
}: UserTableProps) {
  const { t } = useI18n();

  if (error) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("user.loadFailed", { message: errorMessage(error) })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
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
              <TableCell>{t("user.field.code")}</TableCell>
              <TableCell>{t("user.field.name")}</TableCell>
              <TableCell>{t("user.field.role")}</TableCell>
              <TableCell>{t("user.field.active")}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => {
              const isSelf = row.code === selfCode;
              const disabled = isSelf || mutationLoading;
              return (
                <TableRow key={row.id} hover>
                  <TableCell>{row.code}</TableCell>
                  <TableCell>{row.name}</TableCell>
                  <TableCell>
                    <Chip
                      variant="outlined"
                      size="small"
                      label={t(`user.role.${row.role}`)}
                    />
                  </TableCell>
                  <TableCell>
                    <Tooltip
                      title={
                        isSelf
                          ? t("user.cannotDeactivateSelf")
                          : t(row.active ? "user.active" : "user.inactive")
                      }
                    >
                      <span>
                        <FormControlLabel
                          sx={{ ml: 0 }}
                          control={
                            <Switch
                              size="small"
                              checked={row.active}
                              disabled={disabled}
                              onChange={(e) =>
                                onToggle(row.code, e.target.checked)
                              }
                            />
                          }
                          label=""
                        />
                      </span>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="textSecondary">{t("user.empty")}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}