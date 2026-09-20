import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

/**
 * Filepath section. Placeholder only — out of scope for this spec.
 * The shape mirrors the General / Members sections so the page
 * composition stays uniform: a centered "coming soon" message in a
 * single Box.
 */
export function ConfigurationFilepathSection() {
  const { t } = useI18n();
  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 1,
        py: 8,
      }}
      data-testid="config-filepath-placeholder"
    >
      <Typography variant="h6">
        {t("project.configuration.section.filepath")}
      </Typography>
      <Typography color="text.secondary">
        {t("project.configuration.filepath.placeholder")}
      </Typography>
    </Box>
  );
}