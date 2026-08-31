import { useState, type ReactNode } from "react";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  InputAdornment,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Launch as LaunchIcon,
  Search as SearchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import type {
  Annotation,
  ApiError,
  CrfForm,
  CrfItem,
  CrfOption,
  CrfUnit,
  DomainAnnotation,
} from "../../../shared/api";
import { useGetCrfForm } from "../data/list";
import {
  useGetCrfItem,
  useSearchCrfAnnotations,
  useSearchCrfDomainAnnotations,
  useSearchCrfForms,
  useSearchCrfItems,
  useSearchCrfOptions,
  useSearchCrfUnits,
} from "../data/search";

type Tab =
  | "forms"
  | "items"
  | "units"
  | "options"
  | "domains"
  | "annotations";

const routeApi = getRouteApi("/_authed/project/$projectCode/crf/search");

interface NavigateArgs {
  projectCode: string;
  versionId: number | null;
  formId: number;
  focus: string;
}

/**
 * Hook factory: returns a callback that navigates into the form
 * detail page with the row's `focus` set so the detail page can
 * scroll the matching anchor into view. Captures the project's
 * `versionId` once so all row clicks go to the same version.
 */
function useOpenFormDetail(
  projectCode: string,
  versionId: number | null,
): (args: Omit<NavigateArgs, "projectCode" | "versionId">) => void {
  const navigate = useNavigate();
  return ({ formId, focus }) => {
    void navigate({
      to: "/project/$projectCode/crf/$formId",
      params: { projectCode, formId: String(formId) },
      search: {
        versionId: versionId ?? undefined,
        focus,
      },
    });
  };
}

/**
 * Renders the owning form for a Units / Options row as
 * `Chip(form.code) + form.name`. Resolves
 * `itemId → item → formId → form` via the cached get-by-id
 * hooks; React Query dedupes so N rows under the same item
 * share a single HTTP round-trip per lookup. Falls back to
 * `#<formId>` while loading or on error.
 */
function UnitRowFormCell({ itemId }: { itemId: number }) {
  const item = useGetCrfItem(itemId);
  const formId = item.data?.formId ?? null;
  const form = useGetCrfForm(formId);
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      <Chip
        size="small"
        variant="outlined"
        label={form.data?.code ?? `#${formId ?? itemId}`}
        sx={{ minWidth: 70 }}
      />
      <Typography variant="body2">{form.data?.name ?? ""}</Typography>
    </Box>
  );
}

/**
 * Renders an item for a Units / Options row as
 * `Chip(item.code) + item.name`. Falls back to `#<itemId>`
 * while loading or on error.
 */
function UnitRowItemCell({ itemId }: { itemId: number }) {
  const item = useGetCrfItem(itemId);
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      <Chip
        size="small"
        variant="outlined"
        label={item.data?.code ?? `#${itemId}`}
        sx={{ minWidth: 70 }}
      />
      <Typography variant="body2">{item.data?.name ?? ""}</Typography>
    </Box>
  );
}

/**
 * Renders the owning form for an Items-tab row as
 * `Chip(form.code) + form.name`. Resolves `formId → form` via the
 * cached get-by-id hook; React Query dedupes across rows. Falls
 * back to `#<id>` while loading or on error so the table stays
 * usable before the lookup returns.
 */
function ItemRowFormCell({ formId }: { formId: number }) {
  const form = useGetCrfForm(formId);
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      <Chip
        size="small"
        variant="outlined"
        label={form.data?.code ?? `#${formId}`}
        sx={{ minWidth: 70 }}
      />
      <Typography variant="body2">{form.data?.name ?? ""}</Typography>
    </Box>
  );
}

interface ColumnDef<T> {
  key: string;
  label: string;
  render: (row: T) => ReactNode;
  width?: number;
}

interface TableProps<T> {
  rows: T[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  emptyText: string;
  errorText: string;
  columns: ColumnDef<T>[];
  onRowClick: (row: T) => void;
}

function ResultTable<T extends { id: number }>({
  rows,
  loading,
  error,
  onRetry,
  emptyText,
  errorText,
  columns,
  onRowClick,
}: TableProps<T>) {
  const { t } = useI18n();

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {errorText.replace("{message}", errorMessage(error))}
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
        sx={{ maxHeight: "calc(100vh - 180px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              {columns.map((c) => (
                <TableCell
                  key={c.key}
                  sx={c.width != null ? { width: c.width } : undefined}
                >
                  {c.label}
                </TableCell>
              ))}
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow
                key={row.id}
                hover
                onClick={() => onRowClick(row)}
                sx={{ cursor: "pointer" }}
              >
                {columns.map((c) => (
                  <TableCell key={c.key}>{c.render(row)}</TableCell>
                ))}
                <TableCell
                  align="right"
                  onClick={(e) => e.stopPropagation()}
                >
                  <Tooltip title={t("crf.globalSearch.row.openTooltip")}>
                    <IconButton
                      size="small"
                      aria-label="open"
                      onClick={() => onRowClick(row)}
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
            <Typography color="text.secondary">{emptyText}</Typography>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}

