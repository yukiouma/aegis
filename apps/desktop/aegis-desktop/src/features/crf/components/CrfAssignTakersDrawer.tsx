import { Box, Button, Drawer, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

interface Props {
  open: boolean;
  onClose: () => void;
}

/**
 * Empty placeholder drawer for the assign-takers flow. Per spec,
 * has no content yet — just title + body placeholder + close.
 */
export function CrfAssignTakersDrawer({ open, onClose }: Props) {
  const { t } = useI18n();
  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">{t("crf.assignTakers.title")}</Typography>
        <Typography color="textSecondary">
          {t("crf.assignTakers.placeholder")}
        </Typography>
        <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
          <Button onClick={onClose}>{t("common.close")}</Button>
        </Box>
      </Box>
    </Drawer>
  );
}