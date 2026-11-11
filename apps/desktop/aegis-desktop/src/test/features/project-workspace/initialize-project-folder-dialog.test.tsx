import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";

import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { InitializeProjectFolderDialog } from "../../../features/project-workspace/components/InitializeProjectFolderDialog";

const TEMPLATES = [
  {
    id: 1,
    name: "Standard Project Folder Structure",
    rootpath: "/templates/standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
  {
    id: 2,
    name: "Minimal",
    rootpath: "/templates/minimal",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

/** What the command answers: two created items, one skip. */
const REPORT = {
  root: "D:\\studies\\ak001\\001",
  created: [
    "D:\\studies\\ak001\\001\\01_raw",
    "D:\\studies\\ak001\\001\\01_raw\\spec.docx",
  ],
  skipped: [
    {
      path: "D:\\studies\\ak001\\001\\02_clean",
      reason: "alreadyExists",
      detail: null,
    },
  ],
};

function wrapperFor(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <AegisThemeProvider>
        <AegisI18nProvider>
          <QueryClientProvider client={client}>{children}</QueryClientProvider>
        </AegisI18nProvider>
      </AegisThemeProvider>
    );
  };
}

function renderDialog(overrides: {
  onClose?: () => void;
  onInitialized?: (report: unknown) => void;
} = {}) {
  const onClose = overrides.onClose ?? vi.fn();
  const onInitialized = overrides.onInitialized ?? vi.fn();
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  render(
    <InitializeProjectFolderDialog
      open
      projectCode="AK001-001"
      onClose={onClose}
      onInitialized={onInitialized}
    />,
    { wrapper: wrapperFor(client) },
  );
  return { onClose, onInitialized };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd === "list_structures") return Promise.resolve(TEMPLATES);
    if (cmd === "initialize_project_folder") return Promise.resolve(REPORT);
    return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
  });
});
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

describe("InitializeProjectFolderDialog", () => {
  it("lists the template structures", async () => {
    renderDialog();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("list_structures", {
        level: "template",
      }),
    );
    await userEvent.click(screen.getByRole("combobox"));
    expect(
      await screen.findByRole("option", {
        name: "Standard Project Folder Structure",
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Minimal" })).toBeInTheDocument();
  });

  it("keeps confirm disabled until a structure is selected", async () => {
    renderDialog();
    const confirm = await screen.findByRole("button", { name: /confirm/i });
    expect(confirm).toBeDisabled();

    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    expect(screen.getByRole("button", { name: /confirm/i })).toBeEnabled();
  });

  it("runs the command and shows the report", async () => {
    const { onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("initialize_project_folder", {
        structureId: 2,
        study: "AK001-001",
        branch: null,
      }),
    );
    expect(screen.getByTestId("initialize-report-created")).toHaveTextContent(
      "2",
    );
    expect(screen.getByTestId("initialize-report-skipped")).toHaveTextContent(
      "1",
    );
    // Each skipped node is listed with its path and the reason.
    expect(
      screen.getByText("D:\\studies\\ak001\\001\\02_clean"),
    ).toBeInTheDocument();
    expect(screen.getByText(/already exists/i)).toBeInTheDocument();
    expect(onInitialized).toHaveBeenCalledWith(REPORT);
  });

  it("keeps the dialog open and shows the error when the command fails", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "list_structures") return Promise.resolve(TEMPLATES);
      if (cmd === "initialize_project_folder") {
        return Promise.reject({ kind: "network", message: "boom" });
      }
      return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
    });
    renderDialog();
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(await screen.findByText(/boom/)).toBeInTheDocument();
    // Still open — the user can retry or cancel.
    expect(screen.getByRole("combobox")).toBeInTheDocument();
  });

  it("cancels without running the command", async () => {
    const { onClose, onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
    expect(onInitialized).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith(
      "initialize_project_folder",
      expect.anything(),
    );
  });
});
