// Barrel for the React Query layer. Pages should import from this
// module rather than reaching into individual files, so a future
// resource split (e.g. splitting `user.ts` into `user.ts` +
// `projectMembers.ts`) can happen without touching pages.

export { QueryProvider } from "./client";
export { queryKeys } from "./queryKeys";

export { useLogin, useLoginDomain } from "./auth";
export { useHealthz, useIsLoggedIn } from "./bootstrap";
export {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useRegisterUser,
  useLogout,
  useUpdateUser,
} from "./user";
export {
  useCreateProject,
  useListProjects,
  useProject,
  useUpdateProject,
} from "./project";
export { useListProducts } from "./product";

// Re-export the React Query primitive that pages may need for ad-hoc
// cache interactions (e.g. `queryClient.setQueryData`).
export { useQueryClient } from "@tanstack/react-query";