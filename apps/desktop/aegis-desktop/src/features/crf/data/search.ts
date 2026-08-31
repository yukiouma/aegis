import { useQuery, type UseQueryResult } from "@tanstack/react-query";

import {
  api,
  type Annotation,
  type ApiError,
  type CrfForm,
  type CrfItem,
  type CrfOption,
  type CrfUnit,
  type DomainAnnotation,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query/keys";

interface EnabledOptions {
  enabled?: boolean;
}

/**
 * React-Query hook for `GET /api/crf/versions/{id}/forms/search`.
 * Disabled when `versionId` is unset / non-positive, when the trimmed
 * fragment is empty, or when the caller passes `enabled: false`. The
 * page passes `enabled` to gate the fetch on the active tab — only
 * the currently-visible tab actually issues an HTTP call.
 */
export function useSearchCrfForms(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfForm[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfForm[], ApiError>({
    queryKey: queryKeys.crf.searchFormsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfFormsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfItems(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfItem[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfItem[], ApiError>({
    queryKey: queryKeys.crf.searchItemsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfItemsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfUnits(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfUnit[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfUnit[], ApiError>({
    queryKey: queryKeys.crf.searchUnitsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfUnitsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfOptions(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<CrfOption[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<CrfOption[], ApiError>({
    queryKey: queryKeys.crf.searchOptionsByVersion(versionId ?? 0, fragment),
    queryFn: () => api.searchCrfOptionsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfDomainAnnotations(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<DomainAnnotation[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<DomainAnnotation[], ApiError>({
    queryKey: queryKeys.crf.searchDomainAnnotationsByVersion(
      versionId ?? 0,
      fragment,
    ),
    queryFn: () =>
      api.searchCrfDomainAnnotationsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

export function useSearchCrfAnnotations(
  versionId: number | null,
  fragment: string,
  options: EnabledOptions = {},
): UseQueryResult<Annotation[], ApiError> {
  const trimmed = fragment.trim();
  return useQuery<Annotation[], ApiError>({
    queryKey: queryKeys.crf.searchAnnotationsByVersion(
      versionId ?? 0,
      fragment,
    ),
    queryFn: () => api.searchCrfAnnotationsByVersion(versionId!, fragment),
    enabled:
      options.enabled !== false &&
      versionId != null &&
      versionId > 0 &&
      trimmed !== "",
  });
}

/**
 * Fetch a single CRF item by id. Used by the Units / Options /
 * Annotations tables to resolve `itemId → item.formId` (and
 * `item.code`) for row rendering and click navigation. React Query
 * dedupes identical `id` lookups across rows so 50 units under the
 * same item share a single HTTP round-trip.
 */
export function useGetCrfItem(
  id: number | null,
): UseQueryResult<CrfItem, ApiError> {
  return useQuery<CrfItem, ApiError>({
    queryKey: queryKeys.crf.item(id ?? 0),
    queryFn: () => api.getCrfItemById(id!),
    enabled: id != null && id > 0,
  });
}