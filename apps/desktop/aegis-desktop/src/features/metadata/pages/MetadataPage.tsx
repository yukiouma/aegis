import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Card,
  CardHeader,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import {
  Analytics as AnalyticsIcon,
  Architecture as ArchitectureIcon,
  Storage as StorageIcon,
} from "@aegis/ui/icons";

// Kinds shown on the page. Ordering is significant: index 0 is rendered
// in the left card, index 1 in the right card.
const KINDS = ["sdtm", "adam"] as const;
type Kind = (typeof KINDS)[number];

interface BlockSpec {
  kind: Kind;
  blockKey: "metadata.block.sdtm" | "metadata.block.adam";
  terminologyTarget: "/terminology/sdtm" | "/terminology/adam";
  terminologyIcon: typeof StorageIcon;
}

const BLOCKS: BlockSpec[] = [
  {
    kind: "sdtm",
    blockKey: "metadata.block.sdtm",
    terminologyTarget: "/terminology/sdtm",
    terminologyIcon: StorageIcon,
  },
  {
    kind: "adam",
    blockKey: "metadata.block.adam",
    terminologyTarget: "/terminology/adam",
    terminologyIcon: AnalyticsIcon,
  },
];

export function MetadataPage() {
  const { t } = useI18n();
  const navigate = useNavigate();

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Typography variant="h4">{t("metadata.heading")}</Typography>

      <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
        {BLOCKS.map((block) => (
          <Card key={block.kind} sx={{ flex: 1, minWidth: 320 }}>
            <CardHeader title={t(block.blockKey)} />
            <List disablePadding>
              <ListItem disablePadding>
                <Tooltip title={t("metadata.disabled.tooltip")}>
                  {/* span wrapper so Tooltip can attach to a disabled button */}
                  <span style={{ width: "100%" }}>
                    <ListItemButton disabled>
                      <ListItemIcon>
                        <ArchitectureIcon />
                      </ListItemIcon>
                      <ListItemText primary={t("metadata.item.domainModel")} />
                    </ListItemButton>
                  </span>
                </Tooltip>
              </ListItem>
              <ListItem disablePadding>
                <ListItemButton
                  onClick={() => navigate({ to: block.terminologyTarget })}
                >
                  <ListItemIcon>
                    <block.terminologyIcon />
                  </ListItemIcon>
                  <ListItemText primary={t("metadata.item.terminology")} />
                </ListItemButton>
              </ListItem>
            </List>
          </Card>
        ))}
      </Box>
    </Box>
  );
}