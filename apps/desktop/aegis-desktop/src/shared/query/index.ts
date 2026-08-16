// Barrel for the shared query layer. Re-exports the central React Query
// wiring (client + key factory) so feature barrels can import from a single
// path without reaching into individual files.

export { QueryProvider, queryClient } from "./client";
export { queryKeys } from "./keys";
export { useQueryClient } from "@tanstack/react-query";