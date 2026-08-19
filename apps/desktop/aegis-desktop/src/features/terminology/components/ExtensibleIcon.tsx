import { Tooltip } from "@aegis/ui/mui";
import { NorthEast as NorthEastIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

export interface ExtensibleIconProps {
  visible: boolean;
}

/**
 * Renders a small ↗ icon after a code cell when the codelist /
 * code-item is extensible. Returns `null` when `visible` is false
 * so the icon is omitted from the layout entirely.
 */
export function ExtensibleIcon({ visible }: ExtensibleIconProps) {
  const { t } = useI18n();
  if (!visible) return null;
  return (
    <Tooltip title={t("terminology.extensible")}>
      <NorthEastIcon
        fontSize="small"
        aria-label={t("terminology.extensible")}
        sx={{ ml: 0.5, verticalAlign: "middle" }}
      />
    </Tooltip>
  );
}