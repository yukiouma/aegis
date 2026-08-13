import React from "react";
import {
  createFileRoute,
  Outlet,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../../api";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

export const Route = createFileRoute("/_layout")({
  // Every page under this layout requires a session. A failing
  // `is_logged_in` (a broken token store) counts as logged out, so the
  // user lands on the splash rather than seeing the router throw.
  beforeLoad: async () => {
    let loggedIn = false;
    try {
      loggedIn = await api.isLoggedIn();
    } catch {
      loggedIn = false;
    }
    if (!loggedIn) {
      throw redirect({ to: "/splash" });
    }
  },
  component: AppLayout,
});

export default function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          ml: `${sidebarOpen ? 240 : 56}px`,
          transition: "margin 0.3s",
        }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}