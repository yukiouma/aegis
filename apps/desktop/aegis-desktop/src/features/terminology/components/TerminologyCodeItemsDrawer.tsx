import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Drawer,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import {
  useFindCodeListByName,
  useLatestSdtmTerminologyVersionId,
  useListCodeItems,
} from "../data";
import { CodeItemTable } from "./CodeItemTable";

export interface TerminologyCodeItemsDrawerProps {
  open: boolean;
  code: string | null;
  onClose: () => void;
}

/**
 * Lists the code items of the code list a terminology code refers to.
 *
 * Resolution: the latest SDTM terminology version → the code list whose
 * `name` matches the code → that code list's items. The list is read
 * only: no create, edit, or delete affordances, and no "same code in
 * other codelists" dialog.
 *
 * Owns its own queries — the caller controls `open`, `code`, and
 * `onClose` only.
 */
export function TerminologyCodeItemsDrawer({
  open,
  code,
  onClose,
}: TerminologyCodeItemsDrawerProps) {
  const { t } = useI18n();
  const activeCode = open ? code : null;
  const versionId = useLatestSdtmTerminologyVersionId();
  const codelistsQuery = useFindCodeListByName(versionId, activeCode);
  const codelist = codelistsQuery.codelist;

  const itemsQuery = useListCodeItems(codelist?.id ?? null);
  const rows = itemsQuery.data?.pages.flatMap((p) => p.items) ?? [];
  const hasMore = itemsQuery.hasNextPage ?? false;

  const noVersion = versionId == null;
  const resolving = codelistsQuery.isLoading;
  const lookupError = codelistsQuery.error ?? itemsQuery.error;

  return (
    <Drawer
      anchor="right"
      open={open && !!code}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 720 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t("terminology.codeitem.drawer.title", { code: code ?? "" })}
        </Typography>

        {noVersion ? (
          <Alert severity="info">
            {t("terminology.codeitem.drawer.noVersion")}
          </Alert>
        ) : resolving ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : lookupError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {t("terminology.codeitem.loadFailed", {
                message: errorMessage(lookupError),
              })}
            </Alert>
            <Box>
              <Button
                onClick={() => {
                  void codelistsQuery.refetch();
                  void itemsQuery.refetch();
                }}
              >
                {t("common.retry")}
              </Button>
            </Box>
          </Box>
        ) : !codelist ? (
          <Typography color="text.secondary">
            {t("terminology.codeitem.drawer.notFound", { code: code ?? "" })}
          </Typography>
        ) : (
          <>
            <Typography variant="body2" color="text.secondary">
              {`${codelist.name} · ${codelist.code}`}
            </Typography>
            <CodeItemTable
              rows={rows}
              loading={itemsQuery.isLoading}
              mutationLoading={false}
              error={itemsQuery.error}
              canMutate={false}
              onRetry={() => void itemsQuery.refetch()}
              onCreate={() => {}}
              onEdit={() => {}}
              onDelete={() => {}}
              emptyMessage={t("terminology.codeitem.empty")}
              bottomSlot={(scrollEl) => (
                <InfiniteScrollSentinel
                  root={scrollEl}
                  onIntersect={() => void itemsQuery.fetchNextPage()}
                  hasMore={hasMore}
                  loading={itemsQuery.isFetchingNextPage}
                />
              )}
            />
          </>
        )}
      </Box>
    </Drawer>
  );
}
