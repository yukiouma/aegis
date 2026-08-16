import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";

const mockGetAll = vi.fn();
const mockFocus = vi.fn();
const mockShow = vi.fn();
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getAllWebviewWindows: (...args: unknown[]) => mockGetAll(...args),
}));

import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { ProjectWorkspaceLayout } from "../../../features/project-workspace/pages/ProjectWorkspaceLayout";

afterEach(() => cleanup());
beforeEach(() => {
  mockGetAll.mockReset();
  mockFocus.mockReset();
  mockShow.mockReset();
  mockFocus.mockResolvedValue(undefined);
  mockShow.mockResolvedValue(undefined);
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() {
      return 0;
    },
  });
});

async function renderLayout(initialPath: string) {
  const history = createMemoryHistory({ initialEntries: [initialPath] });

  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });

  // Parent dynamic segment routes the bare `/project/<code>` shape.
  const projectRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/project/$projectCode",
    component: ProjectWorkspaceLayout,
  });

  const dashboardRoute = createRoute({
    getParentRoute: () => projectRoute,
    path: "dashboard",
    component: () => <div>dashboard-slot</div>,
  });

  const configurationRoute = createRoute({
    getParentRoute: () => projectRoute,
    path: "configuration",
    component: () => <div>configuration-slot</div>,
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([
      projectRoute.addChildren([dashboardRoute, configurationRoute]),
    ]),
    history,
  });

  await act(async () => {
    await router.load();
  });

  const result = render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <RouterProvider router={router} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );

  return { ...result, router };
}

describe("ProjectWorkspaceLayout", () => {
  it("renders the projectCode as the sidebar title", async () => {
    await renderLayout("/project/DEMO-001/dashboard");
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByText("DEMO-001")).toBeInTheDocument();
  });

  it("renders Dashboard and Configuration menu entries", async () => {
    await renderLayout("/project/DEMO-001/dashboard");
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Configuration")).toBeInTheDocument();
  });

  it("renders the focus-main footer Button", async () => {
    await renderLayout("/project/DEMO-001/dashboard");
    expect(
      screen.getByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();
  });

  it("clicking the focus-main Button calls setFocus + show on the main window", async () => {
    mockGetAll.mockResolvedValue([
      { label: "main", setFocus: mockFocus, show: mockShow },
      { label: "project:DEMO-001", setFocus: vi.fn(), show: vi.fn() },
    ]);
    await renderLayout("/project/DEMO-001/dashboard");
    await userEvent.click(
      screen.getByRole("button", { name: /back to main/i }),
    );
    await waitFor(() => {
      expect(mockGetAll).toHaveBeenCalled();
      expect(mockFocus).toHaveBeenCalledTimes(1);
      expect(mockShow).toHaveBeenCalledTimes(1);
    });
  });

  it("does nothing when no main window is present", async () => {
    mockGetAll.mockResolvedValue([
      { label: "project:DEMO-001", setFocus: vi.fn(), show: vi.fn() },
    ]);
    await renderLayout("/project/DEMO-001/dashboard");
    await userEvent.click(
      screen.getByRole("button", { name: /back to main/i }),
    );
    await waitFor(() => expect(mockGetAll).toHaveBeenCalled());
    expect(mockFocus).not.toHaveBeenCalled();
    expect(mockShow).not.toHaveBeenCalled();
  });

  it("renders an icon-only footer button when the sidebar is collapsed", async () => {
    mockGetAll.mockResolvedValue([
      { label: "main", setFocus: mockFocus, show: mockShow },
    ]);
    await renderLayout("/project/DEMO-001/dashboard");

    // Sanity: expanded state shows the labelled Button.
    expect(
      screen.getByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();

    // Collapse the sidebar via the built-in toggle.
    await userEvent.click(
      screen.getByRole("button", { name: /toggle sidebar/i }),
    );

    // After collapse: still findable by the same accessible name (the
    // IconButton carries it), but the element is now an SVG-based
    // IconButton — verify by absence of the "Back to main" text.
    expect(screen.queryByText(/back to main/i)).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /back to main/i }),
    ).toBeInTheDocument();

    // Clicking it still focuses the main window.
    await userEvent.click(
      screen.getByRole("button", { name: /back to main/i }),
    );
    await waitFor(() => {
      expect(mockFocus).toHaveBeenCalledTimes(1);
      expect(mockShow).toHaveBeenCalledTimes(1);
    });
  });
});