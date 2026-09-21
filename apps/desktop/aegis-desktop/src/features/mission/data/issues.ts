import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type AppendCommentInput,
  type CreateIssueInput,
  type IssueState,
  type IssueViewResponse,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * List every issue attached to a mission. We deliberately fetch ALL
 * states in one call (no server-side `?state=` filter) and filter on
 * the client. The CRF detail page reads `openIssueCountByTarget`
 * derived from this single query to drive chip badges, so a state-
 * filtered fetch would force a second round-trip just to count
 * opened issues.
 */
export function useListIssuesByMission(missionId: number | null) {
  return useQuery<IssueViewResponse[], ApiError>({
    queryKey: queryKeys.mission.issuesByMission(missionId ?? -1),
    queryFn: () => api.listIssuesByMission(missionId!, {}),
    enabled: missionId != null,
    staleTime: 0,
  });
}

/**
 * Open a new issue against a mission. `targetItem` is set by the
 * caller — the form chip passes `undefined` for a whole-mission
 * issue; an item chip passes the item code. On success the per-mission
 * list is invalidated so the chip's red-dot badge updates reactively.
 */
export function useCreateIssue() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; body: CreateIssueInput }
  >({
    mutationFn: ({ missionId, body }) => api.createIssue(missionId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}

/**
 * Flip an issue's open/closed state. Backend is idempotent
 * (closing a closed issue is a no-op success; same for reopening) so
 * the client doesn't guard against no-op transitions. Same
 * invalidation pattern as `useCreateIssue`.
 */
export function usePatchIssueState() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; issueId: number; next: IssueState }
  >({
    mutationFn: ({ issueId, next }) => api.patchIssueState(issueId, next),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}

/**
 * Append a comment to an issue's thread. Comments are append-only —
 * no edit / delete endpoints. The returned `IssueViewResponse`
 * carries the updated `comments` array; invalidation ensures every
 * observer picks it up.
 */
export function useAppendComment() {
  const qc = useQueryClient();
  return useMutation<
    IssueViewResponse,
    ApiError,
    { missionId: number; issueId: number; body: AppendCommentInput }
  >({
    mutationFn: ({ issueId, body }) => api.appendComment(issueId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({
        queryKey: queryKeys.mission.issuesByMission(vars.missionId),
      });
    },
  });
}