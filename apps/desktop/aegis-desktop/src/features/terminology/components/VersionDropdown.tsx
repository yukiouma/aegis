import { FormControl, InputLabel, MenuItem, Select } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type {
  TerminologyKind,
  TerminologyVersionView,
} from "../../../shared/api";

export interface VersionDropdownProps {
  kind: TerminologyKind;
  versions: TerminologyVersionView[];
  value: number | null;
  onChange: (id: number | null) => void;
  disabled?: boolean;
}

/**
 * `<Select>` of terminology versions filtered by `kind`. Disabled
 * with helper text when the filtered list is empty; the parent
 * page renders the empty-state message in the table area.
 */
export function VersionDropdown({
  kind,
  versions,
  value,
  onChange,
  disabled,
}: VersionDropdownProps) {
  const { t } = useI18n();
  const filtered = versions.filter((v) => v.kind === kind);
  const empty = filtered.length === 0;

  return (
    <FormControl size="small" sx={{ minWidth: 220 }}>
      <InputLabel id={`version-label-${kind}`}>
        {empty ? t("terminology.version.placeholder") : t("terminology.version.helper")}
      </InputLabel>
      <Select<number | null>
        labelId={`version-label-${kind}`}
        value={value}
        label={
          empty
            ? t("terminology.version.placeholder")
            : t("terminology.version.helper")
        }
        onChange={(e) => onChange(e.target.value as number | null)}
        disabled={disabled || empty}
      >
        {filtered.map((v) => (
          <MenuItem key={v.id} value={v.id}>
            {`${v.kind.toUpperCase()} — ${v.name}`}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}