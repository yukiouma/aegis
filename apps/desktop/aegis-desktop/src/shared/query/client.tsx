import {
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import React from "react";

// Default options rationale:
// - `staleTime: Infinity`: Tauri calls hit a local sidecar. There is no
//   network to mask; remounting the same query already triggers a fetch
//   via `useQuery`'s mount semantics. Keeps the devtools quiet.
// - `retry: false`: sidecar failures are real bugs, not transient. Bail.
// - `refetchOnWindowFocus / refetchOnReconnect: false`: same reasoning.
// Per-query overrides live in the hook files (e.g. `bootstrap.ts` pins
// `staleTime: 0` for health/login-status probes).
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      retry: false,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
    },
    mutations: { retry: false },
  },
});

export function QueryProvider({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
      {import.meta.env.DEV && (
        <ReactQueryDevtools initialIsOpen={false} buttonPosition="bottom-left" />
      )}
    </QueryClientProvider>
  );
}