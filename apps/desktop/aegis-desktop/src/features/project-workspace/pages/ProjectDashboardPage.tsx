import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

/**
 * Placeholder Dashboard page for a project workspace window.
 * Real content (charts, KPIs, recent activity) is out of scope for
 * the workspace-window feature and arrives in a later spec.
 */
export function ProjectDashboardPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.dashboard.heading", { projectCode })}
      </Typography>
      <Typography color="textSecondary">
        {t("workspace.placeholder")}
      </Typography>
    </Box>
  );
}