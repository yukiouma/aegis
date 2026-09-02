import { useCallback, useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type UpdateUserBody,
  type UserView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * All users. Consumed by the drawer's member pickers. Default
 * `enabled: true` because the drawer is the only consumer and only
 * opens for root/admin, where the call always succeeds in practice.
 */
export function useListUsers(options?: { enabled?: boolean }) {
  return useQuery<UserView[], ApiError>({
    queryKey: queryKeys.user.list(),
    queryFn: () => api.listUsers(),
    enabled: options?.enabled ?? true,
  });
}

/**
 * Resolve a `userCode` to its display `name` using the cached
 * `useListUsers` query. Falls back to the userCode itself when the
 * list has not loaded yet, or the user is not in the list (e.g. the
 * user was deactivated after the mission was created).
 *
 * The lookup is intentional: assignees carry `userCode` on the wire
 * (`AssigneeViewResponse` has no `name`), and the UI needs a
 * human-readable label without changing the API.
 */
export function useUserNameMap() {
  const usersQuery = useListUsers();
  const map = useMemo(
    () => new Map(usersQuery.data?.map((u) => [u.code, u.name] as const)),
    [usersQuery.data],
  );
  return useCallback(
    (userCode: string) => map.get(userCode) ?? userCode,
    [map],
  );
}

/**
 * Update an existing user. On success: invalidates the user list cache
 * so the management page reflects the new active state on the next
 * render. Also invalidates `user.current()` since the current user's
 * own row could be the one being updated by a sibling admin and the
 * `UserFooter` reads the same cache entry.
 */
export function useUpdateUser() {
  const qc = useQueryClient();
  return useMutation<
    UserView,
    ApiError,
    { code: string; body: UpdateUserBody }
  >({
    mutationFn: ({ code, body }) => api.updateUser(code, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.user.list() });
      qc.invalidateQueries({ queryKey: queryKeys.user.current() });
    },
  });
}