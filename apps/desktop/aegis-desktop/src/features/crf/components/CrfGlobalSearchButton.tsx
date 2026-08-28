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
 * IconButton that opens a floating menu. The single menu item
 * navigates to the CRF Global Search page for the current
 * project. Used in the form-list toolbar and the detail page
 * header — the global-search page itself renders no second
 * copy of this control.
 */
export function CrfGlobalSearchButton({
  projectCode,
}: {
  projectCode: string;
}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const open = Boolean(anchorEl);

  return (
    <>
      <Tooltip title={t("crf.toolbar.globalSearchHint")}>
        <IconButton
          aria-label={t("crf.toolbar.globalSearchHint")}
          aria-controls={open ? "crf-float-menu" : undefined}
          aria-haspopup="true"
          aria-expanded={open ? "true" : undefined}
          onClick={(e) => setAnchorEl(e.currentTarget)}
          size="small"
        >
          <WidgetsIcon />
        </IconButton>
      </Tooltip>
      <Menu
        id="crf-float-menu"
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