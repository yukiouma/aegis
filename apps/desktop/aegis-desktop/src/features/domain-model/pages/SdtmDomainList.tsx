import { useEffect, useMemo, useState } from "react";
import { Box, Typography } from "@aegis/ui/mui";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useI18n } from "@aegis/ui/i18n";

import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import { useCurrentUser } from "../../auth";
import {
  useCreateSdtmDomain,
  useDeleteSdtmDomain,
  useListSdtmDomains,
  useListSdtmVersions,
  useUpdateSdtmDomain,
} from "../data";
import type {
  CreateSdtmDomainInput,
  SdtmDomainView,
} from "../../../shared/api";
import {
  DeleteDomainDialog,
  DomainEditDrawer,
  DomainFilterBar,
  DomainTable,
  LanguageDropdown,
  VersionDropdown,
} from "../components";

export function SdtmDomainList() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const versionsQuery = useListSdtmVersions();

  const routeSearch = useSearch({ strict: false }) as {
    versionId?: number;
    lang?: string;
  };
  const urlVersionId = routeSearch.versionId;
  const urlLang = routeSearch.lang;

  const [searchFragment, setSearchFragment] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<SdtmDomainView | null>(null);

  type DomainDrawerState =
    | { mode: "edit"; row: SdtmDomainView }
    | { mode: "create" }
    | null;
  const [domainDrawer, setDomainDrawer] = useState<DomainDrawerState>(null);

  const versions = versionsQuery.data ?? [];

  const selectedVersionId = useMemo<number | null>(() => {
    if (urlVersionId != null && versions.some((v) => v.id === urlVersionId)) {
      return urlVersionId;
    }
    return versions[0]?.id ?? null;
  }, [urlVersionId, versions]);

  useEffect(() => {
    if (versions.length === 0) return;
    const urlIsValid =
      urlVersionId != null && versions.some((v) => v.id === urlVersionId);
    if (urlIsValid) return;
    const fallback = versions[0].id;
    void navigate({
      to: "/domain-model/sdtm",
      replace: true,
      search: (prev) => ({
        ...prev,
        versionId: fallback,
      }),
    });
  }, [urlVersionId, versions, navigate]);

  const setSelectedVersionId = (id: number | null) => {
    void navigate({
      to: "/domain-model/sdtm",
      search: (prev) => ({
        ...prev,
        versionId: id ?? undefined,
        lang: undefined,
      }),
    });
  };

  const domainsQuery = useListSdtmDomains(selectedVersionId);

  const allDomains = domainsQuery.data ?? [];
  const availableLanguages = useMemo(() => {
    const set = new Set<string>();
    for (const d of allDomains) {
      for (const desc of d.descriptions) set.add(desc.lang);
    }
    return [...set].sort();
  }, [allDomains]);

  const selectedLang = useMemo<string | null>(() => {
    if (urlLang && availableLanguages.includes(urlLang)) return urlLang;
    return availableLanguages[0] ?? null;
  }, [urlLang, availableLanguages]);

  useEffect(() => {
    if (availableLanguages.length === 0) return;
    const urlIsValid = urlLang != null && availableLanguages.includes(urlLang);
    if (urlIsValid) return;
    const fallback = availableLanguages[0];
    void navigate({
      to: "/domain-model/sdtm",
      replace: true,
      search: (prev) => ({ ...prev, lang: fallback }),
    });
  }, [urlLang, availableLanguages, navigate]);

  const setSelectedLang = (lang: string | null) => {
    void navigate({
      to: "/domain-model/sdtm",
      search: (prev) => ({ ...prev, lang: lang ?? undefined }),
    });
  };

  const debouncedFragment = useDebouncedValue(searchFragment, {
    delayMs: 300,
    maxWaitMs: 1000,
  });

  const trimmedFragment = debouncedFragment.trim().toLowerCase();
  const filteredRows = useMemo(() => {
    if (!trimmedFragment) return allDomains;
    return allDomains.filter((row) => {
      if (row.name.toLowerCase().includes(trimmedFragment)) return true;
      return row.descriptions.some(
        (d) =>
          d.details.description.toLowerCase().includes(trimmedFragment) ||
          d.details.structure.toLowerCase().includes(trimmedFragment),
      );
    });
  }, [allDomains, trimmedFragment]);

  const deleteDomain = useDeleteSdtmDomain();
  const updateDomain = useUpdateSdtmDomain();
  const createDomain = useCreateSdtmDomain();
  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const noVersions = !versionsQuery.isLoading && versions.length === 0;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>

      {noVersions ? (
        <Typography color="text.secondary">
          {t("domainModel.sdtm.noVersions")}
        </Typography>
      ) : (
        <>
          <Box
            sx={{
              display: "flex",
              gap: 2,
              alignItems: "center",
              flexWrap: "wrap",
            }}
          >
            <DomainFilterBar
              query={searchFragment}
              onQueryChange={setSearchFragment}
            />
            <VersionDropdown
              versions={versions}
              value={selectedVersionId}
              onChange={setSelectedVersionId}
            />
            <LanguageDropdown
              options={availableLanguages}
              value={selectedLang}
              onChange={setSelectedLang}
            />
          </Box>

          <DomainTable
            rows={filteredRows}
            loading={domainsQuery.isLoading}
            error={domainsQuery.error}
            canMutate={canMutate}
            selectedLang={selectedLang}
            onRetry={() => domainsQuery.refetch()}
            onCreate={() => setDomainDrawer({ mode: "create" })}
            onDelete={(row) => setConfirmDelete(row)}
            onNavigate={(row) =>
              navigate({
                to: "/domain-model/sdtm/$domainId",
                params: { domainId: String(row.id) },
                search: { lang: selectedLang ?? undefined },
              })
            }
            emptyMessage={
              trimmedFragment
                ? t("domainModel.sdtm.noMatches")
                : t("domainModel.sdtm.empty")
            }
          />
        </>
      )}

      <DeleteDomainDialog
        open={confirmDelete !== null}
        row={confirmDelete}
        onClose={() => setConfirmDelete(null)}
        onConfirm={(row) =>
          deleteDomain.mutate(row.id, {
            onSuccess: () => setConfirmDelete(null),
          })
        }
        pending={deleteDomain.isPending}
        error={deleteDomain.error}
      />

      {domainDrawer?.mode === "edit" && (
        <DomainEditDrawer
          open
          mode="edit"
          row={domainDrawer.row}
          onClose={() => setDomainDrawer(null)}
          onUpdate={(_id, body) =>
            updateDomain.mutate(
              { id: domainDrawer.row.id, body },
              { onSuccess: () => setDomainDrawer(null) },
            )
          }
          canMutate={canMutate}
          mutationError={updateDomain.error ?? null}
          mutationPending={updateDomain.isPending}
        />
      )}

      {domainDrawer?.mode === "create" && (
        <DomainEditDrawer
          open
          mode="create"
          row={{} as SdtmDomainView}
          versionId={selectedVersionId ?? undefined}
          onClose={() => setDomainDrawer(null)}
          onUpdate={() => {}}
          onCreate={(input: CreateSdtmDomainInput) =>
            createDomain.mutate(input, {
              onSuccess: () => setDomainDrawer(null),
            })
          }
          canMutate={canMutate}
          mutationError={createDomain.error ?? null}
          mutationPending={createDomain.isPending}
        />
      )}
    </Box>
  );
}