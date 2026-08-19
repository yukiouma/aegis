import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CodeItemView,
  type CodeListView,
  type CreateCodeItemInput,
  type CreateCodeListInput,
  type CreateTerminologyVersionInput,
  type TerminologyVersionView,
  type UpdateCodeItemInput,
  type UpdateCodeListInput,
  type UpdateTerminologyVersionInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

// ---- Versions ----

/**
 * All terminology versions. Consumed by the version dropdown on every
 * terminology page; always enabled since the dropdown renders for
 * every authenticated user.
 */
export function useListTerminologyVersions() {
  return useQuery<TerminologyVersionView[], ApiError>({
    queryKey: queryKeys.terminology.versions(),
    queryFn: () => api.listTerminologyVersions(),
  });
}

export function useCreateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    CreateTerminologyVersionInput
  >({
    mutationFn: api.createTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useUpdateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    { id: number; body: UpdateTerminologyVersionInput }
  >({
    mutationFn: ({ id, body }) => api.updateTerminologyVersion(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useDeleteTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: api.deleteTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

// ---- Code lists ----

/**
 * Codelists for a given terminology version. `versionId === null` (or 0)
 * disables the query — the dropdown on the page starts unselected until
 * the user picks a version.
 */
export function useListCodeLists(versionId: number | null) {
  return useQuery<CodeListView[], ApiError>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0),
    queryFn: () => api.listCodeLists(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

/**
 * Single codelist by id. `id === null` disables the query — the page
 * starts disabled until the route provides the id.
 */
export function useGetCodeList(id: number | null) {
  return useQuery<CodeListView, ApiError>({
    queryKey: queryKeys.terminology.codeList(id ?? 0),
    queryFn: () => api.getCodeListById(id!),
    enabled: id != null && id > 0,
  });
}

export function useCreateCodeList() {
  const qc = useQueryClient();
  return useMutation<CodeListView, ApiError, CreateCodeListInput>({
    mutationFn: api.createCodeList,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeLists(created.versionId),
      });
    },
  });
}

export function useUpdateCodeList() {
  const qc = useQueryClient();
  return useMutation<
    CodeListView,
    ApiError,
    { id: number; body: UpdateCodeListInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeList(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeLists(updated.versionId),
      });
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeList(updated.id),
      });
    },
  });
}

export function useDeleteCodeList() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; versionId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeList(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeLists(vars.versionId),
      });
    },
  });
}

// ---- Code items ----

/**
 * Code items for a given codelist. `codelistId === null` (or 0) disables
 * the query until the page knows which codelist to load.
 */
export function useListCodeItems(codelistId: number | null) {
  return useQuery<CodeItemView[], ApiError>({
    queryKey: queryKeys.terminology.codeItems(codelistId ?? 0),
    queryFn: () => api.listCodeItems(codelistId!),
    enabled: codelistId != null && codelistId > 0,
  });
}

export function useCreateCodeItem() {
  const qc = useQueryClient();
  return useMutation<CodeItemView, ApiError, CreateCodeItemInput>({
    mutationFn: api.createCodeItem,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeItems(created.codelistId),
      });
    },
  });
}

export function useUpdateCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    CodeItemView,
    ApiError,
    { id: number; body: UpdateCodeItemInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeItem(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeItems(updated.codelistId),
      });
    },
  });
}

export function useDeleteCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; codelistId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeItem(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeItems(vars.codelistId),
      });
    },
  });
}