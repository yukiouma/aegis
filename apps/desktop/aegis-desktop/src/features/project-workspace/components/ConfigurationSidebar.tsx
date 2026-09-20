import {
  Box,
  Divider,
  List,
  ListItemButton,
  ListItemText,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export type ConfigurationSection = "general" | "members" | "filepath";

export interface ConfigurationSidebarProps {
  value: ConfigurationSection;
  onChange: (next: ConfigurationSection) => void;
  width?: number;
}

const ORDER: ConfigurationSection[] = ["general", "members", "filepath"];

/**
 * Right-side nav for the configuration page. Fixed-width column with
 * a vertical List of three ListItemButtons. The active item gets the
 * MUI `selected` treatment (background + primary tint) so the user
 * always knows which section is on screen.
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
      <List dense disablePadding>
        {ORDER.map((section) => (
          <ListItemButton
            key={section}
            selected={section === value}
            onClick={() => onChange(section)}
            data-testid={`config-section-${section}`}
          >
            <ListItemText
              primary={t(`project.configuration.section.${section}` as const)}
            />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}