import { useNavigate } from "@tanstack/react-router";
import { IconButton, Tooltip } from "@aegis/ui/mui";
import { Add as AddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { TerminologyKind } from "../../../shared/api";

export interface ImportButtonProps {
  /** Kind whose terminology page the user is on; passed via ?kind= */
  kind: TerminologyKind;
}

/**
 * Opens the Import Terminology page for the current kind. The destination
 * route (`/terminology/import`) reads the kind from `?kind=` so the form
 * can pre-select the matching ButtonGroup value.
 */
export function ImportButton({ kind }: ImportButtonProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const label = t("terminology.import.title");

  return (
    <Tooltip title={label}>
      <IconButton
        aria-label={label}
        onClick={() =>
          navigate({ to: "/terminology/import", search: { kind } })
        }
      >
        <AddIcon />
      </IconButton>
    </Tooltip>
  );
}