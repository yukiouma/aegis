import { useMutation, useQueryClient } from "@tanstack/react-query";
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";

import { api, type ApiError } from "../../../shared/api";

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