import {
  Alert,
  Box,
  Chip,
  CircularProgress,
  IconButton,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams } from "@tanstack/react-router";

import { CrfGlobalSearchButton } from "../components";
import { useGetCrfForm } from "../data/list";
import { errorMessage } from "../../../shared/api/error";

export function CrfDetailPage() {
  const { t } = useI18n();
  const { projectCode, formId } = useParams({ strict: false }) as {
    projectCode: string;
    formId?: string;
  };
  const navigate = useNavigate();
  const id =
    formId != null && Number.isFinite(Number(formId)) && Number(formId) > 0
      ? Number(formId)
      : null;
  const query = useGetCrfForm(id);

  const back = () =>
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: (prev: Record<string, unknown>) => prev,
    });

  if (id == null) {
    return (
      <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
        <Box
          sx={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 2,
            flexWrap: "wrap",
          }}
        >
          <IconButton aria-label={t("crf.detail.back")} onClick={back}>
            <ArrowBackIcon />
          </IconButton>
          <Typography variant="h4">{t("crf.detail.title")}</Typography>
        </Box>
        <Alert severity="error">{t("common.invalidId")}</Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box
        sx={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 2,
          flexWrap: "wrap",
        }}
      >
        <IconButton aria-label={t("crf.detail.back")} onClick={back}>
          <ArrowBackIcon />
        </IconButton>
        {query.data && <Chip label={query.data.code} variant="outlined" />}
        {query.data && (
          <Typography variant="h5">{query.data.name}</Typography>
        )}
        {!query.data && !query.isError && (
          <Typography variant="h5">{t("crf.detail.title")}</Typography>
        )}
        <Box sx={{ flexGrow: 1 }} />
        <CrfGlobalSearchButton projectCode={projectCode} />
      </Box>

      {query.isFetching && !query.data && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      {query.isError && (
        <Alert severity="error">{errorMessage(query.error)}</Alert>
      )}
      {!query.isFetching && (
        <Alert severity="info">{t("crf.detail.placeholder")}</Alert>
      )}
    </Box>
  );
}