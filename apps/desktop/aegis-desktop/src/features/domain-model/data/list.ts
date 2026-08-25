import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateSdtmVariableInput,
  type SdtmDomainView,
  type SdtmVariableView,
  type SdtmVersionView,
  type UpdateSdtmDomainInput,
  type UpdateSdtmVariableInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

export function useListSdtmVersions() {
  return useQuery<SdtmVersionView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmVersions(),
    queryFn: () => api.listSdtmVersions(),
  });
}

export function useListSdtmDomains(versionId: number | null) {
  return useQuery<SdtmDomainView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmDomains(versionId ?? 0),
    queryFn: () => api.listSdtmDomainsByVersion(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

export function useDeleteSdtmDomain() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: (id) => api.deleteSdtmDomain(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["domainModel", "sdtmDomains"] });
    },
  });
}

export function useGetSdtmDomain(id: number | null) {
  return useQuery<SdtmDomainView, ApiError>({
    queryKey: queryKeys.domainModel.sdtmDomain(id ?? 0),
    queryFn: () => api.getSdtmDomainById(id!),
    enabled: id != null && id > 0,
  });
}

export function useUpdateSdtmDomain() {
  const qc = useQueryClient();
  return useMutation<
    SdtmDomainView,
    ApiError,
    { id: number; body: UpdateSdtmDomainInput }
  >({
    mutationFn: ({ id, body }) => api.updateSdtmDomain(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: queryKeys.domainModel.sdtmDomain(updated.id),
      });
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmDomains", updated.versionId],
      });
    },
  });
}

export function useListSdtmVariables(domainId: number | null) {
  return useQuery<SdtmVariableView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmVariables(domainId ?? 0),
    queryFn: () => api.listSdtmVariablesByDomain(domainId!),
    enabled: domainId != null && domainId > 0,
  });
}

export function useCreateSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<SdtmVariableView, ApiError, CreateSdtmVariableInput>({
    mutationFn: (input) => api.createSdtmVariable(input),
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: queryKeys.domainModel.sdtmVariables(created.domainId),
      });
    },
  });
}

export function useUpdateSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<
    SdtmVariableView,
    ApiError,
    { id: number; body: UpdateSdtmVariableInput }
  >({
    mutationFn: ({ id, body }) => api.updateSdtmVariable(id, body),
    // We don't know the domainId from `{id}` alone; on the happy path the
    // caller can invalidate manually after a reorder. We invalidate the
    // variables list as a coarse fallback (keyed only by `domainModel`).
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmVariables"],
      });
    },
  });
}

export function useDeleteSdtmVariable() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: (id) => api.deleteSdtmVariable(id),
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmVariables"],
      });
    },
  });
}