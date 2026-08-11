import { ReactNode } from "react";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render, type RenderOptions } from "@testing-library/react";
// NOTE: this import is temporarily commented out because routeTree.gen.ts
// does not exist yet. Restore it in Task 3 Step 5 once the route files are
// created.
// import { routeTree } from "../routes/routeTree.gen";

interface RenderInRouterOptions extends Omit<RenderOptions, "wrapper"> {
  initialEntries?: string[];
}

/**
 * Render a component in a minimal router at "/" — for testing a single page
 * in isolation (no real layout, no Sidebar). Use `renderWithFullRouter` to
 * exercise the full `__root.tsx` layout and navigation.
 */
export function renderInRouter(
  ui: ReactNode,
  { initialEntries = ["/"], ...renderOptions }: RenderInRouterOptions = {},
) {
  const history = createMemoryHistory({ initialEntries });

  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });

  const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <>{ui}</>,
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([indexRoute]),
    history,
  });

  return {
    ...render(<RouterProvider router={router} />, renderOptions),
    router,
  };
}

interface RenderWithFullRouterOptions extends Omit<RenderOptions, "wrapper"> {
  initialEntries?: string[];
}

// Workaround body for Task 1 — restored in Task 3 Step 5 once routeTree.gen.ts exists.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function _placeholder(initialEntries: string[] = ["/"]) {
  const history = createMemoryHistory({ initialEntries });
  // const router = createRouter({ routeTree, history });
  const router = createRouter({
    routeTree: createRootRoute({ component: () => null }),
    history,
  });
  return router;
}

/**
 * Render the full app routeTree (including `__root.tsx` layout) with an
 * in-memory history. Use this for tests that exercise the Sidebar, layout,
 * or navigation between real routes.
 */
export function renderWithFullRouter({
  initialEntries = ["/"],
  ...renderOptions
}: RenderWithFullRouterOptions = {}) {
  const router = _placeholder(initialEntries);
  return {
    ...render(<RouterProvider router={router} />, renderOptions),
    router,
  };
}
