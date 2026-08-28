import { Button } from "@aegis/ui/mui";
import { Search as SearchIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate } from "@tanstack/react-router";

/**
 * Button that navigates to the CRF Global Search page for the
 * current project. Replaces the spec's "MoreVert + Menu" pattern
 * with a direct navigation button.
 */
export function CrfGlobalSearchButton({
  projectCode,
}: {
  projectCode: string;
}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  return (
    <Button
      startIcon={<SearchIcon />}
      variant="outlined"
      size="small"
      onClick={() =>
        navigate({
          to: "/project/$projectCode/crf/search",
          params: { projectCode },
        })
      }
      title={t("crf.toolbar.globalSearchHint")}
    >
      {t("crf.toolbar.globalSearch")}
    </Button>
  );
}