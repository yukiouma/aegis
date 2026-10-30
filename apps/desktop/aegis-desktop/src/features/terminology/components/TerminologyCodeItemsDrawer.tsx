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
  const versions = useLatestSdtmTerminologyVersionId();
  const codelistsQuery = useFindCodeListByName(versions.versionId, activeCode);
  const codelist = codelistsQuery.codelist;

  const itemsQuery = useListCodeItems(codelist?.id ?? null);
  const rows = itemsQuery.data?.pages.flatMap((p) => p.items) ?? [];
  const hasMore = itemsQuery.hasNextPage ?? false;

  // "No version imported", "still loading" and "the request failed" are
  // three different states — they must not collapse into one message.
  const versionsFailed = versions.error != null;
  const noVersion = versions.versionId == null && !versionsFailed;
  const resolving = versions.isLoading || codelistsQuery.isLoading;
  const lookupError = versions.error ?? codelistsQuery.error ?? itemsQuery.error;

  function handleRetry() {
    // Refetch only the query that failed: the code-item query is
    // disabled until a code list is resolved, and firing it anyway
    // would ask the server for an unfiltered page of code items.
    if (versionsFailed) {
      versions.refetch();
      return;
    }
    if (codelistsQuery.error) {
      void codelistsQuery.refetch();
      return;
    }
    void itemsQuery.refetch();
  }

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

        {lookupError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {versionsFailed
                ? t("terminology.codeitem.drawer.versionsLoadFailed", {
                    message: errorMessage(lookupError),
                  })
                : t("terminology.codeitem.drawer.loadFailed", {
                    code: code ?? "",
                    message: errorMessage(lookupError),
                  })}
            </Alert>
            <Box>
              <Button onClick={handleRetry}>{t("common.retry")}</Button>
            </Box>
          </Box>
        ) : resolving ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : noVersion ? (
          <Alert severity="info">
            {t("terminology.codeitem.drawer.noVersion")}
          </Alert>
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
