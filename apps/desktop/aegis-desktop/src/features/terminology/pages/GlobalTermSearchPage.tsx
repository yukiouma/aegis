import { useState, type ReactNode } from "react";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import type {
  ApiError,
  CodeItemView,
  CodeListView,
  TerminologyKind,
} from "../../../shared/api";
import {
  useGetCodeList,
  useSearchCodeItems,
  useSearchCodeLists,
} from "../data";
import { DescriptionsCell } from "../components/DescriptionsCell";
import { TermFilterBar } from "../components/TermFilterBar";

type Tab = "codelists" | "codeitems";

const routeApi = getRouteApi("/_authed/_layout/terminology/$kind/search");

/**
 * One-cell rendering of a parent codelist's name. Falls back to the
 * numeric id while the codelist is loading or has errored, so the table
 * never breaks because of a single missing codelist.
 */
function CodelistNameCell({ codelistId }: { codelistId: number }) {
  const { data } = useGetCodeList(codelistId);
  return <>{data?.name ?? `#${codelistId}`}</>;
}

interface SearchCodeListTableProps {
  rows: CodeListView[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  onOpen: (row: CodeListView) => void;
  bottomSlot: (scrollEl: HTMLElement | null) => ReactNode;
}

function SearchCodeListTable({
  rows,
  loading,
  error,
  onRetry,
  onOpen,
  bottomSlot,
}: SearchCodeListTableProps) {
  const { t } = useI18n();
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codelist.loadFailed", {
            message: errorMessage(error),
          })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      <TableContainer
        component={Paper}
        ref={setScrollEl}
        sx={{ maxHeight: "calc(100vh - 180px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codelist.field.code")}</TableCell>
              <TableCell>{t("terminology.codelist.field.name")}</TableCell>
              <TableCell>
                {t("terminology.codelist.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codelist.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id} hover>
                <TableCell>
                  <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                    <span>{row.code}</span>
                    {row.extensible && (
                      <Tooltip title={t("terminology.extensible")}>
                        <Chip label="EXT" size="small" />
                      </Tooltip>
                    )}
                  </Box>
                </TableCell>
                <TableCell>{row.name}</TableCell>
                <TableCell>{row.submissionValue}</TableCell>
                <TableCell>
                  <DescriptionsCell
                    synonym={row.synonym}
                    definition={row.definition}
                    nciPreferredTerm={row.nciPreferredTerm}
                  />
                </TableCell>
                <TableCell align="right">
                  <Tooltip title={t("common.open")}>
                    <IconButton
                      size="small"
                      aria-label={`open ${row.code}`}
                      onClick={() => onOpen(row)}
                    >
                      <LaunchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="text.secondary">
              {t("terminology.codelist.noMatches")}
            </Typography>
          </Box>
        )}
        {bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}

interface SearchCodeItemTableProps {
  rows: CodeItemView[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  onOpen: (row: CodeItemView) => void;
  bottomSlot: (scrollEl: HTMLElement | null) => ReactNode;
}

function SearchCodeItemTable({
  rows,
  loading,
  error,
  onRetry,
  onOpen,
  bottomSlot,
}: SearchCodeItemTableProps) {
  const { t } = useI18n();
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codeitem.loadFailed.search", {
            message: errorMessage(error),
          })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      <TableContainer
        component={Paper}
        ref={setScrollEl}
        sx={{ maxHeight: "calc(100vh - 240px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codeitem.field.code")}</TableCell>
              <TableCell>{t("terminology.codeitem.field.codelist")}</TableCell>
              <TableCell>
                {t("terminology.codeitem.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codeitem.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id} hover>
                <TableCell>{row.code}</TableCell>
                <TableCell>
                  <CodelistNameCell codelistId={row.codelistId} />
                </TableCell>
                <TableCell>{row.submissionValue}</TableCell>
                <TableCell>
                  <DescriptionsCell
                    synonym={row.synonym}
                    definition={row.definition}
                    nciPreferredTerm={row.nciPreferredTerm}
                  />
                </TableCell>
                <TableCell align="right">
                  <Tooltip title={t("common.open")}>
                    <IconButton
                      size="small"
                      aria-label={`open ${row.code}`}
                      onClick={() => onOpen(row)}
                    >
                      <LaunchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="text.secondary">
              {t("terminology.codeitem.noMatches")}
            </Typography>
          </Box>
        )}
        {bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}

export function GlobalTermSearchPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();

  const kind = (params.kind as TerminologyKind) ?? "sdtm";
  const versionId = search.versionId ?? null;

  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("codelists");

  const debouncedFragment = useDebouncedValue(query, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmedFragment = debouncedFragment.trim();
  const showTables = trimmedFragment.length > 0;

  const codeListsQuery = useSearchCodeLists(versionId, {
    fragment: debouncedFragment,
  });
  const codeItemsQuery = useSearchCodeItems(versionId, {
    fragment: debouncedFragment,
  });

  const goBack = () => {
    void navigate({
      to: kind === "sdtm" ? "/terminology/sdtm" : "/terminology/adam",
      search: versionId != null ? { versionId } : undefined,
    });
  };

  const openCodelist = (codelistId: number) => {
    void navigate({
      to: "/terminology/$kind/codelists/$codelistId",
      params: { kind, codelistId },
      search: versionId != null ? { versionId } : undefined,
    });
  };

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("terminology.search.backTooltip")}>
          <IconButton
            aria-label={t("terminology.search.backTooltip")}
            onClick={goBack}
          >
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <Box sx={{ flex: 1 }}>
          <TermFilterBar
            query={query}
            onQueryChange={setQuery}
            placeholder={t("terminology.search.placeholder")}
          />
        </Box>
        <ToggleButtonGroup
          exclusive
          value={tab}
          onChange={(_, v: Tab | null) => {
            if (v) setTab(v);
          }}
          size="small"
          aria-label={t("terminology.search.open")}
        >
          <ToggleButton value="codelists">
            {t("terminology.search.tab.codelists")}
          </ToggleButton>
          <ToggleButton value="codeitems">
            {t("terminology.search.tab.codeitems")}
          </ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {!showTables ? (
        <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
          <Typography color="text.secondary">
            {t("terminology.search.emptyInput")}
          </Typography>
        </Box>
      ) : tab === "codelists" ? (
        <SearchCodeListTable
          rows={codeListsQuery.data?.pages.flatMap((p) => p.items) ?? []}
          loading={codeListsQuery.isLoading}
          error={codeListsQuery.error}
          onRetry={() => void codeListsQuery.refetch()}
          onOpen={(row) => openCodelist(row.id)}
          bottomSlot={(scrollEl) => (
            <InfiniteScrollSentinel
              root={scrollEl}
              onIntersect={() => void codeListsQuery.fetchNextPage()}
              hasMore={codeListsQuery.hasNextPage ?? false}
              loading={codeListsQuery.isFetchingNextPage}
            />
          )}
        />
      ) : (
        <SearchCodeItemTable
          rows={codeItemsQuery.data?.pages.flatMap((p) => p.items) ?? []}
          loading={codeItemsQuery.isLoading}
          error={codeItemsQuery.error}
          onRetry={() => void codeItemsQuery.refetch()}
          onOpen={(row) => openCodelist(row.codelistId)}
          bottomSlot={(scrollEl) => (
            <InfiniteScrollSentinel
              root={scrollEl}
              onIntersect={() => void codeItemsQuery.fetchNextPage()}
              hasMore={codeItemsQuery.hasNextPage ?? false}
              loading={codeItemsQuery.isFetchingNextPage}
            />
          )}
        />
      )}
    </Box>
  );
}
