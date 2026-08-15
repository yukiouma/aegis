import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

/**
 * Placeholder Configuration page for a project workspace window.
 * Real content (project settings, member management, integrations)
 * is out of scope for the workspace-window feature.
 */
export function ProjectConfigurationPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.configuration.heading", { projectCode })}
      </Typography>
      <Typography color="textSecondary">
        {t("workspace.placeholder")}
      </Typography>
    </Box>
  );
}