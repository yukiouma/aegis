import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { api, type ApiError, type ProjectView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
import { useCurrentUser } from "../../auth";

/**
 * Client-side leader check. Hides the per-row assignee-edit icon for
 * users who are not project leaders. The server-side `MissionUsecase
 * ::ensure_leader` is the authoritative gate — this hook only governs
 * UI affordance.
 *
 * Returns `null` while any of the dependent queries is still loading,
 * so callers can distinguish "not a leader" (`false`) from "unknown
 * yet" (`null`) and avoid flashing the icon during a refresh.
 *
 * Uses its own auto-enabled `get_project_by_code` query rather than
 * `useProject` from `project-list`, because that hook is
 * manual-trigger (`enabled: false`) — it only fetches when the drawer
 * explicitly calls `refetch()`. We need an auto-enabled read here
 * since the icon needs the answer on first mount.
 */
export function useIsProjectLeader(
  projectCode: string | null,
): boolean | null {
  const currentUser = useCurrentUser();
  const projectQuery = useQuery<ProjectView, ApiError>({
    queryKey:
      projectCode === null || projectCode === ""
        ? ["project", "byCode", "__disabled__"]
        : queryKeys.project.byCode(projectCode),
    queryFn: () => {
      if (projectCode === null || projectCode === "") {
        throw new Error("useIsProjectLeader disabled");
      }
      return api.getProjectByCode(projectCode);
    },
    enabled: projectCode != null && projectCode !== "",
    staleTime: 0,
  });
  const project = projectQuery.data;

  return useMemo(() => {
    if (projectCode == null || projectCode === "") return null;
    if (currentUser.isLoading || projectQuery.isFetching) return null;
    if (!currentUser.data || !project) return null;
    const myCode = currentUser.data.code;
    const leaders = project.members?.leaders ?? [];
    const unblindLeaders = project.unblindMembers?.leaders ?? [];
    return (
      leaders.some((u) => u.code === myCode) ||
      unblindLeaders.some((u) => u.code === myCode)
    );
  }, [
    projectCode,
    currentUser.isLoading,
    currentUser.data,
    projectQuery.isFetching,
    project,
  ]);
}