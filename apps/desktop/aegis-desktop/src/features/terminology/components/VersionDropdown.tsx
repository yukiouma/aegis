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
 *
 * The label is rendered as a sibling `<InputLabel>` (no `label` prop
 * on `<Select>`) so MUI's FormControl automatically notches the
 * label up to the top edge once a value is selected, instead of
 * overlapping the chosen text in the middle of the field.
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
  const labelText = empty
    ? t("terminology.version.placeholder")
    : t("terminology.version.helper");

  return (
    <FormControl size="small" sx={{ minWidth: 220 }} disabled={disabled || empty}>
      <InputLabel id={`version-label-${kind}`}>{labelText}</InputLabel>
      <Select<number | null>
        labelId={`version-label-${kind}`}
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