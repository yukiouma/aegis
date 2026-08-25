import { TextField } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export interface DomainFilterBarProps {
  query: string;
  onQueryChange: (next: string) => void;
  /**
   * Override the default label/placeholder i18n key. The default
   * `domainModel.sdtm.filter.placeholder` is "Filter by name or description",
   * used by the SDTM list page. The detail page uses
   * `domainModel.sdtm.detail.filter.placeholder` ("Filter by name or label").
   */
  placeholderKey?: string;
}

export function DomainFilterBar({
  query,
  onQueryChange,
  placeholderKey,
}: DomainFilterBarProps) {
  const { t } = useI18n();
  return (
    <TextField
      size="small"
      label={t(placeholderKey ?? "domainModel.sdtm.filter.placeholder")}
      value={query}
      onChange={(e) => onQueryChange(e.target.value)}
      sx={{ flex: 1 }}
    />
  );
}