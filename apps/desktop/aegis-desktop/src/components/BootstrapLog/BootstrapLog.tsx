import { Paper, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { LogEntry, LogLevel } from "./types";

const LEVEL_COLOR: Record<LogLevel, string> = {
  info: "text.secondary",
  success: "success.main",
  error: "error.main",
};

export interface BootstrapLogProps {
  entries: LogEntry[];
}

/** Scrollable, append-only transcript of what the page has done so far. */
export function BootstrapLog({ entries }: BootstrapLogProps) {
  const { t } = useI18n();

  return (
    <Paper
      variant="outlined"
      data-testid="bootstrap-log"
      sx={{ mt: 2, p: 1.5, maxHeight: 200, overflowY: "auto" }}
    >
      {entries.map((entry) => (
        <Typography
          key={entry.id}
          data-testid={`bootstrap-log-${entry.level}`}
          variant="body2"
          sx={{ fontFamily: "monospace", color: LEVEL_COLOR[entry.level] }}
        >
          {t(entry.key, entry.params)}
        </Typography>
      ))}
    </Paper>
  );
}
