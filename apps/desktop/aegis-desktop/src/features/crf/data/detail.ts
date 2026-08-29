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

/**
 * Collect every annotation in a form detail whose `domainAnnotationId`
 * matches the supplied id. Walks the four owner buckets (form-level,
 * item, option, unit) so the cascade catches them all.
 */
function collectAnnotationIdsForDomain(
  detail: CrfFormDetail,
  domainAnnotationId: number,
): number[] {
  const ids: number[] = [];
  const collect = (a: { domainAnnotationId: number; id: number }) => {
    if (a.domainAnnotationId === domainAnnotationId) ids.push(a.id);
  };
  detail.formAnnotations.forEach(collect);
  for (const item of detail.items) {
    item.annotations.forEach(collect);
    for (const opt of item.options) opt.annotations.forEach(collect);
    for (const u of item.units) u.annotations.forEach(collect);
  }
  return ids;
}

export function useDeleteDomainAnnotation() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { id: number; formId: number }>({
    // Cascade: delete every annotation pointing at this domain
    // annotation (across form / item / option / unit owners), then
    // delete the domain annotation itself. Sequential so a failure on
    // any step surfaces to the user via the standard mutation error
    // path; the caller's dialog won't close until the whole sequence
    // succeeds.
    mutationFn: async ({ id, formId }) => {
      const detail = qc.getQueryData<CrfFormDetail>(
        queryKeys.crf.formDetail(formId),
      );
      const annotationIds = detail
        ? collectAnnotationIdsForDomain(detail, id)
        : [];
      for (const annId of annotationIds) {
        await api.deleteCrfAnnotation(annId);
      }
      await api.deleteCrfDomainAnnotation(id);
    },
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
