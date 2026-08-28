import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CrfForm,
  type CrfVersion,
  type CreateCrfFormInput,
  type UpdateCrfFormInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query/keys";

export function useListCrfVersions(projectCode: string | null) {
  return useQuery<CrfVersion[], ApiError>({
    queryKey: queryKeys.crf.versionsByProject(projectCode ?? ""),
    queryFn: () => api.listCrfVersions(projectCode!),
    enabled: projectCode != null && projectCode !== "",
  });
}

export function useListCrfForms(versionId: number | null) {
  return useQuery<CrfForm[], ApiError>({
    queryKey: queryKeys.crf.formsByVersion(versionId ?? 0),
    queryFn: () => api.listCrfFormsByVersion(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

export function useGetCrfForm(id: number | null) {
  return useQuery<CrfForm, ApiError>({
    queryKey: queryKeys.crf.form(id ?? 0),
    queryFn: () => api.getCrfFormById(id!),
    enabled: id != null && Number.isFinite(id) && id > 0,
  });
}

export function useCreateCrfForm() {
  const qc = useQueryClient();
  return useMutation<
    CrfForm,
    ApiError,
    { versionId: number; body: CreateCrfFormInput }
  >({
    mutationFn: ({ versionId, body }) => api.createCrfForm(versionId, body),
    onSuccess: (created) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(created.versionId),
      });
    },
  });
}

export function useUpdateCrfForm() {
  const qc = useQueryClient();
  return useMutation<
    CrfForm,
    ApiError,
    { id: number; body: UpdateCrfFormInput }
  >({
    mutationFn: ({ id, body }) => api.updateCrfForm(id, body),
    onSuccess: (updated) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(updated.versionId),
      });
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.form(updated.id),
      });
    },
  });
}

export function useDeleteCrfForm() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; versionId: number }>({
    mutationFn: ({ id }) => api.deleteCrfForm(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.crf.formsByVersion(vars.versionId),
      });
    },
  });
}