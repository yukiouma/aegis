import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateProjectInput,
  type ProjectView,
  type UpdateProjectBody,
} from "../api";
import { queryKeys } from "./queryKeys";

/**
 * All projects. `staleTime: 0` overrides the global `Infinity` so the
 * list is treated as immediately stale — when the user navigates
 * away from `/projects` and comes back, the re-mount refetches via
 * the default `refetchOnMount: true`. Cached data is still rendered
 * first (no loading flicker); the refresh runs in the background.
 */
export function useListProjects() {
  return useQuery<ProjectView[], ApiError>({
    queryKey: queryKeys.project.all(),
    queryFn: () => api.listProjects(),
    staleTime: 0,
  });
}

/**
 * Single project by code. Manual-trigger (matches `useDomainUserInfo`
 * in `user.ts`) — `enabled: false` by default; the drawer drives
 * the fetch with `refetch()` so opening the edit drawer fires a
 * fresh read without auto-firing on every mount. `staleTime: 0`
 * keeps the read always-fresh before edit.
 */
export function useProject(code: string | null) {
  return useQuery<ProjectView, ApiError>({
    queryKey:
      code === null
        ? ["project", "byCode", "__disabled__"]
        : queryKeys.project.byCode(code),
    queryFn: () => {
      if (code === null) throw new Error("useProject disabled");
      return api.getProjectByCode(code);
    },
    enabled: false,
    staleTime: 0,
  });
}

/**
 * Create project. On success: invalidates the project list cache so
 * the next render shows the new row. Does NOT clear the cache
 * (unlike logout) — the current user is unaffected.
 */
export function useCreateProject() {
  const qc = useQueryClient();
  return useMutation<ProjectView, ApiError, CreateProjectInput>({
    mutationFn: (input) => api.createProject(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.project.all() }),
  });
}

/**
 * Update project. On success: invalidates the project list AND the
 * single-by-code entry for the updated row, so both the table and a
 * follow-up edit-open show the new values.
 */
export function useUpdateProject() {
  const qc = useQueryClient();
  return useMutation<
    ProjectView,
    ApiError,
    { code: string; body: UpdateProjectBody }
  >({
    mutationFn: ({ code, body }) => api.updateProject(code, body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.project.all() });
      qc.invalidateQueries({ queryKey: queryKeys.project.byCode(vars.code) });
    },
  });
}