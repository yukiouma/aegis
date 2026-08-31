import { useState } from "react";
import {
  IconButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Tooltip,
} from "@aegis/ui/mui";
import {
  Widgets as WidgetsIcon,
  Search as SearchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate } from "@tanstack/react-router";

/**
 * IconButton that opens a floating menu of CRF helper pages (a
 * "tools" / "utilities" menu). Today the menu has a single entry:
 * the CRF Global Search page for the current project. New helper
 * entries land here as the feature grows. Used in the form-list
 * toolbar and the detail page header — the global-search page
 * itself renders no second copy of this control.
 *
 * `versionId` is optional; when present the menu forwards it as
 * `?versionId=` so the search page opens on the same version the
 * user was browsing. Omitting it is safe (the search page shows
 * its empty-input hint).
 */
export function CrfToolsMenu({
  projectCode,
  versionId,
}: {
  projectCode: string;
  versionId?: number | null;
}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const open = Boolean(anchorEl);

  return (
    <>
      <Tooltip title={t("crf.toolbar.toolsMenuHint")}>
        <IconButton
          aria-label={t("crf.toolbar.toolsMenuHint")}
          aria-controls={open ? "crf-tools-menu" : undefined}
          aria-haspopup="true"
          aria-expanded={open ? "true" : undefined}
          onClick={(e) => setAnchorEl(e.currentTarget)}
          size="small"
        >
          <WidgetsIcon />
        </IconButton>
      </Tooltip>
      <Menu
        id="crf-tools-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={() => setAnchorEl(null)}
        slotProps={{ paper: { sx: { minWidth: 200 } } }}
      >
        <MenuItem
          onClick={() => {
            setAnchorEl(null);
            navigate({
              to: "/project/$projectCode/crf/search",
              params: { projectCode },
              search: versionId != null ? { versionId } : undefined,
            });
          }}
        >
          <ListItemIcon>
            <SearchIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>{t("crf.toolbar.globalSearch")}</ListItemText>
        </MenuItem>
      </Menu>
    </>
  );
}