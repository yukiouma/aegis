import { useQuery } from "@tanstack/react-query";

import { api, type ApiError, type UserView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * Current signed-in user. Fires on mount — `UserFooter` is the only
 * consumer and it renders inside `AppLayout` (which only mounts
 * post-auth), so the call always succeeds in production. There is
 * no `enabled` option: the call should always run when the sidebar
 * shows.
 */
export function useCurrentUser() {
  return useQuery<UserView, ApiError>({
    queryKey: queryKeys.user.current(),
    queryFn: () => api.getCurrentUser(),
  });
}