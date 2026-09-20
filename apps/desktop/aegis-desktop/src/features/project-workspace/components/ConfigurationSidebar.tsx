import {
  Box,
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from "@aegis/ui/mui";
import { Folder, People, Settings } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import type { ComponentType } from "react";

export type ConfigurationSection = "general" | "members" | "filepath";

export interface ConfigurationSidebarProps {
  value: ConfigurationSection;
  onChange: (next: ConfigurationSection) => void;
  width?: number;
}

interface SectionEntry {
  key: ConfigurationSection;
  icon: ComponentType;
}

const ORDER: SectionEntry[] = [
  { key: "general", icon: Settings },
  { key: "members", icon: People },
  { key: "filepath", icon: Folder },
];

/**
 * Right-side nav for the configuration page. Fixed-width column with
 * a vertical List of three ListItemButtons. The active item gets the
 * MUI `selected` treatment (background + primary tint) so the user
 * always knows which section is on screen.
 *
 * Each entry carries an icon on the left so the nav reads like the
 * main workspace sidebar. Items get horizontal padding so the icon
 * and label don't hug the container edges.
 *
 * No routing here — selection is in-page state, driven by `value` /
 * `onChange`. The parent owns the state so it can also rehydrate on
 * back-navigation or external deep-links later without restructuring
 * this component.
 */
export function ConfigurationSidebar({
  value,
  onChange,
  width = 220,
}: ConfigurationSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      component="nav"
      aria-label="configuration-section"
      sx={{
        width,
        flexShrink: 0,
        borderLeft: 1,
        borderColor: "divider",
        bgcolor: "background.paper",
      }}
    >
      <Divider />
      <List disablePadding sx={{ py: 1 }}>
        {ORDER.map(({ key: section, icon: Icon }) => (
          <ListItemButton
            key={section}
            selected={section === value}
            onClick={() => onChange(section)}
            data-testid={`config-section-${section}`}
            sx={{ px: 2, py: 1.25 }}
          >
            <ListItemIcon sx={{ minWidth: 36 }}>
              <Icon />
            </ListItemIcon>
            <ListItemText
              primary={t(`project.configuration.section.${section}` as const)}
            />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}