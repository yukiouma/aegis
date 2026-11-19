import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api, type ApiError, type UpdateStatusDto } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * The current update status.
 *
 * `refetchInterval` is active only while a download is running: the
 * global client sets `staleTime: Infinity`, so without this the
 * progress bar would never move. The download has to be *seen* first —
 * the status event below is what brings the frontend from "idle" to
 * "downloading" — and once the download settles the polling stops.
 */
export function useUpdateStatus() {
  return useQuery<UpdateStatusDto, ApiError>({
    queryKey: queryKeys.updater.status(),
    queryFn: () => api.updateStatus(),
    refetchInterval: (query) =>
      query.state.data?.status.kind === "downloading" ? 2000 : false,
  });
}

/**
 * Runs a check now. The manual trigger behind the Settings button; it
 * bypasses the hourly cadence by design.
 */
export function useCheckForUpdates() {
  const qc = useQueryClient();
  return useMutation<UpdateStatusDto, ApiError, void>({
    mutationFn: () => api.checkForUpdates(),
    onSuccess: (dto) => {
      qc.setQueryData(queryKeys.updater.status(), dto);
    },
  });
}

/**
 * Installs the downloaded release. On Windows the process exits inside
 * the call, so this promise never resolves there.
 */
export function useInstallUpdate() {
  return useMutation<void, ApiError, void>({
    mutationFn: () => api.installReadyUpdate(),
  });
}

/**
 * Subscribes to `aegis:update-status` and invalidates the status
 * query, so a change the frontend did not ask for still reaches it: a
 * download that starts after the page mounted, a check that failed in
 * the background, the release that is ready to install.
 *
 * The event carries the new phase and nothing else — the status itself
 * is read through the command, so there is one source of truth and the
 * two cannot disagree. The dynamic import keeps the listener off the
 * synchronous startup path, the same way `useListenForSettingsChanges`
 * does.
 */
export function useUpdateStatusEvent() {
  const qc = useQueryClient();
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen("aegis:update-status", () => {
          void qc.invalidateQueries({ queryKey: queryKeys.updater.status() });
        });
      } catch {
        // A failed subscription is not fatal: the status is still read
        // when the query mounts, and the manual check still refreshes
        // it. Only the push is lost.
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [qc]);
}
