import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ProjectTable } from "../../../features/project-list/components/ProjectTable";
import type { ApiError, ProjectView } from "../../../shared/api";

const baseRow: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  tags: [],
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

afterEach(() => cleanup());

function renderTable(props: {
  rows?: ProjectView[];
  loading?: boolean;
  error?: ApiError | null;
  canEdit?: boolean;
  onOpenCreate?: () => void;
  onOpenEdit?: (code: string) => void;
  onOpenWorkspace?: (code: string) => void;
} = {}) {
  const onOpenCreate = props.onOpenCreate ?? vi.fn();
  const onOpenEdit = props.onOpenEdit ?? vi.fn();
  const onOpenWorkspace = props.onOpenWorkspace ?? vi.fn();
  return {
    onOpenCreate,
    onOpenEdit,
    onOpenWorkspace,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectTable
            rows={props.rows ?? [baseRow]}
            loading={props.loading ?? false}
            error={props.error ?? null}
            canEdit={props.canEdit ?? true}
            onOpenCreate={onOpenCreate}
            onOpenEdit={onOpenEdit}
            onOpenWorkspace={onOpenWorkspace}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}

describe("ProjectTable — column rendering", () => {
  it("renders all five column headers", () => {
    renderTable();
    expect(screen.getByRole("columnheader", { name: /^code$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^description$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^leaders$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^status$/i })).toBeInTheDocument();
  });

  it("renders a code and description cell for the row", () => {
    renderTable();
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("Alpha project")).toBeInTheDocument();
  });

  it("renders an outlined chip for members.leaders and a filled chip for unblindMembers.leaders", () => {
    renderTable();
    const aliceChip = screen.getByText("Alice").closest(".MuiChip-root");
    const bobChip = screen.getByText("Bob").closest(".MuiChip-root");
    expect(aliceChip).toHaveClass("MuiChip-outlined");
    expect(bobChip).toHaveClass("MuiChip-filled");
  });

  it("renders an em-dash when both leader arrays are empty", () => {
    renderTable({
      rows: [
        {
          ...baseRow,
          members: { leaders: [], workers: [] },
          unblindMembers: { leaders: [], workers: [] },
        },
      ],
    });
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("renders a CheckCircle icon for active=true", () => {
    renderTable();
    expect(screen.getByTestId("CheckCircleIcon")).toBeInTheDocument();
  });

  it("renders a Cancel icon for active=false", () => {
    renderTable({ rows: [{ ...baseRow, active: false }] });
    expect(screen.getByTestId("CancelIcon")).toBeInTheDocument();
  });
});

describe("ProjectTable — operation column role gating", () => {
  it("renders Add, Edit, and OpenInNew when canEdit=true", () => {
    renderTable({ canEdit: true });
    expect(screen.getByRole("button", { name: /add project/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /edit project/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /open project/i })).toBeInTheDocument();
  });

  it("hides Add and Edit when canEdit=false but still renders OpenInNew enabled", () => {
    renderTable({ canEdit: false });
    expect(screen.queryByRole("button", { name: /add project/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /edit project/i })).not.toBeInTheDocument();
    const openBtn = screen.getByRole("button", { name: /open project/i });
    expect(openBtn).toBeInTheDocument();
    expect(openBtn).not.toBeDisabled();
  });

  it("calls onOpenCreate when Add is clicked", async () => {
    const { onOpenCreate } = renderTable({ canEdit: true });
    await userEvent.click(screen.getByRole("button", { name: /add project/i }));
    expect(onOpenCreate).toHaveBeenCalledTimes(1);
  });

  it("calls onOpenEdit(row.code) when Edit is clicked", async () => {
    const { onOpenEdit } = renderTable({ canEdit: true });
    await userEvent.click(screen.getByRole("button", { name: /edit project/i }));
    expect(onOpenEdit).toHaveBeenCalledWith("alpha");
  });
});

describe("ProjectTable — OpenInNew workspace action", () => {
  it("calls onOpenWorkspace(row.code) when OpenInNew is clicked", async () => {
    const { onOpenWorkspace } = renderTable();
    await userEvent.click(screen.getByRole("button", { name: /open project/i }));
    expect(onOpenWorkspace).toHaveBeenCalledTimes(1);
    expect(onOpenWorkspace).toHaveBeenCalledWith("alpha");
  });

  it("OpenInNew is enabled regardless of canEdit", () => {
    renderTable({ canEdit: false });
    const openBtn = screen.getByRole("button", { name: /open project/i });
    expect(openBtn).not.toBeDisabled();
  });
});

describe("ProjectTable — empty / loading / error", () => {
  it("shows an empty-state message when rows is empty", () => {
    renderTable({ rows: [] });
    expect(screen.getByText(/no projects yet/i)).toBeInTheDocument();
  });

  it("shows an error alert when error is set", () => {
    renderTable({
      rows: [],
      error: {
        kind: "http",
        status: 500,
        code: "server",
        message: "boom",
      },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/failed to load projects/i);
  });
});