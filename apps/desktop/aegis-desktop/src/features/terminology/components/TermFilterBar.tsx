import { InputAdornment, TextField } from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

export interface TermFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  placeholder?: string;
}

/**
 * Search field for the terminology pages. Pure controlled component
 * — the page owns the query state. When `placeholder` is omitted
 * the default codelist placeholder is used; code-item pages pass
 * their own.
 */
export function TermFilterBar({
  query,
  onQueryChange,
  placeholder,
}: TermFilterBarProps) {
  const { t } = useI18n();
  return (
    <TextField
      size="small"
      placeholder={
        placeholder ?? t("terminology.codelist.search.placeholder")
      }
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
      sx={{ flex: 1 }}
    />
  );
}