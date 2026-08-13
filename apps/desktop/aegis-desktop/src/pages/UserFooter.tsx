import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  Typography,
} from "@aegis/ui/mui";
import { Logout } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { api } from "../api";
import type { Role, UserView } from "../api";

interface UserFooterProps {
  /** Whether the surrounding sidebar drawer is open. When false, hide
   *  the name + chip and show only the logout icon. */
  sidebarOpen: boolean;
}

/**
 * Pinned to the bottom of the Sidebar. Shows the signed-in user's name
 * (with an optional role chip for root / admin) and a logout button
 * gated by a confirm dialog. On confirm: calls `api.logout` and
 * navigates to `/login`. The `_layout` `beforeLoad` guard already
 * redirects an authenticated user away from `/login`, so once the
 * tokens are cleared the navigation lands cleanly.
 */
export function UserFooter({ sidebarOpen }: UserFooterProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [user, setUser] = useState<UserView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const view = await api.getCurrentUser();
        if (!cancelled) setUser(view);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function onConfirmLogout() {
    setConfirmOpen(false);
    await api.logout();
    await navigate({ to: "/login" });
  }

  const showRoleChip =
    user?.role === ("root" as Role) || user?.role === ("admin" as Role);

  const roleLabel =
    user?.role === ("root" as Role)
      ? t("app.user.role.root")
      : user?.role === ("admin" as Role)
        ? t("app.user.role.admin")
        : null;

  return (
    <>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1, minWidth: 0 }}>
        {sidebarOpen && showRoleChip && (
          <Chip size="small" label={roleLabel} />
        )}
        {sidebarOpen && (
          <Typography
            variant="body2"
            noWrap
            sx={{ flexGrow: 1, minWidth: 0 }}
            color={error ? "error" : "textPrimary"}
          >
            {error ? t("app.user.loadFailed") : (user?.name ?? t("app.user.unknownUser"))}
          </Typography>
        )}
        <IconButton
          aria-label={t("app.user.logout")}
          onClick={() => setConfirmOpen(true)}
          size="small"
        >
          <Logout />
        </IconButton>
      </Box>
      <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
        <DialogTitle>{t("app.user.logout.confirmTitle")}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("app.user.logout.confirmMessage")}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmOpen(false)}>
            {t("app.user.logout.cancel")}
          </Button>
          <Button onClick={() => void onConfirmLogout()} variant="contained">
            {t("app.user.logout.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}