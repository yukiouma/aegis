import { useMutation } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type UpdateUserCredentialInput,
  type UserCredentialView,
} from "../../../shared/api";

/**
 * Update the current user's password. Wraps
 * `api.updateUserCredential` (the `update_user_credential` Tauri
 * command). No cache to invalidate — the SettingsPage calls
 * `useLogout()` immediately after a successful mutation, which clears
 * the entire cache and closes the auth session.
 */
export function useUpdatePassword() {
  return useMutation<UserCredentialView, ApiError, UpdateUserCredentialInput>({
    mutationFn: (input) => api.updateUserCredential(input),
  });
}
