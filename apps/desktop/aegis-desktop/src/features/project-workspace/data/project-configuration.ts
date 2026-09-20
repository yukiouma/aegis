import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { api, type ApiError, type ProjectView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
import { useCurrentUser } from "../../auth";

/**
 * Configuration page data hook. Auto-fetches the project (the page is
 * long-lived, unlike the edit drawer that uses manual-trigger
 * `useProject`) and joins it with the current user to derive the
 * leader flag. The leader flag returns `null` while either query is
 * still loading — the page uses that to suppress Save buttons until
 * the truth is in.
 *
 * Re-implements the leader check (rather than calling
 * `useIsProjectLeader` from `features/mission`) because we need the
 * project data alongside the leader flag, and `useIsProjectLeader`
 * doesn't expose `data`. Both end up sharing the same
 * `queryKeys.project.byCode(code)` cache, so a second observer
 * refetching under our feet (e.g. the assign-mission drawer
 * mounting later) doesn't shake the leader flag.
 */
export function useProjectConfiguration(projectCode: string) {
  const currentUser = useCurrentUser();
  const projectQuery = useQuery<ProjectView, ApiError>({
    queryKey: queryKeys.project.byCode(projectCode),
    queryFn: () => api.getProjectByCode(projectCode),
    enabled: projectCode !== null && projectCode !== "",
    staleTime: 0,
  });

  const isLoading = currentUser.isLoading || projectQuery.isLoading;
  const isError = currentUser.isError || projectQuery.isError;
  const error = currentUser.error ?? projectQuery.error ?? null;

  const isLeader = useMemo<boolean | null>(() => {
    if (isLoading) return null;
    if (!currentUser.data || !projectQuery.data) return null;
    const myCode = currentUser.data.code;
    const project = projectQuery.data;
    return (
      project.members.leaders.some((u) => u.code === myCode) ||
      project.unblindMembers.leaders.some((u) => u.code === myCode)
    );
  }, [isLoading, currentUser.data, projectQuery.data]);

  return {
    data: projectQuery.data,
    isLeader,
    isLoading,
    isError,
    error,
  };
}