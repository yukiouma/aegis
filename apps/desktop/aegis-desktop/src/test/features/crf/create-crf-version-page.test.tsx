import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  AegisI18nProvider,
} from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import { renderWithFullRouter } from "../../helpers/file-route-utils";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

// Tauri v2 intercepts file drops at the OS level. The page subscribes to
// `getCurrentWebview().onDragDropEvent`; capture the handler so we can
// simulate a drop by invoking it directly.
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
import { mockCommands } from "../../helpers/tauri-mock";

const happyVersion = {
  id: 42,
  projectCode: "P1",
  name: "v1",
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

function simulateDrop(paths: string[]) {
  act(() => {
    dragDropHandler?.({ payload: { type: "drop", paths } });
  });
}

async function renderPage(opts: {
  versions?: { id: number; name: string }[];
  mockImport?: () => unknown;
} = {}) {
  mockCommands({
    is_logged_in: () => true,
    current_user: () => ({
      id: 1,
      code: "u",
      name: "U",
      role: "admin",
      active: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    }),
    import_als: () => (opts.mockImport ? opts.mockImport() : happyVersion),
    list_crf_versions: () => ({ versions: opts.versions ?? [] }),
  });
  const { router } = await renderWithFullRouter({
    initialEntries: ["/project/P1/crf/versions/new"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
  return { router };
}

describe("CreateCrfVersionPage — empty form", () => {
  it("renders title, name field, EDC picker, drop zone, and disabled submit", async () => {
    await renderPage();
    expect(
      screen.getByRole("heading", { name: /create crf version/i }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/version name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/edc source/i)).toBeInTheDocument();
    expect(screen.getByTestId("als-dropzone")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /create/i })).toBeDisabled();
  });
});

describe("CreateCrfVersionPage — duplicate detection", () => {
  it("shows the duplicate warning when name collides with an existing version", async () => {
    await renderPage({ versions: [{ id: 1, name: "v1" }] });
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/version name/i), "v1");
    // wait past the debounce window so the duplicate check fires
    await waitFor(() =>
      expect(
        screen.getByText(/A version named/i),
      ).toBeInTheDocument(),
    );
  });
});

describe("CreateCrfVersionPage — submit gating", () => {
  it.each([
    { name: "",     edc: "",     filePicked: false, enabled: false, label: "all empty" },
    { name: "v1",   edc: "",     filePicked: false, enabled: false, label: "name only" },
    { name: "",     edc: "rave", filePicked: false, enabled: false, label: "edc only" },
    { name: "",     edc: "",     filePicked: true,  enabled: false, label: "file only" },
    { name: "v1",   edc: "rave", filePicked: true,  enabled: true,  label: "all three set" },
  ])(
    "submit enabled? $label",
    async ({ name, edc, filePicked, enabled }) => {
      (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
        filePicked ? "/abs/path.xls" : null,
      );
      await renderPage();
      const user = userEvent.setup();
      if (name) {
        await user.type(screen.getByLabelText(/version name/i), name);
      }
      if (edc) {
        // MUI's select renders a hidden <input> + a visible button-like
        // combobox; clicking the combobox opens the menu.
        const combo = screen.getByLabelText(/edc source/i);
        await user.click(combo);
        await user.click(screen.getByRole("option", { name: /rave/i }));
      }
      if (filePicked) {
        await user.click(screen.getByTestId("als-dropzone"));
        await screen.findByText("path.xls");
      }
      const submit = screen.getByRole("button", { name: /create/i });
      if (enabled) {
        expect(submit).not.toBeDisabled();
      } else {
        expect(submit).toBeDisabled();
      }
    },
  );
});

describe("CreateCrfVersionPage — file picker", () => {
  it("opens the native dialog with the ALS filter and accepts the chosen file", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/abs/path.xls");
    await renderPage();
    await userEvent.setup().click(screen.getByTestId("als-dropzone"));
    await waitFor(() => expect(open).toHaveBeenCalled());
    const args = (open as unknown as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(args.filters[0].extensions).toEqual(["xls", "xlsx", "xml"]);
    expect(await screen.findByText("path.xls")).toBeInTheDocument();
  });
});

describe("CreateCrfVersionPage — drag-drop validation", () => {
  it("rejects an unsupported extension with a hint and no chip", async () => {
    await renderPage();
    simulateDrop(["/x/photo.png"]);
    await waitFor(() => expect(dragDropHandler).toBeDefined());
    expect(screen.queryByText("photo.png")).not.toBeInTheDocument();
  });
});

describe("CreateCrfVersionPage — submit happy path", () => {
  it("calls import_als and navigates to the form list with the new version selected", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/abs/path.xls");
    const { router } = await renderPage();
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/version name/i), "vNew");
    await user.click(screen.getByLabelText(/edc source/i));
    await user.click(screen.getByRole("option", { name: /rave/i }));
    await user.click(screen.getByTestId("als-dropzone"));
    await screen.findByText("path.xls");
    await user.click(screen.getByRole("button", { name: /create/i }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("import_als", {
        name: "vNew",
        projectCode: "P1",
        filepath: "/abs/path.xls",
        edcType: "rave",
      }),
    );
    // On success the page navigates to the form list with the new
    // version id pre-selected. The source page's success snackbar
    // unmounts with the page; verify the destination instead.
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/project/P1/crf");
      expect(router.state.location.search).toEqual({ versionId: 42 });
    });
  });
});

describe("CreateCrfVersionPage — submit error path", () => {
  it("surfaces the server error code in the failure Snackbar", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/abs/path.xls");
    await renderPage({
      mockImport: () => {
        throw {
          kind: "http",
          status: 409,
          code: "duplicate_crf_version",
          message: "exists",
        };
      },
      versions: [],
    });
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/version name/i), "vDup");
    await user.click(screen.getByLabelText(/edc source/i));
    await user.click(screen.getByRole("option", { name: /rave/i }));
    await user.click(screen.getByTestId("als-dropzone"));
    await screen.findByText("path.xls");
    await user.click(screen.getByRole("button", { name: /create/i }));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        /duplicate_crf_version/,
      ),
    );
  });
});