import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import { Alert, Box, CircularProgress } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser } from "../../auth";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import {
  useCreateSdtmVariable,
  useDeleteSdtmVariable,
  useGetSdtmDomain,
  useListSdtmVariables,
  useUpdateSdtmDomain,
  useUpdateSdtmVariable,
} from "../data";
import type {
  CreateSdtmVariableInput,
  SdtmVariableView,
  UpdateSdtmDomainInput,
  UpdateSdtmVariableInput,
} from "../../../shared/api";
import {
  DeleteVariableDialog,
  DomainEditDrawer,
  DomainFilterBar,
  DomainHeaderTable,
  LanguageDropdown,
  VariableEditDrawer,
  VariableTable,
} from "../components";

type VariableDrawerState =
  | { mode: "create" }
  | { mode: "edit"; row: SdtmVariableView }
  | null;

export function SdtmDomainDetail() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = useParams({
    from: "/_authed/_layout/domain-model/sdtm/$domainId",
  });
  const routeSearch = useSearch({
    from: "/_authed/_layout/domain-model/sdtm/$domainId",
  });
  const domainId = Number(params.domainId);

  const currentUser = useCurrentUser();
  const role = currentUser.data?.role;
  const canMutate = role === "admin" || role === "root";

  const domainQuery = useGetSdtmDomain(
    Number.isFinite(domainId) && domainId > 0 ? domainId : null,
  );
  const variablesQuery = useListSdtmVariables(
    Number.isFinite(domainId) && domainId > 0 ? domainId : null,
  );

  const allVariables = variablesQuery.data ?? [];

  const availableLanguages = useMemo(() => {
    const set = new Set<string>();
    for (const d of domainQuery.data?.descriptions ?? []) set.add(d.lang);
    for (const v of allVariables) {
      for (const desc of v.descriptions) set.add(desc.lang);
    }
    return [...set].sort();
  }, [domainQuery.data, allVariables]);

  const urlLang = routeSearch.lang;
  const selectedLang = useMemo<string | null>(() => {
    if (urlLang && availableLanguages.includes(urlLang)) return urlLang;
    return availableLanguages[0] ?? null;
  }, [urlLang, availableLanguages]);

  useEffect(() => {
    if (availableLanguages.length === 0) return;
    if (urlLang && availableLanguages.includes(urlLang)) return;
    const fallback = availableLanguages[0];
    void navigate({
      to: "/domain-model/sdtm/$domainId",
      params: { domainId: String(domainId) },
      search: { lang: fallback },
      replace: true,
    });
  }, [availableLanguages, urlLang, navigate, domainId]);

  const [searchFragment, setSearchFragment] = useState("");
  const debouncedFragment = useDebouncedValue(searchFragment, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmed = debouncedFragment.trim().toLowerCase();
  const filteredRows = useMemo(() => {
    if (!trimmed) return allVariables;
    return allVariables.filter((v) => {
      if (v.name.toLowerCase().includes(trimmed)) return true;
      const desc = selectedLang
        ? v.descriptions.find((d) => d.lang === selectedLang)?.details.label
        : undefined;
      return desc != null && desc.toLowerCase().includes(trimmed);
    });
  }, [allVariables, trimmed, selectedLang]);

  const [editDomainDrawerOpen, setEditDomainDrawerOpen] = useState(false);
  const [variableDrawer, setVariableDrawer] =
    useState<VariableDrawerState>(null);
  const [confirmDelete, setConfirmDelete] = useState<SdtmVariableView | null>(
    null,
  );
  const [reorderFailed, setReorderFailed] = useState<string | null>(null);

  const initialSequence = useMemo(() => {
    if (allVariables.length === 0) return 1;
    return Math.max(...allVariables.map((v) => v.variableSequence)) + 1;
  }, [allVariables]);

  const updateDomain = useUpdateSdtmDomain();
  const createVariable = useCreateSdtmVariable();
  const updateVariable = useUpdateSdtmVariable();
  const deleteVariable = useDeleteSdtmVariable();

  function handleBack() {
    const versionId = domainQuery.data?.versionId;
    void navigate({
      to: "/domain-model/sdtm",
      search: {
        versionId: versionId ?? undefined,
        lang: selectedLang ?? undefined,
      },
    });
  }

  function handleReorder(orderedIds: number[]) {
    setReorderFailed(null);
    orderedIds.forEach((id, index) => {
      const newSeq = index + 1;
      const original = allVariables.find((v) => v.id === id);
      if (!original || original.variableSequence === newSeq) return;
      updateVariable.mutate(
        { id, body: { variableSequence: newSeq } },
        {
          onError: (err: unknown) => {
            setReorderFailed(
              t("domainModel.sdtm.detail.reorderFailed", {
                message: String(err),
              }),
            );
          },
        },
      );
    });
  }

  if (!Number.isFinite(domainId) || domainId <= 0) {
    return (
      <Box sx={{ p: 4 }}>
        <Alert severity="error">
          {t("domainModel.sdtm.detail.loadFailed", {
            message: "invalid domain id",
          })}
        </Alert>
      </Box>
    );
  }

  if (domainQuery.isLoading) {
    return (
      <Box sx={{ p: 4, display: "flex", justifyContent: "center" }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <DomainHeaderTable
        domain={domainQuery.data}
        loading={domainQuery.isLoading}
        error={domainQuery.error ?? null}
        canMutate={canMutate && !!domainQuery.data}
        selectedLang={selectedLang}
        onEdit={() => setEditDomainDrawerOpen(true)}
        onBack={handleBack}
      />

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
          placeholderKey="domainModel.sdtm.detail.filter.placeholder"
        />
        <LanguageDropdown
          options={availableLanguages}
          value={selectedLang}
          onChange={(lang) => {
            void navigate({
              to: "/domain-model/sdtm/$domainId",
              params: { domainId: String(domainId) },
              search: { lang: lang ?? undefined },
              replace: true,
            });
          }}
        />
      </Box>

      {reorderFailed && (
        <Alert severity="warning" onClose={() => setReorderFailed(null)}>
          {reorderFailed}
        </Alert>
      )}

      <VariableTable
        rows={filteredRows}
        loading={variablesQuery.isLoading}
        error={variablesQuery.error ?? null}
        canMutate={canMutate}
        selectedLang={selectedLang}
        onRetry={() => variablesQuery.refetch()}
        onCreate={() => setVariableDrawer({ mode: "create" })}
        onEdit={(row) => setVariableDrawer({ mode: "edit", row })}
        onDelete={(row) => setConfirmDelete(row)}
        onReorder={handleReorder}
        emptyMessage={
          trimmed
            ? t("domainModel.sdtm.detail.noMatches")
            : t("domainModel.sdtm.detail.empty")
        }
      />

      {domainQuery.data && (
        <DomainEditDrawer
          open={editDomainDrawerOpen}
          row={domainQuery.data}
          onClose={() => setEditDomainDrawerOpen(false)}
          onUpdate={(_id: number, body: UpdateSdtmDomainInput) =>
            updateDomain.mutate(
              { id: domainQuery.data!.id, body },
              { onSuccess: () => setEditDomainDrawerOpen(false) },
            )
          }
          canMutate={canMutate}
          mutationError={updateDomain.error ?? null}
          mutationPending={updateDomain.isPending}
        />
      )}

      <VariableEditDrawer
        open={variableDrawer !== null}
        mode={variableDrawer?.mode ?? "create"}
        row={variableDrawer?.mode === "edit" ? variableDrawer.row : undefined}
        domainId={domainId}
        initialSequence={initialSequence}
        onClose={() => setVariableDrawer(null)}
        onCreate={(input: CreateSdtmVariableInput) =>
          createVariable.mutate(input, {
            onSuccess: () => setVariableDrawer(null),
          })
        }
        onUpdate={(id: number, body: UpdateSdtmVariableInput) =>
          updateVariable.mutate(
            { id, body },
            { onSuccess: () => setVariableDrawer(null) },
          )
        }
        canMutate={canMutate}
        mutationError={createVariable.error ?? updateVariable.error ?? null}
        mutationPending={
          createVariable.isPending || updateVariable.isPending
        }
      />

      <DeleteVariableDialog
        open={confirmDelete !== null}
        row={confirmDelete}
        onClose={() => setConfirmDelete(null)}
        onConfirm={(row) =>
          deleteVariable.mutate(row.id, {
            onSuccess: () => setConfirmDelete(null),
          })
        }
        pending={deleteVariable.isPending}
        error={deleteVariable.error}
      />
    </Box>
  );
}