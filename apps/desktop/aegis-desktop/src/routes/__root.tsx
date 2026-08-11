import React from "react";
import { createRootRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

export const Route = createRootRoute({
  component: RootLayout,
});

export default function RootLayout() {
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
