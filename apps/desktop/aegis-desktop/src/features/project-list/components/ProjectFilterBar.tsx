import {
  Box,
  Checkbox,
  FormControlLabel,
  InputAdornment,
  TextField,
} from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

export interface ProjectFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  involve: boolean;
  onInvolveChange: (value: boolean) => void;
}

/**
 * Search field + Involve toggle. Pure controlled component — the
 * orchestrator owns the state. The search field matches across code,
 * description, leaders, and tag values (see `ProjectListPage`'s
 * filter); it stays enabled even when no current user is loaded,
 * and toggling Involve with no user just produces an empty result.
 */
export function ProjectFilterBar({
  query,
  onQueryChange,
  involve,
  onInvolveChange,
}: ProjectFilterBarProps) {
  const { t } = useI18n();

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
      <TextField
        size="small"
        placeholder={t("project.search.label")}
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        slotProps={{
          input: {
            startAdornment: (
              <InputAdornment position="start">
                <SearchIcon fontSize="small" />
              </InputAdornment>
            ),
          },
        }}
        sx={{ minWidth: 420 }}
      />
      <FormControlLabel
        sx={{ ml: "auto" }}
        control={
          <Checkbox
            checked={involve}
            onChange={(event) => onInvolveChange(event.target.checked)}
          />
        }
        label={t("project.involve")}
      />
    </Box>
  );
}
