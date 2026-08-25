import { FormControl, InputLabel, MenuItem, Select } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import type { SdtmVersionView } from "../../../shared/api";

export interface VersionDropdownProps {
  versions: SdtmVersionView[];
  value: number | null;
  onChange: (id: number | null) => void;
  disabled?: boolean;
}

export function VersionDropdown({
  versions,
  value,
  onChange,
  disabled,
}: VersionDropdownProps) {
  const { t } = useI18n();
  const empty = versions.length === 0;
  const labelText = empty
    ? t("domainModel.sdtm.version.placeholder")
    : t("domainModel.sdtm.version.label");

  return (
    <FormControl size="small" sx={{ minWidth: 220 }} disabled={disabled || empty}>
      <InputLabel id="domain-model-version-label">{labelText}</InputLabel>
      <Select<number | null>
        labelId="domain-model-version-label"
        label={labelText}
        value={value ?? ""}
        onChange={(e) =>
          onChange(e.target.value === "" ? null : Number(e.target.value))
        }
      >
        {versions.map((v) => (
          <MenuItem key={v.id} value={v.id}>
            {v.name}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}