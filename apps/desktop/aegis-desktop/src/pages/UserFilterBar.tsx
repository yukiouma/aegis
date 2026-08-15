import { InputAdornment, TextField } from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

export interface UserFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
}

/**
 * Search field for the User management page. Pure controlled
 * component — `UserList` owns the query state. The leading Search
 * icon signals purpose; the placeholder hints at the match
 * semantics (name or code, case-insensitive).
 */
export function UserFilterBar({
  query,
  onQueryChange,
}: UserFilterBarProps) {
  const { t } = useI18n();
  return (
    <TextField
      size="small"
      placeholder={t("user.search.placeholder")}
      value={query}
      onChange={(e) => onQueryChange(e.target.value)}
      slotProps={{
        input: {
          startAdornment: (
            <InputAdornment position="start">
              <SearchIcon fontSize="small" />
            </InputAdornment>
          ),
        },
      }}
      sx={{ minWidth: 320 }}
    />
  );
}
