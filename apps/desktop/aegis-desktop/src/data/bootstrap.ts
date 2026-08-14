import { useQuery } from "@tanstack/react-query";

import { api, type ApiError } from "../api";
import { queryKeys } from "./queryKeys";

/**
 * Health probe. Defaults to `enabled: false` because the bootstrap
 * page drives the call manually via `refetch()` — auto-firing on
 * mount would re-fetch every time React remounts the page.
 *
 * `staleTime: 0` opts out of the global `Infinity` default: even if a
 * future consumer flips `enabled` to true, the cached value is
 * treated as immediately stale and the next read hits the server.
 */
export function useHealthz(options?: { enabled?: boolean }) {
  return useQuery<string, ApiError>({
    queryKey: queryKeys.bootstrap.health(),
    queryFn: () => api.healthz(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}

/**
 * Login-status probe. Same manual-trigger contract as `useHealthz`.
 * Lives under `queryKeys.auth.*` (not `bootstrap.*`) because login
 * and logout mutations invalidate it — those are auth concerns.
 */
export function useIsLoggedIn(options?: { enabled?: boolean }) {
  return useQuery<boolean, ApiError>({
    queryKey: queryKeys.auth.loginStatus(),
    queryFn: () => api.isLoggedIn(),
    enabled: options?.enabled ?? false,
    staleTime: 0,
  });
}