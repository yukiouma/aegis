import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";

import { api, type ApiError, type Identity, type RegisterUserInput, type RegisterUserResponse, type UpdateUserBody, type UserView } from "../api";
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
 * cache entry that `useLogin` / `useLoginDomain` invalidate. Also
 * closes every project workspace window (label-prefixed `project:`)
 * BEFORE clearing the cache, so workspace pages can't issue stale
 * fetches against a logged-out session.
 */
export function useLogout() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, void>({
    mutationFn: () => api.logout(),
    onSuccess: async () => {
      // Close every project workspace window BEFORE clearing the
      // cache. Workspace windows have their own React tree and their
      // own query client — closing them first means the cached data
      // is never read again, and there is no ordering window during
      // which a workspace page could issue a stale fetch.
      //
      // The window enumeration is wrapped in try/catch so logout
      // still succeeds in environments where Tauri isn't available
      // (jsdom tests, future non-Tauri hosts): cache clearing is the
      // critical side effect, window cleanup is best-effort.
      try {
        const all = await getAllWebviewWindows();
        await Promise.all(
          all
            .filter((w) => w.label.startsWith("project:"))
            .map((w) => w.close()),
        );
      } catch {
        // Swallow — see comment above.
      }
      qc.clear();
    },
  });
}

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