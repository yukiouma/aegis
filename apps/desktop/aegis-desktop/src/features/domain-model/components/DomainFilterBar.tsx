import { TextField } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export interface DomainFilterBarProps {
  query: string;
  onQueryChange: (next: string) => void;
}

export function DomainFilterBar({ query, onQueryChange }: DomainFilterBarProps) {
  const { t } = useI18n();
  return (
    <TextField
      size="small"
      label={t("domainModel.sdtm.filter.placeholder")}
      value={query}
      onChange={(e) => onQueryChange(e.target.value)}
      sx={{ minWidth: 280 }}
    />
  );
}