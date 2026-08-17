import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { ProjectDrawer } from "../../../features/project-list/components/ProjectDrawer";
import type {
  ProductView,
  ProjectView,
  UpdateProjectBody,
  UserView,
} from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderInRouter } from "../../../test/helpers/file-route-utils";

const productFixture: ProductView = {
  id: 10,
  code: "prod-a",
  name: "Product A",
  description: "",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture2: UserView = {
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha description",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [],
    workers: [],
  },
  tags: [],
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderDrawer(
  mode: "closed" | "create" | "edit",
  code: string | null = null,
) {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ProjectDrawer mode={mode} code={code} onClose={vi.fn()} />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectDrawer — closed", () => {
  it("does not render any form fields when mode is 'closed'", async () => {
    await renderDrawer("closed");
    expect(screen.queryByLabelText(/\bcode\b/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/\bdescription\b/i)).not.toBeInTheDocument();
  });
});

describe("ProjectDrawer — create mode", () => {
  it("shows 'Create project' title and an enabled code field", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    expect(
      screen.getByRole("heading", { name: /create project/i }),
    ).toBeInTheDocument();
    const codeField = screen.getByLabelText(/\bcode\b/i);
    expect(codeField).toBeInTheDocument();
    expect(codeField).not.toBeDisabled();
  });

  it("does not show the active switch in create mode", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
  });

  it("disables Submit until code, description, and product are set", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    const submit = await screen.findByRole("button", { name: /^create$/i });
    expect(submit).toBeDisabled();
  });

  it("calls api.createProject with the assembled shape on Submit", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      create_project: () => projectFixture,
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");

    const productInput = screen.getByLabelText(/\bproduct\b/i);
    await userEvent.click(productInput);
    await userEvent.click(screen.getByRole("option", { name: /prod-a/i }));

    const memberLeadersInput = screen.getByLabelText(/^members\s*—\s*leaders$/i);
    await userEvent.click(memberLeadersInput);
    await userEvent.click(screen.getByRole("option", { name: /alice/i }));

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "create_project",
        expect.objectContaining({
          code: "newproj",
          description: "New desc",
          productId: 10,
          members: expect.objectContaining({
            leaders: expect.arrayContaining(["alice"]),
          }),
        }),
      );
    });
  });
});

describe("ProjectDrawer — edit mode", () => {
  it("fetches the project via get_project_by_code and pre-fills the form", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "alpha",
      });
    });

    const codeField = await screen.findByLabelText(/\bcode\b/i);
    expect(codeField).toBeDisabled();
    expect(codeField).toHaveValue("alpha");
    expect(screen.getByLabelText(/\bdescription\b/i)).toHaveValue(
      "Alpha description",
    );
    expect(screen.getByRole("switch")).toBeInTheDocument();
  });

  it("calls api.updateProject with { code, body } (no code in body) on Submit", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    const descriptionField = await screen.findByLabelText(/\bdescription\b/i);
    await waitFor(() =>
      expect(descriptionField).toHaveValue("Alpha description"),
    );

    await userEvent.clear(descriptionField);
    await userEvent.type(descriptionField, "Edited");

    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));

    const expectedBody: UpdateProjectBody = expect.objectContaining({
      description: "Edited",
      productId: 10,
      active: true,
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_project", {
        code: "alpha",
        body: expectedBody,
      });
    });
  });
});

describe("ProjectDrawer — mutation error", () => {
  it("shows an Alert with the error message when create_project fails", async () => {
    mockCommands({
      list_products: () => [productFixture],
      list_users: () => [userFixture, userFixture2],
      create_project: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");

    const productInput = screen.getByLabelText(/\bproduct\b/i);
    await userEvent.click(productInput);
    await userEvent.click(screen.getByRole("option", { name: /prod-a/i }));

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /server: boom/i,
    );
  });
});