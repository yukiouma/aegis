import { FormControl, InputLabel, MenuItem, Select } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export interface LanguageDropdownProps {
  options: string[];
  value: string | null;
  onChange: (lang: string | null) => void;
  disabled?: boolean;
}

export function LanguageDropdown({
  options,
  value,
  onChange,
  disabled,
}: LanguageDropdownProps) {
  const { t } = useI18n();
  const empty = options.length === 0;
  return (
    <FormControl size="small" sx={{ minWidth: 160 }} disabled={disabled || empty}>
      <InputLabel id="domain-model-lang-label">
        {t("domainModel.sdtm.lang.label")}
      </InputLabel>
      <Select<string | null>
        labelId="domain-model-lang-label"
        label={t("domainModel.sdtm.lang.label")}
        value={value ?? ""}
        onChange={(e) =>
          onChange(e.target.value === "" ? null : String(e.target.value))
        }
      >
        {options.map((code) => (
          <MenuItem key={code} value={code}>
            {code}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}