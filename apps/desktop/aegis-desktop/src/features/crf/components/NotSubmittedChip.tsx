import { Chip } from "@aegis/ui/mui";

interface Props {
  /**
   * Fires when the user clicks the chip's built-in delete icon.
   * Callers should clear the owner-level `notSubmitted` flag back
   * to `false` (no cascade needed — the cascade only runs on a
   * `false → true` transition). When unset, the chip renders
   * without a delete affordance.
   */
  onDelete?: (event: React.MouseEvent) => void;
}

/**
 * Hard-coded English "[NOT SUBMITTED]" tag rendered next to a
 * form / item / option / unit when its `notSubmitted` flag is
 * true. The label is intentionally not localised — the chip
 * reads as a system flag rather than user-facing copy.
 */
export function NotSubmittedChip({ onDelete }: Props) {
  return (
    <Chip
      label="[NOT SUBMITTED]"
      variant="outlined"
      size="small"
      data-testid="not-submitted-chip"
      onDelete={onDelete}
    />
  );
}