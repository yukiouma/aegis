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
 * `projectCode` so the success handler can invalidate that project's
 * mission list — the drawer always lives in the context of a project,
 * so the caller already has it.
 */
export function useAddAssignee(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<
    AssigneeViewResponse,
    ApiError,
    { missionId: number; body: AssigneeDataArg }
  >({
    mutationFn: ({ missionId, body }) => api.addAssignee(missionId, body),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode),
      });
    },
  });
}

/**
 * Remove an assignee from a mission. Invalidates the project's
 * mission list — the assignees array on the mission changes, and the
 * next read picks that up.
 */
export function useRemoveAssignee(projectCode: string) {
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
        queryKey: queryKeys.mission.byProject(projectCode),
      });
    },
  });
}

/**
 * Create a new mission with the first assignee baked in. The drawer
 * uses this when no mission exists yet for a CRF form: the picked
 * user becomes the first assignee on the brand-new mission. On
 * success the project's mission list is invalidated so the table
 * cell re-renders with the new chips.
 */
export function useCreateMission(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<MissionViewResponse, ApiError, CreateMissionInput>({
    mutationFn: (input) => api.createMission(input),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.byProject(projectCode),
      });
    },
  });
}