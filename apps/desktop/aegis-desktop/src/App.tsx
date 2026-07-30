import { useState } from "react";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import { Home as HomeIcon, Settings as SettingsIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";

// MUI icon components require SvgIconProps; the Sidebar's `icon` slot is
// typed as the no-required-props `ComponentType`. Wrap each icon in a
// no-arg function so the assignment type-checks.
const HomeMenuIcon = () => <HomeIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;

type Page = "home" | "settings";

function pageFromLink(link: string): Page {
  return link === "/settings" ? "settings" : "home";
}

export default function App() {
  const { t } = useI18n();
  const [page, setPage] = useState<Page>("home");
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const menu: MenuItem[] = [
    { link: "/home", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => setPage(pageFromLink(link)),
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
        {page === "settings" ? <SettingsPage /> : <HomePage />}
      </Box>
    </Box>
  );
}