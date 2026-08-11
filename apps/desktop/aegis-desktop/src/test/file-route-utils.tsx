import { ReactNode } from "react";
import { act } from "react";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render, type RenderOptions } from "@testing-library/react";
import { routeTree } from "../routes/routeTree.gen";

interface RenderInRouterOptions extends Omit<RenderOptions, "wrapper"> {
  initialEntries?: string[];
}

/**
 * Render a component in a minimal router at "/" — for testing a single page
 * in isolation (no real layout, no Sidebar). Use `renderWithFullRouter` to
 * exercise the full `__root.tsx` layout and navigation.
 */
export async function renderInRouter(
  ui: ReactNode,
  { initialEntries = ["/"], ...renderOptions }: RenderInRouterOptions = {},
) {
  const history = createMemoryHistory({ initialEntries });

  const Page = () => <>{ui}</>;

  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });

  const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: Page,
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([indexRoute]),
    history,
  });

  // Trigger an initial match and wait for it to settle before rendering.
  // router.load() is async because it goes through the data-loading pipeline;
  // awaiting it (inside `act` so React flushes the resulting state update)
  // ensures the matched route's component has rendered into the DOM by the
  // time the caller makes assertions.
  await act(async () => {
    await router.load();
  });

  const result = render(<RouterProvider router={router} />, renderOptions);

  return {
    ...result,
    router,
  };
}

interface RenderWithFullRouterOptions extends RenderOptions {
  initialEntries?: string[];
}

/**
 * Render the full app routeTree (including `__root.tsx` layout) with an
 * in-memory history. Use this for tests that exercise the Sidebar, layout,
 * or navigation between real routes.
 */
export async function renderWithFullRouter({
  initialEntries = ["/"],
  ...renderOptions
}: RenderWithFullRouterOptions = {}) {
  const history = createMemoryHistory({ initialEntries });
  const router = createRouter({ routeTree, history });

  await act(async () => {
    await router.load();
  });

  const result = render(<RouterProvider router={router} />, renderOptions);

  return {
    ...result,
    router,
  };
}
