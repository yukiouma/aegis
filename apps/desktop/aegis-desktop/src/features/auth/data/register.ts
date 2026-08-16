import { useMutation, useQuery } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type Identity,
  type RegisterUserInput,
  type RegisterUserResponse,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

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