export function CrfGlobalSearchPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();
  const projectCode = params.projectCode;
  const versionId = search.versionId ?? null;

  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("forms");

  const debouncedFragment = useDebouncedValue(query, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmedFragment = debouncedFragment.trim();
  const showTables = trimmedFragment.length > 0;

  const formsQ = useSearchCrfForms(versionId, debouncedFragment, {
    enabled: tab === "forms",
  });
  const itemsQ = useSearchCrfItems(versionId, debouncedFragment, {
    enabled: tab === "items",
  });
  const unitsQ = useSearchCrfUnits(versionId, debouncedFragment, {
    enabled: tab === "units",
  });
  const optionsQ = useSearchCrfOptions(versionId, debouncedFragment, {
    enabled: tab === "options",
  });
  const domainsQ = useSearchCrfDomainAnnotations(versionId, debouncedFragment, {
    enabled: tab === "domains",
  });
  const annotationsQ = useSearchCrfAnnotations(versionId, debouncedFragment, {
    enabled: tab === "annotations",
  });

  const goBack = () => {
    void navigate({
      to: "/project/$projectCode/crf",
      params: { projectCode },
      search: versionId != null ? { versionId } : undefined,
    });
  };

  const openFormDetail = useOpenFormDetail(projectCode, versionId);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("crf.detail.back")}>
          <IconButton
            aria-label={t("crf.detail.back")}
            onClick={goBack}
          >
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <TextField
          size="small"
          sx={{ flex: 1 }}
          placeholder={t("crf.globalSearch.searchPlaceholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          slotProps={{
            input: {
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon fontSize="small" />
                </InputAdornment>
              ),
            },
          }}
        />
      </Box>
      <Box>
        <ToggleButtonGroup
          exclusive
          value={tab}
          onChange={(_, v: Tab | null) => {
            if (v) setTab(v);
          }}
          size="small"
          aria-label="crf global search tab"
          sx={{ display: "flex", width: "100%" }}
        >
          <ToggleButton sx={{ flex: 1 }} value="forms">
            {t("crf.globalSearch.tab.forms")}
          </ToggleButton>
          <ToggleButton sx={{ flex: 1 }} value="items">
            {t("crf.globalSearch.tab.items")}
          </ToggleButton>
          <ToggleButton sx={{ flex: 1 }} value="units">
            {t("crf.globalSearch.tab.units")}
          </ToggleButton>
          <ToggleButton sx={{ flex: 1 }} value="options">
            {t("crf.globalSearch.tab.options")}
          </ToggleButton>
          <ToggleButton sx={{ flex: 1 }} value="domains">
            {t("crf.globalSearch.tab.domainAnnotations")}
          </ToggleButton>
          <ToggleButton sx={{ flex: 1 }} value="annotations">
            {t("crf.globalSearch.tab.annotations")}
          </ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {!showTables ? (
        <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
          <Typography color="text.secondary">
            {t("crf.globalSearch.emptyInput")}
          </Typography>
        </Box>
      ) : tab === "forms" ? (
        <ResultTable<CrfForm>
          rows={formsQ.data ?? []}
          loading={formsQ.isLoading}
          error={formsQ.error}
          onRetry={() => void formsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.forms")}
          errorText={t("crf.globalSearch.loadFailed.forms")}
          columns={[
            {
              key: "form",
              label: t("crf.globalSearch.col.form"),
              render: (row) => (
                <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                  <Chip
                    size="small"
                    variant="outlined"
                    label={row.code}
                    sx={{ minWidth: 70 }}
                  />
                  <Typography variant="body2">{row.name}</Typography>
                </Box>
              ),
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({ formId: row.id, focus: `form-${row.id}` })
          }
        />
      ) : tab === "items" ? (
        <ResultTable<CrfItem>
          rows={itemsQ.data ?? []}
          loading={itemsQ.isLoading}
          error={itemsQ.error}
          onRetry={() => void itemsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.items")}
          errorText={t("crf.globalSearch.loadFailed.items")}
          columns={[
            {
              key: "form",
              label: t("crf.globalSearch.col.form"),
              render: (row) => <ItemRowFormCell formId={row.formId} />,
            },
            {
              key: "item",
              label: t("crf.globalSearch.col.item"),
              render: (row) => (
                <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                  <Chip
                    size="small"
                    variant="outlined"
                    label={row.code}
                    sx={{ minWidth: 70 }}
                  />
                  <Typography variant="body2">{row.name}</Typography>
                </Box>
              ),
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({
              formId: row.formId,
              focus: `item-${row.id}`,
            })
          }
        />
      ) : tab === "units" ? (
        <ResultTable<CrfUnit>
          rows={unitsQ.data ?? []}
          loading={unitsQ.isLoading}
          error={unitsQ.error}
          onRetry={() => void unitsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.units")}
          errorText={t("crf.globalSearch.loadFailed.units")}
          columns={[
            {
              key: "form",
              label: t("crf.globalSearch.col.form"),
              render: (row) => <UnitRowFormCell itemId={row.itemId} />,
            },
            {
              key: "item",
              label: t("crf.globalSearch.col.item"),
              render: (row) => <UnitRowItemCell itemId={row.itemId} />,
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.unitValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            const itemId = row.itemId;
            const item = unitsQ.data
              ? null
              : null; // intentional: avoid TS narrowing complaints
            void (async () => {
              // Resolve the form id via the cached item query.
              // The component above already fetched the item; here
              // we just look it up via the same queryFn to avoid
              // a second React Query instance.
              try {
                const fetched = await import("../../../shared/api").then(
                  (m) => m.api.getCrfItemById(itemId),
                );
                if (!fetched) return;
                openFormDetail({
                  formId: fetched.formId,
                  focus: `unit-${row.id}`,
                });
              } catch {
                // swallow — no-op when item resolution fails.
              }
            })();
            // Reference `item` so the no-op assignment isn't unused.
            void item;
          }}
        />
      ) : tab === "options" ? (
        <ResultTable<CrfOption>
          rows={optionsQ.data ?? []}
          loading={optionsQ.isLoading}
          error={optionsQ.error}
          onRetry={() => void optionsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.options")}
          errorText={t("crf.globalSearch.loadFailed.options")}
          columns={[
            {
              key: "form",
              label: t("crf.globalSearch.col.form"),
              render: (row) => <UnitRowFormCell itemId={row.itemId} />,
            },
            {
              key: "item",
              label: t("crf.globalSearch.col.item"),
              render: (row) => <UnitRowItemCell itemId={row.itemId} />,
            },
            {
              key: "value",
              label: t("crf.globalSearch.col.optionValue"),
              render: (row) => row.value,
            },
          ]}
          onRowClick={(row) => {
            void (async () => {
              try {
                const fetched = await import("../../../shared/api").then(
                  (m) => m.api.getCrfItemById(row.itemId),
                );
                if (!fetched) return;
                openFormDetail({
                  formId: fetched.formId,
                  focus: `option-${row.id}`,
                });
              } catch {
                // swallow
              }
            })();
          }}
        />
      ) : tab === "domains" ? (
        <ResultTable<DomainAnnotation>
          rows={domainsQ.data ?? []}
          loading={domainsQ.isLoading}
          error={domainsQ.error}
          onRetry={() => void domainsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.domainAnnotations")}
          errorText={t("crf.globalSearch.loadFailed.domainAnnotations")}
          columns={[
            {
              key: "form",
              label: t("crf.globalSearch.col.form"),
              render: (row) => <ItemRowFormCell formId={row.formId} />,
            },
            {
              key: "name",
              label: t("crf.globalSearch.col.name"),
              render: (row) => row.name,
            },
            {
              key: "description",
              label: t("crf.globalSearch.col.description"),
              render: (row) => row.description,
            },
          ]}
          onRowClick={(row) =>
            openFormDetail({
              formId: row.formId,
              focus: `domain-${row.id}`,
            })
          }
        />
      ) : (
        <ResultTable<Annotation>
          rows={annotationsQ.data ?? []}
          loading={annotationsQ.isLoading}
          error={annotationsQ.error}
          onRetry={() => void annotationsQ.refetch()}
          emptyText={t("crf.globalSearch.noMatches.annotations")}
          errorText={t("crf.globalSearch.loadFailed.annotations")}
          columns={[
            {
              key: "content",
              label: t("crf.globalSearch.col.content"),
              render: (row) => row.content,
            },
            {
              key: "assign",
              label: t("crf.globalSearch.col.assign"),
              render: (row) => (row.assign ? "✓" : ""),
            },
            {
              key: "owner",
              label: t("crf.globalSearch.col.owner"),
              render: (row) => `${row.owner.kind}:${row.owner.id}`,
            },
          ]}
          onRowClick={(row) => {
            const owner = row.owner;
            if (owner.kind === "form") {
              openFormDetail({
                formId: owner.id,
                focus: `annotation-${row.id}`,
              });
              return;
            }
            if (owner.kind === "item") {
              void (async () => {
                try {
                  const fetched = await import("../../../shared/api").then(
                    (m) => m.api.getCrfItemById(owner.id),
                  );
                  if (!fetched) return;
                  openFormDetail({
                    formId: fetched.formId,
                    focus: `annotation-${row.id}`,
                  });
                } catch {
                  // swallow
                }
              })();
              return;
            }
            // option / unit owners: no top-level getOption / getUnit
            // endpoint today. Click is a no-op until that lands.
          }}
        />
      )}
    </Box>
  );
}