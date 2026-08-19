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
  const labelText = empty
    ? t("terminology.version.placeholder")
    : t("terminology.version.helper");

  return (
    <FormControl size="small" sx={{ minWidth: 220 }} disabled={disabled || empty}>
      <InputLabel id={`version-label-${kind}`}>{labelText}</InputLabel>
      <Select<number | null>
        labelId={`version-label-${kind}`}
        label={labelText}
        value={value ?? ""}
        onChange={(e) =>
          onChange(
            e.target.value === "" ? null : Number(e.target.value),
          )
        }
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