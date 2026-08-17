import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { ProjectListPage } from "../../../features/project-list/pages/ProjectListPage";
import type { ProjectView, UserView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderInRouter } from "../../../test/helpers/file-route-utils";

const projectA: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "alice", name: "Alice" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  tags: [{ key: "Product", value: "DEMO-001" }],
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectB: ProjectView = {
  id: 2,
  code: "beta",
  description: "Beta project",
  members: { leaders: [], workers: [] },
  unblindMembers: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  tags: [{ key: "Product", value: "OTHER-002" }],
  active: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectC: ProjectView = {
  ...projectA,
  id: 3,
  code: "gamma",
  description: "Gamma project",
  members: { leaders: [{ code: "zoe", name: "Zoe" }], workers: [] },
  unblindMembers: { leaders: [], workers: [] },
  tags: [{ key: "Client", value: "ACME" }],
};

const adminUser: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const generalUser: UserView = {
  ...adminUser,
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderPage(user: UserView, projects: ProjectView[]) {
  mockCommands({
    current_user: () => user,
    list_projects: () => projects,
  });
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ProjectListPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectListPage — basic rendering", () => {
  it("renders one row per project", async () => {
    await renderPage(adminUser, [projectA, projectB]);
    expect(await screen.findByText("alpha")).toBeInTheDocument();
    expect(await screen.findByText("beta")).toBeInTheDocument();
  });
});

describe("ProjectListPage — search filter", () => {
  it("filters rows by code (case-insensitive)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "BET");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("beta")).toBeInTheDocument();
    });
  });

  it("filters rows by description", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "gamma project");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("gamma")).toBeInTheDocument();
    });
  });

  it("filters rows by leader code/name", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "zoe");
    await waitFor(() => {
      expect(screen.queryByText("alpha")).not.toBeInTheDocument();
      expect(screen.getByText("gamma")).toBeInTheDocument();
    });
  });
});

describe("ProjectListPage — Involve filter", () => {
  it("shows only projects where the current user is in any members array when Involve is checked", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.getByText("beta")).toBeInTheDocument();
      expect(screen.queryByText("gamma")).not.toBeInTheDocument();
    });
  });

  it("search AND Involve combine (commutative order)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/search/i), "alpha");
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.queryByText("beta")).not.toBeInTheDocument();
      expect(screen.queryByText("gamma")).not.toBeInTheDocument();
    });
  });
});

describe("ProjectListPage — role gating", () => {
  it("shows the Add button for admin", async () => {
    await renderPage(adminUser, [projectA]);
    expect(
      await screen.findByRole("button", { name: /add project/i }),
    ).toBeInTheDocument();
  });

  it("hides the Add button for general users", async () => {
    await renderPage(generalUser, [projectA]);
    await screen.findByText("alpha");
    expect(
      screen.queryByRole("button", { name: /add project/i }),
    ).not.toBeInTheDocument();
  });

  it("opens the drawer with mode='create' when Add is clicked", async () => {
    await renderPage(adminUser, [projectA]);
    await userEvent.click(
      await screen.findByRole("button", { name: /add project/i }),
    );
    expect(
      await screen.findByRole("heading", { name: /create project/i }),
    ).toBeInTheDocument();
  });
});

describe("ProjectListPage — tag filter", () => {
  it("filters rows by tag value substring (case-insensitive)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "demo");
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.queryByText("beta")).not.toBeInTheDocument();
      expect(screen.queryByText("gamma")).not.toBeInTheDocument();
    });
  });

  it("leaves all rows visible when tag filter is empty", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    expect(screen.getByText("beta")).toBeInTheDocument();
    expect(screen.getByText("gamma")).toBeInTheDocument();
  });

  it("combines tag filter with the existing search filter (AND)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "demo");
    await userEvent.type(screen.getByLabelText(/search/i), "ALPHA");
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.queryByText("beta")).not.toBeInTheDocument();
    });
  });
});