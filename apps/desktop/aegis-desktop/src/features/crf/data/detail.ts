import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateAnnotationInput,
  type CreateDomainAnnotationInput,
  type CrfFormDetail,
  type UpdateAnnotationInput,
  type UpdateDomainAnnotationInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query/keys";

export function useCrfFormDetail(id: number | null) {
  return useQuery<CrfFormDetail, ApiError>({
    queryKey: queryKeys.crf.formDetail(id ?? 0),
    queryFn: () => api.getCrfFormDetails(id!),
    enabled: id != null && id > 0,
  });
}

export function useCreateDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.createCrfDomainAnnotation>>,
    ApiError,
    { formId: number; body: CreateDomainAnnotationInput }
  >({
    mutationFn: ({ formId, body }) => api.createCrfDomainAnnotation(formId, body),
    onSuccess: (_d, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useUpdateDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.updateCrfDomainAnnotation>>,
    ApiError,
    { id: number; formId: number; body: UpdateDomainAnnotationInput }
  >({
    mutationFn: ({ id, body }) => api.updateCrfDomainAnnotation(id, body),
    onSuccess: (_d, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useDeleteDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; formId: number }>({
    mutationFn: ({ id }) => api.deleteCrfDomainAnnotation(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useCreateAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.createCrfAnnotation>>,
    ApiError,
    { formId: number; body: CreateAnnotationInput }
  >({
    mutationFn: ({ body }) => api.createCrfAnnotation(body),
    onSuccess: (_a, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useUpdateAnnotation() {
  const qc = useQueryClient();
  return useMutation<
    Awaited<ReturnType<typeof api.updateCrfAnnotation>>,
    ApiError,
    { id: number; formId: number; body: UpdateAnnotationInput }
  >({
    mutationFn: ({ id, body }) => api.updateCrfAnnotation(id, body),
    onSuccess: (_a, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}

export function useDeleteAnnotation() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; formId: number }>({
    mutationFn: ({ id }) => api.deleteCrfAnnotation(id),
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
    },
  });
}
