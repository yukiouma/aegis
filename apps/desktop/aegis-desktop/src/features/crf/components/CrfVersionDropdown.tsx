import { FormControl, InputLabel, MenuItem } from "@aegis/ui/mui";
import { Select } from "@aegis/ui/mui";
import type { CrfVersion } from "../../../shared/api";

interface Props {
  versions: CrfVersion[];
  value: number | null;
  onChange: (versionId: number) => void;
  disabled?: boolean;
}

/**
 * Select dropdown of CRF versions. Disabled when there are no
 * versions yet; placeholder shown when value is null.
 */
export function CrfVersionDropdown({
  versions,
  value,
  onChange,
  disabled,
}: Props) {
  return (
    <FormControl size="small" sx={{ minWidth: 200 }} disabled={disabled}>
      <InputLabel id="crf-version-select-label">Version</InputLabel>
      <Select<number | "">
        labelId="crf-version-select-label"
        label="Version"
        value={value ?? ""}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (Number.isFinite(v) && v > 0) onChange(v);
        }}
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