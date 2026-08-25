import {
  Alert,
  Box,
  Button,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Edit as EditIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { SdtmDomainView } from "../../../shared/api";

export interface DomainHeaderTableProps {
  domain: SdtmDomainView | undefined;
  loading: boolean;
  error: unknown;
  canMutate: boolean;
  selectedLang: string | null;
  onEdit: () => void;
  onBack: () => void;
}

const cellEllipsis = {
  whiteSpace: "nowrap" as const,
  overflow: "hidden",
  textOverflow: "ellipsis",
  maxWidth: 360,
};

export function DomainHeaderTable({
  domain,
  error,
  canMutate,
  selectedLang,
  onEdit,
  onBack,
}: DomainHeaderTableProps) {
  const { t } = useI18n();

  if (error && !domain) {
    return (
      <TableContainer component={Paper}>
        <Table size="small">
          <TableBody>
            <TableRow>
              <TableCell sx={{ width: 48 }}>
                <Tooltip title={t("common.back")}>
                  <IconButton onClick={onBack} aria-label={t("common.back")}>
                    <ArrowBackIcon />
                  </IconButton>
                </Tooltip>
              </TableCell>
              <TableCell colSpan={5}>
                <Alert severity="error">
                  {t("domainModel.sdtm.detail.loadFailed", {
                    message: errorMessage(error),
                  })}
                </Alert>
                <Box sx={{ mt: 1 }}>
                  <Button onClick={onBack}>{t("common.back")}</Button>
                </Box>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </TableContainer>
    );
  }

  const d = selectedLang
    ? domain?.descriptions.find((x) => x.lang === selectedLang)
    : undefined;
  const description = d?.details.description ?? "";
  const structure = d?.details.structure ?? "";

  return (
    <TableContainer component={Paper}>
      <Table size="small">
        <TableBody>
          <TableRow>
            <TableCell sx={{ width: 48 }}>
              <Tooltip title={t("domainModel.sdtm.detail.backTooltip")}>
                <IconButton
                  onClick={onBack}
                  aria-label={t("common.back")}
                >
                  <ArrowBackIcon />
                </IconButton>
              </Tooltip>
            </TableCell>
            <TableCell>
              <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                {domain?.name ?? ""}
              </Typography>
            </TableCell>
            <TableCell sx={cellEllipsis} title={description}>
              {description}
            </TableCell>
            <TableCell sx={cellEllipsis} title={structure}>
              {structure}
            </TableCell>
            <TableCell>{domain?.category ?? ""}</TableCell>
            <TableCell sx={{ width: 64 }} align="right">
              {canMutate && domain && (
                <Tooltip title={t("domainModel.sdtm.detail.editTooltip")}>
                  <IconButton
                    size="small"
                    aria-label={t("domainModel.sdtm.detail.editTooltip")}
                    onClick={onEdit}
                  >
                    <EditIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              )}
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </TableContainer>
  );
}