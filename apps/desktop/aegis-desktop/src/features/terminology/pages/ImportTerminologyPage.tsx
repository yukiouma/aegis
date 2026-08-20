import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Snackbar,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  TerminologyKind,
  TerminologyVersionView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

type Kind = TerminologyKind | null;

function basename(path: string): string {
  return path.replace(/^.*[\\/]/, "");
}

export function ImportTerminologyPage() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { t } = useI18n();

  // Read `kind` from the URL query string. The route file validates the
  // value via a zod schema; any absent or invalid value leaves the
  // ButtonGroup unselected. `strict: false` lets the page render even
  // when the surrounding test router doesn't register the import route.
  const search = useSearch({ strict: false }) as { kind?: string };
  const initialKind: Kind =
    search?.kind === "sdtm" || search?.kind === "adam" ? search.kind : null;

  const [kind, setKind] = useState<Kind>(initialKind);
  const [filepath, setFilepath] = useState<string | null>(null);
  const [dropError, setDropError] = useState(false);
  const [snackbar, setSnackbar] = useState<{
    open: boolean;
    severity: "success" | "error";
    message: string;
  }>({ open: false, severity: "success", message: "" });

  const importMutation = useMutation<
    TerminologyVersionView,
    ApiError,
    { kind: TerminologyKind; filepath: string }
  >({
    mutationFn: ({ kind, filepath }) => api.importTerminology(kind, filepath),
    onSuccess: (version) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
      setSnackbar({
        open: true,
        severity: "success",
        message: t("terminology.import.success", { name: version.name }),
      });
    },
    onError: (err) => {
      setSnackbar({
        open: true,
        severity: "error",
        message: t("terminology.import.failure", {
          message: errorMessage(err),
        }),
      });
    },
  });

  const backLink = kind === null ? "/terminology/sdtm" : `/terminology/${kind}`;

  async function pickFile() {
    const path = await open({
      multiple: false,
      filters: [{ name: "Excel", extensions: ["xls", "xlsx"] }],
    });
    if (typeof path === "string") setFilepath(path);
  }

  // Tauri v2 intercepts file drops at the OS level, so the DOM `drop` event
  // never fires inside the webview. Subscribe to the webview's drag-drop
  // event stream instead — its `drop` payload contains absolute filesystem
  // paths that the Rust `import_terminology` command can read directly.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const path = event.payload.paths[0];
        if (!path) return;
        const lower = path.toLowerCase();
        if (!lower.endsWith(".xls") && !lower.endsWith(".xlsx")) {
          setDropError(true);
          window.setTimeout(() => setDropError(false), 1500);
          return;
        }
        setFilepath(path);
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const canSubmit =
    kind !== null && filepath !== null && !importMutation.isPending;
  const fileName = filepath ? basename(filepath) : null;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("common.back")}>
          <IconButton
            onClick={() => navigate({ to: backLink })}
            aria-label={t("common.back")}
          >
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <Typography variant="h5">{t("terminology.import.title")}</Typography>
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
          <Typography>{t("terminology.import.importing")}</Typography>
        </Box>
      ) : (
        <>
          <ToggleButtonGroup
            exclusive
            value={kind}
            onChange={(_, v) => setKind(v)}
            aria-label={t("terminology.import.title")}
          >
            <ToggleButton value="sdtm">SDTM</ToggleButton>
            <ToggleButton value="adam">ADaM</ToggleButton>
          </ToggleButtonGroup>

          {filepath === null ? (
            <Box
              role="button"
              tabIndex={0}
              onClick={pickFile}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") pickFile();
              }}
              sx={(theme) => ({
                p: 4,
                border: "2px dashed",
                borderColor: dropError
                  ? theme.palette.error.main
                  : theme.palette.divider,
                borderRadius: 1,
                textAlign: "center",
                cursor: "pointer",
              })}
            >
              <Typography>
                {dropError
                  ? t("terminology.import.fileTypeHint")
                  : t("terminology.import.dropZone")}
              </Typography>
            </Box>
          ) : (
            <Chip
              label={fileName}
              onDelete={() => setFilepath(null)}
              sx={{ alignSelf: "flex-start" }}
            />
          )}

          <Button
            variant="contained"
            disabled={!canSubmit}
            onClick={() =>
              importMutation.mutate({ kind: kind!, filepath: filepath! })
            }
          >
            {t("common.submit")}
          </Button>
        </>
      )}

      <Snackbar
        open={snackbar.open}
        autoHideDuration={4000}
        onClose={() => setSnackbar((s) => ({ ...s, open: false }))}
      >
        <Alert
          severity={snackbar.severity}
          onClose={() => setSnackbar((s) => ({ ...s, open: false }))}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Box>
  );
}