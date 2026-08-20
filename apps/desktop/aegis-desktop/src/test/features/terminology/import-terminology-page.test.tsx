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
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

// Tauri v2 intercepts file drops at the OS level, so the DOM `drop` event
// never fires inside the webview. The page subscribes to
// `getCurrentWebview().onDragDropEvent`; we capture that handler here so
// the tests can simulate a drop by invoking it directly.
let dragDropHandler:
  | ((event: { payload: { type: string; paths: string[] } }) => void)
  | undefined;
const dragDropUnlisten = vi.fn();
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (
      handler: (event: { payload: { type: string; paths: string[] } }) => void,
    ) => {
      dragDropHandler = handler;
      return Promise.resolve(dragDropUnlisten);
    },
  }),
}));

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ImportTerminologyPage } from
  "../../../features/terminology/pages/ImportTerminologyPage";
import { mockCommands } from "../../helpers/tauri-mock";

function simulateDrop(paths: string[]) {
  act(() => {
    dragDropHandler?.({ payload: { type: "drop", paths } });
  });
}

const versionView = {
  id: 42,
  kind: "sdtm" as const,
  name: "2026-03-27",
  createdAt: "2026-03-27T00:00:00Z",
  updatedAt: "2026-03-27T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  (open as unknown as ReturnType<typeof vi.fn>).mockReset();
  dragDropHandler = undefined;
  dragDropUnlisten.mockReset();
});
afterEach(() => cleanup());

async function renderPage(opts: { initialEntries?: string[]; mockImport?: () => unknown } = {}) {
  mockCommands({
    import_terminology: () =>
      opts.mockImport ? opts.mockImport() : versionView,
  });
  const Page = () => (
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ImportTerminologyPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>
  );

  // Render with a router that mounts the page at /terminology/import so
  // `useSearch({ strict: false })` actually has a matched route to read
  // the `?kind=` value from. The shared `renderInRouter` helper only
  // registers `/`, which produces a "Not Found" body.
  const history = createMemoryHistory({
    initialEntries: opts.initialEntries ?? ["/terminology/import"],
  });
  const rootRoute = createRootRoute({
    component: () => <Outlet />,
  });
  const importRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/terminology/import",
    component: Page,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([importRoute]),
    history,
  });
  await act(async () => {
    await router.load();
  });
  return render(<RouterProvider router={router} />);
}

describe("ImportTerminologyPage — empty form", () => {
  it("renders the back arrow, title, ButtonGroup, drop zone, and disabled submit", async () => {
    await renderPage();
    expect(screen.getByRole("button", { name: /back/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /import terminology/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "SDTM" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "ADaM" })).toBeInTheDocument();
    expect(screen.getByText(/drop an \.xls or \.xlsx file here/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
  });
});

describe("ImportTerminologyPage — kind pre-selection", () => {
  it("pre-selects SDTM when ?kind=sdtm is in the URL", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    expect(screen.getByRole("button", { name: "SDTM" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "ADaM" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });
});

describe("ImportTerminologyPage — file picker", () => {
  it("calls open() with the right filter and stores the resolved path", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await waitFor(() => {
      expect(open).toHaveBeenCalledWith({
        multiple: false,
        filters: [{ name: "Excel", extensions: ["xls", "xlsx"] }],
      });
    });
    await waitFor(() => {
      expect(screen.getByText("sdtm.xls")).toBeInTheDocument();
    });
  });
});

describe("ImportTerminologyPage — drop validation", () => {
  it("rejects a .pdf drop with a flash hint and no state change", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    simulateDrop(["/Users/me/Downloads/report.pdf"]);
    await waitFor(() => {
      expect(screen.getByText(/only \.xls or \.xlsx files are supported/i)).toBeInTheDocument();
    });
    expect(screen.queryByText("report.pdf")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
  });

  it("accepts a .xlsx drop and shows the basename", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    simulateDrop(["/Users/me/Downloads/sdtm.xlsx"]);
    await waitFor(() => {
      expect(screen.getByText("sdtm.xlsx")).toBeInTheDocument();
    });
  });
});

describe("ImportTerminologyPage — submit", () => {
  it("invokes import_terminology, hides the form, then shows the success Snackbar on resolve", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("sdtm.xls");
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("import_terminology", {
        kind: "sdtm",
        filepath: "/tmp/sdtm.xls",
      });
    });
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/imported terminology version 2026-03-27/i);
    });
  });

  it("switches kind before submit so the API call uses the user’s final choice", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/adam.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("adam.xls");
    await userEvent.click(screen.getByRole("button", { name: "ADaM" }));
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("import_terminology", {
        kind: "adam",
        filepath: "/tmp/adam.xls",
      });
    });
  });

  it("shows the error Snackbar when the API rejects with Http 409", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({
      initialEntries: ["/terminology/import?kind=sdtm"],
      mockImport: () => {
        throw { kind: "http", status: 409, code: "duplicate", message: "exists" };
      },
    });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("sdtm.xls");
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/import failed/i);
    });
  });
});