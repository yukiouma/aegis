import { useState } from "react";
import {
  Box,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useNavigate, useParams } from "@tanstack/react-router";

import { CrfToolsMenu } from "../components";

export function CrfGlobalSearchPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const navigate = useNavigate();
  const [fragment, setFragment] = useState("");

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
        <IconButton
          aria-label={t("crf.detail.back")}
          onClick={() =>
            navigate({
              to: "/project/$projectCode/crf",
              params: { projectCode },
            })
          }
        >
          <ArrowBackIcon />
        </IconButton>
        <Typography variant="h4">
          {t("crf.globalSearch.heading", { projectCode })}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <CrfToolsMenu projectCode={projectCode} />
      </Box>

      <TextField
        size="small"
        placeholder={t("crf.globalSearch.searchPlaceholder")}
        value={fragment}
        onChange={(e) => setFragment(e.target.value)}
        fullWidth
      />

      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>{t("crf.globalSearch.col.form")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.item")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.option")}</TableCell>
              <TableCell>{t("crf.globalSearch.col.annotation")}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            <TableRow>
              <TableCell colSpan={4} align="center">
                <Box sx={{ py: 3, color: "text.secondary" }}>
                  {t("crf.globalSearch.empty")}
                </Box>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </TableContainer>
    </Box>
  );
}