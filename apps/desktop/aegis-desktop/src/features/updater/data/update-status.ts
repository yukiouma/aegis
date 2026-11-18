import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api, type ApiError, type UpdateStatusDto } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * The current update status.
 *
 * `refetchInterval` is active only while a download is running: the
 * global client sets `staleTime: Infinity`, so without this the
 * progress bar would never move. Once the download settles the polling
 * stops and the status is refreshed only by the `aegis:update-ready`
 * event.
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
 * Subscribes to `aegis:update-ready` and invalidates the status query,
 * so a release that finishes downloading after the page mounted is
 * picked up. The dynamic import keeps the listener off the synchronous
 * startup path, the same way `useListenForSettingsChanges` does.
 */
export function useUpdateReadyEvent() {
  const qc = useQueryClient();
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen("aegis:update-ready", () => {
        void qc.invalidateQueries({ queryKey: queryKeys.updater.status() });
      });
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [qc]);
}
