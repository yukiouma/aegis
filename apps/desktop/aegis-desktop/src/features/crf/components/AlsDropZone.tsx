import { useEffect, useState } from "react";
import { Box, Chip, Typography } from "@aegis/ui/mui";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useI18n } from "@aegis/ui/i18n";

const ALLOWED_EXTS = ["xls", "xlsx", "xml"] as const;

function basename(path: string): string {
  return path.replace(/^.*[\\/]/, "");
}

function isAllowed(path: string): boolean {
  const lower = path.toLowerCase();
  return ALLOWED_EXTS.some((ext) => lower.endsWith(`.${ext}`));
}

export interface AlsDropZoneProps {
  filepath: string | null;
  onFilepathChange: (next: string | null) => void;
}

/**
 * Drop zone + chip for an ALS file. Renders the drop zone when no
 * filepath is set, and a chip with a clear button when one is.
 * Drag-and-drop is wired via Tauri's `onDragDropEvent` (v2 webview
 * API); click-to-pick uses `tauri-plugin-dialog`'s `open()` with
 * an `.xls / .xlsx / .xml` filter.
 */
export function AlsDropZone({ filepath, onFilepathChange }: AlsDropZoneProps) {
  const { t } = useI18n();
  const [dropError, setDropError] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const path = event.payload.paths[0];
        if (!path) return;
        if (!isAllowed(path)) {
          setDropError(true);
          window.setTimeout(() => setDropError(false), 1500);
          return;
        }
        onFilepathChange(path);
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onFilepathChange]);

  async function pickFile() {
    const picked = await open({
      multiple: false,
      filters: [{ name: "ALS", extensions: [...ALLOWED_EXTS] }],
    });
    if (typeof picked === "string") onFilepathChange(picked);
  }

  const fileName = filepath ? basename(filepath) : null;

  if (filepath !== null) {
    return (
      <Chip
        label={fileName ?? ""}
        onDelete={() => onFilepathChange(null)}
        sx={{ alignSelf: "flex-start" }}
      />
    );
  }

  return (
    <Box
      data-testid="als-dropzone"
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
          ? t("crf.import.errors.fileTypeHint")
          : t("crf.import.dropZone")}
      </Typography>
    </Box>
  );
}