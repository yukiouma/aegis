import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api, type ApiError, type Identity, type RegisterUserInput, type RegisterUserResponse, type UserView } from "../api";
import { queryKeys } from "./queryKeys";

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

/**
 * Domain identity for the register flow. Disabled by default;
 * the register page drives the lookup manually via `refetch()` so
 * the fetch happens once, on demand, after the user lands on
 * `/register`. Inherits the global `staleTime: Infinity` because
 * the consumer never re-mounts.
 */
export function useDomainUserInfo(options?: { enabled?: boolean }) {
  return useQuery<Identity, ApiError>({
    queryKey: queryKeys.user.domainIdentity(),
    queryFn: () => api.getDomainUserInfo(),
    enabled: options?.enabled ?? false,
  });
}

/**
 * Register mutation. No cache to invalidate — the user lands on
 * `/login` next, where login-status is re-probed by `bootstrap.ts`.
 */
export function useRegisterUser() {
  return useMutation<RegisterUserResponse, ApiError, RegisterUserInput>({
    mutationFn: (input) => api.registerUser(input),
  });
}

/**
 * Logout mutation. Clears the entire cache so no stale user data
 * leaks across the auth boundary — including the login-status probe
 * cache entry that `useLogin` / `useLoginDomain` invalidate.
 */
export function useLogout() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, void>({
    mutationFn: () => api.logout(),
    onSuccess: () => qc.clear(),
  });
}