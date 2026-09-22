import { useEffect, useMemo, useRef } from "react";

import {
  type ApiError,
  type ProjectView,
  type SdtmDomainView,
} from "../../../shared/api";
import {
  useListSdtmDomains,
  useListSdtmVersions,
} from "../../domain-model/data";
import { useProject, useUpdateProject } from "../../project-list/data/projects";

export interface SdtmContext {
  /** Resolved SDTMIG version id. `null` while versions are still
   *  pending or none exist. */
  versionId: number | null;
  /** Resolved language code (project's `configurations.language`
   *  or `"en"` when unset). */
  language: string;
  /** SDTM domains for the resolved version. Empty while loading
   *  or unresolved. */
  domains: SdtmDomainView[];
  /** `true` while versions or domains are still fetching. */
  loading: boolean;
  /** First non-null error from versions or domains query, if any. */
  error: ApiError | null;
}

/**
 * Resolve the project's SDTMIG version + language for the CRF detail
 * page, and expose the SDTM domain list for the resolved version.
 *
 * Resolution rules:
 *   1. If `configurations.sdtmig.versionId` is unset → pick the
 *      SDTM version with the highest `id`.
 *   2. If the configured `versionId` exists in the versions list →
 *      use it directly.
 *   3. If the configured `versionId` is stale but `versionName`
 *      still exists → fire a one-shot `useUpdateProject` to swap
 *      in the matching `versionId` and use it.
 *   4. Otherwise → fall back to (1).
 *
 * Language is `project.configurations.language ?? "en"`.
 */
export function useSdtmContext(projectCode: string): SdtmContext {
  const projectQuery = useProject(projectCode, { enabled: true });
  const versionsQuery = useListSdtmVersions();
  const updateProject = useUpdateProject();

  const versions = versionsQuery.data ?? [];
  const project: ProjectView | undefined = projectQuery.data;

  const sdtmig = project?.configurations.sdtmig;

  const resolvedVersionId = useMemo<number | null>(() => {
    if (versions.length === 0) return null;
    const configuredId = sdtmig?.versionId ?? null;
    if (configuredId == null) {
      return versions.reduce(
        (max, v) => (v.id > max ? v.id : max),
        versions[0]!.id,
      );
    }
    if (versions.some((v) => v.id === configuredId)) {
      return configuredId;
    }
    // Stale id — try name recovery.
    const byName = versions.find(
      (v) =>
        sdtmig?.versionName &&
        v.name.toLowerCase() === sdtmig.versionName.toLowerCase(),
    );
    if (byName) return byName.id;
    return versions.reduce(
      (max, v) => (v.id > max ? v.id : max),
      versions[0]!.id,
    );
  }, [versions, sdtmig]);

  // Fire the recovery mutation at most once per stale id per session.
  const correctedRef = useRef<Set<number>>(new Set());
  useEffect(() => {
    if (!project || versions.length === 0) return;
    const configuredId = sdtmig?.versionId ?? null;
    if (configuredId == null) return;
    if (versions.some((v) => v.id === configuredId)) return;
    if (correctedRef.current.has(configuredId)) return;
    const byName = versions.find(
      (v) =>
        sdtmig?.versionName &&
        v.name.toLowerCase() === sdtmig.versionName.toLowerCase(),
    );
    if (!byName) return;
    correctedRef.current.add(configuredId);
    updateProject.mutate({
      code: projectCode,
      body: {
        configurations: {
          language: project.configurations.language,
          tags: project.configurations.tags,
          sdtmig: { versionId: byName.id, versionName: byName.name },
        },
      },
    });
  }, [project, versions, sdtmig, projectCode, updateProject]);

  const domainsQuery = useListSdtmDomains(resolvedVersionId);
  const domains = resolvedVersionId == null ? [] : (domainsQuery.data ?? []);

  const language = project?.configurations.language ?? "en";
  const loading =
    versionsQuery.isLoading ||
    (resolvedVersionId != null && domainsQuery.isLoading);
  const error =
    (versionsQuery.error as ApiError | null) ??
    (domainsQuery.error as ApiError | null) ??
    null;

  return {
    versionId: resolvedVersionId,
    language,
    domains,
    loading,
    error,
  };
}