import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type AssigneeDataArg,
  type AssigneeViewResponse,
  type CreateMissionInput,
  type MissionKind,
  type MissionViewResponse,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * List missions for a project. `enabled` is gated on a non-empty
 * `projectCode` so the page doesn't fire an empty-keyed query while
 * the project is still resolving.
 */
export function useListMissionsByProject(
  projectCode: string | null,
  kind: MissionKind = "crf",
) {
  return useQuery<MissionViewResponse[], ApiError>({
    queryKey: queryKeys.mission.byProject(projectCode ?? "", kind),
    queryFn: () => api.listMissionsByProject(projectCode!, kind),
    enabled: projectCode != null && projectCode !== "",
    staleTime: 0,
  });
}

/**
 * Add an assignee to an existing mission. The factory takes the
 * `projectCode` and `kind` so the success handler can invalidate the
 * exact kind-bearing cache key (`queryKeys.mission.byProject`
 * includes `kind` in the tuple — using a kind-less key would miss
 * the real cache entry). Defaults `kind` to `"crf"` to match the
 * drawer's current scope.
 */
export function useAddAssignee(
  projectCode: string,
  kind: MissionKind = "crf",
) {
  const qc = useQueryClient();
  return useMutation<
    AssigneeViewResponse,
    ApiError,
    { missionId: number; body: AssigneeDataArg }
  >({
    mutationFn: ({ missionId, body }) => api.addAssignee(missionId, body),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, kind),
      });
    },
  });
}

/**
 * Remove an assignee from a mission. Invalidates the project's
 * kind-bearing mission list — the assignees array on the mission
 * changes, and the next read picks that up.
 */
export function useRemoveAssignee(
  projectCode: string,
  kind: MissionKind = "crf",
) {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { missionId: number; assigneeId: number }
  >({
    mutationFn: ({ missionId, assigneeId }) =>
      api.removeAssignee(missionId, assigneeId),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, kind),
      });
    },
  });
}

/**
 * Create a new mission with the first assignee baked in. The drawer
 * uses this when no mission exists yet for a CRF form: the picked
 * user becomes the first assignee on the brand-new mission. On
 * success the project's kind-bearing mission list is invalidated so
 * the table cell re-renders with the new chips.
 */
export function useCreateMission(
  projectCode: string,
  kind: MissionKind = "crf",
) {
  const qc = useQueryClient();
  return useMutation<MissionViewResponse, ApiError, CreateMissionInput>({
    mutationFn: (input) => api.createMission(input),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode, kind),
      });
    },
  });
}