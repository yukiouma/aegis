import { Alert, Box, Skeleton, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { useListCrfVersions } from "../../crf/data";
import { errorMessage } from "../../../shared/api/error";
import { CrfStatusPieCard } from "../components/CrfStatusPieCard";

/**
 * Responsive grid for the dashboard's chart cards. One column on narrow
 * windows, two on medium, three on wide — so the grid keeps growing as
 * more chart types are added to the dashboard.
 */
function DashboardGrid({ children }: { children: ReactNode }) {
  return (
    <Box
      sx={{
        display: "grid",
        gap: 2,
        gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr", lg: "1fr 1fr 1fr" },
      }}
    >
      {children}
    </Box>
  );
}

/**
 * Dashboard page for a project workspace window. Shows one status chart
 * per CRF version; further charts (KPIs, recent activity) are added to
 * the grid as they land.
 */
export function ProjectDashboardPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const versionsQuery = useListCrfVersions(projectCode);

  const versions = versionsQuery.data ?? [];

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.dashboard.heading", { projectCode })}
      </Typography>

      <Typography variant="h6" gutterBottom>
        {t("workspace.dashboard.crfStatusTitle")}
      </Typography>

      {versionsQuery.isLoading && (
        <DashboardGrid>
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} variant="rounded" height={260} />
          ))}
        </DashboardGrid>
      )}

      {versionsQuery.isError && (
        <Alert severity="error">{errorMessage(versionsQuery.error)}</Alert>
      )}

      {versionsQuery.isSuccess && versions.length === 0 && (
        <Typography color="textSecondary">
          {t("workspace.dashboard.noCrfVersions")}
        </Typography>
      )}

      {versionsQuery.isSuccess && versions.length > 0 && (
        <DashboardGrid>
          {versions.map((version) => (
            <CrfStatusPieCard key={version.id} version={version} />
          ))}
        </DashboardGrid>
      )}
    </Box>
  );
}
