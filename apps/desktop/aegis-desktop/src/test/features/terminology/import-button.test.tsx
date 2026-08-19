import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { afterEach, describe, expect, it } from "vitest";

import { ImportButton } from
  "../../../features/terminology/components/ImportButton";

afterEach(() => cleanup());

async function renderButton(kind: "sdtm" | "adam", initialEntry: string) {
  const history = createMemoryHistory({ initialEntries: [initialEntry] });
  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });
  const pageRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/terminology/$kind",
    component: () => (
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ImportButton kind={kind} />
        </AegisI18nProvider>
      </AegisThemeProvider>
    ),
  });
  // Register the import destination so `navigate({ to: "/terminology/import" })`
  // type-checks and resolves to a real route. Mirror the production
  // route's search schema so `location.search.kind` is typed correctly.
  const importRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/terminology/import",
    component: () => <div data-testid="import-page" />,
    validateSearch: (raw): { kind?: "sdtm" | "adam" } => ({
      kind: raw.kind === "sdtm" || raw.kind === "adam" ? raw.kind : undefined,
    }),
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([pageRoute, importRoute]),
    history,
  });
  await act(async () => {
    await router.load();
  });
  render(<RouterProvider router={router} />);
  return router;
}

describe("ImportButton", () => {
  it("navigates to the import page with ?kind=sdtm when clicked on the SDTM page", async () => {
    const router = await renderButton("sdtm", "/terminology/sdtm");
    await userEvent.click(
      screen.getByRole("button", { name: /import terminology/i }),
    );
    const loc = router.state.location;
    expect(loc.pathname).toBe("/terminology/import");
    expect(loc.search.kind).toBe("sdtm");
  });

  it("navigates with ?kind=adam when clicked on the ADaM page", async () => {
    const router = await renderButton("adam", "/terminology/adam");
    await userEvent.click(
      screen.getByRole("button", { name: /import terminology/i }),
    );
    const loc = router.state.location;
    expect(loc.pathname).toBe("/terminology/import");
    expect(loc.search.kind).toBe("adam");
  });
});