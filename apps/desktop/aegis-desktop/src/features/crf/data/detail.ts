import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type AnnotationOwner,
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

/**
 * Look up the current `notSubmitted` flag for an owner in the cached
 * detail. Returns `null` if the owner is not found (e.g. the cache is
 * empty or stale) — callers use that to skip the cascade rather than
 * risk deleting annotations that should stay.
 */
function readCurrentNotSubmitted(
  detail: CrfFormDetail,
  owner: AnnotationOwner,
): boolean | null {
  switch (owner.kind) {
    case "form":
      return detail.form.notSubmitted;
    case "item": {
      const found = detail.items.find((i) => i.item.id === owner.id);
      return found ? found.item.notSubmitted : null;
    }
    case "option":
      for (const item of detail.items) {
        const opt = item.options.find((o) => o.option.id === owner.id);
        if (opt) return opt.option.notSubmitted;
      }
      return null;
    case "unit":
      for (const item of detail.items) {
        const u = item.units.find((uu) => uu.unit.id === owner.id);
        if (u) return u.unit.notSubmitted;
      }
      return null;
  }
}

/**
 * Collect every annotation in the form detail attached to `owner`.
 * Walks the relevant buckets:
 *   - form → form-level + every item / option / unit annotation
 *   - item → the item's annotations + each option / unit annotation
 *   - option → the option's annotations only
 *   - unit → the unit's annotations only
 *
 * Returning an empty array when the owner is not found keeps the
 * cascade a no-op rather than throwing inside the mutation body.
 */
function collectAnnotationIdsForOwner(
  detail: CrfFormDetail,
  owner: AnnotationOwner,
): number[] {
  const ids: number[] = [];
  if (owner.kind === "form") {
    detail.formAnnotations.forEach((a) => ids.push(a.id));
    for (const item of detail.items) {
      item.annotations.forEach((a) => ids.push(a.id));
      item.options.forEach((o) =>
        o.annotations.forEach((a) => ids.push(a.id)),
      );
      item.units.forEach((u) =>
        u.annotations.forEach((a) => ids.push(a.id)),
      );
    }
    return ids;
  }
  if (owner.kind === "item") {
    const item = detail.items.find((i) => i.item.id === owner.id);
    if (!item) return ids;
    item.annotations.forEach((a) => ids.push(a.id));
    item.options.forEach((o) =>
      o.annotations.forEach((a) => ids.push(a.id)),
    );
    item.units.forEach((u) =>
      u.annotations.forEach((a) => ids.push(a.id)),
    );
    return ids;
  }
  if (owner.kind === "option") {
    for (const item of detail.items) {
      const opt = item.options.find((o) => o.option.id === owner.id);
      if (opt) opt.annotations.forEach((a) => ids.push(a.id));
    }
    return ids;
  }
  // unit
  for (const item of detail.items) {
    const u = item.units.find((uu) => uu.unit.id === owner.id);
    if (u) u.annotations.forEach((a) => ids.push(a.id));
  }
  return ids;
}

/**
 * Update the `notSubmitted` flag on a form / item / option / unit
 * owner. When the flag transitions from `false` to `true`, the
 * owner is wiped so a "not submitted" owner holds no annotations:
 *   - form → all annotations AND all domain annotations in the form
 *   - item → item's annotations + every option / unit annotation
 *   - option / unit → own annotations only
 * Annotations are deleted before domain annotations, and both before
 * the PATCH, so a halfway failure surfaces through the standard
 * mutation error path and the cache never lands in a "not submitted
 * owner still references deleted annotations" state. The `true →
 * false` transition just lifts the flag with no cascade.
 */
export function useUpdateOwnerNotSubmitted() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    {
      formId: number;
      owner: AnnotationOwner;
      notSubmitted: boolean;
    }
  >({
    mutationFn: async ({ formId, owner, notSubmitted }) => {
      const detail = qc.getQueryData<CrfFormDetail>(
        queryKeys.crf.formDetail(formId),
      );
      const current = detail
        ? readCurrentNotSubmitted(detail, owner)
        : null;
      if (
        current === false &&
        notSubmitted === true &&
        detail
      ) {
        const annIds = collectAnnotationIdsForOwner(detail, owner);
        for (const annId of annIds) {
          await api.deleteCrfAnnotation(annId);
        }
        // Form-level cascade also wipes every domain annotation on
        // the form. The spec for the form owner says "remove all the
        // (domain_)annotations in this form" — domain annotations
        // belong here too. Without this, marking a form not-submitted
        // would leave its domain annotations in place, so the next
        // "new domain annotation" would coexist with stale ones.
        // Deleting domain annotations AFTER annotations (and BEFORE
        // the form's notSubmitted PATCH) means a halfway failure
        // leaves the form either fully populated or fully empty of
        // both kinds — never half-empty with dangling references.
        if (owner.kind === "form") {
          for (const d of detail.domainAnnotations) {
            await api.deleteCrfDomainAnnotation(d.id);
          }
        }
      }
      switch (owner.kind) {
        case "form":
          await api.updateCrfForm(owner.id, { notSubmitted });
          break;
        case "item":
          await api.updateCrfItem(owner.id, { notSubmitted });
          break;
        case "option":
          await api.updateCrfOption(owner.id, { notSubmitted });
          break;
        case "unit":
          await api.updateCrfUnit(owner.id, { notSubmitted });
          break;
      }
    },
    onSuccess: (_void, vars) => {
      void qc.invalidateQueries({ queryKey: queryKeys.crf.formDetail(vars.formId) });
      // Also invalidate the single-form query that drives the header
      // chip (`useGetCrfForm` reads `form.notSubmitted` from there).
      // Without this, the form-level [NOT SUBMITTED] chip in the
      // page header stays stale until the next manual refetch — and
      // for the item / option / unit cases it doesn't matter (those
      // chips read from the form detail), but invalidating the
      // single-form query is cheap and keeps the rule uniform: every
      // success of this hook refreshes every cache that could expose
      // the changed flag.
      void qc.invalidateQueries({ queryKey: queryKeys.crf.form(vars.formId) });
    },
  });
}
