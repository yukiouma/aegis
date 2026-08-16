import {
  Box,
  Checkbox,
  FormControlLabel,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export interface ProjectFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  involve: boolean;
  onInvolveChange: (value: boolean) => void;
}

/**
 * Search field + Involve toggle. Pure controlled component — the
 * orchestrator owns the state. The search field stays enabled even
 * when no current user is loaded; toggling Involve with no user just
 * produces an empty result.
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
        label={t("project.search.label")}
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        sx={{ minWidth: 320 }}
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