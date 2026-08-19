import { FormControl, MenuItem, Select } from "@aegis/ui/mui";
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
 * The `label` prop is on `<Select>` only — NOT paired with a separate
 * `<InputLabel>` child — so MUI manages the float / shrink state
 * itself. Pairing both causes the label to overlap the selected
 * text once a value is chosen. `displayEmpty` keeps the floating
 * label visible while the user hasn't picked a version yet.
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
      <Select<number | null>
        label={labelText}
        value={value ?? ""}
        displayEmpty
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