import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

export function HomePage() {
  const { t } = useI18n();
  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("home.heading")}
      </Typography>
      <Typography variant="body1">{t("home.welcome")}</Typography>
    </Box>
  );
}