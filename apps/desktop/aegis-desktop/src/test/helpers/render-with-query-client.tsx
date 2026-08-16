import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderOptions } from "@testing-library/react";
import type { ReactElement } from "react";

/**
 * Build a fresh `QueryClient` for one test. Caches must not bleed
 * between tests, so each render site gets its own unless the caller
 * passes one in via `options.client`.
 */
export function makeTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

/**
 * Render `ui` wrapped in a `QueryClientProvider`. Returns the standard
 * `@testing-library/react` render result plus the `client` so tests
 * can inspect cache state and spy on methods like `invalidateQueries`.
 */
export function renderWithQueryClient(
  ui: ReactElement,
  options?: { client?: QueryClient } & RenderOptions,
) {
  const client = options?.client ?? makeTestQueryClient();
  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { ...render(ui, { wrapper: Wrapper, ...options }), client };
}