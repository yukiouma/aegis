import { useMutation, useQueryClient } from "@tanstack/react-query";

import { api, type ApiError } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

// Login mutation. On success, invalidates the login-status probe so
// the auth-gated layout re-derives its auth state on the next render.
// The transport throws a structured `ApiError` on failure, which
// surfaces unchanged through `mutation.error`.
export function useLogin() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, { code: string; password: string }>({
    mutationFn: (vars) => api.login(vars.code, vars.password),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}

// Domain-account login mutation. Same invalidation contract as
// `useLogin` — the post-login render path is identical regardless
// of which method landed the user.
export function useLoginDomain() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, void>({
    mutationFn: () => api.loginDomain(),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.auth.loginStatus() }),
  });
}