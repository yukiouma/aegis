import {
  Box,
  Button,
  Checkbox,
  Chip,
  Drawer,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export type CrfStatusFilter = "approved" | "pending";

interface Props {
  open: boolean;
  searchInput: string;
  onSearchInputChange: (value: string) => void;
  statusSelected: CrfStatusFilter[];
  onStatusSelectedChange: (value: CrfStatusFilter[]) => void;
  involvedChecked: boolean;
  onInvolvedCheckedChange: (value: boolean) => void;
  onClear: () => void;
  onApply: () => void;
}

/**
 * Right-anchored filter drawer. Search text, status multi-select,
 * and the "Involved" checkbox are all wired to page-owned state
 * and applied via the page's `filteredRows` selector.
 */
export function CrfFormFilterDrawer({
  open,
  searchInput,
  onSearchInputChange,
  statusSelected,
  onStatusSelectedChange,
  involvedChecked,
  onInvolvedCheckedChange,
  onClear,
  onApply,
}: Props) {
  const { t } = useI18n();
  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onApply}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">{t("crf.filter.title")}</Typography>
        <TextField
          size="small"
          label={t("crf.filter.search")}
          value={searchInput}
          onChange={(e) => onSearchInputChange(e.target.value)}
        />
        <FormControl size="small">
          <InputLabel id="crf-filter-status-label">
            {t("crf.filter.status")}
          </InputLabel>
          <Select
            labelId="crf-filter-status-label"
            label={t("crf.filter.status")}
            multiple
            value={statusSelected}
            onChange={(e) => {
              const v = e.target.value;
              onStatusSelectedChange(
                Array.isArray(v) ? (v as CrfStatusFilter[]) : [],
              );
            }}
            renderValue={(selected) => (
              <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
                {selected.map((s) => (
                  <Chip
                    key={s}
                    label={t(
                      s === "approved"
                        ? "crf.filter.status.approved"
                        : "crf.filter.status.pending",
                    )}
                    size="small"
                  />
                ))}
              </Box>
            )}
          >
            <MenuItem value="approved">
              {t("crf.filter.status.approved")}
            </MenuItem>
            <MenuItem value="pending">
              {t("crf.filter.status.pending")}
            </MenuItem>
          </Select>
        </FormControl>
        <FormControlLabel
          control={
            <Checkbox
              checked={involvedChecked}
              onChange={(e) => onInvolvedCheckedChange(e.target.checked)}
            />
          }
          label={t("crf.filter.involved")}
        />
        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClear}>{t("common.clear")}</Button>
          <Button variant="contained" onClick={onApply}>
            {t("common.apply")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}