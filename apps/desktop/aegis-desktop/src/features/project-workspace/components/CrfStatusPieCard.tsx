import {
  Alert,
  Box,
  Card,
  CardContent,
  CardHeader,
  Skeleton,
  Typography,
  useTheme,
} from "@aegis/ui/mui";
import { PieChart } from "@aegis/ui/charts";
import {
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { useListCrfForms } from "../../crf/data";
import { errorMessage } from "../../../shared/api/error";
import type { CrfVersion } from "../../../shared/api";
import { ContrastArcLabel } from "./ContrastArcLabel";

/** Height of the pie's SVG drawing area, in px. */
const CHART_HEIGHT = 220;

/**
 * Gap between the two arcs, in degrees. At the radii this card uses
 * (roughly 90–110 px) 2° is a ~3 px gap in the surface colour, which
 * is what keeps the two fills from reading as a single shape.
 */
const ARC_PADDING_ANGLE = 2;

/**
 * Below this arc angle the on-arc percentage is dropped — a sliver has
 * no room for it, and a label that overflows its arc is worse than no
 * label at all. The legend carries the value either way.
 */
const ARC_LABEL_MIN_ANGLE = 25;

interface CrfStatusPieCardProps {
  /** The CRF version whose forms this card summarises. */
  version: CrfVersion;
}

/**
 * One CRF version's form-approval status, as a pie chart.
 *
 * The slices are derived from `CrfForm.approved` alone: `Approved` is
 * the count of approved forms and `Pending` is everything else. The
 * `notSubmitted` flag is deliberately not charted — it is a separate
 * submission flag, not an approval state, and the aggregate CRF status
 * chip treats approval as binary too.
 *
 * The colours come from the MUI theme (`success` / `warning`) so the
 * card matches the `CrfStatusChip` on the CRF list page and follows the
 * active theme pack. Because those two fills are close under
 * protanopia, identity is carried by text rather than by hue: every
 * arc states its own percentage, and every legend row carries a status
 * icon as well as its label.
 */
export function CrfStatusPieCard({ version }: CrfStatusPieCardProps) {
  const { t } = useI18n();
  const theme = useTheme();
  const formsQuery = useListCrfForms(version.id);

  const forms = formsQuery.data ?? [];
  const total = forms.length;
  const approved = forms.filter((f) => f.approved).length;
  const pending = total - approved;

  const rows = [
    {
      key: "approved",
      label: t("crf.toolbar.statusApproved"),
      count: approved,
      color: theme.palette.success.main,
      icon: <VerifiedIcon fontSize="small" />,
    },
    {
      key: "pending",
      label: t("crf.toolbar.statusPending"),
      count: pending,
      color: theme.palette.warning.main,
      icon: <PendingActionsIcon fontSize="small" />,
    },
  ];

  return (
    <Card variant="outlined">
      <CardHeader
        title={version.name}
        slotProps={{ title: { variant: "h6" } }}
      />
      <CardContent>
        {formsQuery.isLoading && (
          <Skeleton variant="rounded" height={CHART_HEIGHT} />
        )}

        {formsQuery.isError && (
          <Alert severity="error">{errorMessage(formsQuery.error)}</Alert>
        )}

        {formsQuery.isSuccess && total === 0 && (
          <Typography color="textSecondary">
            {t("workspace.dashboard.noCrfForms")}
          </Typography>
        )}

        {formsQuery.isSuccess && total > 0 && (
          <>
            <PieChart
              height={CHART_HEIGHT}
              hideLegend
              slots={{ pieArcLabel: ContrastArcLabel }}
              series={[
                {
                  data: rows.map((row) => ({
                    id: row.key,
                    value: row.count,
                    label: row.label,
                    color: row.color,
                  })),
                  paddingAngle: ARC_PADDING_ANGLE,
                  arcLabel: (item) =>
                    `${Math.round((item.value / total) * 100)}%`,
                  arcLabelMinAngle: ARC_LABEL_MIN_ANGLE,
                },
              ]}
            />
            <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5 }}>
              {rows.map((row) => (
                <Box
                  key={row.key}
                  sx={{ display: "flex", alignItems: "center", gap: 1 }}
                >
                  <Box
                    data-testid={`crf-status-icon-${row.key}`}
                    sx={{ color: row.color, display: "flex" }}
                  >
                    {row.icon}
                  </Box>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ flexGrow: 1 }}
                  >
                    {row.label}
                  </Typography>
                  <Typography
                    variant="body2"
                    data-testid={`crf-status-count-${row.key}`}
                  >
                    {row.count} (
                    {Math.round((row.count / total) * 100)}%)
                  </Typography>
                </Box>
              ))}
            </Box>
          </>
        )}
      </CardContent>
    </Card>
  );
}
