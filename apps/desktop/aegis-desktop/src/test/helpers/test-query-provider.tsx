import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

/**
 * Test-only `QueryProvider` that creates a fresh `QueryClient` on
 * every render. Use this in route/page tests so cached entries from
 * earlier tests do not bleed into later ones — the production
 * `QueryProvider` (in `src/data/client.tsx`) is a singleton over a
 * module-level client, which would share cache across tests in the
 * same run.
 */
export function TestQueryProvider({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}