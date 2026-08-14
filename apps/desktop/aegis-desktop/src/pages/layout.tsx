import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import {
  Home as HomeIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { UserFooter } from "./UserFooter";

const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

/**
 * Authenticated app shell: the `Sidebar` plus the active child route.
 * Lives in `src/pages/` (not a route file) so TanStack Router can code-
 * split the route file cleanly. The route file imports this as the
 * component for `/_layout`.
 */
export function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  const menu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: <UserFooter sidebarOpen={sidebarOpen} />,
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
