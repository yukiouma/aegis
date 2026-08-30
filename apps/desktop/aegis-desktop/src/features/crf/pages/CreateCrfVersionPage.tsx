import { useEffect, useState } from "react";
import { useNavigate, useParams } from "@tanstack/react-router";

import {
  Alert,
  Box,
  Button,
  CircularProgress,
  IconButton,
  MenuItem,
  Snackbar,
  TextField,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, CrfEdcType } from "../../../shared/api";
import { queryKeys, queryClient } from "../../../shared/query";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import { useListCrfVersions } from "../data/list";
import { useImportAls } from "../data/import";
import { AlsDropZone } from "../components/AlsDropZone";

/**
 * Create a new CRF version under a project by uploading an ALS file.
 *
 * User enters a version name (with duplicate check against the
 * project's existing versions), selects an EDC source type
 * (RAVE / eCollect V6 / eCollect Legacy), and picks an ALS file
 * (.xls / .xlsx / .xml). Submit sends { name, projectCode, filepath,
 * edcType } to the Rust `import_als` command, which parses the
 * ALS, pre-validates against the same kind-shape rules the server
 * enforces on `bulk_create_form`, creates the version, then issues
 * one bulk form-create per form. On success, navigates back to the
 * CRF form list with the new version id selected.
 */
export function CreateCrfVersionPage() {
  const navigate = useNavigate();
  const { t } = useI18n();

  const params = useParams({ strict: false }) as { projectCode?: string };
  const projectCode = params.projectCode ?? "";

  const [name, setName] = useState("");
  const [edcType, setEdcType] = useState<CrfEdcType | "">("");
  const [filepath, setFilepath] = useState<string | null>(null);

  const debouncedName = useDebouncedValue(name, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmed = debouncedName.trim();
  const versionsQuery = useListCrfVersions(projectCode || null);
  const duplicate =
    trimmed.length > 0 &&
    (versionsQuery.data ?? []).some((v) => v.name === trimmed);

  const importMutation = useImportAls();

  const canSubmit =
    trimmed.length > 0 &&
    !duplicate &&
    edcType !== "" &&
    filepath !== null &&
    !importMutation.isPending;

  function goBack() {
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
    });
  }

  // On success, invalidate the version list and the new version's
  // form list, then navigate to the form list with the new version
  // id pre-selected.
  useEffect(() => {
    if (!importMutation.data) return;
    const view = importMutation.data;
    queryClient.invalidateQueries({
      queryKey: queryKeys.crf.versionsByProject(projectCode),
    });
    queryClient.invalidateQueries({
      queryKey: queryKeys.crf.formsByVersion(view.id),
    });
    navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: { versionId: view.id },
      replace: true,
    });
    // fire once on success only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [importMutation.data]);

  function submit() {
    if (!canSubmit || filepath === null) return;
    importMutation.mutate({
      name: trimmed,
      projectCode,
      filepath,
      edcType: edcType as CrfEdcType,
    });
  }

  const snackbarOpen = importMutation.isError || importMutation.isSuccess;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("common.back")}>
          <IconButton onClick={goBack} aria-label={t("common.back")}>
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <Typography variant="h5">{t("crf.import.title")}</Typography>
      </Box>

      {importMutation.isPending ? (
        <Box
          sx={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 2,
            py: 8,
          }}
        >
          <CircularProgress />
          <Typography>{t("crf.import.importing")}</Typography>
        </Box>
      ) : (
        <>
          <TextField
            label={t("crf.import.nameLabel")}
            placeholder={t("crf.import.namePlaceholder")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            error={duplicate}
            helperText={
              duplicate
                ? t("crf.import.errors.nameDuplicate", { name: trimmed })
                : ""
            }
          />

          <TextField
            select
            label={t("crf.import.edcTypeLabel")}
            value={edcType}
            onChange={(e) =>
              setEdcType(e.target.value as CrfEdcType | "")
            }
            error={edcType === ""}
            helperText={
              edcType === "" ? t("crf.import.errors.edcTypeRequired") : ""
            }
          >
            <MenuItem value="rave">{t("crf.import.edcTypeRave")}</MenuItem>
            <MenuItem value="ecollectV6">
              {t("crf.import.edcTypeEcollectV6")}
            </MenuItem>
            <MenuItem value="ecollectLegacy">
              {t("crf.import.edcTypeEcollectLegacy")}
            </MenuItem>
          </TextField>

          <AlsDropZone
            filepath={filepath}
            onFilepathChange={setFilepath}
          />

          <Button
            variant="contained"
            disabled={!canSubmit}
            onClick={submit}
          >
            {t("crf.import.submit")}
          </Button>
        </>
      )}

      <Snackbar
        open={snackbarOpen}
        autoHideDuration={4000}
        onClose={() => {
          if (importMutation.isError) importMutation.reset();
        }}
      >
        <Alert severity={importMutation.isError ? "error" : "success"}>
          {importMutation.isError
            ? t("crf.import.failure", {
                message: errorMessage(importMutation.error as ApiError),
              })
            : t("crf.import.success", {
                name: importMutation.data?.name ?? "",
              })}
        </Alert>
      </Snackbar>
    </Box>
  );
}