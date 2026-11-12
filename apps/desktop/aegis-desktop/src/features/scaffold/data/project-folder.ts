import { useMutation } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type InitReport,
} from "../../../shared/api";

/**
 * Materialise a structure template into a project folder on disk.
 *
 * The command does real filesystem work on the desktop machine, so it
 * is not retried — a retry would re-run a partially-applied scaffold.
 */
export function useInitializeProjectFolder() {
  return useMutation<
    InitReport,
    ApiError,
    { structureId: number; study: string; branch?: string | null }
  >({
    mutationFn: (input) => api.initializeProjectFolder(input),
  });
}
